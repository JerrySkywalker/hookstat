//! Privacy-preserving canonical concepts for HookStat reliability records.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Coding-agent host. The model intentionally remains multi-runtime even while
/// v0.1 development is limited to Codex.
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

/// The runtime-specific surface from which a record originated.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    CodexSessionJsonl,
    CodexStateDatabase,
    CodexAppServerLive,
    OpenTelemetry,
    SyntheticFixture,
}

impl EvidenceKind {
    pub const fn as_storage(self) -> &'static str {
        match self {
            Self::CodexSessionJsonl => "codex_session_jsonl",
            Self::CodexStateDatabase => "codex_state_database",
            Self::CodexAppServerLive => "codex_app_server_live",
            Self::OpenTelemetry => "open_telemetry",
            Self::SyntheticFixture => "synthetic_fixture",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "codex_session_jsonl" => Some(Self::CodexSessionJsonl),
            "codex_state_database" => Some(Self::CodexStateDatabase),
            "codex_app_server_live" => Some(Self::CodexAppServerLive),
            "open_telemetry" => Some(Self::OpenTelemetry),
            "synthetic_fixture" => Some(Self::SyntheticFixture),
            _ => None,
        }
    }
}

/// How completely an evidence source observes the invocation denominator.
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

/// Whether a source is allowed to underpin user-facing reliability claims.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAdmission {
    Admitted,
    BlockedDataSourceDecisionRequired,
    SyntheticFixtureOnly,
}

/// The compact proof summary carried with every report.
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
    /// The governed HS-G01 stop state. This must never be rendered as healthy
    /// zero evidence.
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

    /// A deterministic test-only source. It is intentionally not admitted.
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

/// A configured lifecycle point. An event name alone is not a handler identity.
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

/// Native terminal status. Only execution failures contribute to failure rate.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStatus {
    Completed,
    Failed,
    Blocked,
    Stopped,
    TimedOut,
    ProtocolFailure,
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
            _ => None,
        }
    }

    pub const fn is_execution_failure(self) -> bool {
        matches!(self, Self::Failed | Self::TimedOut | Self::ProtocolFailure)
    }
}

/// Stable per-handler attribution. `key` is expected to be a source-defined
/// stable identifier or one-way fingerprint, never a raw command line.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HandlerIdentity {
    pub key: String,
    pub label: String,
    pub event: HookEvent,
}

/// A normalized execution record containing no prompt or tool-payload field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookInvocation {
    /// Opaque key for the source surface; a filesystem path is not required.
    pub source_key: String,
    /// Opaque source-record identity used for idempotent ingestion.
    pub source_record_id: String,
    pub runtime: Runtime,
    pub evidence_kind: EvidenceKind,
    pub coverage: EvidenceCoverage,
    pub handler: HandlerIdentity,
    pub occurred_at_unix_ms: i64,
    pub terminal_status: TerminalStatus,
    pub duration_ms: Option<u64>,
    /// Optional bounded fingerprint, never raw stderr/stdout, a prompt, or tool data.
    pub error_fingerprint: Option<String>,
}

impl HookInvocation {
    pub fn validate(&self) -> Result<(), ValidationError> {
        for (name, value) in [
            ("source_key", &self.source_key),
            ("source_record_id", &self.source_record_id),
            ("handler.key", &self.handler.key),
            ("handler.label", &self.handler.label),
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
            .is_some_and(|duration_ms| duration_ms > i64::MAX as u64)
        {
            return Err(ValidationError::new("duration_ms"));
        }
        if self
            .error_fingerprint
            .as_deref()
            .is_some_and(|fingerprint| fingerprint.is_empty() || fingerprint.len() > 128)
        {
            return Err(ValidationError::new("error_fingerprint"));
        }
        Ok(())
    }
}

/// A validation error intentionally reports only a field name, never the data.
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
    fn blocked_qualification_cannot_look_healthy() {
        let qualification = SourceQualification::blocked();
        assert_eq!(
            qualification.admission,
            EvidenceAdmission::BlockedDataSourceDecisionRequired
        );
        assert_eq!(qualification.coverage, EvidenceCoverage::NotAdmitted);
        assert!(!qualification.handler_identity_proven);
    }

    #[test]
    fn blocked_and_stopped_are_not_execution_failures() {
        assert!(!TerminalStatus::Blocked.is_execution_failure());
        assert!(!TerminalStatus::Stopped.is_execution_failure());
        assert!(TerminalStatus::TimedOut.is_execution_failure());
    }
}
