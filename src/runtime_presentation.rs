//! Ephemeral current-runtime hook presentation.
//!
//! This module is deliberately separate from the ledger and receipt model.
//! Runtime-owned strings such as commands, matchers, and source paths are
//! useful to render locally, but are neither evidence nor durable metadata.

use crate::domain::{HandlerIdentity, HookEvent, HookInvocation, Runtime};
use crate::identity::runtime_location_fingerprint;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

/// Presentation identity for the exact Codex v0.151.0 runtime event surface.
///
/// This identity is deliberately independent from [`HookEvent`]. A runtime
/// event can be known and localized before HookStat has admitted reliability
/// semantics for it (notably `interrupt`).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum KnownRuntimeEvent {
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PreCompact,
    PostCompact,
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    SubagentStart,
    SubagentStop,
    Stop,
    Interrupt,
}

impl KnownRuntimeEvent {
    /// The exact camelCase spelling serialized by the pinned v0.151.0 wire
    /// contract. Known aliases normalize to this value for presentation.
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::PreToolUse => "preToolUse",
            Self::PermissionRequest => "permissionRequest",
            Self::PostToolUse => "postToolUse",
            Self::PreCompact => "preCompact",
            Self::PostCompact => "postCompact",
            Self::SessionStart => "sessionStart",
            Self::SessionEnd => "sessionEnd",
            Self::UserPromptSubmit => "userPromptSubmit",
            Self::SubagentStart => "subagentStart",
            Self::SubagentStop => "subagentStop",
            Self::Stop => "stop",
            Self::Interrupt => "interrupt",
        }
    }

    /// Accepts exact v0.151.0 wire values and historical HookStat spellings
    /// that may coexist with a live response during an in-memory refresh. Only
    /// recognized values normalize; unknown runtime identities remain verbatim.
    pub fn from_codex_wire_name(value: &str) -> Option<Self> {
        match value {
            "preToolUse" | "PreToolUse" | "pre_tool_use" => Some(Self::PreToolUse),
            "permissionRequest" | "PermissionRequest" | "permission_request" => {
                Some(Self::PermissionRequest)
            }
            "postToolUse" | "PostToolUse" | "post_tool_use" => Some(Self::PostToolUse),
            "preCompact" | "PreCompact" | "pre_compact" => Some(Self::PreCompact),
            "postCompact" | "PostCompact" | "post_compact" => Some(Self::PostCompact),
            "sessionStart" | "SessionStart" | "session_start" => Some(Self::SessionStart),
            "sessionEnd" | "SessionEnd" | "session_end" => Some(Self::SessionEnd),
            "userPromptSubmit" | "UserPromptSubmit" | "user_prompt_submit" => {
                Some(Self::UserPromptSubmit)
            }
            "subagentStart" | "SubagentStart" | "subagent_start" => Some(Self::SubagentStart),
            "subagentStop" | "SubagentStop" | "subagent_stop" => Some(Self::SubagentStop),
            "stop" | "Stop" => Some(Self::Stop),
            "interrupt" | "Interrupt" => Some(Self::Interrupt),
            _ => None,
        }
    }

    pub const fn reliability_event(self) -> Option<HookEvent> {
        match self {
            Self::PreToolUse => Some(HookEvent::PreToolUse),
            Self::PermissionRequest => Some(HookEvent::PermissionRequest),
            Self::PostToolUse => Some(HookEvent::PostToolUse),
            Self::PreCompact => Some(HookEvent::PreCompact),
            Self::PostCompact => Some(HookEvent::PostCompact),
            Self::SessionStart => Some(HookEvent::SessionStart),
            Self::SessionEnd => Some(HookEvent::SessionEnd),
            Self::UserPromptSubmit => Some(HookEvent::UserPromptSubmit),
            Self::SubagentStart => Some(HookEvent::SubagentStart),
            Self::SubagentStop => Some(HookEvent::SubagentStop),
            Self::Stop => Some(HookEvent::Stop),
            Self::Interrupt => None,
        }
    }
}

/// The exact Codex v0.151.0 event surface qualified by G40. `hooks/list`
/// reports handlers, so a known event with zero current handlers needs an
/// explicit empty descriptor to remain visible in the event catalog. Unknown
/// names returned by the runtime are still added verbatim below.
const PINNED_CODEX_EVENTS: &[KnownRuntimeEvent] = &[
    KnownRuntimeEvent::PreToolUse,
    KnownRuntimeEvent::PermissionRequest,
    KnownRuntimeEvent::PostToolUse,
    KnownRuntimeEvent::PreCompact,
    KnownRuntimeEvent::PostCompact,
    KnownRuntimeEvent::SessionStart,
    KnownRuntimeEvent::SessionEnd,
    KnownRuntimeEvent::UserPromptSubmit,
    KnownRuntimeEvent::SubagentStart,
    KnownRuntimeEvent::SubagentStop,
    KnownRuntimeEvent::Stop,
    KnownRuntimeEvent::Interrupt,
];

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
    /// The runtime context (`cwd` in Codex `hooks/list`) that reported this
    /// event. It is kept local so same-named events from distinct contexts are
    /// never merged into a fabricated single current state.
    pub runtime_context: String,
    /// Canonical v0.151.0 camelCase wire spelling for a known event, or the
    /// exact raw runtime spelling for an unknown future event.
    pub runtime_event_name: String,
    pub known_event: Option<KnownRuntimeEvent>,
    /// Optional reliability identity. This can be `None` even when
    /// `known_event` is present.
    pub canonical_event: Option<HookEvent>,
    /// Runtime-provided description for an unknown event only. Known event
    /// descriptions are semantic locale resources selected during rendering.
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
    /// A privacy-safe HandlerIdentity key derived from the runtime's exact
    /// local location fields. It is present only when the runtime gives the
    /// source path and positional key material needed to prove the bridge.
    pub reliability_handler_key: Option<String>,
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
    Unavailable,
}

#[derive(Clone, Eq, PartialEq)]
pub struct HistoricalHandlerIdentity {
    pub handler_key: String,
    pub event: HookEvent,
}

impl HistoricalHandlerIdentity {
    /// Bridges admitted, persistence-safe handler identities into the
    /// presentation-time join. No raw runtime presentation material crosses
    /// this boundary.
    pub fn from_handler(handler: &HandlerIdentity) -> Self {
        Self {
            handler_key: handler.key.clone(),
            event: handler.event,
        }
    }

    /// Builds a unique identity set from admitted historical invocations.
    /// Repeated observations of one handler do not make its current-runtime
    /// join ambiguous; conflicting handler/event records remain distinct and
    /// therefore fail closed at join time when necessary.
    pub fn from_invocations(values: &[HookInvocation]) -> Vec<Self> {
        let mut identities = BTreeMap::new();
        for value in values {
            let identity = Self::from_handler(&value.handler);
            identities.insert((identity.handler_key.clone(), identity.event), identity);
        }
        identities.into_values().collect()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct JoinedRuntimeHandler<'a> {
    pub handler: &'a RuntimeHandlerPresentation,
    pub join: ReliabilityJoinState,
}

/// The reliability resource is separate from catalog discovery. A failed
/// reliability load cannot be represented as "no history" for a live hook.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ReliabilityHistory<'a> {
    Available(&'a [HistoricalHandlerIdentity]),
    Unavailable,
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
#[derive(Clone)]
pub struct RuntimeCatalogResource {
    state: RuntimeCatalogResourceState,
    explicit_refreshes: u64,
    accepted_snapshot: Option<RuntimePresentationSnapshot>,
}

impl Default for RuntimeCatalogResource {
    fn default() -> Self {
        Self {
            state: RuntimeCatalogResourceState::Empty,
            explicit_refreshes: 0,
            accepted_snapshot: None,
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

    /// The last accepted catalog survives a failed refresh as explicitly stale
    /// state. It is never replaced with ledger-derived installation guesses.
    pub fn accepted_snapshot(&self) -> Option<&RuntimePresentationSnapshot> {
        self.accepted_snapshot.as_ref()
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

    pub fn accepted(&mut self, snapshot: RuntimePresentationSnapshot) {
        self.accepted_snapshot = Some(snapshot);
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
        let mut events = BTreeMap::<(String, String), RuntimeEventPresentation>::new();
        let mut issues = Vec::new();
        for context in contexts {
            let context_name = text(context, "cwd").ok_or(RuntimeCatalogParseError)?;
            for known_event in PINNED_CODEX_EVENTS {
                let event_name = known_event.wire_name();
                events
                    .entry((context_name.to_owned(), event_name.to_owned()))
                    .or_insert_with(|| RuntimeEventPresentation {
                        runtime_context: context_name.to_owned(),
                        runtime_event_name: event_name.to_owned(),
                        known_event: Some(*known_event),
                        canonical_event: known_event.reliability_event(),
                        description: None,
                        handlers: Vec::new(),
                    });
            }
            collect_issues(
                context,
                "warnings",
                RuntimeCatalogIssueSeverity::Warning,
                Some(context_name),
                &mut issues,
            );
            collect_issues(
                context,
                "errors",
                RuntimeCatalogIssueSeverity::Error,
                Some(context_name),
                &mut issues,
            );
            let hooks = context
                .get("hooks")
                .and_then(Value::as_array)
                .ok_or(RuntimeCatalogParseError)?;
            for item in hooks {
                let raw_event_name = text(item, "eventName").ok_or(RuntimeCatalogParseError)?;
                let known_event = KnownRuntimeEvent::from_codex_wire_name(raw_event_name);
                let event_name = known_event
                    .map(KnownRuntimeEvent::wire_name)
                    .unwrap_or(raw_event_name);
                let event = events
                    .entry((context_name.to_owned(), event_name.to_owned()))
                    .or_insert_with(|| RuntimeEventPresentation {
                        runtime_context: context_name.to_owned(),
                        runtime_event_name: event_name.to_owned(),
                        known_event,
                        canonical_event: known_event.and_then(KnownRuntimeEvent::reliability_event),
                        description: known_event
                            .is_none()
                            .then(|| {
                                text(item, "eventDescription")
                                    .or_else(|| text(item, "description"))
                                    .map(str::to_owned)
                            })
                            .flatten(),
                        handlers: Vec::new(),
                    });
                if event.known_event.is_none() && event.description.is_none() {
                    event.description = text(item, "eventDescription")
                        .or_else(|| text(item, "description"))
                        .map(str::to_owned);
                }
                event
                    .handlers
                    .push(parse_handler(item, event.canonical_event)?);
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
        self.join_available_reliability(history)
    }

    /// Joins an independently loaded reliability resource without conflating a
    /// load failure with an unobserved handler.
    pub fn join_reliability_with_history<'a>(
        &'a self,
        history: ReliabilityHistory<'_>,
    ) -> Vec<JoinedRuntimeHandler<'a>> {
        match history {
            ReliabilityHistory::Available(history) => self.join_available_reliability(history),
            ReliabilityHistory::Unavailable => self
                .events
                .iter()
                .flat_map(|event| {
                    event
                        .handlers
                        .iter()
                        .map(move |handler| JoinedRuntimeHandler {
                            handler,
                            join: if event.canonical_event.is_some() {
                                ReliabilityJoinState::Unavailable
                            } else {
                                ReliabilityJoinState::Unsupported
                            },
                        })
                })
                .collect(),
        }
    }

    fn join_available_reliability<'a>(
        &'a self,
        history: &[HistoricalHandlerIdentity],
    ) -> Vec<JoinedRuntimeHandler<'a>> {
        let candidates = self
            .events
            .iter()
            .flat_map(|event| {
                event.handlers.iter().map(move |handler| {
                    history_candidates(
                        event.canonical_event,
                        handler.reliability_handler_key.as_deref(),
                        history,
                    )
                })
            })
            .collect::<Vec<_>>();
        let mut claimants = BTreeMap::<String, usize>::new();
        for candidate in candidates.iter().flatten() {
            if candidate.len() == 1 {
                *claimants
                    .entry(candidate.iter().next().unwrap().clone())
                    .or_default() += 1;
            }
        }
        self.events
            .iter()
            .flat_map(|event| event.handlers.iter().map(move |handler| (event, handler)))
            .zip(candidates)
            .map(|((event, handler), candidate)| JoinedRuntimeHandler {
                handler,
                join: resolve_join(event.canonical_event, candidate, &claimants),
            })
            .collect()
    }

    /// Historical identities excluded from the current catalog remain a
    /// history/Changes concern; they are never projected as installed now.
    pub fn historical_not_installed<'a>(
        &self,
        history: &'a [HistoricalHandlerIdentity],
    ) -> Vec<&'a HistoricalHandlerIdentity> {
        let installed = self
            .events
            .iter()
            .flat_map(|event| {
                event.handlers.iter().filter_map(move |handler| {
                    let event = event.canonical_event?;
                    let handler_key = handler
                        .reliability_handler_key
                        .as_deref()
                        .filter(|value| !value.is_empty())?;
                    Some((handler_key.to_owned(), event.as_storage().to_owned()))
                })
            })
            .collect::<BTreeSet<_>>();
        history
            .iter()
            .filter(|historical| {
                !installed.contains(&(
                    historical.handler_key.clone(),
                    historical.event.as_storage().to_owned(),
                ))
            })
            .collect()
    }
}

fn parse_handler(
    item: &Value,
    canonical_event: Option<HookEvent>,
) -> Result<RuntimeHandlerPresentation, RuntimeCatalogParseError> {
    let runtime_catalog_id = text(item, "key")
        .ok_or(RuntimeCatalogParseError)?
        .to_owned();
    let managed = boolean(item, "isManaged").ok_or(RuntimeCatalogParseError)?;
    let enabled = boolean(item, "enabled").ok_or(RuntimeCatalogParseError)?;
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
        reliability_handler_key: runtime_reliability_handler_key(item, canonical_event),
        runtime_catalog_id,
        enabled,
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

fn runtime_reliability_handler_key(
    item: &Value,
    canonical_event: Option<HookEvent>,
) -> Option<String> {
    let event = canonical_event?;
    let source_path = text(item, "sourcePath")?;
    let raw_key = text(item, "key")?;
    let parts = raw_key.split(':').collect::<Vec<_>>();
    let group_index = parts
        .get(parts.len().checked_sub(2)?)
        .and_then(|value| value.parse::<usize>().ok())?;
    let handler_index = parts.last().and_then(|value| value.parse::<usize>().ok())?;
    Some(format!(
        "hk_{}",
        runtime_location_fingerprint(Path::new(source_path), event, group_index, handler_index)
    ))
}

fn history_candidates(
    canonical_event: Option<HookEvent>,
    reliability_handler_key: Option<&str>,
    history: &[HistoricalHandlerIdentity],
) -> Option<BTreeSet<String>> {
    let event = canonical_event?;
    let handler_key = reliability_handler_key.filter(|value| !value.is_empty())?;
    Some(
        history
            .iter()
            .filter(|historical| historical.event == event && historical.handler_key == handler_key)
            .map(|historical| historical.handler_key.clone())
            .collect(),
    )
}

fn resolve_join(
    canonical_event: Option<HookEvent>,
    candidates: Option<BTreeSet<String>>,
    claimants: &BTreeMap<String, usize>,
) -> ReliabilityJoinState {
    if canonical_event.is_none() {
        return ReliabilityJoinState::Unsupported;
    }
    let Some(candidates) = candidates else {
        return ReliabilityJoinState::NoHistory;
    };
    if candidates.is_empty() {
        return ReliabilityJoinState::NoHistory;
    }
    if candidates.len() != 1 {
        return ReliabilityJoinState::Ambiguous;
    }
    let Some(handler_key) = candidates.into_iter().next() else {
        return ReliabilityJoinState::Ambiguous;
    };
    if claimants.get(&handler_key) == Some(&1) {
        ReliabilityJoinState::Matched { handler_key }
    } else {
        ReliabilityJoinState::Ambiguous
    }
}

fn parse_trust(value: Option<&str>, managed: bool) -> RuntimeTrust {
    if managed {
        return RuntimeTrust::Managed;
    }
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "trusted" | "not_required" => RuntimeTrust::Trusted,
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
    use crate::domain::ExecutionMode;
    use serde_json::json;

    fn catalog() -> Value {
        json!({
            "result": {"data": [{
                "cwd": "C:/synthetic/workspace",
                "warnings": ["synthetic warning"],
                "errors": ["synthetic error"],
                "hooks": [
                    {"key":"fixture:0:0","eventName":"preToolUse","handlerType":"command","command":"synthetic command --with-a-very-long-safe-argument","matcher":"^SyntheticTool$","source":"user","sourcePath":"C:/synthetic/hooks.json","enabled":true,"isManaged":false,"trustStatus":"trusted","async":false,"timeoutSec":9,"additionalContextLimit":64},
                    {"key":"fixture:0:1","eventName":"postToolUse","handlerType":"mcp_tool","mcpServer":"synthetic-server","mcpTool":"synthetic-tool","source":"project","sourcePath":"C:/synthetic/hooks.json","enabled":false,"isManaged":false,"trustStatus":"untrusted"},
                    {"key":"fixture:0:2","eventName":"userPromptSubmit","handlerType":"prompt","enabled":true,"isManaged":false,"trustStatus":"modified"},
                    {"key":"fixture:0:3","eventName":"subagentStart","handlerType":"agent","enabled":true,"isManaged":true,"trustStatus":"trusted"},
                    {"key":"fixture:0:4","eventName":"interrupt","handlerType":"command","command":"synthetic interrupt","enabled":true,"isManaged":false,"trustStatus":"trusted"},
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

    fn event_in_context<'a>(
        snapshot: &'a RuntimePresentationSnapshot,
        context: &str,
        name: &str,
    ) -> &'a RuntimeEventPresentation {
        snapshot
            .events
            .iter()
            .find(|event| event.runtime_context == context && event.runtime_event_name == name)
            .unwrap()
    }

    fn observed_identity(
        snapshot: &RuntimePresentationSnapshot,
        event_name: &str,
    ) -> HistoricalHandlerIdentity {
        let event = event(snapshot, event_name);
        let key = event.handlers[0]
            .reliability_handler_key
            .clone()
            .expect("synthetic local handler has a safe join key");
        HistoricalHandlerIdentity::from_handler(&HandlerIdentity {
            key,
            revision: "hr_synthetic".into(),
            label: "Synthetic observed handler".into(),
            source_kind: "synthetic".into(),
            event: event.canonical_event.expect("canonical synthetic event"),
            matcher_identity: "m_synthetic".into(),
            structural_identity: "synthetic:0".into(),
            execution_mode: ExecutionMode::Sync,
        })
    }

    fn catalog_with_duplicate_pre_tool_handler() -> Value {
        let mut value = catalog();
        let duplicate = value["result"]["data"][0]["hooks"][0].clone();
        value["result"]["data"][0]["hooks"]
            .as_array_mut()
            .expect("synthetic hooks array")
            .push(duplicate);
        value
    }

    #[test]
    fn preserves_codex_human_fields_only_in_memory() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        assert_eq!(snapshot.events.len(), PINNED_CODEX_EVENTS.len() + 1);
        assert_eq!(snapshot.issues.len(), 2);
        let command = &event(&snapshot, "preToolUse").handlers[0];
        assert_eq!(
            event(&snapshot, "preToolUse").runtime_context,
            "C:/synthetic/workspace"
        );
        assert_eq!(command.matcher.as_deref(), Some("^SyntheticTool$"));
        assert_eq!(command.source.as_deref(), Some("user"));
        assert_eq!(
            command.source_path.as_deref(),
            Some("C:/synthetic/hooks.json")
        );
        assert_eq!(command.mode, Some(RuntimeHandlerMode::Sync));
        assert_eq!(command.timeout_seconds, Some(9));
        assert_eq!(command.additional_context_limit, Some(64));
        assert_eq!(
            command.reliability_handler_key,
            Some(format!(
                "hk_{}",
                runtime_location_fingerprint(
                    Path::new("C:/synthetic/hooks.json"),
                    HookEvent::PreToolUse,
                    0,
                    0,
                )
            ))
        );
        assert!(matches!(
            command.handler_kind,
            RuntimeHandlerKind::Command { .. }
        ));
        assert!(matches!(
            event(&snapshot, "postToolUse").handlers[0].handler_kind,
            RuntimeHandlerKind::McpTool { .. }
        ));
        assert!(matches!(
            event(&snapshot, "userPromptSubmit").handlers[0].handler_kind,
            RuntimeHandlerKind::Prompt
        ));
        assert!(matches!(
            event(&snapshot, "subagentStart").handlers[0].handler_kind,
            RuntimeHandlerKind::Agent
        ));
        assert!(event(&snapshot, "postToolUse").handlers[0].needs_review);
        assert!(event(&snapshot, "userPromptSubmit").handlers[0].needs_review);
        assert!(event(&snapshot, "subagentStart").handlers[0].managed);
        assert!(!event(&snapshot, "postToolUse").handlers[0].enabled);
        assert_eq!(event(&snapshot, "preToolUse").installed_count(), 1);
        assert_eq!(
            event(&snapshot, "preToolUse").known_event,
            Some(KnownRuntimeEvent::PreToolUse)
        );
        assert_eq!(event(&snapshot, "preToolUse").description, None);
        assert_eq!(event(&snapshot, "preToolUse").active_count(), 1);
        assert_eq!(event(&snapshot, "postToolUse").active_count(), 0);
        assert_eq!(event(&snapshot, "postToolUse").needs_review_count(), 1);
        let zero_handler = event(&snapshot, "sessionEnd");
        assert_eq!(zero_handler.installed_count(), 0);
        assert_eq!(zero_handler.active_count(), 0);
        assert_eq!(zero_handler.needs_review_count(), 0);
        assert_eq!(
            event(&snapshot, "interrupt").known_event,
            Some(KnownRuntimeEvent::Interrupt)
        );
        assert_eq!(event(&snapshot, "interrupt").description, None);
    }

    #[test]
    fn owner_observed_pinned_plus_real_wire_event_has_one_semantic_row() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let pre_tool_use = snapshot
            .events
            .iter()
            .filter(|event| {
                event.runtime_context == "C:/synthetic/workspace"
                    && event.known_event == Some(KnownRuntimeEvent::PreToolUse)
            })
            .collect::<Vec<_>>();
        assert_eq!(pre_tool_use.len(), 1);
        assert_eq!(pre_tool_use[0].runtime_event_name, "preToolUse");
        assert_eq!(pre_tool_use[0].installed_count(), 1);
    }

    #[test]
    fn same_context_known_casing_variants_share_presentation_identity() {
        let mut response = catalog();
        let mut legacy_alias = response["result"]["data"][0]["hooks"][0].clone();
        legacy_alias["key"] = json!("fixture:0:6");
        legacy_alias["eventName"] = json!("PreToolUse");
        response["result"]["data"][0]["hooks"]
            .as_array_mut()
            .unwrap()
            .push(legacy_alias);

        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&response, 42).unwrap();
        let rows = snapshot
            .events
            .iter()
            .filter(|event| event.known_event == Some(KnownRuntimeEvent::PreToolUse))
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].runtime_context, "C:/synthetic/workspace");
        assert_eq!(rows[0].runtime_event_name, "preToolUse");
        assert_eq!(rows[0].installed_count(), 2);
    }

    #[test]
    fn requires_authoritative_status_fields_and_preserves_not_required_trust() {
        let mut missing_enabled = catalog();
        missing_enabled["result"]["data"][0]["hooks"][0]
            .as_object_mut()
            .unwrap()
            .remove("enabled");
        assert!(RuntimePresentationSnapshot::from_codex_hooks_list(&missing_enabled, 42).is_err());

        let mut missing_managed = catalog();
        missing_managed["result"]["data"][0]["hooks"][0]
            .as_object_mut()
            .unwrap()
            .remove("isManaged");
        assert!(RuntimePresentationSnapshot::from_codex_hooks_list(&missing_managed, 42).is_err());

        let mut not_required = catalog();
        not_required["result"]["data"][0]["hooks"][0]["trustStatus"] = json!("not_required");
        let snapshot =
            RuntimePresentationSnapshot::from_codex_hooks_list(&not_required, 42).unwrap();
        assert!(matches!(
            event(&snapshot, "preToolUse").handlers[0].trust,
            RuntimeTrust::Trusted
        ));
        assert_eq!(event(&snapshot, "preToolUse").active_count(), 1);
    }

    #[test]
    fn preserves_same_named_events_from_distinct_runtime_contexts() {
        let mut response = catalog();
        let mut second_handler = response["result"]["data"][0]["hooks"][0].clone();
        second_handler["key"] = json!("fixture:1:0");
        response["result"]["data"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "cwd": "C:/synthetic/second-workspace",
                "hooks": [second_handler]
            }));

        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&response, 42).unwrap();
        assert_eq!(snapshot.events.len(), PINNED_CODEX_EVENTS.len() * 2 + 1);
        assert_eq!(
            event_in_context(&snapshot, "C:/synthetic/second-workspace", "preToolUse").handlers[0]
                .runtime_catalog_id,
            "fixture:1:0"
        );
    }

    #[test]
    fn preserves_interrupt_and_future_event_without_canonical_reliability() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let interrupt = event(&snapshot, "interrupt");
        let future = event(&snapshot, "FutureRuntimeEvent");
        assert_eq!(interrupt.known_event, Some(KnownRuntimeEvent::Interrupt));
        assert_eq!(future.known_event, None);
        assert_eq!(future.runtime_event_name, "FutureRuntimeEvent");
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
    fn normalizes_existing_codex_wire_event_forms() {
        let mut response = catalog();
        response["result"]["data"][0]["hooks"][0]["eventName"] = json!("sessionStart");
        response["result"]["data"][0]["hooks"][1]["eventName"] = json!("stop");
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&response, 42).unwrap();
        assert_eq!(
            event(&snapshot, "sessionStart").canonical_event,
            Some(HookEvent::SessionStart)
        );
        assert_eq!(
            event(&snapshot, "stop").canonical_event,
            Some(HookEvent::Stop)
        );
    }

    #[test]
    fn joins_from_current_catalog_through_admitted_handler_identity() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let history = vec![
            observed_identity(&snapshot, "preToolUse"),
            HistoricalHandlerIdentity {
                handler_key: "historical-only".into(),
                event: HookEvent::Stop,
            },
        ];
        let joined = snapshot.join_reliability(&history);
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:0"
                && matches!(&joined.join, ReliabilityJoinState::Matched { handler_key } if handler_key == &history[0].handler_key)
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:2"
                && matches!(joined.join, ReliabilityJoinState::NoHistory)
        }));
        assert!(joined.iter().any(|joined| {
            joined.handler.runtime_catalog_id == "fixture:0:4"
                && matches!(joined.join, ReliabilityJoinState::Unsupported)
        }));
        assert_eq!(snapshot.historical_not_installed(&history).len(), 1);
    }

    #[test]
    fn historical_identity_requires_matching_event_and_handler_key() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let observed = observed_identity(&snapshot, "preToolUse");
        let same_key_other_event = HistoricalHandlerIdentity {
            handler_key: observed.handler_key.clone(),
            event: HookEvent::Stop,
        };
        let history = [observed, same_key_other_event];
        let historical_only = snapshot.historical_not_installed(&history);
        assert_eq!(historical_only.len(), 1);
        assert!(matches!(historical_only[0].event, HookEvent::Stop));
    }

    #[test]
    fn duplicate_current_candidates_are_ambiguous_and_not_reported_as_removed() {
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(
            &catalog_with_duplicate_pre_tool_handler(),
            42,
        )
        .unwrap();
        let history = vec![
            observed_identity(&snapshot, "preToolUse"),
            HistoricalHandlerIdentity {
                handler_key: "historical-only".into(),
                event: HookEvent::Stop,
            },
        ];
        let joined = snapshot.join_reliability(&history);
        assert_eq!(
            joined
                .iter()
                .filter(|joined| joined.handler.runtime_catalog_id == "fixture:0:0")
                .count(),
            2
        );
        assert!(
            joined
                .iter()
                .filter(|joined| joined.handler.runtime_catalog_id == "fixture:0:0")
                .all(|joined| matches!(joined.join, ReliabilityJoinState::Ambiguous))
        );
        let historical_only = snapshot.historical_not_installed(&history);
        assert_eq!(historical_only.len(), 1);
        assert_eq!(historical_only[0].handler_key, "historical-only");
    }

    #[test]
    fn catalog_refresh_is_explicit_and_period_independent() {
        let mut resource = RuntimeCatalogResource::default();
        resource.request_initial_load();
        resource.period_switched();
        assert_eq!(resource.explicit_refreshes(), 0);
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Loading);
        let snapshot = RuntimePresentationSnapshot::from_codex_hooks_list(&catalog(), 42).unwrap();
        let observed = observed_identity(&snapshot, "preToolUse");
        resource.accepted(snapshot);
        resource.period_switched();
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Ready);
        resource.request_explicit_refresh();
        assert_eq!(resource.explicit_refreshes(), 1);
        resource.failed();
        assert_eq!(resource.state(), RuntimeCatalogResourceState::Error);
        assert!(resource.accepted_snapshot().is_some());
        let catalog_after_failure = resource.accepted_snapshot().unwrap();
        assert!(
            catalog_after_failure
                .join_reliability(&[observed])
                .iter()
                .any(|joined| matches!(joined.join, ReliabilityJoinState::Matched { .. }))
        );
        assert!(
            catalog_after_failure
                .join_reliability_with_history(ReliabilityHistory::Unavailable)
                .iter()
                .any(|joined| {
                    joined.handler.runtime_catalog_id == "fixture:0:0"
                        && matches!(joined.join, ReliabilityJoinState::Unavailable)
                })
        );
    }

    #[test]
    fn invalid_catalog_is_rejected_without_fabricating_handlers() {
        assert!(
            RuntimePresentationSnapshot::from_codex_hooks_list(&json!({"result": {}}), 42).is_err()
        );
        assert!(
            RuntimePresentationSnapshot::from_codex_hooks_list(
                &json!({"result": {"data": [{"cwd": "C:/synthetic/workspace"}]}}),
                42,
            )
            .is_err()
        );
    }
}
