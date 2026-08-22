//! Runtime-neutral canonical evidence and lifecycle correlation.
//!
//! This module owns neither runtime wire formats nor handler-definition
//! semantics. Runtime integrations normalize bounded metadata into
//! CanonicalEvidence, route it through one authority per coverage domain, and
//! resolve the opaque handler reference before ledger attribution.

use crate::domain::{EvidenceCoverage, TerminalStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

const SCHEMA_VERSION: u8 = 1;
const MAX_OPAQUE_REFERENCE_LEN: usize = 128;
/// Bounded ledger taxonomy marker for a reconciled lifecycle contradiction.
/// It permits a conservative, non-destructive correction of a prior terminal
/// row rather than leaving a disproven terminal result in a denominator.
pub const CORRELATION_CONFLICT_FINGERPRINT: &str = "evidence_conflict";

macro_rules! bounded_opaque_reference {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvidenceError> {
                let value = value.into();
                validate_opaque_reference($field, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

bounded_opaque_reference!(RuntimeId, "runtime");
bounded_opaque_reference!(RuntimeInstance, "runtime_instance");
bounded_opaque_reference!(InvocationKey, "invocation_key");
bounded_opaque_reference!(RuntimeHandlerRef, "runtime_handler_ref");
bounded_opaque_reference!(EventFamily, "event");
bounded_opaque_reference!(SourceScope, "source_scope");
bounded_opaque_reference!(RevisionRef, "revision_ref");

/// The only two production evidence paths.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTransport {
    Native,
    Ipc,
}

/// Runtime lifecycle facts are normalized before they become invocations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLifecycle {
    Started,
    Completed,
}

/// Qualification of the observed source surface, independent from one record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCoverage {
    Complete,
    Partial,
    EventLimited,
    IdentityLimited,
    LiveOnly,
    Durable,
    Unknown,
    SyntheticFixture,
}

/// Completeness of one correlated invocation, independent from source scope.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationCoverage {
    Complete,
    Incomplete,
    BestEffort,
    Unknown,
}

/// Lifecycle qualification state for a runtime-native source.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeAdmissionState {
    Unavailable,
    Discovered,
    Qualified,
    Admitted,
    Degraded,
    Revoked,
}

impl NativeAdmissionState {
    const fn is_admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

/// A bounded canonical lifecycle record. It contains no command, payload, or
/// stream fields and has no runtime-specific wire representation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalEvidence {
    pub schema_version: u8,
    pub runtime: RuntimeId,
    pub runtime_instance: RuntimeInstance,
    pub invocation_key: InvocationKey,
    pub runtime_handler_ref: RuntimeHandlerRef,
    pub event: EventFamily,
    pub lifecycle: EvidenceLifecycle,
    pub occurred_at_unix_ms: i64,
    pub terminal_status: Option<TerminalStatus>,
    pub duration_ms: Option<u64>,
    pub source_scope: SourceScope,
    pub revision_ref: Option<RevisionRef>,
    pub evidence_transport: EvidenceTransport,
    pub source_coverage: SourceCoverage,
    pub invocation_coverage: InvocationCoverage,
}

impl CanonicalEvidence {
    pub fn validate(&self) -> Result<(), EvidenceError> {
        if self.schema_version != SCHEMA_VERSION || self.occurred_at_unix_ms < 0 {
            return Err(EvidenceError::Invalid("canonical_evidence"));
        }
        for (field, value) in [
            ("runtime", self.runtime.as_str()),
            ("runtime_instance", self.runtime_instance.as_str()),
            ("invocation_key", self.invocation_key.as_str()),
            ("runtime_handler_ref", self.runtime_handler_ref.as_str()),
            ("event", self.event.as_str()),
            ("source_scope", self.source_scope.as_str()),
        ] {
            validate_opaque_reference(field, value)?;
        }
        if let Some(revision_ref) = &self.revision_ref {
            validate_opaque_reference("revision_ref", revision_ref.as_str())?;
        }
        if self
            .duration_ms
            .is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(EvidenceError::Invalid("duration_ms"));
        }
        match self.lifecycle {
            EvidenceLifecycle::Started
                if self.terminal_status.is_some()
                    || self.duration_ms.is_some()
                    || !matches!(
                        self.invocation_coverage,
                        InvocationCoverage::Incomplete | InvocationCoverage::Unknown
                    ) =>
            {
                Err(EvidenceError::Invalid("started_lifecycle"))
            }
            EvidenceLifecycle::Completed
                if self.terminal_status.is_none()
                    || !matches!(
                        self.invocation_coverage,
                        InvocationCoverage::Complete
                            | InvocationCoverage::BestEffort
                            | InvocationCoverage::Unknown
                    ) =>
            {
                Err(EvidenceError::Invalid("completed_lifecycle"))
            }
            _ => Ok(()),
        }
    }

    fn correlation_key(&self) -> CorrelationKey {
        CorrelationKey {
            runtime: self.runtime.clone(),
            runtime_instance: self.runtime_instance.clone(),
            invocation_key: self.invocation_key.clone(),
        }
    }

    fn coverage_domain(&self) -> CoverageDomain {
        CoverageDomain {
            runtime: self.runtime.clone(),
            event: self.event.clone(),
            source_scope: self.source_scope.clone(),
        }
    }
}

/// The domain on which exactly one transport is production authority.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CoverageDomain {
    pub runtime: RuntimeId,
    pub event: EventFamily,
    pub source_scope: SourceScope,
}

/// An explicit authority rule. Native is authoritative only after admission.
/// Every other state makes IPC the sole production authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DomainAuthority {
    pub domain: CoverageDomain,
    pub native_admission: NativeAdmissionState,
}

impl DomainAuthority {
    pub const fn production_transport(&self) -> EvidenceTransport {
        if self.native_admission.is_admitted() {
            EvidenceTransport::Native
        } else {
            EvidenceTransport::Ipc
        }
    }
}

/// A complete non-overlapping domain authority table.
#[derive(Clone, Debug, Default)]
pub struct AuthorityRouter {
    rules: BTreeMap<CoverageDomain, DomainAuthority>,
}

impl AuthorityRouter {
    pub fn new(values: impl IntoIterator<Item = DomainAuthority>) -> Result<Self, EvidenceError> {
        let mut rules = BTreeMap::new();
        for value in values {
            if rules.insert(value.domain.clone(), value).is_some() {
                return Err(EvidenceError::DuplicateAuthorityDomain);
            }
        }
        Ok(Self { rules })
    }

    pub fn route(&self, evidence: &CanonicalEvidence) -> EvidenceRoute {
        let Some(authority) = self.rules.get(&evidence.coverage_domain()) else {
            return EvidenceRoute::Unconfigured;
        };
        if evidence.evidence_transport == authority.production_transport() {
            EvidenceRoute::Production
        } else {
            EvidenceRoute::Shadow
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceRoute {
    Production,
    Shadow,
    Unconfigured,
}

/// Runtime-neutral correlation key. A changed handler reference is a
/// conflicting fact, not a second invocation to double count.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CorrelationKey {
    pub runtime: RuntimeId,
    pub runtime_instance: RuntimeInstance,
    pub invocation_key: InvocationKey,
}

/// A reconciled record ready for runtime-specific identity resolution.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorrelatedEvidence {
    pub correlation_key: CorrelationKey,
    pub runtime_handler_ref: RuntimeHandlerRef,
    pub event: EventFamily,
    pub occurred_at_unix_ms: i64,
    pub terminal_status: TerminalStatus,
    pub duration_ms: Option<u64>,
    pub source_scope: SourceScope,
    pub revision_ref: Option<RevisionRef>,
    pub evidence_transport: EvidenceTransport,
    pub source_coverage: SourceCoverage,
    pub invocation_coverage: InvocationCoverage,
    pub conflicting_evidence: bool,
}

impl CorrelatedEvidence {
    /// Maps to existing ledger coverage meanings after an external identity
    /// resolver constructs a HookInvocation.
    pub const fn legacy_coverage(&self) -> EvidenceCoverage {
        match self.invocation_coverage {
            InvocationCoverage::Incomplete | InvocationCoverage::Unknown => {
                EvidenceCoverage::Unknown
            }
            InvocationCoverage::BestEffort => EvidenceCoverage::BestEffort,
            InvocationCoverage::Complete => match self.source_coverage {
                SourceCoverage::Complete => EvidenceCoverage::Complete,
                SourceCoverage::SyntheticFixture => EvidenceCoverage::SyntheticFixture,
                SourceCoverage::Unknown => EvidenceCoverage::Unknown,
                SourceCoverage::Partial
                | SourceCoverage::EventLimited
                | SourceCoverage::IdentityLimited
                | SourceCoverage::LiveOnly
                | SourceCoverage::Durable => EvidenceCoverage::Partial,
            },
        }
    }

    pub const fn error_fingerprint(&self) -> Option<&'static str> {
        if self.conflicting_evidence {
            Some(CORRELATION_CONFLICT_FINGERPRINT)
        } else {
            None
        }
    }
}

/// Central lifecycle reconciler. Authority routing occurs before records reach
/// it, so it remains runtime- and transport-neutral.
#[derive(Clone, Debug, Default)]
pub struct EvidenceCorrelator {
    entries: BTreeMap<CorrelationKey, LifecyclePair>,
}

impl EvidenceCorrelator {
    pub fn observe(
        &mut self,
        evidence: CanonicalEvidence,
    ) -> Result<CorrelationOutcome, EvidenceError> {
        evidence.validate()?;
        let key = evidence.correlation_key();
        let pair = self.entries.entry(key.clone()).or_default();
        let changed = match evidence.lifecycle {
            EvidenceLifecycle::Started => pair.observe_start(evidence),
            EvidenceLifecycle::Completed => pair.observe_completion(evidence),
        };
        if !changed {
            return Ok(CorrelationOutcome::Duplicate);
        }
        Ok(CorrelationOutcome::Produced(pair.correlated(key)))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorrelationOutcome {
    Produced(CorrelatedEvidence),
    Duplicate,
}

#[derive(Clone, Debug, Default)]
struct LifecyclePair {
    start: Option<CanonicalEvidence>,
    completion: Option<CanonicalEvidence>,
    conflicting: bool,
}

impl LifecyclePair {
    fn observe_start(&mut self, evidence: CanonicalEvidence) -> bool {
        match &self.start {
            None => {
                self.start = Some(evidence);
                self.mark_incompatible_identity();
                true
            }
            Some(previous) if previous == &evidence => false,
            Some(_) => {
                self.conflicting = true;
                true
            }
        }
    }

    fn observe_completion(&mut self, evidence: CanonicalEvidence) -> bool {
        match &self.completion {
            None => {
                self.completion = Some(evidence);
                self.mark_incompatible_identity();
                true
            }
            Some(previous) if previous == &evidence => false,
            Some(_) => {
                self.conflicting = true;
                true
            }
        }
    }

    fn mark_incompatible_identity(&mut self) {
        let (Some(start), Some(completion)) = (&self.start, &self.completion) else {
            return;
        };
        if start.runtime_handler_ref != completion.runtime_handler_ref
            || start.event != completion.event
            || start.source_scope != completion.source_scope
            || start.revision_ref != completion.revision_ref
            || start.evidence_transport != completion.evidence_transport
        {
            self.conflicting = true;
        }
    }

    fn correlated(&self, correlation_key: CorrelationKey) -> CorrelatedEvidence {
        let base = self
            .start
            .as_ref()
            .or(self.completion.as_ref())
            .expect("a changed lifecycle pair has evidence");
        if self.conflicting {
            return CorrelatedEvidence {
                correlation_key,
                runtime_handler_ref: base.runtime_handler_ref.clone(),
                event: base.event.clone(),
                occurred_at_unix_ms: base.occurred_at_unix_ms,
                terminal_status: TerminalStatus::Unknown,
                duration_ms: None,
                source_scope: base.source_scope.clone(),
                revision_ref: base.revision_ref.clone(),
                evidence_transport: base.evidence_transport,
                source_coverage: SourceCoverage::Unknown,
                invocation_coverage: InvocationCoverage::Unknown,
                conflicting_evidence: true,
            };
        }
        match (&self.start, &self.completion) {
            (Some(start), Some(completion)) => CorrelatedEvidence {
                correlation_key,
                runtime_handler_ref: start.runtime_handler_ref.clone(),
                event: start.event.clone(),
                occurred_at_unix_ms: start.occurred_at_unix_ms,
                terminal_status: completion
                    .terminal_status
                    .unwrap_or(TerminalStatus::Unknown),
                duration_ms: completion.duration_ms,
                source_scope: start.source_scope.clone(),
                revision_ref: start.revision_ref.clone(),
                evidence_transport: start.evidence_transport,
                source_coverage: merge_source_coverage(
                    start.source_coverage,
                    completion.source_coverage,
                ),
                invocation_coverage: InvocationCoverage::Complete,
                conflicting_evidence: false,
            },
            (Some(start), None) => CorrelatedEvidence {
                correlation_key,
                runtime_handler_ref: start.runtime_handler_ref.clone(),
                event: start.event.clone(),
                occurred_at_unix_ms: start.occurred_at_unix_ms,
                terminal_status: TerminalStatus::Incomplete,
                duration_ms: None,
                source_scope: start.source_scope.clone(),
                revision_ref: start.revision_ref.clone(),
                evidence_transport: start.evidence_transport,
                source_coverage: start.source_coverage,
                invocation_coverage: InvocationCoverage::Incomplete,
                conflicting_evidence: false,
            },
            (None, Some(completion)) => CorrelatedEvidence {
                correlation_key,
                runtime_handler_ref: completion.runtime_handler_ref.clone(),
                event: completion.event.clone(),
                occurred_at_unix_ms: completion.occurred_at_unix_ms,
                terminal_status: completion
                    .terminal_status
                    .unwrap_or(TerminalStatus::Unknown),
                duration_ms: completion.duration_ms,
                source_scope: completion.source_scope.clone(),
                revision_ref: completion.revision_ref.clone(),
                evidence_transport: completion.evidence_transport,
                source_coverage: completion.source_coverage,
                invocation_coverage: InvocationCoverage::BestEffort,
                conflicting_evidence: false,
            },
            (None, None) => unreachable!("a changed lifecycle pair has evidence"),
        }
    }
}

fn merge_source_coverage(left: SourceCoverage, right: SourceCoverage) -> SourceCoverage {
    if left == right {
        left
    } else {
        SourceCoverage::Unknown
    }
}

/// Guarded production ingress. Shadow and unconfigured values cannot produce a
/// record for ledger attribution and therefore cannot enter a denominator.
#[derive(Clone, Debug)]
pub struct RuntimeNeutralEvidenceCore {
    router: AuthorityRouter,
    correlator: EvidenceCorrelator,
}

impl RuntimeNeutralEvidenceCore {
    pub fn new(router: AuthorityRouter) -> Self {
        Self {
            router,
            correlator: EvidenceCorrelator::default(),
        }
    }

    pub fn ingest(
        &mut self,
        evidence: CanonicalEvidence,
    ) -> Result<CoreIngestOutcome, EvidenceError> {
        evidence.validate()?;
        match self.router.route(&evidence) {
            EvidenceRoute::Production => match self.correlator.observe(evidence)? {
                CorrelationOutcome::Produced(value) => Ok(CoreIngestOutcome::Produced(value)),
                CorrelationOutcome::Duplicate => Ok(CoreIngestOutcome::Duplicate),
            },
            EvidenceRoute::Shadow => Ok(CoreIngestOutcome::Shadow),
            EvidenceRoute::Unconfigured => Ok(CoreIngestOutcome::Unconfigured),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreIngestOutcome {
    Produced(CorrelatedEvidence),
    Duplicate,
    Shadow,
    Unconfigured,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceError {
    Invalid(&'static str),
    DuplicateAuthorityDomain,
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(field) => write!(formatter, "invalid canonical evidence field: {field}"),
            Self::DuplicateAuthorityDomain => {
                formatter.write_str("duplicate evidence authority domain")
            }
        }
    }
}

impl std::error::Error for EvidenceError {}

fn validate_opaque_reference(field: &'static str, value: &str) -> Result<(), EvidenceError> {
    if value.is_empty()
        || value.len() > MAX_OPAQUE_REFERENCE_LEN
        || value.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':'))
        })
    {
        return Err(EvidenceError::Invalid(field));
    }
    Ok(())
}
