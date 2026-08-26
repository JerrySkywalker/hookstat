use hookstat::admission::{
    IpcAdmissionState, IpcIntegrationKind, V031_COOPERATIVE_IPC_ADMISSION,
    V031_TRANSPARENT_SHIM_ADMISSION,
};
use hookstat::evidence::EvidenceTransport;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn v031_admits_cooperative_ipc_but_not_the_transparent_shim() {
    assert_eq!(
        V031_COOPERATIVE_IPC_ADMISSION.integration,
        IpcIntegrationKind::Cooperative
    );
    assert_eq!(
        V031_COOPERATIVE_IPC_ADMISSION.state,
        IpcAdmissionState::Admitted
    );
    assert!(V031_COOPERATIVE_IPC_ADMISSION.production_admitted());

    assert_eq!(
        V031_TRANSPARENT_SHIM_ADMISSION.integration,
        IpcIntegrationKind::TransparentShim
    );
    assert_eq!(
        V031_TRANSPARENT_SHIM_ADMISSION.state,
        IpcAdmissionState::QualifiedNotAdmittedPerformance
    );
    assert!(!V031_TRANSPARENT_SHIM_ADMISSION.production_admitted());
}

#[test]
fn not_admitted_is_not_a_third_evidence_transport() {
    const fn transport_name(value: EvidenceTransport) -> &'static str {
        match value {
            EvidenceTransport::Native => "native",
            EvidenceTransport::Ipc => "ipc",
        }
    }

    assert_eq!(transport_name(EvidenceTransport::Native), "native");
    assert_eq!(transport_name(EvidenceTransport::Ipc), "ipc");
    assert_eq!(
        IpcAdmissionState::QualifiedNotAdmittedPerformance.as_str(),
        "qualified_not_admitted_performance"
    );
}

#[test]
fn packaged_shim_reports_non_production_admission() {
    let output = Command::new(env!("CARGO_BIN_EXE_hookstat-hook"))
        .arg("--admission-status")
        .output()
        .expect("hookstat-hook admission status must execute");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "transparent_shim=qualified_not_admitted_performance production_admitted=false"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn ordinary_activation_sources_cannot_select_the_non_admitted_shim() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/main.rs",
        "src/codex.rs",
        "src/proxy.rs",
        "src/native.rs",
        "src/runtime/codex.rs",
    ] {
        let source = fs::read_to_string(root.join(relative)).unwrap();
        assert!(
            !source.contains("hookstat-hook") && !source.contains("hookstat_hook"),
            "ordinary activation source must not select the non-admitted shim: {relative}"
        );
    }
}
