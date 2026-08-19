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
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
                let handler_key = format!(
                    "hk_{}",
                    short_hash(
                        format!(
                            "{}|{}|{}|{}",
                            source_kind.label(),
                            event.as_storage(),
                            matcher_identity,
                            structural_identity
                        )
                        .as_bytes()
                    )
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
        "SessionStart" => Some(HookEvent::SessionStart),
        "SessionEnd" => Some(HookEvent::SessionEnd),
        "UserPromptSubmit" => Some(HookEvent::UserPromptSubmit),
        "PreToolUse" => Some(HookEvent::PreToolUse),
        "PostToolUse" => Some(HookEvent::PostToolUse),
        "PermissionRequest" => Some(HookEvent::PermissionRequest),
        "PreCompact" => Some(HookEvent::PreCompact),
        "PostCompact" => Some(HookEvent::PostCompact),
        "Stop" => Some(HookEvent::Stop),
        "SubagentStart" => Some(HookEvent::SubagentStart),
        "SubagentStop" => Some(HookEvent::SubagentStop),
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
}
