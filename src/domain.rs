//! Privacy-preserving canonical concepts for HookStat reliability records.
//!
//! The types in this module deliberately describe evidence rather than a
//! particular runtime implementation. Codex v0.1 can use opt-in instrumented
//! receipts; future runtimes can provide passive durable evidence without
//! changing storage, analytics, JSON, or TUI consumers.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    Codex,
    DeepSeekHarness,
    OpenCode,
}

impl Runtime {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::DeepSeekHarness => "deepseek_harness",
            Self::OpenCode => "opencode",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "deepseek_harness" => Some(Self::DeepSeekHarness),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::DeepSeekHarness => "DeepSeek Harness",
            Self::OpenCode => "OpenCode",
        }
    }
}

/// The implementation class of a source. It is intentionally separate from
/// `EvidenceKind`: callers can reason about passive versus instrumented data
/// while retaining the runtime-specific surface name.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceClass {
    Passive,
    Instrumented,
    SyntheticFixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    CodexSessionJsonl,
    CodexStateDatabase,
    CodexAppServerLive,
    CodexInstrumentedReceipt,
    OpenTelemetry,
    SyntheticFixture,
}

impl EvidenceKind {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::CodexSessionJsonl => "codex_session_jsonl",
            Self::CodexStateDatabase => "codex_state_database",
            Self::CodexAppServerLive => "codex_app_server_live",
            Self::CodexInstrumentedReceipt => "codex_instrumented_receipt",
            Self::OpenTelemetry => "open_telemetry",
            Self::SyntheticFixture => "synthetic_fixture",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "codex_session_jsonl" => Some(Self::CodexSessionJsonl),
            "codex_state_database" => Some(Self::CodexStateDatabase),
            "codex_app_server_live" => Some(Self::CodexAppServerLive),
            "codex_instrumented_receipt" => Some(Self::CodexInstrumentedReceipt),
            "open_telemetry" => Some(Self::OpenTelemetry),
            "synthetic_fixture" => Some(Self::SyntheticFixture),
            _ => None,
        }
    }

    pub const fn source_class(self) -> EvidenceSourceClass {
        match self {
            Self::CodexSessionJsonl
            | Self::CodexStateDatabase
            | Self::CodexAppServerLive
            | Self::OpenTelemetry => EvidenceSourceClass::Passive,
            Self::CodexInstrumentedReceipt => EvidenceSourceClass::Instrumented,
            Self::SyntheticFixture => EvidenceSourceClass::SyntheticFixture,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCoverage {
    Complete,
    Partial,
    SyncOnly,
    BestEffort,
    Unknown,
    NotAdmitted,
    SyntheticFixture,
}

impl EvidenceCoverage {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::SyncOnly => "sync_only",
            Self::BestEffort => "best_effort",
            Self::Unknown => "unknown",
            Self::NotAdmitted => "not_admitted",
            Self::SyntheticFixture => "synthetic_fixture",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "complete" => Some(Self::Complete),
            "partial" => Some(Self::Partial),
            "sync_only" => Some(Self::SyncOnly),
            "best_effort" => Some(Self::BestEffort),
            "unknown" => Some(Self::Unknown),
            "not_admitted" => Some(Self::NotAdmitted),
            "synthetic_fixture" => Some(Self::SyntheticFixture),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAdmission {
    AdmittedPassive,
    AdmittedInstrumented,
    BlockedDataSourceDecisionRequired,
    SyntheticFixtureOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceQualification {
    pub admission: EvidenceAdmission,
    pub primary_source: Option<EvidenceKind>,
    pub coverage: EvidenceCoverage,
    pub handler_identity_proven: bool,
    pub invocation_denominator_proven: bool,
    pub terminal_status_proven: bool,
}

impl SourceQualification {
    pub const fn blocked() -> Self {
        Self {
            admission: EvidenceAdmission::BlockedDataSourceDecisionRequired,
            primary_source: None,
            coverage: EvidenceCoverage::NotAdmitted,
            handler_identity_proven: false,
            invocation_denominator_proven: false,
            terminal_status_proven: false,
        }
    }

    pub const fn instrumented() -> Self {
        Self {
            admission: EvidenceAdmission::AdmittedInstrumented,
            primary_source: Some(EvidenceKind::CodexInstrumentedReceipt),
            coverage: EvidenceCoverage::Partial,
            handler_identity_proven: true,
            invocation_denominator_proven: true,
            terminal_status_proven: true,
        }
    }

    pub const fn synthetic_fixture() -> Self {
        Self {
            admission: EvidenceAdmission::SyntheticFixtureOnly,
            primary_source: Some(EvidenceKind::SyntheticFixture),
            coverage: EvidenceCoverage::SyntheticFixture,
            handler_identity_proven: true,
            invocation_denominator_proven: true,
            terminal_status_proven: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PermissionRequest,
    PreCompact,
    PostCompact,
    Stop,
    SubagentStart,
    SubagentStop,
}

impl HookEvent {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PermissionRequest => "permission_request",
            Self::PreCompact => "pre_compact",
            Self::PostCompact => "post_compact",
            Self::Stop => "stop",
            Self::SubagentStart => "subagent_start",
            Self::SubagentStop => "subagent_stop",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "session_start" => Some(Self::SessionStart),
            "session_end" => Some(Self::SessionEnd),
            "user_prompt_submit" => Some(Self::UserPromptSubmit),
            "pre_tool_use" => Some(Self::PreToolUse),
            "post_tool_use" => Some(Self::PostToolUse),
            "permission_request" => Some(Self::PermissionRequest),
            "pre_compact" => Some(Self::PreCompact),
            "post_compact" => Some(Self::PostCompact),
            "stop" => Some(Self::Stop),
            "subagent_start" => Some(Self::SubagentStart),
            "subagent_stop" => Some(Self::SubagentStop),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PermissionRequest => "PermissionRequest",
            Self::PreCompact => "PreCompact",
            Self::PostCompact => "PostCompact",
            Self::Stop => "Stop",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Sync,
    Async,
    /// Runtime-effective discovery did not expose a mode. This is distinct
    /// from an observed synchronous handler and must not be guessed.
    Unknown,
}

impl ExecutionMode {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Async => "async",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "sync" => Some(Self::Sync),
            "async" => Some(Self::Async),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Native terminal status. `Incomplete` and `Unknown` intentionally do not
/// become failures or successes; their presence keeps coverage visible.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStatus {
    Completed,
    Failed,
    Blocked,
    Stopped,
    TimedOut,
    ProtocolFailure,
    Incomplete,
    Unknown,
}

impl TerminalStatus {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Stopped => "stopped",
            Self::TimedOut => "timed_out",
            Self::ProtocolFailure => "protocol_failure",
            Self::Incomplete => "incomplete",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "blocked" => Some(Self::Blocked),
            "stopped" => Some(Self::Stopped),
            "timed_out" => Some(Self::TimedOut),
            "protocol_failure" => Some(Self::ProtocolFailure),
            "incomplete" => Some(Self::Incomplete),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    pub const fn is_execution_failure(self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut | Self::ProtocolFailure)
    }

    pub const fn is_terminal_sample(self) -> bool {
        !matches!(self, Self::Incomplete | Self::Unknown)
    }
}

/// Stable per-handler attribution. Its fields are fingerprints or structural
/// metadata, not a raw command string, matcher text, stdin, or hook output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HandlerIdentity {
    pub key: String,
    pub revision: String,
    pub label: String,
    pub source_kind: String,
    pub event: HookEvent,
    pub matcher_identity: String,
    pub structural_identity: String,
    pub execution_mode: ExecutionMode,
}

/// A normalized execution record containing no prompt, tool payload, stdin,
/// stdout, stderr, or raw hook command field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookInvocation {
    pub source_key: String,
    pub source_record_id: String,
    pub runtime: Runtime,
    pub evidence_kind: EvidenceKind,
    pub coverage: EvidenceCoverage,
    pub handler: HandlerIdentity,
    pub occurred_at_unix_ms: i64,
    pub terminal_status: TerminalStatus,
    pub duration_ms: Option<u64>,
    /// Bounded taxonomy only (for example `exit_nonzero`), never stream data.
    pub error_fingerprint: Option<String>,
}

impl HookInvocation {
    pub fn validate(&self) -> Result<(), ValidationError> {
        for (name, value) in [
            ("source_key", &self.source_key),
            ("source_record_id", &self.source_record_id),
            ("handler.key", &self.handler.key),
            ("handler.revision", &self.handler.revision),
            ("handler.label", &self.handler.label),
            ("handler.source_kind", &self.handler.source_kind),
            ("handler.matcher_identity", &self.handler.matcher_identity),
            (
                "handler.structural_identity",
                &self.handler.structural_identity,
            ),
        ] {
            if value.trim().is_empty() || value.len() > 256 {
                return Err(ValidationError::new(name));
            }
        }
        if self.occurred_at_unix_ms < 0 {
            return Err(ValidationError::new("occurred_at_unix_ms"));
        }
        if self
            .duration_ms
            .is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(ValidationError::new("duration_ms"));
        }
        if self
            .error_fingerprint
            .as_deref()
            .is_some_and(|value| value.is_empty() || value.len() > 128)
        {
            return Err(ValidationError::new("error_fingerprint"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    field: &'static str,
}

impl ValidationError {
    pub(crate) const fn new(field: &'static str) -> Self {
        Self { field }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid reliability metadata in {}", self.field)
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrumented_evidence_is_explicitly_admitted_but_partial() {
        let qualification = SourceQualification::instrumented();
        assert_eq!(
            qualification.admission,
            EvidenceAdmission::AdmittedInstrumented
        );
        assert_eq!(qualification.coverage, EvidenceCoverage::Partial);
    }

    #[test]
    fn incomplete_and_control_results_are_not_execution_failures() {
        assert!(!TerminalStatus::Blocked.is_execution_failure());
        assert!(!TerminalStatus::Incomplete.is_execution_failure());
        assert!(!TerminalStatus::Unknown.is_terminal_sample());
        assert!(TerminalStatus::TimedOut.is_execution_failure());
    }
}
