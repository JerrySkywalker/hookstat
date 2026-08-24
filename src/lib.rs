//! HookStat core library.
//!
//! The canonical ledger, analytics, report, and TUI consume `HookInvocation`
//! independently of the evidence source. v0.1 admits an opt-in Codex
//! instrumented receipt source while keeping passive evidence preferred for
//! future runtimes.

pub mod analytics;
pub mod codex;
pub mod diagnostics;
pub mod domain;
pub mod evidence;
#[allow(dead_code)]
mod hook_shim;
pub mod identity;
pub mod interface_preferences;
pub mod ipc;
#[allow(dead_code)]
mod ipc_client;
pub mod ledger;
pub mod native;
pub mod observability;
#[cfg(feature = "performance-harness")]
pub mod performance;
pub mod proxy;
#[cfg(feature = "performance-harness")]
pub mod qualification;
pub mod receipt;
pub mod render;
pub mod report;
pub mod runtime;
pub mod tui;
pub mod workbench;

pub use domain::{EvidenceAdmission, EvidenceCoverage, EvidenceSourceClass, SourceQualification};
pub use report::{MachineReport, instrumented_report, synthetic_fixture_report};

pub const PRODUCT_NAME: &str = "HookStat";
pub const CODEX_HISTORICAL_INGESTION_STATUS: &str = "ADMITTED_OPT_IN_INSTRUMENTED_RECEIPTS";
