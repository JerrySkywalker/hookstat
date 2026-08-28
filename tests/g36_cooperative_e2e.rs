//! G36 cooperative producer proof against the actual G35 broker/WAL.

#[allow(dead_code)]
#[path = "../src/ipc_client.rs"]
mod ipc_client;

use hookstat::ipc::{BrokerConfig, BrokerHost};
use ipc_client::{
    Completion, CooperativeProducer, ExitClassification, LifecycleFrame, LocalEndpoint,
    ObservationDisposition, ProducerPolicy, TerminalOutcome,
};
use std::time::Duration;

fn lifecycle() -> LifecycleFrame {
    LifecycleFrame {
        runtime: "controlled_runtime".into(),
        runtime_instance: "controlled_instance".into(),
        invocation: "controlled_invocation".into(),
        handler: "controlled_handler".into(),
        event: "controlled_event".into(),
        source_scope: "controlled_scope".into(),
        revision: Some("controlled_revision".into()),
        occurred_at_unix_ms: 1_700_000_000_000,
    }
}

fn staging_producer(root: &std::path::Path) -> CooperativeProducer {
    CooperativeProducer::new(
        LocalEndpoint::from_state_root(root).unwrap(),
        ProducerPolicy {
            connect_timeout: Duration::from_millis(100),
            acknowledgement_timeout: Duration::from_millis(100),
        },
    )
    .unwrap()
}

#[test]
fn cooperative_start_complete_reaches_the_broker_without_application_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = BrokerConfig::for_state_root(temp.path());
    config.ack_timeout = Duration::from_millis(100);
    let host = BrokerHost::start(config).unwrap();
    let producer = staging_producer(temp.path());
    assert_eq!(
        producer.emit_start(lifecycle()),
        ObservationDisposition::Accepted
    );
    assert_eq!(
        producer.emit_complete(
            lifecycle(),
            Completion {
                terminal_status: TerminalOutcome::Completed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(0),
                duration_ms: 1,
            },
        ),
        ObservationDisposition::Accepted
    );
    assert_eq!(host.health().accepted, 2);
    host.stop();
}

#[test]
fn broker_absence_and_mid_lifecycle_exit_are_fail_open_observation_gaps() {
    let absent = tempfile::tempdir().unwrap();
    let absent_producer = staging_producer(absent.path());
    assert_eq!(
        absent_producer.emit_start(lifecycle()),
        ObservationDisposition::Unavailable
    );

    let temp = tempfile::tempdir().unwrap();
    let mut config = BrokerConfig::for_state_root(temp.path());
    config.ack_timeout = Duration::from_millis(100);
    let host = BrokerHost::start(config).unwrap();
    let producer = staging_producer(temp.path());
    assert_eq!(
        producer.emit_start(lifecycle()),
        ObservationDisposition::Accepted
    );
    host.stop();
    assert_eq!(
        producer.emit_complete(
            lifecycle(),
            Completion {
                terminal_status: TerminalOutcome::Completed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(0),
                duration_ms: 1,
            },
        ),
        ObservationDisposition::Unavailable
    );
}
