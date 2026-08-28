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
