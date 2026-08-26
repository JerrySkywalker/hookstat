use hookstat::admission::{V031_COOPERATIVE_IPC_ADMISSION, V031_TRANSPARENT_SHIM_ADMISSION};
use hookstat::ipc::{
    BrokerConfig, BrokerHost, IpcError, IpcFrame, MAX_IPC_FRAME_BYTES, MAX_WAL_BYTES,
};

#[test]
fn producer_and_transport_remain_bounded_without_persistence_or_cli_dependencies() {
    assert_eq!(MAX_IPC_FRAME_BYTES, 1_024);
    assert_eq!(MAX_WAL_BYTES, 64 * 1_024 * 1_024);

    let producer_source = include_str!("../src/ipc_client.rs")
        .split_once("\n#[cfg(test)]")
        .expect("the source guard must exclude only its test module")
        .0;
    for forbidden in [
        "serde_json",
        "sync_data",
        "ReceiptSpool",
        "crate::ledger",
        "crate::cli",
        "std::process::Command",
    ] {
        assert!(
            !producer_source.contains(forbidden),
            "cooperative producer gained forbidden hot-path dependency: {forbidden}"
        );
    }

    let temporary = tempfile::tempdir().unwrap();
    let mut invalid_queue = BrokerConfig::for_state_root(temporary.path());
    invalid_queue.queue_capacity = 16_385;
    assert!(matches!(
        BrokerHost::start(invalid_queue),
        Err(IpcError::Invalid("broker_config"))
    ));
    assert!(!temporary.path().join("ipc").exists());

    let mut invalid_connections = BrokerConfig::for_state_root(temporary.path());
    invalid_connections.max_connections = 129;
    assert!(matches!(
        BrokerHost::start(invalid_connections),
        Err(IpcError::Invalid("broker_config"))
    ));
    assert!(!temporary.path().join("ipc").exists());
}

#[test]
fn diagnostics_remain_control_plane_and_release_authority_stays_locked() {
    assert!(!IpcFrame::BrokerDiagnosticsRequest.is_lifecycle());
    assert!(V031_COOPERATIVE_IPC_ADMISSION.production_admitted());
    assert!(!V031_TRANSPARENT_SHIM_ADMISSION.production_admitted());

    let diagnostics_source = include_str!("../src/diagnostics.rs");
    assert!(diagnostics_source.contains("shadow_in_denominator: false"));
    assert!(diagnostics_source.contains("transparent_shim_active: false"));
    assert!(diagnostics_source.contains("evidence_transport_count: 2"));
    assert!(diagnostics_source.contains("third_transport_present: false"));
}
