//! HookStat core library.
//!
//! This train deliberately stops short of a Codex historical ingestion adapter:
//! HS-G01 did not admit a durable source with per-handler terminal outcomes. The
//! canonical model, synthetic analytics, and HookStat-owned ledger below are
//! therefore non-claiming infrastructure rather than a v0.1 runtime integration.

pub mod analytics;
pub mod domain;
pub mod ledger;
pub mod render;
pub mod report;

pub use domain::{EvidenceAdmission, EvidenceCoverage, SourceQualification};
pub use report::{MachineReport, blocked_report, synthetic_fixture_report};

/// Stable project display name.
pub const PRODUCT_NAME: &str = "HookStat";

/// Current product admission state. It must change only with a new evidence
/// qualification that meets the governed v0.1 requirements.
pub const CODEX_HISTORICAL_INGESTION_STATUS: &str = "BLOCKED_DATA_SOURCE_DECISION_REQUIRED";
