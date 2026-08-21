//! Read-only operational diagnostics shared by the CLI and Reliability Center.
//!
//! The model contains only stable identifiers, bounded counts, versions, and
//! evidence timestamps. It never retains configuration paths, hook commands,
//! receipt payloads, process output, credentials, or session content.

use crate::codex::{self, EffectiveDiscoverySummary, InstrumentationDisposition};
use crate::domain::{EvidenceCoverage, Runtime};
use crate::ledger::Ledger;
use crate::receipt::{ReceiptScan, ReceiptSpool};
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DIAGNOSTICS_SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Fail,
    Unknown,
    Unsupported,
}

impl DiagnosticStatus {
    pub const fn severity(self) -> u8 {
        match self {
            Self::Pass | Self::Unsupported => 0,
            Self::Unknown | Self::Warning => 1,
            Self::Fail => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCheckId {
    HookStatBinary,
    CodexBinary,
    EffectiveRuntime,
    Instrumentation,
    Trust,
    ReceiptSpool,
    Ledger,
    ReceiptIntegrity,
    EvidenceCoverage,
    PathIdentity,
    EvidenceFreshness,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DiagnosticFact {
    Runtime {
        runtime: Runtime,
    },
    Version {
        value: String,
    },
    HandlerCounts {
        discovered: u64,
        instrumented: u64,
        unsupported: u64,
    },
    LedgerInvocations {
        count: u64,
    },
    ReceiptRecords {
        count: u64,
    },
    ReceiptIntegrity {
        incomplete: u64,
        malformed: u64,
    },
    Coverage {
        coverage: EvidenceCoverage,
    },
    EvidenceAgeMinutes {
        age_minutes: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticCheck {
    pub id: DiagnosticCheckId,
    pub status: DiagnosticStatus,
    pub facts: Vec<DiagnosticFact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticsReport {
    pub schema_version: u8,
    pub read_only: bool,
    pub generated_at_unix_ms: i64,
    pub overall_status: DiagnosticStatus,
    pub checks: Vec<DiagnosticCheck>,
}

impl DiagnosticsReport {
    pub fn empty(now_unix_ms: i64) -> Self {
        Self {
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            read_only: true,
            generated_at_unix_ms: now_unix_ms,
            overall_status: DiagnosticStatus::Unknown,
            checks: Vec::new(),
        }
    }
}

/// Collects the supported diagnostic checks without creating, repairing, or
/// changing HookStat, Codex, receipt, or trust state.
pub fn collect(data_root: &Path, now_unix_ms: i64) -> DiagnosticsReport {
    let mut checks = vec![DiagnosticCheck {
        id: DiagnosticCheckId::HookStatBinary,
        status: DiagnosticStatus::Pass,
        facts: vec![DiagnosticFact::Version {
            value: env!("CARGO_PKG_VERSION").to_owned(),
        }],
    }];

    checks.push(codex_binary_check());

    let static_discovery = codex::discover_default().ok().map(|value| value.summary);
    let effective_discovery = std::env::current_dir()
        .ok()
        .and_then(|cwd| codex::discover_effective(&cwd).ok())
        .map(|value| value.summary);
    checks.push(effective_runtime_check(effective_discovery.as_ref()));
    checks.push(instrumentation_check(static_discovery.as_ref()));
    checks.push(trust_check(effective_discovery.as_ref()));

    let spool_root = data_root.join("receipts");
    let records_root = spool_root.join("records");
    let mut scan = None;
    if !records_root.exists() {
        checks.push(DiagnosticCheck {
            id: DiagnosticCheckId::ReceiptSpool,
            status: DiagnosticStatus::Warning,
            facts: Vec::new(),
        });
    } else {
        match ReceiptSpool::open_existing(&spool_root) {
            Ok(spool) => {
                let writable_attribute = std::fs::metadata(spool.root().join("records"))
                    .map(|metadata| !metadata.permissions().readonly())
                    .unwrap_or(false);
                let value = spool.scan();
                let record_count = value.invocations.len() as u64;
                scan = Some(value);
                checks.push(DiagnosticCheck {
                    id: DiagnosticCheckId::ReceiptSpool,
                    status: if writable_attribute {
                        DiagnosticStatus::Pass
                    } else {
                        DiagnosticStatus::Warning
                    },
                    facts: vec![DiagnosticFact::ReceiptRecords {
                        count: record_count,
                    }],
                });
            }
            Err(_) => checks.push(DiagnosticCheck {
                id: DiagnosticCheckId::ReceiptSpool,
                status: DiagnosticStatus::Fail,
                facts: Vec::new(),
            }),
        }
    }

    checks.push(ledger_check(&data_root.join("ledger.sqlite3")));
    checks.push(receipt_integrity_check(scan.as_ref()));
    checks.push(coverage_check(
        static_discovery.as_ref(),
        effective_discovery.as_ref(),
    ));
    checks.push(path_identity_check());
    checks.push(evidence_freshness_check(scan.as_ref(), now_unix_ms));

    let overall_status = checks
        .iter()
        .map(|check| check.status)
        .max_by_key(|status| status.severity())
        .unwrap_or(DiagnosticStatus::Unknown);
    DiagnosticsReport {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        read_only: true,
        generated_at_unix_ms: now_unix_ms,
        overall_status,
        checks,
    }
}

fn codex_binary_check() -> DiagnosticCheck {
    match codex_version() {
        CodexVersion::Present(value) => DiagnosticCheck {
            id: DiagnosticCheckId::CodexBinary,
            status: DiagnosticStatus::Pass,
            facts: vec![DiagnosticFact::Version { value }],
        },
        CodexVersion::Missing => DiagnosticCheck {
            id: DiagnosticCheckId::CodexBinary,
            status: DiagnosticStatus::Fail,
            facts: Vec::new(),
        },
        CodexVersion::Unknown => DiagnosticCheck {
            id: DiagnosticCheckId::CodexBinary,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        },
    }
}

enum CodexVersion {
    Present(String),
    Missing,
    Unknown,
}

fn codex_version() -> CodexVersion {
    let child = Command::new("codex")
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return CodexVersion::Missing;
        }
        Err(_) => return CodexVersion::Unknown,
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => return CodexVersion::Unknown,
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return CodexVersion::Unknown;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let Ok(output) = child.wait_with_output() else {
        return CodexVersion::Unknown;
    };
    if !output.status.success() {
        return CodexVersion::Unknown;
    }
    let Ok(output) = std::str::from_utf8(&output.stdout) else {
        return CodexVersion::Unknown;
    };
    let Some(line) = output.lines().next() else {
        return CodexVersion::Unknown;
    };
    let line = line.trim();
    let safe = line.len() <= 80
        && line.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '.' | '-' | '_')
        });
    if !safe || !line.to_ascii_lowercase().starts_with("codex") {
        return CodexVersion::Unknown;
    }
    let value = line.strip_prefix("codex").unwrap_or(line).trim();
    if value.is_empty() {
        CodexVersion::Unknown
    } else {
        CodexVersion::Present(value.to_owned())
    }
}

fn effective_runtime_check(value: Option<&EffectiveDiscoverySummary>) -> DiagnosticCheck {
    match value {
        Some(summary) => DiagnosticCheck {
            id: DiagnosticCheckId::EffectiveRuntime,
            status: if summary.unsupported_or_uninstrumentable == 0 {
                DiagnosticStatus::Pass
            } else {
                DiagnosticStatus::Warning
            },
            facts: vec![DiagnosticFact::HandlerCounts {
                discovered: summary.discovered as u64,
                instrumented: summary
                    .handlers
                    .iter()
                    .filter(|handler| {
                        handler.disposition == InstrumentationDisposition::AlreadyInstrumented
                    })
                    .count() as u64,
                unsupported: summary.unsupported_or_uninstrumentable as u64,
            }],
        },
        None => DiagnosticCheck {
            id: DiagnosticCheckId::EffectiveRuntime,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        },
    }
}

fn instrumentation_check(value: Option<&codex::DiscoverySummary>) -> DiagnosticCheck {
    match value {
        Some(summary) => {
            let status = if summary.already_instrumented > 0 {
                DiagnosticStatus::Pass
            } else if summary.discovered > 0 {
                DiagnosticStatus::Warning
            } else {
                DiagnosticStatus::Unknown
            };
            DiagnosticCheck {
                id: DiagnosticCheckId::Instrumentation,
                status,
                facts: vec![DiagnosticFact::HandlerCounts {
                    discovered: summary.discovered as u64,
                    instrumented: summary.already_instrumented as u64,
                    unsupported: summary.unsupported_or_uninstrumentable as u64,
                }],
            }
        }
        None => DiagnosticCheck {
            id: DiagnosticCheckId::Instrumentation,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        },
    }
}

fn trust_check(value: Option<&EffectiveDiscoverySummary>) -> DiagnosticCheck {
    let Some(summary) = value else {
        return DiagnosticCheck {
            id: DiagnosticCheckId::Trust,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        };
    };
    let instrumented = summary
        .handlers
        .iter()
        .filter(|handler| handler.disposition == InstrumentationDisposition::AlreadyInstrumented)
        .collect::<Vec<_>>();
    if instrumented.is_empty() {
        return DiagnosticCheck {
            id: DiagnosticCheckId::Trust,
            status: DiagnosticStatus::Unsupported,
            facts: Vec::new(),
        };
    }
    let statuses = instrumented
        .iter()
        .filter_map(|handler| handler.trusted)
        .collect::<Vec<_>>();
    let status = if statuses.is_empty() {
        DiagnosticStatus::Unknown
    } else if statuses.iter().all(|trusted| *trusted) {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Warning
    };
    DiagnosticCheck {
        id: DiagnosticCheckId::Trust,
        status,
        facts: vec![DiagnosticFact::HandlerCounts {
            discovered: instrumented.len() as u64,
            instrumented: statuses.iter().filter(|trusted| **trusted).count() as u64,
            unsupported: 0,
        }],
    }
}

fn ledger_check(path: &Path) -> DiagnosticCheck {
    match Ledger::open_read_only(path).and_then(|ledger| ledger.invocation_count()) {
        Ok(count) => DiagnosticCheck {
            id: DiagnosticCheckId::Ledger,
            status: DiagnosticStatus::Pass,
            facts: vec![DiagnosticFact::LedgerInvocations { count }],
        },
        Err(_) if !path.exists() => DiagnosticCheck {
            id: DiagnosticCheckId::Ledger,
            status: DiagnosticStatus::Warning,
            facts: Vec::new(),
        },
        Err(_) => DiagnosticCheck {
            id: DiagnosticCheckId::Ledger,
            status: DiagnosticStatus::Fail,
            facts: Vec::new(),
        },
    }
}

fn receipt_integrity_check(scan: Option<&ReceiptScan>) -> DiagnosticCheck {
    match scan {
        Some(scan) => DiagnosticCheck {
            id: DiagnosticCheckId::ReceiptIntegrity,
            status: if scan.malformed > 0 {
                DiagnosticStatus::Fail
            } else if scan.starts_without_completion > 0 {
                DiagnosticStatus::Warning
            } else {
                DiagnosticStatus::Pass
            },
            facts: vec![DiagnosticFact::ReceiptIntegrity {
                incomplete: scan.starts_without_completion,
                malformed: scan.malformed,
            }],
        },
        None => DiagnosticCheck {
            id: DiagnosticCheckId::ReceiptIntegrity,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        },
    }
}

fn coverage_check(
    static_value: Option<&codex::DiscoverySummary>,
    effective_value: Option<&EffectiveDiscoverySummary>,
) -> DiagnosticCheck {
    let Some(static_value) = static_value else {
        return DiagnosticCheck {
            id: DiagnosticCheckId::EvidenceCoverage,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        };
    };
    let unsupported = effective_value
        .map(|summary| summary.unsupported_or_uninstrumentable)
        .unwrap_or(static_value.unsupported_or_uninstrumentable) as u64;
    let status = if effective_value.is_none() {
        DiagnosticStatus::Unknown
    } else if unsupported == 0 {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Warning
    };
    DiagnosticCheck {
        id: DiagnosticCheckId::EvidenceCoverage,
        status,
        facts: vec![DiagnosticFact::HandlerCounts {
            discovered: static_value.discovered as u64,
            instrumented: static_value.already_instrumented as u64,
            unsupported,
        }],
    }
}

#[cfg(windows)]
fn path_identity_check() -> DiagnosticCheck {
    let status = std::env::current_exe()
        .ok()
        .and_then(|path| codex::require_windows_path_identity(&path).ok())
        .map(|_| DiagnosticStatus::Pass)
        .unwrap_or(DiagnosticStatus::Fail);
    DiagnosticCheck {
        id: DiagnosticCheckId::PathIdentity,
        status,
        facts: Vec::new(),
    }
}

#[cfg(not(windows))]
fn path_identity_check() -> DiagnosticCheck {
    DiagnosticCheck {
        id: DiagnosticCheckId::PathIdentity,
        status: DiagnosticStatus::Unsupported,
        facts: Vec::new(),
    }
}

fn evidence_freshness_check(scan: Option<&ReceiptScan>, now_unix_ms: i64) -> DiagnosticCheck {
    let latest = scan.and_then(|scan| {
        scan.invocations
            .iter()
            .map(|value| value.occurred_at_unix_ms)
            .max()
    });
    let Some(latest) = latest else {
        return DiagnosticCheck {
            id: DiagnosticCheckId::EvidenceFreshness,
            status: DiagnosticStatus::Unknown,
            facts: Vec::new(),
        };
    };
    let age_minutes = now_unix_ms.saturating_sub(latest).max(0) as u64 / 60_000;
    DiagnosticCheck {
        id: DiagnosticCheckId::EvidenceFreshness,
        status: if age_minutes <= 60 * 24 * 7 {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Warning
        },
        facts: vec![DiagnosticFact::EvidenceAgeMinutes { age_minutes }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn absent_state_is_reported_without_creating_anything() {
        let directory = tempdir().unwrap();
        let root = directory.path().join("missing");
        let report = collect(&root, 1_000);
        assert!(report.read_only);
        assert!(!root.exists());
        assert!(report.checks.iter().any(|check| {
            check.id == DiagnosticCheckId::ReceiptSpool && check.status == DiagnosticStatus::Warning
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == DiagnosticCheckId::Ledger && check.status == DiagnosticStatus::Warning
        }));
    }

    #[test]
    fn serialized_diagnostics_expose_no_private_operational_fields() {
        let report = DiagnosticsReport {
            schema_version: 1,
            read_only: true,
            generated_at_unix_ms: 1,
            overall_status: DiagnosticStatus::Warning,
            checks: vec![DiagnosticCheck {
                id: DiagnosticCheckId::ReceiptIntegrity,
                status: DiagnosticStatus::Warning,
                facts: vec![DiagnosticFact::ReceiptIntegrity {
                    incomplete: 2,
                    malformed: 0,
                }],
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        for forbidden in [
            "command",
            "prompt",
            "stdout",
            "stderr",
            "credential",
            "path",
        ] {
            assert!(!json.contains(forbidden));
        }
    }
}
