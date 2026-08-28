//! HSIP v1 reference producer and integration-admission receipt surface.
//!
//! The reference producer is deliberately a conformance instrument. It uses
//! the same bounded `CooperativeProducer` and local protocol as an integration
//! would, but it is never a runtime adapter or production authority.

use crate::ipc::{
    BrokerAcknowledgement, Completion, ExitClassification, IPC_FRAME_HEADER_BYTES, IPC_MAGIC,
    IPC_PROTOCOL_VERSION, IpcError, IpcFrame, LifecycleFrame, LocalEndpoint, MAX_IPC_FRAME_BYTES,
    MAX_IPC_REFERENCE_BYTES, ObservationDisposition, ProducerPolicy, TerminalOutcome,
};
use crate::ipc_client::{CooperativeProducer, IpcClient};
use serde::{Deserialize, Serialize};

/// This invariant is deliberately present in the public conformance surface so
/// a reference run cannot be confused with admitted runtime coverage.
pub const REFERENCE_PRODUCER_PRODUCTION_AUTHORITY: bool = false;
/// Maximum accepted serialized candidate descriptor. This is an admission
/// metadata bound, not an HSIP lifecycle-frame allowance.
pub const MAX_INTEGRATION_CANDIDATE_JSON_BYTES: usize = 1_024;

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

    /// Produces one deterministic valid or invalid HSIP v1 decoder fixture.
    /// The values contain only synthetic structural identifiers and never raw
    /// Hook/runtime content.
    pub fn encoded_wire_fixture(&self, fixture: ReferenceWireFixture) -> Result<Vec<u8>, IpcError> {
        if fixture == ReferenceWireFixture::OversizedFrame {
            // The bounded broker reader checks this valid v1 header before it
            // allocates or reads the declared payload. This intentionally is
            // not an overlong byte vector: it proves the transport's declared
            // length guard rather than only `IpcFrame::decode`'s in-memory
            // size guard.
            let mut encoded = Vec::with_capacity(IPC_FRAME_HEADER_BYTES);
            encoded.extend_from_slice(&IPC_MAGIC);
            encoded.push(IPC_PROTOCOL_VERSION);
            encoded.push(1); // START
            encoded.extend_from_slice(&0_u16.to_le_bytes());
            let oversized_payload = u16::try_from(MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES + 1)
                .map_err(|_| IpcError::Oversized)?;
            encoded.extend_from_slice(&oversized_payload.to_le_bytes());
            return Ok(encoded);
        }
        if fixture == ReferenceWireFixture::OversizedIdentifier {
            // This fixture must exercise the decoder rather than only the
            // encoder's input validation. Start from a complete, bounded v1
            // frame and change the first reference length (the runtime) to a
            // value just over the accepted identifier bound. The payload
            // length remains internally consistent and under the frame cap;
            // `IpcFrame::decode` rejects it before any broker/WAL state can
            // observe a lifecycle frame.
            let mut encoded = IpcFrame::Start(self.lifecycle()).encode()?;
            encoded[10] =
                u8::try_from(MAX_IPC_REFERENCE_BYTES + 1).map_err(|_| IpcError::Oversized)?;
            return Ok(encoded);
        }
        let mut encoded = IpcFrame::Start(self.lifecycle()).encode()?;
        match fixture {
            ReferenceWireFixture::ValidStart => {}
            ReferenceWireFixture::MalformedMagic => encoded[0] = b'X',
            ReferenceWireFixture::UnknownVersion => encoded[4] = IPC_PROTOCOL_VERSION + 1,
            ReferenceWireFixture::UnknownFrameKind => encoded[5] = u8::MAX,
            ReferenceWireFixture::TrailingPayload => {
                let payload_length = u16::from_le_bytes([encoded[8], encoded[9]]);
                encoded[8..10].copy_from_slice(&(payload_length + 1).to_le_bytes());
                encoded.push(0);
            }
            ReferenceWireFixture::TruncatedFrame => {
                // Keep a complete bounded transport header so the fixture
                // reaches the broker decoder rather than depending on an
                // operating-system half-close. The empty START payload is
                // still structurally truncated at the HSIP lifecycle layer.
                encoded.truncate(IPC_FRAME_HEADER_BYTES);
                encoded[8..10].copy_from_slice(&0_u16.to_le_bytes());
            }
            ReferenceWireFixture::OversizedFrame | ReferenceWireFixture::OversizedIdentifier => {
                unreachable!("handled before normal frame encoding")
            }
        }
        Ok(encoded)
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

/// Deliberately bounded wire fixtures for decoder and broker conformance.
/// They are test input only: a reference producer never emits one of these as
/// lifecycle evidence during ordinary operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceWireFixture {
    ValidStart,
    MalformedMagic,
    UnknownVersion,
    UnknownFrameKind,
    OversizedFrame,
    OversizedIdentifier,
    TrailingPayload,
    TruncatedFrame,
}

/// A small wrapper around the real bounded cooperative producer. It owns no
/// broker shortcut, authority routing, analytics, configuration, or daemon.
#[derive(Clone)]
pub struct ReferenceProducer {
    producer: CooperativeProducer,
    endpoint: LocalEndpoint,
    policy: ProducerPolicy,
}

impl ReferenceProducer {
    pub fn new(endpoint: LocalEndpoint, policy: ProducerPolicy) -> Result<Self, IpcError> {
        Ok(Self {
            producer: CooperativeProducer::new(endpoint.clone(), policy)?,
            endpoint,
            policy,
        })
    }

    pub fn for_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> {
        Self::new(
            LocalEndpoint::from_state_root(root)?,
            ProducerPolicy::default(),
        )
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

    /// Sends one deliberately invalid v1 fixture through the same bounded
    /// local connection/ACK boundary used by a producer. This is conformance
    /// input only; it cannot produce canonical evidence or production
    /// authority.
    pub fn send_wire_fixture_to_broker(
        &self,
        fixture: ReferenceWireFixture,
    ) -> Result<BrokerAcknowledgement, IpcError> {
        let encoded = ReferenceInvocation::new(0, 0).encoded_wire_fixture(fixture)?;
        let mut client = IpcClient::connect_with_timeouts(
            &self.endpoint,
            self.policy.connect_timeout,
            self.policy.acknowledgement_timeout,
        )?;
        client.send_encoded_for_conformance(&encoded)
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IntegrationCandidate {
    pub integration_id: String,
    pub runtime: String,
    pub producer_version_or_sha: String,
    pub package_or_binary_sha256: String,
    pub hookstat_reference_sha: String,
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
    pub hookstat_reference_sha: String,
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
    /// Reads one bounded machine descriptor for a future integration
    /// candidate and produces a non-admitting receipt skeleton. The input has
    /// no field for a path, command, endpoint, payload, or raw output.
    pub fn from_candidate_json(input: &str) -> Result<Self, IpcError> {
        if input.len() > MAX_INTEGRATION_CANDIDATE_JSON_BYTES {
            return Err(IpcError::Invalid("integration_candidate_json"));
        }
        let candidate = serde_json::from_str(input)
            .map_err(|_| IpcError::Invalid("integration_candidate_json"))?;
        Self::for_candidate(candidate)
    }

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
        validate_hookstat_reference_sha(&candidate.hookstat_reference_sha)?;
        validate_identifier("platform", &candidate.platform)?;
        Ok(Self {
            integration_id: candidate.integration_id,
            runtime: candidate.runtime,
            producer_version_or_sha: candidate.producer_version_or_sha,
            package_or_binary_sha256: candidate.package_or_binary_sha256,
            hsip_protocol_version: IPC_PROTOCOL_VERSION,
            hookstat_reference_sha: candidate.hookstat_reference_sha,
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

fn validate_hookstat_reference_sha(value: &str) -> Result<(), IpcError> {
    // HookStat source identity may be a current Git SHA-1 commit (40 hex), a
    // SHA-256 Git object name (64 hex), or a 64-hex content hash. This field
    // is deliberately distinct from the package/binary SHA-256.
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(IpcError::Invalid("hookstat_reference_sha"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_client::{read_frame_bounded, write_frame_bounded};
    use interprocess::local_socket::prelude::*;
    use std::io::ErrorKind;
    use std::time::{Duration, Instant};

    fn candidate() -> IntegrationCandidate {
        IntegrationCandidate {
            integration_id: "example.integration".into(),
            runtime: "example_runtime".into(),
            producer_version_or_sha: "v1.2.3".into(),
            package_or_binary_sha256: "a".repeat(64),
            hookstat_reference_sha: "b".repeat(64),
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
        let mut invalid = candidate();
        invalid.hookstat_reference_sha = "f".repeat(39);
        assert!(IntegrationAdmissionReceiptSkeleton::for_candidate(invalid).is_err());
    }

    #[test]
    fn admission_skeleton_accepts_current_git_head_shape() {
        let mut candidate = candidate();
        candidate.hookstat_reference_sha = "a".repeat(40);
        let receipt = IntegrationAdmissionReceiptSkeleton::for_candidate(candidate).unwrap();
        assert_eq!(receipt.hookstat_reference_sha.len(), 40);
    }

    #[test]
    fn admission_skeleton_consumes_only_bounded_machine_candidate_metadata() {
        let input = serde_json::to_string(&candidate()).unwrap();
        let receipt = IntegrationAdmissionReceiptSkeleton::from_candidate_json(&input).unwrap();
        assert_eq!(receipt.integration_id, "example.integration");
        assert_eq!(
            receipt.admission_disposition,
            IntegrationAdmissionDisposition::Unproven
        );
        assert!(
            IntegrationAdmissionReceiptSkeleton::from_candidate_json(
                &"x".repeat(MAX_INTEGRATION_CANDIDATE_JSON_BYTES + 1)
            )
            .is_err()
        );
    }

    #[test]
    fn wire_fixtures_are_deterministic_bounded_and_fail_closed() {
        let invocation = ReferenceInvocation::new(3, 9);
        let valid = invocation
            .encoded_wire_fixture(ReferenceWireFixture::ValidStart)
            .unwrap();
        assert!(matches!(IpcFrame::decode(&valid), Ok(IpcFrame::Start(_))));

        let malformed_magic = invocation
            .encoded_wire_fixture(ReferenceWireFixture::MalformedMagic)
            .unwrap();
        assert!(matches!(
            IpcFrame::decode(&malformed_magic),
            Err(IpcError::BadMagic)
        ));
        let unknown_version = invocation
            .encoded_wire_fixture(ReferenceWireFixture::UnknownVersion)
            .unwrap();
        assert!(matches!(
            IpcFrame::decode(&unknown_version),
            Err(IpcError::UnsupportedVersion)
        ));
        let unknown_kind = invocation
            .encoded_wire_fixture(ReferenceWireFixture::UnknownFrameKind)
            .unwrap();
        assert!(matches!(
            IpcFrame::decode(&unknown_kind),
            Err(IpcError::Invalid("frame_type"))
        ));
        let oversized = invocation
            .encoded_wire_fixture(ReferenceWireFixture::OversizedFrame)
            .unwrap();
        assert_eq!(oversized.len(), IPC_FRAME_HEADER_BYTES);
        assert_eq!(
            u16::from_le_bytes([oversized[8], oversized[9]]) as usize,
            MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES + 1
        );
        assert!(matches!(
            IpcFrame::decode(&oversized),
            Err(IpcError::Invalid("frame_length"))
        ));
        let oversized_identifier = invocation
            .encoded_wire_fixture(ReferenceWireFixture::OversizedIdentifier)
            .unwrap();
        assert!(oversized_identifier.len() <= MAX_IPC_FRAME_BYTES);
        assert_eq!(
            oversized_identifier[10],
            (MAX_IPC_REFERENCE_BYTES + 1) as u8
        );
        assert!(matches!(
            IpcFrame::decode(&oversized_identifier),
            Err(IpcError::Invalid("runtime"))
        ));
        let trailing = invocation
            .encoded_wire_fixture(ReferenceWireFixture::TrailingPayload)
            .unwrap();
        assert!(matches!(
            IpcFrame::decode(&trailing),
            Err(IpcError::Invalid("trailing_payload"))
        ));
        let truncated = invocation
            .encoded_wire_fixture(ReferenceWireFixture::TruncatedFrame)
            .unwrap();
        assert!(matches!(
            IpcFrame::decode(&truncated),
            Err(IpcError::Truncated)
        ));
    }

    #[test]
    fn reference_producer_never_replays_after_an_uncertain_ack() {
        let temporary = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(temporary.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut first = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "reference producer did not connect"
                        );
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("test listener failed: {error}"),
                }
            };
            assert!(matches!(
                read_frame_bounded(&mut first, Duration::from_millis(100)),
                Ok(IpcFrame::Start(_))
            ));
            drop(first); // The broker received the frame but its ACK is lost.

            let no_replay_deadline = Instant::now() + Duration::from_millis(150);
            loop {
                match listener.accept() {
                    Ok(_) => panic!("reference producer replayed an uncertain lifecycle frame"),
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        if Instant::now() >= no_replay_deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("test listener failed after ACK loss: {error}"),
                }
            }
        });
        let producer = ReferenceProducer::new(
            endpoint,
            ProducerPolicy {
                connect_timeout: Duration::from_millis(100),
                acknowledgement_timeout: Duration::from_millis(100),
            },
        )
        .unwrap();
        assert!(matches!(
            producer.emit_scenario(
                &ReferenceInvocation::new(8, 1),
                ReferenceScenario::StartOnly
            )[0],
            ObservationDisposition::Unavailable | ObservationDisposition::BudgetExhausted
        ));
        server.join().unwrap();
    }

    #[test]
    fn reference_wire_fixture_keeps_the_configured_acknowledgement_budget() {
        let temporary = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(temporary.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut stream = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "reference fixture client did not connect"
                        );
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("test listener failed: {error}"),
                }
            };
            assert!(matches!(
                read_frame_bounded(&mut stream, Duration::from_millis(100)),
                Err(IpcError::BadMagic)
            ));
            // Deliberately exceed the endpoint-probe budget while remaining
            // within the independent acknowledgement budget.
            std::thread::sleep(Duration::from_millis(10));
            write_frame_bounded(
                &mut stream,
                &IpcFrame::Ack(BrokerAcknowledgement::Rejected),
                Duration::from_millis(100),
            )
            .unwrap();
        });
        let producer = ReferenceProducer::new(
            endpoint,
            ProducerPolicy {
                connect_timeout: Duration::from_millis(1),
                acknowledgement_timeout: Duration::from_millis(50),
            },
        )
        .unwrap();
        assert!(matches!(
            producer.send_wire_fixture_to_broker(ReferenceWireFixture::MalformedMagic),
            Ok(BrokerAcknowledgement::Rejected)
        ));
        server.join().unwrap();
    }
}
