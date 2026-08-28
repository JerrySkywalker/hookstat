//! Black-box conformance coverage for the in-repository HSIP v1 reference
//! producer. The producer uses the ordinary bounded local transport and is
//! never evidence authority for a runtime integration.

use hookstat::admission::IpcAdmissionState;
use hookstat::evidence::{
    AuthorityRouter, CoverageDomain, DomainAuthority, EventFamily, NativeAdmissionState, RuntimeId,
    RuntimeNeutralEvidenceCore, SourceScope,
};
use hookstat::hsip_reference::{ReferenceInvocation, ReferenceProducer, ReferenceScenario};
use hookstat::ipc::{
    BrokerConfig, BrokerHost, GroupDurabilityPolicy, IpcFrame, ObservationDisposition,
    ProducerPolicy, Wal,
};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

fn config(root: &std::path::Path) -> BrokerConfig {
    BrokerConfig {
        state_root: root.to_path_buf(),
        queue_capacity: 256,
        max_connections: 128,
        ack_timeout: Duration::from_millis(100),
        idle_timeout: Duration::from_secs(3),
        group_durability: GroupDurabilityPolicy {
            max_records: 64,
            max_bytes: 64 * 1024,
            max_interval: Duration::from_millis(50),
        },
    }
}

fn conformance_policy() -> ProducerPolicy {
    ProducerPolicy {
        connect_timeout: Duration::from_millis(100),
        acknowledgement_timeout: Duration::from_millis(100),
    }
}

fn staging_policy() -> ProducerPolicy {
    ProducerPolicy {
        // This finite staging allowance validates conformance under deliberate
        // scheduler contention. It is not the frozen release-performance
        // measurement, which retains the 1 ms / 2 ms P95/P99 gate.
        connect_timeout: Duration::from_secs(5),
        acknowledgement_timeout: Duration::from_secs(5),
    }
}

fn assert_accepted(outcomes: &[ObservationDisposition]) {
    assert!(
        outcomes
            .iter()
            .all(|outcome| *outcome == ObservationDisposition::Accepted),
        "reference scenario returned non-accepted outcomes: {outcomes:?}"
    );
}

#[test]
fn reference_producer_exercises_real_hsip_broker_wal_for_all_lifecycle_shapes() {
    let temp = tempfile::tempdir().unwrap();
    let host = BrokerHost::start(config(temp.path())).unwrap();
    let producer = ReferenceProducer::new(host.endpoint().clone(), conformance_policy()).unwrap();
    let scenarios = [
        ReferenceScenario::StartComplete,
        ReferenceScenario::StartOnly,
        ReferenceScenario::CompleteOnly,
        ReferenceScenario::DuplicateStart,
        ReferenceScenario::DuplicateComplete,
        ReferenceScenario::CompleteThenStart,
    ];

    let expected_frames = scenarios
        .iter()
        .map(|scenario| {
            assert_accepted(&producer.emit_scenario(
                &ReferenceInvocation::new(1, *scenario as u32 + 1),
                *scenario,
            ));
            match scenario {
                ReferenceScenario::StartOnly | ReferenceScenario::CompleteOnly => 1,
                ReferenceScenario::StartComplete
                | ReferenceScenario::DuplicateStart
                | ReferenceScenario::DuplicateComplete
                | ReferenceScenario::CompleteThenStart => 2,
            }
        })
        .sum::<u64>();

    let health = host.stop();
    assert_eq!(health.accepted, expected_frames);
    assert_eq!(health.dropped, 0);
    assert_eq!(health.rejected, 0);
    assert_eq!(health.ack_timeouts, 0);

    let restarted = BrokerHost::start(config(temp.path())).unwrap();
    assert_eq!(restarted.recovery().frames.len() as u64, expected_frames);
    assert_eq!(restarted.recovery().truncated_tail_bytes, 0);
    restarted.stop();
}

#[test]
fn reference_producer_reports_absence_then_recovers_after_broker_restart_without_replay() {
    let temp = tempfile::tempdir().unwrap();
    let absent = ReferenceProducer::for_state_root(temp.path()).unwrap();
    assert_eq!(
        absent.emit_scenario(
            &ReferenceInvocation::new(2, 1),
            ReferenceScenario::StartOnly
        ),
        vec![ObservationDisposition::Unavailable]
    );

    let host = BrokerHost::start(config(temp.path())).unwrap();
    let producer = ReferenceProducer::new(host.endpoint().clone(), conformance_policy()).unwrap();
    assert_accepted(&producer.emit_scenario(
        &ReferenceInvocation::new(2, 2),
        ReferenceScenario::StartOnly,
    ));
    let first_health = host.stop();
    assert_eq!(first_health.accepted, 1);
    assert!(matches!(
        producer.emit_scenario(
            &ReferenceInvocation::new(2, 2),
            ReferenceScenario::CompleteOnly,
        )[0],
        ObservationDisposition::Unavailable | ObservationDisposition::BudgetExhausted
    ));

    let restarted = BrokerHost::start(config(temp.path())).unwrap();
    assert_eq!(restarted.recovery().frames.len(), 1);
    assert_accepted(&producer.emit_scenario(
        &ReferenceInvocation::new(2, 3),
        ReferenceScenario::StartOnly,
    ));
    let restarted_health = restarted.stop();
    assert_eq!(restarted_health.accepted, 1);

    let final_host = BrokerHost::start(config(temp.path())).unwrap();
    assert_eq!(final_host.recovery().frames.len(), 2);
    final_host.stop();
}

#[test]
fn reference_producer_controlled_matrix_covers_one_five_ten_clients_and_ten_thousand_frames() {
    let temp = tempfile::tempdir().unwrap();
    let mut configuration = config(temp.path());
    configuration.ack_timeout = Duration::from_secs(5);
    let host = BrokerHost::start(configuration).unwrap();

    let mut expected_frames = 0_u64;
    for (stage, clients, samples_per_client) in
        [(1_u32, 1_u32, 1_000_u32), (2, 5, 200), (3, 10, 1_000)]
    {
        let barrier = Arc::new(Barrier::new(clients as usize));
        thread::scope(|scope| {
            for client in 0..clients {
                let barrier = Arc::clone(&barrier);
                let endpoint = host.endpoint().clone();
                scope.spawn(move || {
                    let producer = ReferenceProducer::new(endpoint, staging_policy()).unwrap();
                    barrier.wait();
                    for sequence in 0..samples_per_client {
                        assert_accepted(&producer.emit_scenario(
                            &ReferenceInvocation::new(stage * 100 + client, sequence),
                            ReferenceScenario::StartOnly,
                        ));
                    }
                });
            }
        });
        expected_frames += u64::from(clients) * u64::from(samples_per_client);
    }

    let health = host.stop();
    assert_eq!(expected_frames, 12_000);
    assert_eq!(health.accepted, expected_frames);
    assert_eq!(health.dropped, 0);
    assert_eq!(health.rejected, 0);
    assert_eq!(health.ack_timeouts, 0);
}

#[test]
fn reference_wal_valid_prefix_and_partial_tail_recovery_are_exact() {
    let temp = tempfile::tempdir().unwrap();
    let frame = IpcFrame::Start(ReferenceInvocation::new(4, 1).lifecycle());
    let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
    wal.append(&frame).unwrap();
    wal.flush_group().unwrap();
    drop(wal);
    {
        use std::io::Write;
        let mut tail = std::fs::OpenOptions::new()
            .append(true)
            .open(temp.path().join("ipc-evidence-v1.wal"))
            .unwrap();
        tail.write_all(&[0xAA, 0xBB]).unwrap();
    }

    let mut recovered_wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
    let recovered = recovered_wal.recover_and_replay().unwrap();
    assert_eq!(recovered.frames, vec![frame]);
    assert_eq!(recovered.truncated_tail_bytes, 2);
    let recovered_again = recovered_wal.recover_and_replay().unwrap();
    assert_eq!(recovered_again.frames.len(), 1);
    assert_eq!(recovered_again.truncated_tail_bytes, 0);
}

#[test]
fn reference_duplicate_replay_remains_one_canonical_invocation() {
    let temp = tempfile::tempdir().unwrap();
    let host = BrokerHost::start(config(temp.path())).unwrap();
    let producer = ReferenceProducer::new(host.endpoint().clone(), conformance_policy()).unwrap();
    assert_accepted(&producer.emit_scenario(
        &ReferenceInvocation::new(5, 1),
        ReferenceScenario::DuplicateStart,
    ));
    assert_eq!(host.stop().accepted, 2);

    let restarted = BrokerHost::start(config(temp.path())).unwrap();
    let domain = CoverageDomain {
        runtime: RuntimeId::new("hsip_reference").unwrap(),
        event: EventFamily::new("reference_event").unwrap(),
        source_scope: SourceScope::new("reference_scope").unwrap(),
    };
    let mut core = RuntimeNeutralEvidenceCore::new(
        AuthorityRouter::new([DomainAuthority {
            domain,
            native_admission: NativeAdmissionState::Qualified,
            ipc_admission: IpcAdmissionState::Admitted,
        }])
        .unwrap(),
    );
    let replay = restarted.recovery().ingest_into(&mut core).unwrap();
    assert_eq!(replay.produced, 1);
    assert_eq!(replay.duplicates, 1);
    assert_eq!(replay.shadowed, 0);
    assert_eq!(replay.not_admitted, 0);
    restarted.stop();
}
