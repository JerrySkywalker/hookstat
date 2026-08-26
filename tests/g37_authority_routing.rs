use hookstat::admission::IpcAdmissionState;
use hookstat::evidence::{
    CoverageDomain, DomainAuthority, DomainAuthoritySelection, EventFamily, NativeAdmissionState,
    RuntimeId, SourceScope,
};
use hookstat::runtime::codex::{
    CodexHostPlatform, CodexNativeCapabilityProbe, CodexNativeL2Status, CodexProtocolVersion,
};

fn domain(scope: &str) -> CoverageDomain {
    CoverageDomain {
        runtime: RuntimeId::new("codex").unwrap(),
        event: EventFamily::new("session_start").unwrap(),
        source_scope: SourceScope::new(scope).unwrap(),
    }
}

#[test]
fn ordinary_windows_codex_is_upstream_unavailable_for_native_l2() {
    let status = CodexNativeCapabilityProbe
        .ordinary_session_attach(&CodexProtocolVersion::tested(), CodexHostPlatform::Windows);

    assert_eq!(status, CodexNativeL2Status::UpstreamUnavailable);
    assert_eq!(status.native_admission(), NativeAdmissionState::Unavailable);
}

#[test]
fn unqualified_versions_fail_closed_for_native_l2() {
    let status = CodexNativeCapabilityProbe.ordinary_session_attach(
        &CodexProtocolVersion::new("future", "unqualified"),
        CodexHostPlatform::Windows,
    );

    assert_eq!(status, CodexNativeL2Status::NotQualified);
    assert_eq!(status.native_admission(), NativeAdmissionState::Unavailable);
}

#[test]
fn unavailable_native_routes_only_to_an_admitted_ipc_integration() {
    let native = CodexNativeCapabilityProbe
        .ordinary_session_attach(&CodexProtocolVersion::tested(), CodexHostPlatform::Windows)
        .native_admission();

    let cooperative = DomainAuthority {
        domain: domain("codex_user"),
        native_admission: native,
        ipc_admission: IpcAdmissionState::Admitted,
    };
    assert_eq!(
        cooperative.production_authority(),
        DomainAuthoritySelection::Ipc
    );

    let transparent = DomainAuthority {
        domain: domain("codex_project"),
        native_admission: native,
        ipc_admission: IpcAdmissionState::QualifiedNotAdmittedPerformance,
    };
    assert_eq!(
        transparent.production_authority(),
        DomainAuthoritySelection::NotAdmitted
    );
}
