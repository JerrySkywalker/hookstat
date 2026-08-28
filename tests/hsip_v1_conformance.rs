//! Black-box conformance coverage for the in-repository HSIP v1 reference
//! producer. The producer uses the ordinary bounded local transport and is
//! never evidence authority for a runtime integration.

use hookstat::hsip_reference::{
    REFERENCE_PRODUCER_PRODUCTION_AUTHORITY, ReferenceInvocation, ReferenceProducer,
    ReferenceScenario,
};
use hookstat::ipc::{
    BrokerConfig, BrokerHost, GroupDurabilityPolicy, ObservationDisposition, ProducerPolicy,
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
    assert!(!REFERENCE_PRODUCER_PRODUCTION_AUTHORITY);
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

    let restarted = BrokerHost::start(config(temp.path())).unwrap();
    assert_eq!(restarted.recovery().frames.len(), 1);
    let reconnected =
        ReferenceProducer::new(restarted.endpoint().clone(), conformance_policy()).unwrap();
    assert_accepted(&reconnected.emit_scenario(
        &ReferenceInvocation::new(2, 3),
        ReferenceScenario::CompleteOnly,
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
