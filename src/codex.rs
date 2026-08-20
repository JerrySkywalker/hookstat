//! Codex hook discovery and opt-in instrumentation planning.
//!
//! Discovery reads only supported local configuration layers and returns
//! fingerprints/counts, never raw command text. Apply/restore are intentionally
//! limited to explicit `hooks.json` fixture/config paths; inline TOML,
//! plugin-provided, and managed sources are discovered or reported as coverage
//! limitations but never modified optimistically.

use crate::domain::{ExecutionMode, HandlerIdentity, HookEvent};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

const MARKER: &str = "--hookstat-instrumentation-v1";
const MANIFEST_SCHEMA_VERSION: u8 = 1;
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfigFormat {
    HooksJson,
    InlineToml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    UserHooksJson,
    ProjectHooksJson,
    UserConfigToml,
    ProjectConfigToml,
}
impl SourceKind {
    fn label(self) -> &'static str {
        match self {
            Self::UserHooksJson => "user_hooks_json",
            Self::ProjectHooksJson => "project_hooks_json",
            Self::UserConfigToml => "user_config_toml",
            Self::ProjectConfigToml => "project_config_toml",
        }
    }
    fn format(self) -> ConfigFormat {
        match self {
            Self::UserHooksJson | Self::ProjectHooksJson => ConfigFormat::HooksJson,
            Self::UserConfigToml | Self::ProjectConfigToml => ConfigFormat::InlineToml,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentationDisposition {
    Instrumentable,
    AlreadyInstrumented,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiscoveredHandler {
    pub handler: HandlerIdentity,
    pub source_kind: SourceKind,
    pub disposition: InstrumentationDisposition,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct DiscoverySummary {
    pub discovered: usize,
    pub instrumentable: usize,
    pub already_instrumented: usize,
    pub unsupported_or_uninstrumentable: usize,
    pub coverage_consequences: Vec<String>,
    pub trust_consequences: Vec<String>,
    pub handlers: Vec<DiscoveredHandler>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatedHandler {
    discovered: DiscoveredHandler,
    path: PathBuf,
    config_hash: String,
    group_index: usize,
    handler_index: usize,
    command: String,
    command_windows: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Discovery {
    pub summary: DiscoverySummary,
    handlers: Vec<LocatedHandler>,
}

/// Sanitized runtime-effective view returned by Codex App Server `hooks/list`.
/// It deliberately does not retain or serialize command strings, source paths,
/// plugin identifiers, or raw matcher expressions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectiveHandler {
    pub handler: HandlerIdentity,
    pub source_class: String,
    pub handler_type: String,
    pub enabled: Option<bool>,
    pub trusted: Option<bool>,
    pub managed: Option<bool>,
    pub disposition: InstrumentationDisposition,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct EffectiveDiscoverySummary {
    pub discovered: usize,
    pub command_handlers: usize,
    pub instrumentable: usize,
    pub unsupported_or_uninstrumentable: usize,
    pub source_class_counts: BTreeMap<String, usize>,
    pub sync_count: usize,
    pub async_count: usize,
    pub execution_mode_unknown_count: usize,
    pub handlers: Vec<EffectiveHandler>,
}

#[derive(Clone, Debug)]
pub struct EffectiveDiscovery {
    pub summary: EffectiveDiscoverySummary,
    reconciliation_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct DiscoveryReconciliation {
    pub static_handlers: usize,
    pub effective_handlers: usize,
    pub matched_handlers: usize,
    pub static_only: usize,
    pub effective_only: usize,
    pub coverage_classification: String,
}

#[derive(Clone, Debug)]
pub struct ReconciledDiscovery {
    pub static_discovery: Discovery,
    pub effective_discovery: EffectiveDiscovery,
    pub reconciliation: DiscoveryReconciliation,
}

#[derive(Clone, Debug, Serialize)]
pub struct DryRunReport {
    pub static_discovery: DiscoverySummary,
    pub effective_runtime: Option<EffectiveDiscoverySummary>,
    pub reconciliation: Option<DiscoveryReconciliation>,
    pub effective_runtime_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyManifest {
    pub schema_version: u8,
    pub config_path_fingerprint: String,
    pub original_config_sha256: String,
    pub handlers: BTreeMap<String, ProxyHandler>,
}

/// This private local control-plane material stores original commands only so a
/// user-authorized restore can be exact. It is never written to receipts,
/// SQLite, reports, or repository fixtures containing owner data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProxyHandler {
    pub handler: HandlerIdentity,
    pub command: String,
    pub command_windows: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Journal {
    original_config_sha256: String,
    applied_config_sha256: String,
    backup_file: String,
    manifest_file: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ApplySummary {
    pub applied: usize,
    pub already_instrumented: usize,
    pub unsupported: usize,
    pub trust_review_required: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RestoreSummary {
    pub restored: usize,
    pub already_restored: usize,
    pub drift_detected: usize,
}

#[derive(Debug)]
pub enum CodexError {
    Io(io::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    Invalid(&'static str),
    DriftDetected,
    AppServerUnavailable,
    AppServerTimeout,
    AppServerProtocol,
}
impl fmt::Display for CodexError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => output.write_str("Codex instrumentation filesystem operation failed"),
            Self::Json(_) | Self::Toml(_) => {
                output.write_str("Codex hook configuration is malformed")
            }
            Self::Invalid(field) => write!(output, "Codex hook configuration has invalid {field}"),
            Self::DriftDetected => output
                .write_str("Codex hook configuration drift detected; no modification was made"),
            Self::AppServerUnavailable => {
                output.write_str("Codex App Server effective hook discovery is unavailable")
            }
            Self::AppServerTimeout => {
                output.write_str("Codex App Server effective hook discovery timed out")
            }
            Self::AppServerProtocol => {
                output.write_str("Codex App Server returned an unusable hook-discovery response")
            }
        }
    }
}
impl std::error::Error for CodexError {}
impl From<io::Error> for CodexError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for CodexError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl From<toml::de::Error> for CodexError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}

/// Uses only the documented user/project config locations. Plugins and managed
/// layers are deliberately not enumerated by filesystem guessing; Codex-native
/// trust/config ownership remains authoritative for those sources.
pub fn discover_default() -> Result<Discovery, CodexError> {
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|value| PathBuf::from(value).join(".codex"))
        })
        .or_else(|| std::env::var_os("HOME").map(|value| PathBuf::from(value).join(".codex")))
        .ok_or(CodexError::Invalid("Codex home"))?;
    discover_paths(&[
        codex_home.join("hooks.json"),
        codex_home.join("config.toml"),
        std::env::current_dir()
            .map_err(CodexError::Io)?
            .join(".codex/hooks.json"),
        std::env::current_dir()
            .map_err(CodexError::Io)?
            .join(".codex/config.toml"),
    ])
}

/// Uses the official read-only App Server `hooks/list` surface. The child is
/// terminated after its single response; no thread, session, trust, or config
/// write request is ever sent. Raw response values are parsed in-memory only
/// and immediately reduced to the privacy-preserving structures above.
pub fn discover_effective(cwd: &Path) -> Result<EffectiveDiscovery, CodexError> {
    let mut child = app_server_command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| CodexError::AppServerUnavailable)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(CodexError::AppServerUnavailable)?;
    let mut stdin = child.stdin.take().ok_or(CodexError::AppServerUnavailable)?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                let _ = sender.send(value);
            }
        }
    });
    let response = (|| {
        let initialize = serde_json::json!({
            "method": "initialize",
            "id": 1,
            "params": {"clientInfo": {"name": "hookstat", "version": env!("CARGO_PKG_VERSION")}}
        });
        send_app_server(&mut stdin, &initialize)?;
        receive_response(&receiver, 1)?;
        send_app_server(
            &mut stdin,
            &serde_json::json!({"method": "initialized", "params": {}}),
        )?;
        send_app_server(
            &mut stdin,
            &serde_json::json!({"method": "hooks/list", "id": 2, "params": {"cwds": [cwd]}}),
        )?;
        receive_response(&receiver, 2)
    })();
    drop(stdin);
    // App Server is a long-lived protocol host. The single read-only request
    // is complete, so terminate this temporary child instead of waiting for a
    // server lifetime that is unrelated to discovery.
    let _ = child.kill();
    let _ = child.wait();
    let response = response?;
    parse_effective_response(&response)
}

#[cfg(windows)]
fn app_server_command() -> Command {
    // npm commonly exposes Codex as a .cmd shim. Resolve it explicitly from
    // PATH and let Rust's Windows `Command` support invoke the batch shim; this
    // avoids an extra cmd.exe layer that can interfere with JSONL pipes. This
    // is still the ordinary `codex app-server` command, not a HookStat launcher
    // wrapper.
    let path_entries = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let shim = path_entries
        .iter()
        .map(|path| path.join("codex.cmd"))
        .find(|path| path.is_file())
        .or_else(|| {
            path_entries
                .iter()
                .map(|path| path.join("codex.bat"))
                .find(|path| path.is_file())
        });
    let mut command = Command::new(shim.unwrap_or_else(|| PathBuf::from("codex")));
    command.arg("app-server");
    command
}

#[cfg(not(windows))]
fn app_server_command() -> Command {
    let mut command = Command::new("codex");
    command.arg("app-server");
    command
}

pub fn discover_reconciled_default() -> Result<ReconciledDiscovery, CodexError> {
    let static_discovery = discover_default()?;
    let effective_discovery =
        discover_effective(&std::env::current_dir().map_err(CodexError::Io)?)?;
    let static_keys = static_discovery
        .handlers
        .iter()
        .map(|item| {
            runtime_location_key(
                &item.path,
                item.discovered.handler.event,
                item.group_index,
                item.handler_index,
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    let effective_keys = effective_discovery
        .reconciliation_keys
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let matched_handlers = static_keys.intersection(&effective_keys).count();
    let static_only = static_keys.difference(&effective_keys).count();
    let effective_only = effective_keys.difference(&static_keys).count();
    let coverage_classification = if static_only == 0 && effective_only == 0 {
        "complete_effective_coverage".to_owned()
    } else if static_only == 0 {
        "pass_with_explicit_unsupported_runtime_coverage".to_owned()
    } else {
        "partial_reconciliation".to_owned()
    };
    Ok(ReconciledDiscovery {
        static_discovery,
        effective_discovery,
        reconciliation: DiscoveryReconciliation {
            static_handlers: static_keys.len(),
            effective_handlers: effective_keys.len(),
            matched_handlers,
            static_only,
            effective_only,
            coverage_classification,
        },
    })
}

pub fn default_dry_run() -> Result<DryRunReport, CodexError> {
    match discover_reconciled_default() {
        Ok(value) => Ok(DryRunReport {
            static_discovery: value.static_discovery.summary,
            effective_runtime: Some(value.effective_discovery.summary),
            reconciliation: Some(value.reconciliation),
            effective_runtime_error: None,
        }),
        Err(
            error @ (CodexError::AppServerUnavailable
            | CodexError::AppServerTimeout
            | CodexError::AppServerProtocol),
        ) => Ok(DryRunReport {
            static_discovery: discover_default()?.summary,
            effective_runtime: None,
            reconciliation: None,
            effective_runtime_error: Some(error.to_string()),
        }),
        Err(error) => Err(error),
    }
}

fn send_app_server(stdin: &mut impl Write, value: &Value) -> Result<(), CodexError> {
    serde_json::to_writer(&mut *stdin, value).map_err(CodexError::Json)?;
    stdin.write_all(b"\n").map_err(CodexError::Io)?;
    stdin.flush().map_err(CodexError::Io)
}

fn receive_response(receiver: &mpsc::Receiver<Value>, id: u64) -> Result<Value, CodexError> {
    let deadline = Duration::from_secs(12);
    loop {
        let value = receiver
            .recv_timeout(deadline)
            .map_err(|_| CodexError::AppServerTimeout)?;
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            if value.get("error").is_some() {
                return Err(CodexError::AppServerProtocol);
            }
            return Ok(value);
        }
    }
}

fn parse_effective_response(response: &Value) -> Result<EffectiveDiscovery, CodexError> {
    let contexts = response
        .get("result")
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .ok_or(CodexError::AppServerProtocol)?;
    let mut summary = EffectiveDiscoverySummary::default();
    let mut reconciliation_keys = Vec::new();
    for context in contexts {
        let Some(hooks) = context.get("hooks").and_then(Value::as_array) else {
            continue;
        };
        for item in hooks {
            let Some(raw_key) = item.get("key").and_then(Value::as_str) else {
                continue;
            };
            let event = item
                .get("eventName")
                .and_then(Value::as_str)
                .and_then(parse_event)
                .ok_or(CodexError::AppServerProtocol)?;
            let raw_matcher = item
                .get("matcher")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let matcher_identity = if raw_matcher.is_empty() {
                "any".to_owned()
            } else {
                format!("m_{}", short_hash(raw_matcher.as_bytes()))
            };
            let source_class = effective_source_class(item.get("source").and_then(Value::as_str));
            let raw_type = item
                .get("handlerType")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let handler_type = if raw_type.eq_ignore_ascii_case("command") {
                "command"
            } else {
                "other"
            }
            .to_owned();
            let enabled = item.get("enabled").and_then(Value::as_bool);
            let managed = item.get("isManaged").and_then(Value::as_bool);
            let trusted = trust_status(item.get("trustStatus").and_then(Value::as_str), managed);
            let execution_mode = match item.get("async").and_then(Value::as_bool) {
                Some(true) => ExecutionMode::Async,
                Some(false) => ExecutionMode::Sync,
                None => ExecutionMode::Unknown,
            };
            let display_order = item
                .get("displayOrder")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let structural_identity =
                format!("runtime:{display_order}:{}", short_hash(raw_key.as_bytes()));
            let handler_key = format!("hk_{}", short_hash(raw_key.as_bytes()));
            let raw_revision = item
                .get("currentHash")
                .and_then(Value::as_str)
                .unwrap_or(raw_key);
            let handler = HandlerIdentity {
                key: handler_key.clone(),
                revision: format!("hr_{}", short_hash(raw_revision.as_bytes())),
                label: format!("Codex / {} / {}", event.label(), &handler_key[3..]),
                source_kind: format!("runtime_{source_class}"),
                event,
                matcher_identity,
                structural_identity,
                execution_mode,
            };
            let (disposition, reason) =
                effective_disposition(&source_class, &handler_type, enabled, trusted, managed);
            summary.discovered += 1;
            if handler_type == "command" {
                summary.command_handlers += 1;
            }
            *summary
                .source_class_counts
                .entry(source_class.clone())
                .or_default() += 1;
            match execution_mode {
                ExecutionMode::Sync => summary.sync_count += 1,
                ExecutionMode::Async => summary.async_count += 1,
                ExecutionMode::Unknown => summary.execution_mode_unknown_count += 1,
            }
            match disposition {
                InstrumentationDisposition::Instrumentable => summary.instrumentable += 1,
                InstrumentationDisposition::AlreadyInstrumented => {
                    summary.unsupported_or_uninstrumentable += 1
                }
                InstrumentationDisposition::Unsupported => {
                    summary.unsupported_or_uninstrumentable += 1
                }
            }
            summary.handlers.push(EffectiveHandler {
                handler,
                source_class,
                handler_type,
                enabled,
                trusted,
                managed,
                disposition,
                reason,
            });
            reconciliation_keys.push(effective_location_key(item, raw_key, event));
        }
    }
    summary
        .handlers
        .sort_by(|left, right| left.handler.key.cmp(&right.handler.key));
    Ok(EffectiveDiscovery {
        summary,
        reconciliation_keys,
    })
}

fn effective_location_key(item: &Value, raw_key: &str, event: HookEvent) -> String {
    let parts = raw_key.split(':').collect::<Vec<_>>();
    let group_index = parts
        .get(parts.len().saturating_sub(2))
        .and_then(|value| value.parse::<usize>().ok());
    let handler_index = parts.last().and_then(|value| value.parse::<usize>().ok());
    match (
        item.get("sourcePath").and_then(Value::as_str),
        group_index,
        handler_index,
    ) {
        (Some(path), Some(group_index), Some(handler_index)) => {
            runtime_location_key(Path::new(path), event, group_index, handler_index)
        }
        _ => short_hash(raw_key.as_bytes()),
    }
}

fn effective_source_class(value: Option<&str>) -> String {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "user" => "user".into(),
        "project" => "project".into(),
        "plugin" => "plugin".into(),
        "managed" => "managed".into(),
        _ => "other".into(),
    }
}

fn trust_status(value: Option<&str>, managed: Option<bool>) -> Option<bool> {
    if managed == Some(true) {
        return Some(true);
    }
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "trusted" | "not_required" => Some(true),
        "untrusted" | "modified" => Some(false),
        _ => None,
    }
}

fn effective_disposition(
    source_class: &str,
    handler_type: &str,
    enabled: Option<bool>,
    trusted: Option<bool>,
    managed: Option<bool>,
) -> (InstrumentationDisposition, Option<String>) {
    if handler_type != "command" {
        return (
            InstrumentationDisposition::Unsupported,
            Some("runtime handler is not an executable command handler".into()),
        );
    }
    if managed == Some(true) || matches!(source_class, "plugin" | "managed") {
        return (
            InstrumentationDisposition::Unsupported,
            Some(
                "managed or plugin runtime source is visible but HookStat never mutates it".into(),
            ),
        );
    }
    if enabled != Some(true) {
        return (
            InstrumentationDisposition::Unsupported,
            Some("runtime handler is disabled or its enabled state is unavailable".into()),
        );
    }
    if trusted != Some(true) {
        return (
            InstrumentationDisposition::Unsupported,
            Some("runtime handler is not currently trusted or trust is unavailable".into()),
        );
    }
    if !matches!(source_class, "user" | "project") {
        return (
            InstrumentationDisposition::Unsupported,
            Some("runtime source has no safe HookStat mutation path".into()),
        );
    }
    (InstrumentationDisposition::Instrumentable, None)
}

pub fn discover_paths(paths: &[PathBuf]) -> Result<Discovery, CodexError> {
    let mut located = Vec::new();
    for path in paths {
        if path.exists() {
            located.extend(discover_one(path)?);
        }
    }
    let mut summary = DiscoverySummary::default();
    for item in &located {
        summary.discovered += 1;
        match item.discovered.disposition {
            InstrumentationDisposition::Instrumentable => summary.instrumentable += 1,
            InstrumentationDisposition::AlreadyInstrumented => summary.already_instrumented += 1,
            InstrumentationDisposition::Unsupported => summary.unsupported_or_uninstrumentable += 1,
        }
        summary.handlers.push(item.discovered.clone());
    }
    summary
        .handlers
        .sort_by(|left, right| left.handler.key.cmp(&right.handler.key));
    summary.coverage_consequences.push("Only handlers in supported hooks.json layers can be instrumented; inline TOML, plugin, and managed sources remain unsupported coverage.".into());
    if summary.unsupported_or_uninstrumentable > 0 {
        summary.coverage_consequences.push(
            "Unsupported handlers are not treated as zero-failure or healthy coverage.".into(),
        );
    }
    if summary.instrumentable > 0 {
        summary.trust_consequences.push("Wrapping changes hook commands and may require a Codex trust review; HookStat never approves or edits trust.".into());
    }
    Ok(Discovery {
        summary,
        handlers: located,
    })
}

fn discover_one(path: &Path) -> Result<Vec<LocatedHandler>, CodexError> {
    let source_kind = source_kind(path);
    let raw = fs::read(path)?;
    let config_hash = sha256(&raw);
    let root = match source_kind.format() {
        ConfigFormat::HooksJson => serde_json::from_slice(&raw)?,
        ConfigFormat::InlineToml => {
            let value = toml::from_str::<toml::Value>(&String::from_utf8_lossy(&raw))?;
            serde_json::to_value(value).map_err(CodexError::Json)?
        }
    };
    let Some(hooks) = root.get("hooks").and_then(Value::as_object) else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for (event_name, groups) in hooks {
        let Some(event) = parse_event(event_name) else {
            continue;
        };
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (group_index, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(Value::as_str).unwrap_or("");
            let matcher_identity = if matcher.is_empty() {
                "any".to_owned()
            } else {
                format!("m_{}", short_hash(matcher.as_bytes()))
            };
            let Some(handlers) = group.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for (handler_index, handler) in handlers.iter().enumerate() {
                let kind = handler.get("type").and_then(Value::as_str).unwrap_or("");
                let command = handler.get("command").and_then(Value::as_str).unwrap_or("");
                let command_windows = handler
                    .get("commandWindows")
                    .or_else(|| handler.get("command_windows"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let execution_mode = if handler
                    .get("async")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    && event != HookEvent::SessionEnd
                {
                    ExecutionMode::Async
                } else {
                    ExecutionMode::Sync
                };
                let structural_identity = format!("g{group_index}:h{handler_index}");
                // Codex's effective `hooks/list` key is positional. Hash the
                // equivalent local location instead of the command so static
                // and runtime discovery can reconcile without exposing either
                // source paths or command text.
                let handler_key = format!(
                    "hk_{}",
                    runtime_location_key(path, event, group_index, handler_index)
                );
                let revision = format!("hr_{}", short_hash(canonical_json(handler).as_bytes()));
                let identity = HandlerIdentity {
                    key: handler_key.clone(),
                    revision,
                    label: format!("Codex / {} / {}", event.label(), &handler_key[3..]),
                    source_kind: source_kind.label().into(),
                    event,
                    matcher_identity: matcher_identity.clone(),
                    structural_identity,
                    execution_mode,
                };
                let (disposition, reason) = if source_kind.format() == ConfigFormat::InlineToml {
                    (InstrumentationDisposition::Unsupported, Some("inline TOML is discovered but not mutated because byte-preserving semantic restore is not yet proven".into()))
                } else if kind != "command" || command.trim().is_empty() {
                    (
                        InstrumentationDisposition::Unsupported,
                        Some("only executable command handlers are instrumentable".into()),
                    )
                } else if command.contains(MARKER) {
                    (InstrumentationDisposition::AlreadyInstrumented, None)
                } else {
                    (InstrumentationDisposition::Instrumentable, None)
                };
                result.push(LocatedHandler {
                    discovered: DiscoveredHandler {
                        handler: identity,
                        source_kind,
                        disposition,
                        reason,
                    },
                    path: path.to_path_buf(),
                    config_hash: config_hash.clone(),
                    group_index,
                    handler_index,
                    command: command.to_owned(),
                    command_windows,
                });
            }
        }
    }
    Ok(result)
}

fn source_kind(path: &Path) -> SourceKind {
    let file = path
        .file_name()
        .and_then(|item| item.to_str())
        .unwrap_or_default();
    let user_root = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE").map(|value| PathBuf::from(value).join(".codex"))
        })
        .or_else(|| std::env::var_os("HOME").map(|value| PathBuf::from(value).join(".codex")));
    let project = user_root
        .as_deref()
        .is_none_or(|root| path.parent() != Some(root));
    match (file, project) {
        ("hooks.json", true) => SourceKind::ProjectHooksJson,
        ("hooks.json", false) => SourceKind::UserHooksJson,
        ("config.toml", true) => SourceKind::ProjectConfigToml,
        _ => SourceKind::UserConfigToml,
    }
}

/// Apply only a discovery generated from the same explicit config path. The
/// caller provides HookStat's own data root; this routine never resolves or
/// mutates a default owner configuration implicitly.
pub fn apply(
    discovery: &Discovery,
    data_root: &Path,
    proxy_executable: &Path,
) -> Result<ApplySummary, CodexError> {
    fs::create_dir_all(data_root.join("backups"))?;
    fs::create_dir_all(data_root.join("manifests"))?;
    fs::create_dir_all(data_root.join("journals"))?;
    let mut by_path: BTreeMap<&Path, Vec<&LocatedHandler>> = BTreeMap::new();
    for item in &discovery.handlers {
        by_path.entry(item.path.as_path()).or_default().push(item);
    }
    let mut summary = ApplySummary::default();
    for (path, items) in by_path {
        let instrumentable = items
            .iter()
            .filter(|item| {
                item.discovered.disposition == InstrumentationDisposition::Instrumentable
            })
            .collect::<Vec<_>>();
        summary.already_instrumented += items
            .iter()
            .filter(|item| {
                item.discovered.disposition == InstrumentationDisposition::AlreadyInstrumented
            })
            .count();
        summary.unsupported += items
            .iter()
            .filter(|item| item.discovered.disposition == InstrumentationDisposition::Unsupported)
            .count();
        if instrumentable.is_empty() {
            continue;
        }
        if instrumentable
            .iter()
            .any(|item| item.discovered.source_kind.format() != ConfigFormat::HooksJson)
        {
            summary.unsupported += instrumentable.len();
            continue;
        }
        let original = fs::read(path)?;
        if sha256(&original) != instrumentable[0].config_hash {
            return Err(CodexError::DriftDetected);
        }
        let path_fingerprint = short_hash(path.to_string_lossy().as_bytes());
        let backup_file = format!("{path_fingerprint}-{}.backup", short_hash(&original));
        let backup_path = data_root.join("backups").join(&backup_file);
        if !backup_path.exists() {
            atomic_bytes(&backup_path, &original)?;
        }
        let mut root: Value = serde_json::from_slice(&original)?;
        let mut manifest = ProxyManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            config_path_fingerprint: path_fingerprint.clone(),
            original_config_sha256: sha256(&original),
            handlers: BTreeMap::new(),
        };
        let manifest_file = format!("{path_fingerprint}.json");
        let manifest_path = data_root.join("manifests").join(&manifest_file);
        for item in instrumentable {
            manifest.handlers.insert(
                item.discovered.handler.key.clone(),
                ProxyHandler {
                    handler: item.discovered.handler.clone(),
                    command: item.command.clone(),
                    command_windows: item.command_windows.clone(),
                },
            );
            let handler = root
                .get_mut("hooks")
                .and_then(Value::as_object_mut)
                .and_then(|hooks| hooks.get_mut(item.discovered.handler.event.label()))
                .and_then(Value::as_array_mut)
                .and_then(|groups| groups.get_mut(item.group_index))
                .and_then(|group| group.get_mut("hooks"))
                .and_then(Value::as_array_mut)
                .and_then(|handlers| handlers.get_mut(item.handler_index))
                .ok_or(CodexError::DriftDetected)?;
            let command = proxy_command(
                proxy_executable,
                &manifest_path,
                &item.discovered.handler.key,
            );
            handler["command"] = Value::String(command.clone());
            if handler.get("commandWindows").is_some() {
                handler["commandWindows"] = Value::String(command);
            }
        }
        atomic_json(&manifest_path, &manifest)?;
        let applied = serde_json::to_vec_pretty(&root)?;
        atomic_bytes(path, &applied)?;
        let journal = Journal {
            original_config_sha256: sha256(&original),
            applied_config_sha256: sha256(&applied),
            backup_file,
            manifest_file,
        };
        atomic_json(
            &data_root
                .join("journals")
                .join(format!("{path_fingerprint}.json")),
            &journal,
        )?;
        summary.applied += manifest.handlers.len();
        summary.trust_review_required = true;
    }
    Ok(summary)
}

pub fn restore(config_path: &Path, data_root: &Path) -> Result<RestoreSummary, CodexError> {
    let path_fingerprint = short_hash(config_path.to_string_lossy().as_bytes());
    let journal_path = data_root
        .join("journals")
        .join(format!("{path_fingerprint}.json"));
    if !journal_path.exists() {
        return Ok(RestoreSummary {
            already_restored: 1,
            ..RestoreSummary::default()
        });
    }
    let journal: Journal = serde_json::from_slice(&fs::read(&journal_path)?)?;
    let current = fs::read(config_path)?;
    if sha256(&current) != journal.applied_config_sha256 {
        return Ok(RestoreSummary {
            drift_detected: 1,
            ..RestoreSummary::default()
        });
    }
    let backup = fs::read(data_root.join("backups").join(&journal.backup_file))?;
    if sha256(&backup) != journal.original_config_sha256 {
        return Err(CodexError::DriftDetected);
    }
    atomic_bytes(config_path, &backup)?;
    fs::remove_file(journal_path)?;
    Ok(RestoreSummary {
        restored: 1,
        ..RestoreSummary::default()
    })
}

pub fn load_manifest(path: &Path) -> Result<ProxyManifest, CodexError> {
    let manifest: ProxyManifest = serde_json::from_slice(&fs::read(path)?)?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(CodexError::Invalid("manifest schema"));
    }
    Ok(manifest)
}

pub fn default_data_root() -> Result<PathBuf, CodexError> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|value| PathBuf::from(value).join(".local/share"))
            })
    }
    .ok_or(CodexError::Invalid("user data directory"))?;
    Ok(base.join("HookStat"))
}

fn parse_event(value: &str) -> Option<HookEvent> {
    match value {
        "SessionStart" | "session_start" | "sessionStart" => Some(HookEvent::SessionStart),
        "SessionEnd" | "session_end" | "sessionEnd" => Some(HookEvent::SessionEnd),
        "UserPromptSubmit" | "user_prompt_submit" | "userPromptSubmit" => {
            Some(HookEvent::UserPromptSubmit)
        }
        "PreToolUse" | "pre_tool_use" | "preToolUse" => Some(HookEvent::PreToolUse),
        "PostToolUse" | "post_tool_use" | "postToolUse" => Some(HookEvent::PostToolUse),
        "PermissionRequest" | "permission_request" | "permissionRequest" => {
            Some(HookEvent::PermissionRequest)
        }
        "PreCompact" | "pre_compact" | "preCompact" => Some(HookEvent::PreCompact),
        "PostCompact" | "post_compact" | "postCompact" => Some(HookEvent::PostCompact),
        "Stop" | "stop" => Some(HookEvent::Stop),
        "SubagentStart" | "subagent_start" | "subagentStart" => Some(HookEvent::SubagentStart),
        "SubagentStop" | "subagent_stop" | "subagentStop" => Some(HookEvent::SubagentStop),
        _ => None,
    }
}
fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn short_hash(bytes: &[u8]) -> String {
    sha256(bytes)[..16].to_owned()
}
fn runtime_location_key(
    path: &Path,
    event: HookEvent,
    group_index: usize,
    handler_index: usize,
) -> String {
    let path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    short_hash(
        format!(
            "{path}:{}:{group_index}:{handler_index}",
            event.as_storage()
        )
        .as_bytes(),
    )
}
fn proxy_command(exe: &Path, manifest: &Path, handler: &str) -> String {
    format!(
        "\"{}\" codex proxy --manifest \"{}\" --handler \"{}\" {MARKER}",
        quote_component(exe),
        quote_component(manifest),
        handler
    )
}
fn quote_component(path: &Path) -> String {
    path.to_string_lossy().replace('"', "\\\"")
}
fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), CodexError> {
    atomic_bytes(path, &serde_json::to_vec_pretty(value)?)
}
fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), CodexError> {
    let parent = path.parent().ok_or(CodexError::Invalid("path"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".hookstat-write-{}.tmp",
        WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn fixture(path: &Path) {
        fs::write(path, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"echo private fixture value","timeout":4},{"type":"command","command":"echo second","async":true}]}],"PreToolUse":[{"matcher":"^Bash$","hooks":[{"type":"command","command":"echo deny","commandWindows":"echo deny-windows","statusMessage":"check","additionalContextLimit":500}]}]}}"#).unwrap();
    }
    #[test]
    fn discovery_keeps_two_handlers_on_same_event_distinct_and_private() {
        let temp = tempdir().unwrap();
        let config = temp.path().join("hooks.json");
        fixture(&config);
        let plan = discover_paths(&[config]).unwrap();
        assert_eq!(plan.summary.discovered, 3);
        assert_eq!(plan.summary.instrumentable, 3);
        assert_ne!(
            plan.summary.handlers[0].handler.key,
            plan.summary.handlers[1].handler.key
        );
        let json = serde_json::to_string(&plan.summary).unwrap();
        assert!(!json.contains("private fixture value"));
    }
    #[test]
    fn apply_restore_is_atomic_idempotent_and_detects_drift() {
        let temp = tempdir().unwrap();
        let config = temp.path().join("hooks.json");
        fixture(&config);
        let original = fs::read(&config).unwrap();
        let discovery = discover_paths(std::slice::from_ref(&config)).unwrap();
        let state = temp.path().join("state");
        let first = apply(&discovery, &state, Path::new("hookstat-test")).unwrap();
        assert_eq!(first.applied, 3);
        assert!(first.trust_review_required);
        let after = fs::read(&config).unwrap();
        assert_ne!(after, original);
        let second_plan = discover_paths(std::slice::from_ref(&config)).unwrap();
        assert_eq!(second_plan.summary.already_instrumented, 3);
        assert_eq!(
            apply(&second_plan, &state, Path::new("hookstat-test"))
                .unwrap()
                .already_instrumented,
            3
        );
        assert_eq!(restore(&config, &state).unwrap().restored, 1);
        assert_eq!(fs::read(&config).unwrap(), original);
        assert_eq!(restore(&config, &state).unwrap().already_restored, 1);
        fixture(&config);
        let drift_plan = discover_paths(std::slice::from_ref(&config)).unwrap();
        apply(&drift_plan, &state, Path::new("hookstat-test")).unwrap();
        fs::write(&config, b"{}\n").unwrap();
        assert_eq!(restore(&config, &state).unwrap().drift_detected, 1);
    }
    #[test]
    fn inline_toml_is_read_only_unsupported_coverage() {
        let temp = tempdir().unwrap();
        let config = temp.path().join("config.toml");
        fs::write(
            &config,
            "[[hooks.Stop]]\n[[hooks.Stop.hooks]]\ntype = 'command'\ncommand = 'echo fixture'\n",
        )
        .unwrap();
        let plan = discover_paths(&[config]).unwrap();
        assert_eq!(plan.summary.unsupported_or_uninstrumentable, 1);
    }
    #[test]
    fn effective_runtime_discovery_is_sanitized_and_keeps_plugin_coverage_explicit() {
        let response = serde_json::json!({"id": 2, "result": {"data": [{"hooks": [
            {"key": "runtime:user:stop:0:0", "sourcePath":"C:/private/source/path", "eventName": "stop", "handlerType": "command", "command": "private command", "matcher": "private matcher", "source": "user", "enabled": true, "isManaged": false, "trustStatus": "trusted", "currentHash": "private-current-hash", "displayOrder": 0, "async": false},
            {"key": "private/plugin/path:stop:0:1", "eventName": "stop", "handlerType": "command", "command": "private plugin command", "source": "plugin", "enabled": true, "isManaged": false, "trustStatus": "trusted", "displayOrder": 1}
        ]}]}});
        let parsed = parse_effective_response(&response).unwrap();
        assert_eq!(parsed.summary.discovered, 2);
        assert_eq!(parsed.summary.instrumentable, 1);
        assert_eq!(parsed.summary.unsupported_or_uninstrumentable, 1);
        assert_eq!(parsed.summary.source_class_counts.get("plugin"), Some(&1));
        assert_eq!(parsed.summary.sync_count, 1);
        assert_eq!(parsed.summary.execution_mode_unknown_count, 1);
        assert_eq!(
            parsed.reconciliation_keys[0],
            runtime_location_key(Path::new("C:/private/source/path"), HookEvent::Stop, 0, 0)
        );
        let json = serde_json::to_string(&parsed.summary).unwrap();
        assert!(!json.contains("private command"));
        assert!(!json.contains("private/source/path"));
        assert!(!json.contains("private matcher"));
    }
    #[test]
    fn transformation_preserves_semantics_unknown_fields_and_partial_installation() {
        let temp = tempdir().unwrap();
        let config = temp.path().join("hooks.json");
        fs::write(
            &config,
            r#"{"unrelated":{"keep":[1,2]},"hooks":{"Stop":[{"matcher":"^Bash$","groupUnknown":"keep","hooks":[{"type":"command","command":"same","commandWindows":"windows","async":true,"timeout":9,"statusMessage":"status","additionalContextLimit":42,"unknownHandlerField":{"keep":true}},{"type":"command","command":"same","timeout":7}]}]}}"#,
        )
        .unwrap();
        let before = discover_paths(std::slice::from_ref(&config)).unwrap();
        assert_eq!(before.summary.instrumentable, 2);
        assert_ne!(
            before.summary.handlers[0].handler.key,
            before.summary.handlers[1].handler.key
        );
        let second_revision = before
            .summary
            .handlers
            .iter()
            .find(|item| item.handler.structural_identity == "g0:h1")
            .unwrap()
            .handler
            .revision
            .clone();
        let state = temp.path().join("state");
        apply(&before, &state, Path::new("hookstat-test")).unwrap();
        let applied: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
        let first = &applied["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(applied["unrelated"]["keep"], serde_json::json!([1, 2]));
        assert_eq!(applied["hooks"]["Stop"][0]["groupUnknown"], "keep");
        assert_eq!(first["commandWindows"], first["command"]);
        assert_eq!(first["async"], true);
        assert_eq!(first["timeout"], 9);
        assert_eq!(first["statusMessage"], "status");
        assert_eq!(first["additionalContextLimit"], 42);
        assert_eq!(first["unknownHandlerField"]["keep"], true);
        let partially = discover_paths(std::slice::from_ref(&config)).unwrap();
        assert_eq!(partially.summary.already_instrumented, 2);
        assert_eq!(restore(&config, &state).unwrap().restored, 1);
        let mut changed: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
        changed["hooks"]["Stop"][0]["hooks"][1]["command"] = Value::String("changed".into());
        fs::write(&config, serde_json::to_vec(&changed).unwrap()).unwrap();
        let revised = discover_paths(std::slice::from_ref(&config)).unwrap();
        assert_ne!(
            revised
                .summary
                .handlers
                .iter()
                .find(|item| item.handler.structural_identity == "g0:h1")
                .unwrap()
                .handler
                .revision,
            second_revision
        );
    }
    #[test]
    fn malformed_empty_and_unsupported_configs_fail_closed_without_rewrite() {
        let temp = tempdir().unwrap();
        let malformed = temp.path().join("hooks.json");
        fs::write(&malformed, b"{").unwrap();
        assert!(matches!(
            discover_paths(&[malformed]),
            Err(CodexError::Json(_))
        ));
        let empty = temp.path().join("empty/hooks.json");
        fs::create_dir_all(empty.parent().unwrap()).unwrap();
        fs::write(&empty, b"{\"unrelated\":true}").unwrap();
        assert_eq!(discover_paths(&[empty]).unwrap().summary.discovered, 0);
        let unsupported = temp.path().join("unsupported/hooks.json");
        fs::create_dir_all(unsupported.parent().unwrap()).unwrap();
        fs::write(
            &unsupported,
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"mcp","server":"fixture"}]}]}}"#,
        )
        .unwrap();
        let plan = discover_paths(std::slice::from_ref(&unsupported)).unwrap();
        assert_eq!(plan.summary.unsupported_or_uninstrumentable, 1);
        let original = fs::read(&unsupported).unwrap();
        let state = temp.path().join("unsupported-state");
        let result = apply(&plan, &state, Path::new("hookstat-test")).unwrap();
        assert_eq!(result.applied, 0);
        assert_eq!(fs::read(unsupported).unwrap(), original);
    }
}
