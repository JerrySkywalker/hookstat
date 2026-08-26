//! Codex App Server Native L1 integration.
//!
//! This module is the only place that understands Codex wire records. It
//! accepts runtime-owned App Server notifications but does not launch, wrap, or
//! attach HookStat to an ordinary `codex` session. Raw wire fields are held only
//! while a record is normalized, then reduced to bounded opaque references.

use crate::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus,
};
use crate::evidence::{
    CanonicalEvidence, CorrelatedEvidence, EventFamily, EvidenceError, EvidenceLifecycle,
    EvidenceTransport, InvocationCoverage, InvocationKey, NativeAdmissionState, RevisionRef,
    RuntimeHandlerRef, RuntimeId, RuntimeInstance, SourceCoverage, SourceScope,
};
use crate::native::{
    CapabilityAssessment, NativeCapabilityMatrix, NativeCapabilityProbe, NativeEvidenceReader,
    NativeNormalizer, RuntimeIdentityResolver,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

/// Exact CLI release exercised by the controlled L1 qualification.
pub const CODEX_TESTED_CLI_VERSION: &str = "0.149.0";
/// Peeled `rust-v0.149.0` source commit exercised by the qualification.
pub const CODEX_TESTED_SOURCE_COMMIT: &str = "758ef40f50c1a458425c7cfbf1eb12cbc07af0b0";

/// Host family relevant to ordinary-session Native L2 acquisition.
///
/// This is explicit rather than inferred inside the qualification method so
/// cross-platform tests can prove the Windows release decision on every CI
/// host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexHostPlatform {
    Windows,
    Unix,
    Other,
}

impl CodexHostPlatform {
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(unix) {
            Self::Unix
        } else {
            Self::Other
        }
    }
}

/// Result of qualifying Native acquisition from an ordinary user-launched
/// `codex` process. This is separate from Native L1 protocol qualification:
/// observing lifecycle fields in a controlled App Server does not prove that
/// an external observer can attach to an ordinary session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexNativeL2Status {
    Admitted,
    UpstreamUnavailable,
    NotQualified,
}

impl CodexNativeL2Status {
    /// Only an explicitly admitted L2 result may become Native authority.
    pub const fn native_admission(self) -> NativeAdmissionState {
        match self {
            Self::Admitted => NativeAdmissionState::Admitted,
            Self::UpstreamUnavailable | Self::NotQualified => NativeAdmissionState::Unavailable,
        }
    }
}

/// A version/source pair must be pinned before Native facts can be reused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexProtocolVersion {
    pub cli_version: String,
    pub source_commit: String,
}

impl CodexProtocolVersion {
    pub fn tested() -> Self {
        Self {
            cli_version: CODEX_TESTED_CLI_VERSION.to_owned(),
            source_commit: CODEX_TESTED_SOURCE_COMMIT.to_owned(),
        }
    }

    pub fn new(cli_version: impl Into<String>, source_commit: impl Into<String>) -> Self {
        Self {
            cli_version: cli_version.into(),
            source_commit: source_commit.into(),
        }
    }
}

/// Facts qualified against one exact Codex App Server protocol baseline.
#[derive(Clone, Copy, Debug, Default)]
pub struct CodexNativeCapabilityProbe;

impl CodexNativeCapabilityProbe {
    /// Returns the ordinary-session attach result for the exact qualified
    /// protocol pin and host family.
    ///
    /// In Codex 0.149.0 the shared app-server daemon is Unix-only and the TUI's
    /// non-Unix default-daemon probe returns no endpoint, so an ordinary
    /// Windows `codex` process keeps an embedded App Server that HookStat cannot
    /// passively attach to. Other versions and host families remain
    /// `NotQualified` until their own acquisition proof exists.
    pub fn ordinary_session_attach(
        &self,
        version: &CodexProtocolVersion,
        platform: CodexHostPlatform,
    ) -> CodexNativeL2Status {
        if version != &CodexProtocolVersion::tested() {
            return CodexNativeL2Status::NotQualified;
        }
        match platform {
            CodexHostPlatform::Windows => CodexNativeL2Status::UpstreamUnavailable,
            CodexHostPlatform::Unix | CodexHostPlatform::Other => CodexNativeL2Status::NotQualified,
        }
    }
}

impl NativeCapabilityProbe for CodexNativeCapabilityProbe {
    type Version = CodexProtocolVersion;

    fn probe(&self, version: &CodexProtocolVersion) -> NativeCapabilityMatrix {
        if version != &CodexProtocolVersion::tested() {
            return NativeCapabilityMatrix {
                invocation_start: CapabilityAssessment::NotProven,
                terminal_result: CapabilityAssessment::NotProven,
                stable_handler_attribution: CapabilityAssessment::NotProven,
                duration: CapabilityAssessment::NotProven,
                source_scope: CapabilityAssessment::NotProven,
                revision_attribution: CapabilityAssessment::NotProven,
                ordering_or_correlation: CapabilityAssessment::NotProven,
                replay_or_delivery_characteristics: CapabilityAssessment::NotProven,
                event_surface_completeness: CapabilityAssessment::NotProven,
                privacy_boundary: CapabilityAssessment::NotProven,
                version_compatibility: CapabilityAssessment::Incompatible,
                admission: NativeAdmissionState::Unavailable,
                source_coverage: SourceCoverage::Unknown,
            };
        }

        NativeCapabilityMatrix {
            invocation_start: CapabilityAssessment::Proven,
            terminal_result: CapabilityAssessment::Proven,
            // Codex `run.id` includes a source path and display order. It is a
            // useful ephemeral join key, not a stable HookStat handler key.
            stable_handler_attribution: CapabilityAssessment::NotProven,
            duration: CapabilityAssessment::Proven,
            source_scope: CapabilityAssessment::Proven,
            // `hooks/list.currentHash` is a normalized handler revision hash,
            // but is only joined in memory for the exact active configuration.
            revision_attribution: CapabilityAssessment::Proven,
            ordering_or_correlation: CapabilityAssessment::Proven,
            // The App Server stream publishes no replay cursor or delivery
            // acknowledgement contract in this qualified version.
            replay_or_delivery_characteristics: CapabilityAssessment::NotProven,
            // Core emits these lifecycle messages for synchronous hooks only.
            event_surface_completeness: CapabilityAssessment::NotProven,
            privacy_boundary: CapabilityAssessment::Proven,
            version_compatibility: CapabilityAssessment::Proven,
            // Qualification proves the L1 protocol chain, not a production
            // denominator authority while handler identity remains limited.
            admission: NativeAdmissionState::Qualified,
            source_coverage: SourceCoverage::IdentityLimited,
        }
    }
}

/// Private, transient App Server notification fields. This type deliberately
/// does not implement `Serialize`; `source_path`, output entries, and status
/// text must not cross into canonical evidence or durable storage.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexWireHookParams {
    thread_id: String,
    turn_id: Option<String>,
    run: CodexWireHookRun,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexWireHookRun {
    id: String,
    event_name: String,
    execution_mode: String,
    scope: String,
    source_path: String,
    #[serde(default = "unknown_source")]
    source: String,
    display_order: i64,
    status: String,
    started_at: i64,
    completed_at: Option<i64>,
    duration_ms: Option<i64>,
}

fn unknown_source() -> String {
    "unknown".to_owned()
}

/// A runtime record is intentionally opaque outside this adapter module.
#[derive(Clone)]
pub struct CodexNativeRecord {
    sequence: u64,
    lifecycle: EvidenceLifecycle,
    params: CodexWireHookParams,
}

/// An adapter-owned cursor. A future reader may obtain records from a
/// subscription, a durable replay, or a one-shot batch without changing core.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodexNativeCursor {
    next_sequence: u64,
}

impl CodexNativeCursor {
    pub const fn position(&self) -> u64 {
        self.next_sequence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodexReadOutcome {
    Accepted,
    Ignored,
}

/// A transport-agnostic queue of runtime-owned App Server lifecycle records.
///
/// The queue can be fed by a live JSONL client, a controlled capture, or a
/// future replay implementation. It has no socket or process lifetime
/// assumption, and its state never reaches evidence core.
#[derive(Clone, Default)]
pub struct CodexNativeReader {
    next_sequence: u64,
    records: Vec<CodexNativeRecord>,
}

impl CodexNativeReader {
    pub fn ingest_json(&mut self, value: Value) -> Result<CodexReadOutcome, CodexNativeError> {
        let Some(method) = value.get("method").and_then(Value::as_str) else {
            return Ok(CodexReadOutcome::Ignored);
        };
        let lifecycle = match method {
            "hook/started" => EvidenceLifecycle::Started,
            "hook/completed" => EvidenceLifecycle::Completed,
            _ => return Ok(CodexReadOutcome::Ignored),
        };
        let params = value
            .get("params")
            .cloned()
            .ok_or(CodexNativeError::MissingWireField("params"))?;
        let params = serde_json::from_value(params).map_err(|_| CodexNativeError::MalformedWire)?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.records.push(CodexNativeRecord {
            sequence: self.next_sequence,
            lifecycle,
            params,
        });
        Ok(CodexReadOutcome::Accepted)
    }
}

impl NativeEvidenceReader for CodexNativeReader {
    type Cursor = CodexNativeCursor;
    type Record = CodexNativeRecord;
    type Error = CodexNativeError;

    fn read(
        &mut self,
        cursor: &mut CodexNativeCursor,
    ) -> Result<Vec<CodexNativeRecord>, CodexNativeError> {
        let records = self
            .records
            .iter()
            .filter(|record| record.sequence > cursor.next_sequence)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(last) = records.last() {
            cursor.next_sequence = last.sequence;
        }
        Ok(records)
    }
}

/// Private ephemeral catalog information needed to join an App Server lifecycle
/// event to the exact active handler revision. The raw path never leaves this
/// resolver and is neither serialized nor cloned into canonical evidence.
#[derive(Clone)]
struct CatalogEntry {
    event_name: String,
    source_path: String,
    display_order: i64,
    current_hash: String,
}

/// Codex identity resolution remains deliberately conservative. A revision can
/// be joined from the live `hooks/list` catalog, but the location-based runtime
/// id is not claimed to be stable through declaration reordering.
#[derive(Clone, Default)]
pub struct CodexNativeIdentityResolver {
    catalog: Vec<CatalogEntry>,
}

/// Bounded identity material returned to the normalizer and qualification-only
/// invocation conversion. None of these values contains a raw path or command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexNativeIdentity {
    pub runtime_handler_ref: RuntimeHandlerRef,
    pub revision_ref: Option<RevisionRef>,
    pub stable_handler_attribution_proven: bool,
}

impl CodexNativeIdentityResolver {
    /// Reduces a real App Server `hooks/list` response to an in-memory revision
    /// join table. It intentionally consumes only event, path, display order,
    /// and `currentHash`; commands, output, and trust state are ignored.
    pub fn from_hooks_list(response: &Value) -> Result<Self, CodexNativeError> {
        let contexts = response
            .get("result")
            .and_then(|value| value.get("data"))
            .or_else(|| response.get("data"))
            .and_then(Value::as_array)
            .ok_or(CodexNativeError::MissingWireField("hooks/list.data"))?;
        let mut catalog = Vec::new();
        for context in contexts {
            let Some(hooks) = context.get("hooks").and_then(Value::as_array) else {
                continue;
            };
            for hook in hooks {
                let event_name = required_string(hook, "eventName")?;
                let source_path = required_string(hook, "sourcePath")?;
                let current_hash = required_string(hook, "currentHash")?;
                let display_order = hook
                    .get("displayOrder")
                    .and_then(Value::as_i64)
                    .ok_or(CodexNativeError::MissingWireField("displayOrder"))?;
                catalog.push(CatalogEntry {
                    event_name,
                    source_path,
                    display_order,
                    current_hash,
                });
            }
        }
        Ok(Self { catalog })
    }

    /// Builds a non-persistable HookInvocation for controlled qualification.
    /// The location fingerprint must not be presented as a stable handler key,
    /// and the resulting row remains `NotAdmitted` by construction.
    pub fn qualification_invocation(
        &self,
        evidence: &CorrelatedEvidence,
    ) -> Result<HookInvocation, CodexNativeError> {
        let event = hook_event_from_canonical(evidence.event.as_str())?;
        let location_key = opaque(
            "codex_native_location",
            &[evidence.runtime_handler_ref.as_str()],
        );
        let revision = evidence
            .revision_ref
            .as_ref()
            .map(|value| value.as_str().to_owned())
            .unwrap_or_else(|| "codex_native_revision_unproven".to_owned());
        Ok(HookInvocation {
            source_key: "codex_native_l1_qualification".to_owned(),
            source_record_id: opaque(
                "codex_native_record",
                &[
                    evidence.correlation_key.runtime.as_str(),
                    evidence.correlation_key.runtime_instance.as_str(),
                    evidence.correlation_key.invocation_key.as_str(),
                ],
            ),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::CodexAppServerLive,
            coverage: EvidenceCoverage::NotAdmitted,
            handler: HandlerIdentity {
                key: location_key,
                revision,
                label: "Codex native location-limited handler".to_owned(),
                source_kind: "codex_native_location_limited".to_owned(),
                event,
                matcher_identity: "native_matcher_unavailable".to_owned(),
                structural_identity: evidence.runtime_handler_ref.as_str().to_owned(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: evidence.occurred_at_unix_ms,
            terminal_status: evidence.terminal_status,
            duration_ms: evidence.duration_ms,
            error_fingerprint: evidence
                .terminal_status
                .is_execution_failure()
                .then_some("codex_native_terminal_failure".to_owned()),
        })
    }
}

impl RuntimeIdentityResolver for CodexNativeIdentityResolver {
    type Input = CodexNativeRecord;
    type Resolved = CodexNativeIdentity;
    type Error = CodexNativeError;

    fn resolve(&self, record: &CodexNativeRecord) -> Result<CodexNativeIdentity, CodexNativeError> {
        let run = &record.params.run;
        let revision = self
            .catalog
            .iter()
            .find(|entry| {
                entry.event_name == run.event_name
                    && entry.source_path == run.source_path
                    && entry.display_order == run.display_order
            })
            .map(|entry| opaque("codex_native_revision", &[&entry.current_hash]));
        Ok(CodexNativeIdentity {
            runtime_handler_ref: RuntimeHandlerRef::new(opaque("codex_native_handler", &[&run.id]))
                .map_err(CodexNativeError::Evidence)?,
            revision_ref: revision
                .map(RevisionRef::new)
                .transpose()
                .map_err(CodexNativeError::Evidence)?,
            stable_handler_attribution_proven: false,
        })
    }
}

/// Codex Native normalizer. It allow-lists lifecycle metadata and never maps
/// `sourcePath`, `statusMessage`, output entries, or raw run ids into canonical
/// evidence.
#[derive(Clone, Default)]
pub struct CodexNativeNormalizer {
    identities: CodexNativeIdentityResolver,
}

impl CodexNativeNormalizer {
    pub fn new(identities: CodexNativeIdentityResolver) -> Self {
        Self { identities }
    }

    pub fn identity_resolver(&self) -> &CodexNativeIdentityResolver {
        &self.identities
    }
}

impl NativeNormalizer for CodexNativeNormalizer {
    type Record = CodexNativeRecord;
    type Error = CodexNativeError;

    fn normalize(&self, record: &CodexNativeRecord) -> Result<CanonicalEvidence, CodexNativeError> {
        let run = &record.params.run;
        if run.execution_mode != "sync" {
            return Err(CodexNativeError::UnexpectedExecutionMode);
        }
        let identity = self.identities.resolve(record)?;
        let event = EventFamily::new(canonical_event_name(&run.event_name)?)
            .map_err(CodexNativeError::Evidence)?;
        let source_scope = SourceScope::new(format!(
            "codex_{}_{}",
            canonical_source_name(&run.source)?,
            canonical_scope_name(&run.scope)?,
        ))
        .map_err(CodexNativeError::Evidence)?;
        let runtime_instance =
            RuntimeInstance::new(opaque("codex_app_server", &[&record.params.thread_id]))
                .map_err(CodexNativeError::Evidence)?;
        let turn_id = record
            .params
            .turn_id
            .as_deref()
            .ok_or(CodexNativeError::MissingTurnCorrelation)?;
        let invocation_key = InvocationKey::new(opaque(
            "codex_native_invocation",
            &[&record.params.thread_id, turn_id, &run.id],
        ))
        .map_err(CodexNativeError::Evidence)?;
        let (occurred_at_unix_ms, terminal_status, duration_ms, invocation_coverage) =
            match record.lifecycle {
                EvidenceLifecycle::Started => {
                    if run.status != "running" {
                        return Err(CodexNativeError::UnexpectedStartedStatus);
                    }
                    (
                        unix_seconds_to_millis(run.started_at)?,
                        None,
                        None,
                        InvocationCoverage::Incomplete,
                    )
                }
                EvidenceLifecycle::Completed => {
                    let completed_at = run
                        .completed_at
                        .ok_or(CodexNativeError::MissingCompletionTimestamp)?;
                    let duration_ms = run
                        .duration_ms
                        .map(u64::try_from)
                        .transpose()
                        .map_err(|_| CodexNativeError::InvalidDuration)?;
                    (
                        unix_seconds_to_millis(completed_at)?,
                        Some(canonical_terminal_status(&run.status)?),
                        duration_ms,
                        InvocationCoverage::Complete,
                    )
                }
            };
        let canonical = CanonicalEvidence {
            schema_version: 1,
            runtime: RuntimeId::new("codex").map_err(CodexNativeError::Evidence)?,
            runtime_instance,
            invocation_key,
            runtime_handler_ref: identity.runtime_handler_ref,
            event,
            lifecycle: record.lifecycle,
            occurred_at_unix_ms,
            terminal_status,
            duration_ms,
            source_scope,
            revision_ref: identity.revision_ref,
            evidence_transport: EvidenceTransport::Native,
            source_coverage: SourceCoverage::IdentityLimited,
            invocation_coverage,
        };
        canonical.validate().map_err(CodexNativeError::Evidence)?;
        Ok(canonical)
    }
}

/// The small integration object composes only the four Native responsibilities.
/// It deliberately contains no ordinary-session attach, launcher, proxy, or
/// process-management implementation.
#[derive(Clone)]
pub struct CodexNativeIntegration {
    pub probe: CodexNativeCapabilityProbe,
    pub reader: CodexNativeReader,
    pub normalizer: CodexNativeNormalizer,
}

impl CodexNativeIntegration {
    /// Creates an integration only for a protocol baseline whose capability
    /// facts have been qualified. This prevents a caller from treating an
    /// unpinned future Codex version as silently compatible.
    pub fn with_hooks_list(
        version: &CodexProtocolVersion,
        response: &Value,
    ) -> Result<Self, CodexNativeError> {
        if CodexNativeCapabilityProbe
            .probe(version)
            .version_compatibility
            != CapabilityAssessment::Proven
        {
            return Err(CodexNativeError::IncompatibleProtocol);
        }
        let identities = CodexNativeIdentityResolver::from_hooks_list(response)?;
        Ok(Self {
            probe: CodexNativeCapabilityProbe,
            reader: CodexNativeReader::default(),
            normalizer: CodexNativeNormalizer::new(identities),
        })
    }
}

#[derive(Debug)]
pub enum CodexNativeError {
    Evidence(EvidenceError),
    MalformedWire,
    MissingWireField(&'static str),
    UnexpectedStartedStatus,
    UnexpectedExecutionMode,
    MissingCompletionTimestamp,
    InvalidDuration,
    InvalidTimestamp,
    MissingTurnCorrelation,
    IncompatibleProtocol,
    UnsupportedEvent,
    UnsupportedSource,
    UnsupportedScope,
    NonTerminalCompletion,
}

impl fmt::Display for CodexNativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Evidence(_) => "Codex Native evidence did not meet the canonical boundary",
            Self::MalformedWire => "Codex Native notification was malformed",
            Self::MissingWireField(_) => {
                "Codex Native notification omitted required lifecycle metadata"
            }
            Self::UnexpectedStartedStatus => "Codex Native HookStarted did not have running status",
            Self::UnexpectedExecutionMode => {
                "Codex Native lifecycle notification was not synchronous"
            }
            Self::MissingCompletionTimestamp => {
                "Codex Native HookCompleted omitted completion time"
            }
            Self::InvalidDuration => "Codex Native HookCompleted had invalid duration",
            Self::InvalidTimestamp => "Codex Native Hook notification had invalid timestamp",
            Self::MissingTurnCorrelation => {
                "Codex Native Hook notification omitted turn correlation"
            }
            Self::IncompatibleProtocol => {
                "Codex Native protocol version is not qualified for this integration"
            }
            Self::UnsupportedEvent => {
                "Codex Native Hook event is not supported by this qualified version"
            }
            Self::UnsupportedSource => {
                "Codex Native Hook source is not supported by this qualified version"
            }
            Self::UnsupportedScope => {
                "Codex Native Hook scope is not supported by this qualified version"
            }
            Self::NonTerminalCompletion => {
                "Codex Native HookCompleted did not contain a terminal status"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CodexNativeError {}

fn required_string(value: &Value, field: &'static str) -> Result<String, CodexNativeError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(CodexNativeError::MissingWireField(field))
}

fn unix_seconds_to_millis(value: i64) -> Result<i64, CodexNativeError> {
    value
        .checked_mul(1_000)
        .filter(|value| *value >= 0)
        .ok_or(CodexNativeError::InvalidTimestamp)
}

fn canonical_event_name(value: &str) -> Result<&'static str, CodexNativeError> {
    match value {
        "preToolUse" => Ok("pre_tool_use"),
        "permissionRequest" => Ok("permission_request"),
        "postToolUse" => Ok("post_tool_use"),
        "preCompact" => Ok("pre_compact"),
        "postCompact" => Ok("post_compact"),
        "sessionStart" => Ok("session_start"),
        "sessionEnd" => Ok("session_end"),
        "userPromptSubmit" => Ok("user_prompt_submit"),
        "subagentStart" => Ok("subagent_start"),
        "subagentStop" => Ok("subagent_stop"),
        "stop" => Ok("stop"),
        _ => Err(CodexNativeError::UnsupportedEvent),
    }
}

fn canonical_source_name(value: &str) -> Result<&'static str, CodexNativeError> {
    match value {
        "system" => Ok("system"),
        "user" => Ok("user"),
        "project" => Ok("project"),
        "mdm" => Ok("mdm"),
        "sessionFlags" => Ok("session_flags"),
        "plugin" => Ok("plugin"),
        "cloudRequirements" => Ok("cloud_requirements"),
        "cloudManagedConfig" => Ok("cloud_managed_config"),
        "legacyManagedConfigFile" => Ok("legacy_managed_config_file"),
        "legacyManagedConfigMdm" => Ok("legacy_managed_config_mdm"),
        "unknown" => Ok("unknown"),
        _ => Err(CodexNativeError::UnsupportedSource),
    }
}

fn canonical_scope_name(value: &str) -> Result<&'static str, CodexNativeError> {
    match value {
        "thread" => Ok("thread"),
        "turn" => Ok("turn"),
        _ => Err(CodexNativeError::UnsupportedScope),
    }
}

fn canonical_terminal_status(value: &str) -> Result<TerminalStatus, CodexNativeError> {
    match value {
        "completed" => Ok(TerminalStatus::Completed),
        "failed" => Ok(TerminalStatus::Failed),
        "blocked" => Ok(TerminalStatus::Blocked),
        "stopped" => Ok(TerminalStatus::Stopped),
        "running" => Err(CodexNativeError::NonTerminalCompletion),
        _ => Err(CodexNativeError::NonTerminalCompletion),
    }
}

fn hook_event_from_canonical(value: &str) -> Result<HookEvent, CodexNativeError> {
    match value {
        "pre_tool_use" => Ok(HookEvent::PreToolUse),
        "permission_request" => Ok(HookEvent::PermissionRequest),
        "post_tool_use" => Ok(HookEvent::PostToolUse),
        "pre_compact" => Ok(HookEvent::PreCompact),
        "post_compact" => Ok(HookEvent::PostCompact),
        "session_start" => Ok(HookEvent::SessionStart),
        "session_end" => Ok(HookEvent::SessionEnd),
        "user_prompt_submit" => Ok(HookEvent::UserPromptSubmit),
        "subagent_start" => Ok(HookEvent::SubagentStart),
        "subagent_stop" => Ok(HookEvent::SubagentStop),
        "stop" => Ok(HookEvent::Stop),
        _ => Err(CodexNativeError::UnsupportedEvent),
    }
}

fn opaque(prefix: &str, values: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    format!("{prefix}_{:x}", hasher.finalize())
}
