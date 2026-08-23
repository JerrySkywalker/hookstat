//! Runtime-neutral contracts for runtime-owned evidence.
//!
//! These contracts intentionally do not prescribe a socket, subscription, or
//! durable-log model. An adapter owns its cursor and acquisition mechanism;
//! this module only describes the narrow hand-off into canonical evidence.

use crate::evidence::{CanonicalEvidence, NativeAdmissionState, SourceCoverage};

/// The factual surfaces a Native integration must qualify independently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCapability {
    InvocationStart,
    TerminalResult,
    StableHandlerAttribution,
    Duration,
    SourceScope,
    RevisionAttribution,
    OrderingOrCorrelation,
    ReplayOrDeliveryCharacteristics,
    EventSurfaceCompleteness,
    PrivacyBoundary,
    VersionCompatibility,
}

/// The result of qualifying one Native capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityAssessment {
    Proven,
    NotProven,
    Incompatible,
}

/// A deterministic Native qualification report.
///
/// `admission` is deliberately separate from facts: proving a notification
/// exists does not authorize it as a production denominator source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeCapabilityMatrix {
    pub invocation_start: CapabilityAssessment,
    pub terminal_result: CapabilityAssessment,
    pub stable_handler_attribution: CapabilityAssessment,
    pub duration: CapabilityAssessment,
    pub source_scope: CapabilityAssessment,
    pub revision_attribution: CapabilityAssessment,
    pub ordering_or_correlation: CapabilityAssessment,
    pub replay_or_delivery_characteristics: CapabilityAssessment,
    pub event_surface_completeness: CapabilityAssessment,
    pub privacy_boundary: CapabilityAssessment,
    pub version_compatibility: CapabilityAssessment,
    pub admission: NativeAdmissionState,
    pub source_coverage: SourceCoverage,
}

impl NativeCapabilityMatrix {
    /// Fixed ordering keeps reports and tests deterministic without a map whose
    /// keys might accidentally become a persistence schema.
    pub const fn facts(&self) -> [(NativeCapability, CapabilityAssessment); 11] {
        [
            (NativeCapability::InvocationStart, self.invocation_start),
            (NativeCapability::TerminalResult, self.terminal_result),
            (
                NativeCapability::StableHandlerAttribution,
                self.stable_handler_attribution,
            ),
            (NativeCapability::Duration, self.duration),
            (NativeCapability::SourceScope, self.source_scope),
            (
                NativeCapability::RevisionAttribution,
                self.revision_attribution,
            ),
            (
                NativeCapability::OrderingOrCorrelation,
                self.ordering_or_correlation,
            ),
            (
                NativeCapability::ReplayOrDeliveryCharacteristics,
                self.replay_or_delivery_characteristics,
            ),
            (
                NativeCapability::EventSurfaceCompleteness,
                self.event_surface_completeness,
            ),
            (NativeCapability::PrivacyBoundary, self.privacy_boundary),
            (
                NativeCapability::VersionCompatibility,
                self.version_compatibility,
            ),
        ]
    }
}

/// Reports protocol facts for an exact runtime version or schema baseline.
pub trait NativeCapabilityProbe {
    type Version;

    fn probe(&self, version: &Self::Version) -> NativeCapabilityMatrix;
}

/// Reads runtime-owned records. The cursor is intentionally adapter-owned: it
/// can represent a live session, a replay offset, a durable-log cursor, or a
/// one-shot batch without changing HookStat core.
pub trait NativeEvidenceReader {
    type Cursor;
    type Record;
    type Error;

    fn read(&mut self, cursor: &mut Self::Cursor) -> Result<Vec<Self::Record>, Self::Error>;
}

/// Converts a bounded adapter record into runtime-neutral canonical evidence.
pub trait NativeNormalizer {
    type Record;
    type Error;

    fn normalize(&self, record: &Self::Record) -> Result<CanonicalEvidence, Self::Error>;
}

/// Resolves adapter-native handler information at the runtime boundary.
///
/// A resolver may truthfully return an identity-limited result. It must never
/// manufacture a stable handler key merely to make an evidence source admitted.
pub trait RuntimeIdentityResolver {
    type Input;
    type Resolved;
    type Error;

    fn resolve(&self, input: &Self::Input) -> Result<Self::Resolved, Self::Error>;
}
