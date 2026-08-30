//! Ephemeral current-runtime hook presentation.
//!
//! This module is deliberately separate from the ledger and receipt model.
//! Runtime-owned strings such as commands, matchers, and source paths are
//! useful to render locally, but are neither evidence nor durable metadata.

use crate::domain::{HookEvent, Runtime};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A local, in-memory representation of the current runtime hook catalog.
///
/// It intentionally does not implement `Serialize` or `Debug`: callers must
/// make an explicit, privacy-reviewed projection before logging or exporting
/// any of its runtime-owned presentation strings.
#[derive(Clone, Eq, PartialEq)]
pub struct RuntimePresentationSnapshot {
    pub runtime: Runtime,
    pub captured_at_unix_ms: i64,
    pub events: Vec<RuntimeEventPresentation>,
    pub issues: Vec<RuntimeCatalogIssue>,
}

/// One event reported by the runtime. `runtime_event_name` is intentionally a
/// string rather than a `HookEvent`, so a new runtime event cannot disappear
/// merely because HookStat has no historical reliability taxonomy for it yet.
#[derive(Clone, Eq, PartialEq)]
pub struct RuntimeEventPresentation {
    pub runtime_event_name: String,
    pub canonical_event: Option<HookEvent>,
    pub description: Option<String>,
    pub handlers: Vec<RuntimeHandlerPresentation>,
}

impl RuntimeEventPresentation {
    pub fn installed_count(&self) -> usize {
        self.handlers.len()
    }

    pub fn active_count(&self) -> usize {
        self.handlers
            .iter()
            .filter(|handler| handler.enabled && (handler.managed || handler.trust.is_trusted()))
            .count()
    }

    pub fn needs_review_count(&self) -> usize {
        self.handlers
            .iter()
            .filter(|handler| handler.needs_review)
            .count()
    }
}

/// A runtime-owned handler presentation. Raw strings are local-only and this
/// type intentionally has no serialization or debug implementation.
#[derive(Clone, Eq, PartialEq)]
pub struct RuntimeHandlerPresentation {
    pub runtime_catalog_id: String,
    pub reliability_key_hint: Option<String>,
    pub enabled: bool,
    pub managed: bool,
    pub needs_review: bool,
    pub trust: RuntimeTrust,
    pub matcher: Option<String>,
    pub source: Option<String>,
    pub source_path: Option<String>,
    pub handler_kind: RuntimeHandlerKind,
    pub mode: Option<RuntimeHandlerMode>,
    pub timeout_seconds: Option<u64>,
    pub additional_context_limit: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeHandlerMode {
    Sync,
    Async,
}

#[derive(Clone, Eq, PartialEq)]
pub enum RuntimeHandlerKind {
    Command { command: String },
    McpTool { server: String, tool: String },
    Prompt,
    Agent,
    Unknown { label: String },
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RuntimeTrust {
    Managed,
    Trusted,
    Untrusted,
    Modified,
    Unknown,
}

impl RuntimeTrust {
    pub const fn is_trusted(self) -> bool {
        matches!(self, Self::Managed | Self::Trusted)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RuntimeCatalogIssueSeverity {
    Warning,
    Error,
}

/// Discovery messages are current runtime state, never historical invocation
/// failures. Like the rest of the snapshot, they remain local and ephemeral.
#[derive(Clone, Eq, PartialEq)]
pub struct RuntimeCatalogIssue {
    pub severity: RuntimeCatalogIssueSeverity,
    pub human_message: String,
    pub context: Option<String>,
}

#[derive(Clone, Eq, PartialEq)]
pub enum ReliabilityJoinState {
    Matched { handler_key: String },
    NoHistory,
    Ambiguous,
    Unsupported,
}

#[derive(Clone, Eq, PartialEq)]
pub struct HistoricalHandlerIdentity {
    pub handler_key: String,
    pub event: HookEvent,
    pub reliability_key_hint: Option<String>,
}

#[derive(Clone, Eq, PartialEq)]
pub struct JoinedRuntimeHandler<'a> {
    pub handler: &'a RuntimeHandlerPresentation,
    pub join: ReliabilityJoinState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCatalogResourceState {
    Empty,
    Loading,
    Ready,
    Error,
}

/// Owns catalog refresh intent independently of period analytics. The TUI can
/// ask this controller for work, but period switching never changes its state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCatalogResource {
    state: RuntimeCatalogResourceState,
    explicit_refreshes: u64,
}

impl Default for RuntimeCatalogResource {
    fn default() -> Self {
        Self {
            state: RuntimeCatalogResourceState::Empty,
            explicit_refreshes: 0,
        }
    }
}

impl RuntimeCatalogResource {
    pub const fn state(&self) -> RuntimeCatalogResourceState {
        self.state
    }

    pub const fn explicit_refreshes(&self) -> u64 {
        self.explicit_refreshes
    }

    pub fn request_initial_load(&mut self) {
        if self.state == RuntimeCatalogResourceState::Empty {
            self.state = RuntimeCatalogResourceState::Loading;
        }
    }

    pub fn request_explicit_refresh(&mut self) {
        self.explicit_refreshes += 1;
        self.state = RuntimeCatalogResourceState::Loading;
    }

    /// Deliberately a no-op: changing the reliability period never rediscovers
    /// the runtime catalog.
    pub const fn period_switched(&mut self) {}

    pub fn accepted(&mut self) {
        self.state = RuntimeCatalogResourceState::Ready;
    }

    pub fn failed(&mut self) {
        self.state = RuntimeCatalogResourceState::Error;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeCatalogParseError;

impl fmt::Display for RuntimeCatalogParseError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str("invalid Codex hooks/list response")
    }
}

impl std::error::Error for RuntimeCatalogParseError {}

impl RuntimePresentationSnapshot {
    /// Parses the official Codex `hooks/list` response. The caller owns the
    /// response lifetime; this projection neither writes nor exports it.
    pub fn from_codex_hooks_list(
        response: &Value,
        captured_at_unix_ms: i64,
    ) -> Result<Self, RuntimeCatalogParseError> {
        let contexts = response
            .get("result")
            .and_then(|value| value.get("data"))
            .or_else(|| response.get("data"))
            .and_then(Value::as_array)
            .ok_or(RuntimeCatalogParseError)?;
        let mut events = BTreeMap::<String, RuntimeEventPresentation>::new();
        let mut issues = Vec::new();
        for context in contexts {
            let context_name = text(context, "cwd").map(str::to_owned);
            collect_issues(
                context,
                "warnings",
                RuntimeCatalogIssueSeverity::Warning,
                context_name.as_deref(),
                &mut issues,
            );
            collect_issues(
                context,
                "errors",
                RuntimeCatalogIssueSeverity::Error,
                context_name.as_deref(),
                &mut issues,
            );
            for item in context
                .get("hooks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let event_name = text(item, "eventName").ok_or(RuntimeCatalogParseError)?;
                let event = events.entry(event_name.to_owned()).or_insert_with(|| {
                    RuntimeEventPresentation {
                        runtime_event_name: event_name.to_owned(),
                        canonical_event: canonical_event(event_name),
                        description: text(item, "eventDescription")
                            .or_else(|| text(item, "description"))
                            .map(str::to_owned),
                        handlers: Vec::new(),
                    }
                });
                if event.description.is_none() {
                    event.description = text(item, "eventDescription")
                        .or_else(|| text(item, "description"))
                        .map(str::to_owned);
                }
                event.handlers.push(parse_handler(item)?);
            }
        }
        Ok(Self {
            runtime: Runtime::Codex,
            captured_at_unix_ms,
            events: events.into_values().collect(),
            issues,
        })
    }

    /// Current runtime catalog LEFT JOIN historical reliability. The catalog is
    /// the outer side: absent history cannot hide an installed handler.
    pub fn join_reliability<'a>(
        &'a self,
        history: &[HistoricalHandlerIdentity],
    ) -> Vec<JoinedRuntimeHandler<'a>> {
        self.events
            .iter()
            .flat_map(|event| {
                event
                    .handlers
                    .iter()
                    .map(move |handler| JoinedRuntimeHandler {
                        handler,
                        join: resolve_join(
                            event.canonical_event,
                            handler.reliability_key_hint.as_deref(),
                            history,
                        ),
                    })
            })
            .collect()
    }

    /// Historical identities excluded from the current catalog remain a
    /// history/Changes concern; they are never projected as installed now.
    pub fn historical_not_installed<'a>(
        &self,
        history: &'a [HistoricalHandlerIdentity],
    ) -> Vec<&'a HistoricalHandlerIdentity> {
        let joined = self
            .join_reliability(history)
            .into_iter()
            .filter_map(|joined| match joined.join {
                ReliabilityJoinState::Matched { handler_key } => Some(handler_key),
                ReliabilityJoinState::NoHistory
                | ReliabilityJoinState::Ambiguous
                | ReliabilityJoinState::Unsupported => None,
            })
            .collect::<BTreeSet<_>>();
        history
            .iter()
            .filter(|historical| !joined.contains(&historical.handler_key))
            .collect()
    }
}

fn parse_handler(item: &Value) -> Result<RuntimeHandlerPresentation, RuntimeCatalogParseError> {
    let runtime_catalog_id = text(item, "key")
        .ok_or(RuntimeCatalogParseError)?
        .to_owned();
    let managed = boolean(item, "isManaged").unwrap_or(false);
    let trust = parse_trust(text(item, "trustStatus"), managed);
    let kind = parse_handler_kind(item)?;
    let mode = boolean(item, "async").map(|is_async| {
        if is_async {
            RuntimeHandlerMode::Async
        } else {
            RuntimeHandlerMode::Sync
        }
    });
    Ok(RuntimeHandlerPresentation {
        reliability_key_hint: text(item, "reliabilityKeyHint")
            .or_else(|| text(item, "reliability_key_hint"))
            .map(str::to_owned),
        runtime_catalog_id,
        enabled: boolean(item, "enabled").unwrap_or(true),
        managed,
        needs_review: matches!(trust, RuntimeTrust::Untrusted | RuntimeTrust::Modified),
        trust,
        matcher: text(item, "matcher").map(str::to_owned),
        source: text(item, "source").map(str::to_owned),
        source_path: text(item, "sourcePath").map(str::to_owned),
        handler_kind: kind,
        mode,
        timeout_seconds: unsigned(item, "timeoutSec").or_else(|| unsigned(item, "timeout")),
        additional_context_limit: unsigned(item, "additionalContextLimit"),
    })
}

fn parse_handler_kind(item: &Value) -> Result<RuntimeHandlerKind, RuntimeCatalogParseError> {
    let label = text(item, "handlerType").unwrap_or("unknown");
    if label.eq_ignore_ascii_case("command") {
        return Ok(RuntimeHandlerKind::Command {
            command: text(item, "command").unwrap_or_default().to_owned(),
        });
    }
    if label.eq_ignore_ascii_case("mcp_tool") || label.eq_ignore_ascii_case("mcpTool") {
        return Ok(RuntimeHandlerKind::McpTool {
            server: text(item, "server")
                .or_else(|| text(item, "mcpServer"))
                .unwrap_or_default()
                .to_owned(),
            tool: text(item, "tool")
                .or_else(|| text(item, "mcpTool"))
                .unwrap_or_default()
                .to_owned(),
        });
    }
    if label.eq_ignore_ascii_case("prompt") {
        return Ok(RuntimeHandlerKind::Prompt);
    }
    if label.eq_ignore_ascii_case("agent") {
        return Ok(RuntimeHandlerKind::Agent);
    }
    Ok(RuntimeHandlerKind::Unknown {
        label: label.to_owned(),
    })
}

fn resolve_join(
    canonical_event: Option<HookEvent>,
    reliability_key_hint: Option<&str>,
    history: &[HistoricalHandlerIdentity],
) -> ReliabilityJoinState {
    let Some(event) = canonical_event else {
        return ReliabilityJoinState::Unsupported;
    };
    let Some(hint) = reliability_key_hint.filter(|value| !value.is_empty()) else {
        return ReliabilityJoinState::NoHistory;
    };
    let matches = history
        .iter()
        .filter(|historical| {
            historical.event == event && historical.reliability_key_hint.as_deref() == Some(hint)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => ReliabilityJoinState::NoHistory,
        [only] => ReliabilityJoinState::Matched {
            handler_key: only.handler_key.clone(),
        },
        _ => ReliabilityJoinState::Ambiguous,
    }
}

fn canonical_event(value: &str) -> Option<HookEvent> {
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
        // Interrupt has no admitted invocation/terminal mapping yet.
        "Interrupt" => None,
        _ => None,
    }
}

fn parse_trust(value: Option<&str>, managed: bool) -> RuntimeTrust {
    if managed {
        return RuntimeTrust::Managed;
    }
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "trusted" => RuntimeTrust::Trusted,
        "untrusted" => RuntimeTrust::Untrusted,
        "modified" => RuntimeTrust::Modified,
        _ => RuntimeTrust::Unknown,
    }
}

fn collect_issues(
    context: &Value,
    field: &str,
    severity: RuntimeCatalogIssueSeverity,
    context_name: Option<&str>,
    output: &mut Vec<RuntimeCatalogIssue>,
) {
    let Some(values) = context.get(field).and_then(Value::as_array) else {
        return;
    };
    for value in values {
        let message = value.as_str().or_else(|| text(value, "message"));
        if let Some(message) = message {
            output.push(RuntimeCatalogIssue {
                severity,
                human_message: message.to_owned(),
                context: context_name.map(str::to_owned),
            });
        }
    }
}

fn text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn boolean(value: &Value, field: &str) -> Option<bool> {
    value.get(field).and_then(Value::as_bool)
}

fn unsigned(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn catalog() -> Value {
        json!({
            "result": {"data": [{
                "cwd": "C:/synthetic/workspace",
                "warnings": ["synthetic warning"],
                "errors": ["synthetic error"],
                "hooks": [
                    {"key":"fixture:0:0","eventName":"PreToolUse","eventDescription":"Synthetic pre-tool","handlerType":"command","command":"synthetic command --with-a-very-long-safe-argument","matcher":"^SyntheticTool$","source":"user","sourcePath":"C:/synthetic/hooks.json","enabled":true,"isManaged":false,"trustStatus":"trusted","async":false,"timeoutSec":9,"additionalContextLimit":64,"reliabilityKeyHint":"safe-a"},
                    {"key":"fixture:0:1","eventName":"PostToolUse","handlerType":"mcp_tool","mcpServer":"synthetic-server","mcpTool":"synthetic-tool","source":"project","enabled":false,"isManaged":false,"trustStatus":"untrusted","reliabilityKeyHint":"safe-b"},
                    {"key":"fixture:0:2","eventName":"UserPromptSubmit","handlerType":"prompt","enabled":true,"isManaged":false,"trustStatus":"modified"},
                    {"key":"fixture:0:3","eventName":"SubagentStart","handlerType":"agent","enabled":true,"isManaged":true,"trustStatus":"trusted"},
                    {"key":"fixture:0:4","eventName":"Interrupt","handlerType":"command","command":"synthetic interrupt","enabled":true,"isManaged":false,"trustStatus":"trusted"},
                    {"key":"fixture:0:5","eventName":"FutureRuntimeEvent","handlerType":"future_handler","enabled":true,"isManaged":false,"trustStatus":"trusted"}
                ]
            }]}
        })
    }

    fn event<'a>(
        snapshot: &'a RuntimePresentationSnapshot,
        name: &str,
    ) -> &'a RuntimeEventPresentation {
        snapshot
            .events
            .iter()
            .find(|event| event.runtime_event_name == name)
            .unwrap()
    }

    #[test]
    fn preserves_codex_human_fields_only_in_memory() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        assert_eq!(snapshot.events.len(), 6);
        assert_eq!(snapshot.issues.len(), 2);
        let command = &event(&snapshot, "PreToolUse").handlers[0];
        assert_eq!(command.matcher.as_deref(), Some("^SyntheticTool$"));
        assert_eq!(command.source.as_deref(), Some("user"));
        assert_eq!(
            command.source_path.as_deref(),
            Some("C:/synthetic/hooks.json")
        );
        assert_eq!(command.mode, Some(RuntimeHandlerMode::Sync));
        assert_eq!(command.timeout_seconds, Some(9));
        assert_eq!(command.additional_context_limit, Some(64));
        assert!(matches!(
            command.handler_kind,
            RuntimeHandlerKind::Command { .. }
        ));
        assert!(matches!(
            event(&snapshot, "PostToolUse").handlers[0].handler_kind,
            RuntimeHandlerKind::McpTool { .. }
        ));
        assert!(matches!(
            event(&snapshot, "UserPromptSubmit").handlers[0].handler_kind,
            RuntimeHandlerKind::Prompt
        ));
        assert!(matches!(
            event(&snapshot, "SubagentStart").handlers[0].handler_kind,
            RuntimeHandlerKind::Agent
        ));
        assert!(event(&snapshot, "PostToolUse").handlers[0].needs_review);
        assert!(event(&snapshot, "UserPromptSubmit").handlers[0].needs_review);
        assert!(event(&snapshot, "SubagentStart").handlers[0].managed);
        assert!(!event(&snapshot, "PostToolUse").handlers[0].enabled);
        assert_eq!(event(&snapshot, "PreToolUse").installed_count(), 1);
        assert_eq!(event(&snapshot, "PreToolUse").active_count(), 1);
        assert_eq!(event(&snapshot, "PostToolUse").active_count(), 0);
        assert_eq!(event(&snapshot, "PostToolUse").needs_review_count(), 1);
    }

    #[test]
    fn preserves_interrupt_and_future_event_without_canonical_reliability() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let interrupt = event(&snapshot, "Interrupt");
        let future = event(&snapshot, "FutureRuntimeEvent");
        assert_eq!(interrupt.canonical_event, None);
        assert_eq!(future.canonical_event, None);
        let joined = snapshot.join_reliability(&[]);
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:4"
                && matches!(joined.join, ReliabilityJoinState::Unsupported)
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:5"
                && matches!(joined.join, ReliabilityJoinState::Unsupported)
        }));
    }

    #[test]
    fn joins_from_current_catalog_without_false_attribution() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let history = vec![
            HistoricalHandlerIdentity {
                handler_key: "historical-a".into(),
                event: HookEvent::PreToolUse,
                reliability_key_hint: Some("safe-a".into()),
            },
            HistoricalHandlerIdentity {
                handler_key: "historical-b".into(),
                event: HookEvent::PostToolUse,
                reliability_key_hint: Some("safe-b".into()),
            },
            HistoricalHandlerIdentity {
                handler_key: "historical-b-duplicate".into(),
                event: HookEvent::PostToolUse,
                reliability_key_hint: Some("safe-b".into()),
            },
            HistoricalHandlerIdentity {
                handler_key: "historical-only".into(),
                event: HookEvent::Stop,
                reliability_key_hint: Some("safe-z".into()),
            },
        ];
        let joined = snapshot.join_reliability(&history);
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:0"
                && matches!(&joined.join, ReliabilityJoinState::Matched { handler_key } if handler_key == "historical-a")
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:1"
                && matches!(joined.join, ReliabilityJoinState::Ambiguous)
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:2"
                && matches!(joined.join, ReliabilityJoinState::NoHistory)
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:4"
                && matches!(joined.join, ReliabilityJoinState::Unsupported)
        }));
        assert_eq!(snapshot.historical_not_installed(&history).len(), 3);
    }

    #[test]
    fn catalog_refresh_is_explicit_and_period_independent() {
        let mut resource = RuntimeCatalogResource::default();
        resource.request_initial_load();
        resource.period_switched();
        assert_eq!(resource.explicit_refreshes(), 0);
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Loading);
        resource.accepted();
        resource.period_switched();
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Ready);
        resource.request_explicit_refresh();
        assert_eq!(resource.explicit_refreshes(), 1);
        resource.failed();
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Error);
    }

    #[test]
    fn invalid_catalog_is_rejected_without_fabricating_handlers() {
        assert!(
            RuntimePresentationSnapshot::from_codex_hooks_list(&json!({"result": {}}), 42).is_err()
        );
    }
}
