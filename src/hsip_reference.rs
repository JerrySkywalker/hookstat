//! HSIP v1 reference producer and integration-admission receipt surface.
//!
//! The reference producer is deliberately a conformance instrument. It uses
//! the same bounded `CooperativeProducer` and local protocol as an integration
//! would, but it is never a runtime adapter or production authority.

use crate::ipc::{
    Completion, ExitClassification, IPC_PROTOCOL_VERSION, IpcError, LifecycleFrame, LocalEndpoint,
    MAX_IPC_REFERENCE_BYTES, ObservationDisposition, ProducerPolicy, TerminalOutcome,
};
use crate::ipc_client::CooperativeProducer;
use serde::{Deserialize, Serialize};

/// This invariant is deliberately present in the public conformance surface so
/// a reference run cannot be confused with admitted runtime coverage.
pub const REFERENCE_PRODUCER_PRODUCTION_AUTHORITY: bool = false;

/// A deterministic, sanitized lifecycle used by the in-repository HSIP v1
/// conformance producer. Its values are structural fixture identifiers, never
/// a prompt, command, output stream, token, or filesystem path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceInvocation {
    client: u32,
    sequence: u32,
}

impl ReferenceInvocation {
    pub const fn new(client: u32, sequence: u32) -> Self {
        Self { client, sequence }
    }

    pub fn lifecycle(&self) -> LifecycleFrame {
        LifecycleFrame {
            runtime: "hsip_reference".into(),
            runtime_instance: format!("reference_client_{}", self.client),
            invocation: format!("reference_invocation_{}_{}", self.client, self.sequence),
            handler: "reference_handler".into(),
            event: "reference_event".into(),
            source_scope: "reference_scope".into(),
            revision: Some("hsip_v1".into()),
            occurred_at_unix_ms: 1_000 + i64::from(self.sequence),
        }
    }

    pub const fn completion() -> Completion {
        Completion {
            terminal_status: TerminalOutcome::Completed,
            exit_classification: ExitClassification::ExitCode,
            exit_value: Some(0),
            duration_ms: 1,
        }
    }
}

/// Deterministic lifecycle shapes required to exercise broker and correlator
/// conformance. These are synthetic control evidence, never runtime coverage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceScenario {
    StartComplete,
    StartOnly,
    CompleteOnly,
    DuplicateStart,
    DuplicateComplete,
    CompleteThenStart,
}

/// A small wrapper around the real bounded cooperative producer. It owns no
/// broker shortcut, authority routing, analytics, configuration, or daemon.
#[derive(Clone)]
pub struct ReferenceProducer {
    producer: CooperativeProducer,
}

impl ReferenceProducer {
    pub fn new(endpoint: LocalEndpoint, policy: ProducerPolicy) -> Result<Self, IpcError> {
        Ok(Self {
            producer: CooperativeProducer::new(endpoint, policy)?,
        })
    }

    pub fn for_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> {
        Ok(Self {
            producer: CooperativeProducer::for_state_root(root)?,
        })
    }

    pub fn emit_scenario(
        &self,
        invocation: &ReferenceInvocation,
        scenario: ReferenceScenario,
    ) -> Vec<ObservationDisposition> {
        let lifecycle = invocation.lifecycle();
        let completion = ReferenceInvocation::completion();
        match scenario {
            ReferenceScenario::StartComplete => vec![
                self.producer.emit_start(lifecycle.clone()),
                self.producer.emit_complete(lifecycle, completion),
            ],
            ReferenceScenario::StartOnly => vec![self.producer.emit_start(lifecycle)],
            ReferenceScenario::CompleteOnly => {
                vec![self.producer.emit_complete(lifecycle, completion)]
            }
            ReferenceScenario::DuplicateStart => vec![
                self.producer.emit_start(lifecycle.clone()),
                self.producer.emit_start(lifecycle),
            ],
            ReferenceScenario::DuplicateComplete => vec![
                self.producer.emit_complete(lifecycle.clone(), completion),
                self.producer.emit_complete(lifecycle, completion),
            ],
            ReferenceScenario::CompleteThenStart => vec![
                self.producer.emit_complete(lifecycle.clone(), completion),
                self.producer.emit_start(lifecycle),
            ],
        }
    }
}

/// The bounded outcomes recorded by an external-integration conformance run.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceDisposition {
    Pass,
    Fail,
    Unproven,
}

/// The only permitted admission states for a named integration candidate.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationAdmissionDisposition {
    Admitted,
    NotAdmitted,
    Revoked,
    Degraded,
    Unproven,
}

/// Bounded candidate identity supplied to a future external conformance run.
/// It deliberately accepts identifiers and hashes only, never a command,
/// endpoint, raw configuration, or producer payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationCandidate {
    pub integration_id: String,
    pub runtime: String,
    pub producer_version_or_sha: String,
    pub package_or_binary_sha256: String,
    pub hookstat_sha: String,
    pub platform: String,
}

/// Machine-readable receipt skeleton for a future named integration. G38B
/// creates this skeleton without admitting or selecting that integration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IntegrationAdmissionReceiptSkeleton {
    pub integration_id: String,
    pub runtime: String,
    pub producer_version_or_sha: String,
    pub package_or_binary_sha256: String,
    pub hsip_protocol_version: u8,
    pub hookstat_sha: String,
    pub platform: String,
    pub protocol_conformance: ConformanceDisposition,
    pub correlation: ConformanceDisposition,
    pub fail_open: ConformanceDisposition,
    pub uncertain_write_duplicate_guard: ConformanceDisposition,
    pub privacy: ConformanceDisposition,
    pub security: ConformanceDisposition,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
    pub observation_gaps: Option<u64>,
    pub independent_review: ConformanceDisposition,
    pub admission_disposition: IntegrationAdmissionDisposition,
    pub reference_producer_production_authority: bool,
    pub raw_private_content_captured: bool,
}

impl IntegrationAdmissionReceiptSkeleton {
    pub fn for_candidate(candidate: IntegrationCandidate) -> Result<Self, IpcError> {
        validate_identifier("integration_id", &candidate.integration_id)?;
        validate_identifier("runtime", &candidate.runtime)?;
        validate_identifier(
            "producer_version_or_sha",
            &candidate.producer_version_or_sha,
        )?;
        validate_sha256(
            "package_or_binary_sha256",
            &candidate.package_or_binary_sha256,
        )?;
        validate_sha256("hookstat_sha", &candidate.hookstat_sha)?;
        validate_identifier("platform", &candidate.platform)?;
        Ok(Self {
            integration_id: candidate.integration_id,
            runtime: candidate.runtime,
            producer_version_or_sha: candidate.producer_version_or_sha,
            package_or_binary_sha256: candidate.package_or_binary_sha256,
            hsip_protocol_version: IPC_PROTOCOL_VERSION,
            hookstat_sha: candidate.hookstat_sha,
            platform: candidate.platform,
            protocol_conformance: ConformanceDisposition::Unproven,
            correlation: ConformanceDisposition::Unproven,
            fail_open: ConformanceDisposition::Unproven,
            uncertain_write_duplicate_guard: ConformanceDisposition::Unproven,
            privacy: ConformanceDisposition::Unproven,
            security: ConformanceDisposition::Unproven,
            p50_ms: None,
            p95_ms: None,
            p99_ms: None,
            observation_gaps: None,
            independent_review: ConformanceDisposition::Unproven,
            admission_disposition: IntegrationAdmissionDisposition::Unproven,
            reference_producer_production_authority: REFERENCE_PRODUCER_PRODUCTION_AUTHORITY,
            raw_private_content_captured: false,
        })
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), IpcError> {
    if value.is_empty()
        || value.len() > MAX_IPC_REFERENCE_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(IpcError::Invalid(field));
    }
    Ok(())
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), IpcError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(IpcError::Invalid(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> IntegrationCandidate {
        IntegrationCandidate {
            integration_id: "example.integration".into(),
            runtime: "example_runtime".into(),
            producer_version_or_sha: "v1.2.3".into(),
            package_or_binary_sha256: "a".repeat(64),
            hookstat_sha: "b".repeat(64),
            platform: "windows-x86_64".into(),
        }
    }

    #[test]
    fn reference_fixture_identity_is_stable_and_structural() {
        let first = ReferenceInvocation::new(7, 11).lifecycle();
        let second = ReferenceInvocation::new(7, 11).lifecycle();
        assert_eq!(first, second);
        assert!(first.validate().is_ok());
        assert!(!first.runtime.contains("codex"));
    }

    #[test]
    fn admission_skeleton_is_unproven_not_admitted_and_private_by_shape() {
        let receipt = IntegrationAdmissionReceiptSkeleton::for_candidate(candidate()).unwrap();
        assert_eq!(receipt.hsip_protocol_version, IPC_PROTOCOL_VERSION);
        assert_eq!(
            receipt.admission_disposition,
            IntegrationAdmissionDisposition::Unproven
        );
        assert!(!receipt.reference_producer_production_authority);
        assert!(!receipt.raw_private_content_captured);
        let json = serde_json::to_value(receipt).unwrap();
        assert!(json.get("prompt").is_none());
        assert!(json.get("command").is_none());
        assert!(json.get("stdout").is_none());
        assert!(json.get("stderr").is_none());
    }

    #[test]
    fn admission_skeleton_rejects_non_identifier_and_non_hash_input() {
        let mut invalid = candidate();
        invalid.integration_id = "not an identifier".into();
        assert!(IntegrationAdmissionReceiptSkeleton::for_candidate(invalid).is_err());
        let mut invalid = candidate();
        invalid.package_or_binary_sha256 = "not-a-sha".into();
        assert!(IntegrationAdmissionReceiptSkeleton::for_candidate(invalid).is_err());
    }
}
