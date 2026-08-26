//! Release-visible admission states for IPC integrations.
//!
//! Admission is independent from `EvidenceTransport`: cooperative producers
//! and the transparent shim both use IPC, while a non-admitted integration is
//! a coverage state and never a third evidence path.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcAdmissionState {
    #[default]
    Unavailable,
    Qualified,
    Admitted,
    QualifiedNotAdmittedPerformance,
    Degraded,
    Revoked,
}

impl IpcAdmissionState {
    pub const fn is_admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Qualified => "qualified",
            Self::Admitted => "admitted",
            Self::QualifiedNotAdmittedPerformance => "qualified_not_admitted_performance",
            Self::Degraded => "degraded",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcIntegrationKind {
    Cooperative,
    TransparentShim,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IpcIntegrationAdmission {
    pub integration: IpcIntegrationKind,
    pub state: IpcAdmissionState,
}

impl IpcIntegrationAdmission {
    pub const fn production_admitted(self) -> bool {
        self.state.is_admitted()
    }
}

pub const V031_COOPERATIVE_IPC_ADMISSION: IpcIntegrationAdmission = IpcIntegrationAdmission {
    integration: IpcIntegrationKind::Cooperative,
    state: IpcAdmissionState::Admitted,
};

pub const V031_TRANSPARENT_SHIM_ADMISSION: IpcIntegrationAdmission = IpcIntegrationAdmission {
    integration: IpcIntegrationKind::TransparentShim,
    state: IpcAdmissionState::QualifiedNotAdmittedPerformance,
};
