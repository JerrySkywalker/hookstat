//! Read-only operational diagnostics shared by the CLI and Reliability Center.
//!
//! The model contains only stable identifiers, bounded counts, versions, and
//! evidence timestamps. It never retains configuration paths, hook commands,
//! receipt payloads, process output, credentials, or session content.

use crate::admission::{
    IpcAdmissionState, V031_COOPERATIVE_IPC_ADMISSION, V031_TRANSPARENT_SHIM_ADMISSION,
};
use crate::codex::{self, EffectiveDiscoverySummary, InstrumentationDisposition};
use crate::domain::{EvidenceCoverage, Runtime};
use crate::evidence::{DomainAuthority, DomainAuthoritySelection, NativeAdmissionState};
use crate::ipc::{BrokerDiagnostics, IpcClient, IpcError, LocalEndpoint};
use crate::ledger::Ledger;
use crate::receipt::{ReceiptScan, ReceiptSpool};
use crate::runtime::codex::{
    CODEX_TESTED_CLI_VERSION, CodexHostPlatform, CodexNativeCapabilityProbe, CodexNativeL2Status,
    CodexProtocolVersion,
};
use serde::Serialize;
#[cfg(windows)]
use std::ffi::{OsStr, OsString};
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DIAGNOSTICS_SCHEMA_VERSION: u8 = 2;
pub const MAX_DIAGNOSTIC_AUTHORITY_DOMAINS: usize = 128;
const BROKER_DIAGNOSTIC_QUERY_TIMEOUT: Duration = Duration::from_millis(20);

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
pub struct AuthorityDomainDiagnostic {
    pub runtime: String,
    pub event: String,
    pub source_scope: String,
    pub native_admission: NativeAdmissionState,
    pub ipc_admission: IpcAdmissionState,
    pub selected_authority: DomainAuthoritySelection,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductionEvidenceDiagnostics {
    pub runtime: Runtime,
    pub native_l2_status: CodexNativeL2Status,
    pub native_admission: NativeAdmissionState,
    pub cooperative_ipc_admission: IpcAdmissionState,
    pub transparent_shim_admission: IpcAdmissionState,
    pub transparent_shim_active: bool,
    pub default_authority: DomainAuthoritySelection,
    pub evidence_transport_count: u8,
    pub third_transport_present: bool,
    pub shadow_in_denominator: bool,
    pub domains: Vec<AuthorityDomainDiagnostic>,
    pub domains_truncated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerDiagnosticState {
    Absent,
    Running,
    Unavailable,
    UnsafeState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BrokerDiagnosticsReport {
    pub state: BrokerDiagnosticState,
    pub metrics: Option<BrokerDiagnostics>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticsReport {
    pub schema_version: u8,
    pub read_only: bool,
    pub generated_at_unix_ms: i64,
    pub overall_status: DiagnosticStatus,
    pub checks: Vec<DiagnosticCheck>,
    pub production_evidence: ProductionEvidenceDiagnostics,
    pub broker: BrokerDiagnosticsReport,
}

impl DiagnosticsReport {
    pub fn empty(now_unix_ms: i64) -> Self {
        Self {
            schema_version: DIAGNOSTICS_SCHEMA_VERSION,
            read_only: true,
            generated_at_unix_ms: now_unix_ms,
            overall_status: DiagnosticStatus::Unknown,
            checks: Vec::new(),
            production_evidence: production_evidence_diagnostics(
                CodexNativeL2Status::NotQualified,
                &[],
            ),
            broker: BrokerDiagnosticsReport {
                state: BrokerDiagnosticState::Absent,
                metrics: None,
            },
        }
    }
}

/// Collects the supported diagnostic checks without creating, repairing, or
/// changing HookStat, Codex, receipt, or trust state.
pub fn collect(data_root: &Path, now_unix_ms: i64) -> DiagnosticsReport {
    collect_with_authorities(data_root, now_unix_ms, &[])
}

/// Controlled integrations may supply their already-governed authority table
/// so diagnostics can report the exact selected authority per bounded domain.
/// The default CLI passes no table and truthfully exposes the NOT_ADMITTED
/// fallback instead of inferring integration from a handler declaration.
pub fn collect_with_authorities(
    data_root: &Path,
    now_unix_ms: i64,
    authorities: &[DomainAuthority],
) -> DiagnosticsReport {
    let mut checks = vec![DiagnosticCheck {
        id: DiagnosticCheckId::HookStatBinary,
        status: DiagnosticStatus::Pass,
        facts: vec![DiagnosticFact::Version {
            value: env!("CARGO_PKG_VERSION").to_owned(),
        }],
    }];

    let codex_version = codex_version();
    checks.push(codex_binary_check_from(&codex_version));

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
        production_evidence: production_evidence_diagnostics(
            native_l2_status(&codex_version),
            authorities,
        ),
        broker: broker_diagnostics_report(data_root),
    }
}

fn codex_binary_check_from(version: &CodexVersion) -> DiagnosticCheck {
    match version {
        CodexVersion::Present(value) => DiagnosticCheck {
            id: DiagnosticCheckId::CodexBinary,
            status: DiagnosticStatus::Pass,
            facts: vec![DiagnosticFact::Version {
                value: value.clone(),
            }],
        },
        CodexVersion::Missing => DiagnosticCheck {
            id: DiagnosticCheckId::CodexBinary,
            status: DiagnosticStatus::Fail,
            facts: Vec::new(),
        },
        // A command that resolved but failed, timed out, or returned a
        // malformed version is not proof that Codex is absent. Keep this
        // truthful Unknown result rather than reproducing the Windows shim
        // false-fail that motivated G11.
        CodexVersion::Failed | CodexVersion::TimedOut | CodexVersion::Malformed => {
            DiagnosticCheck {
                id: DiagnosticCheckId::CodexBinary,
                status: DiagnosticStatus::Unknown,
                facts: Vec::new(),
            }
        }
    }
}

fn native_l2_status(version: &CodexVersion) -> CodexNativeL2Status {
    match version {
        CodexVersion::Present(value) if value == CODEX_TESTED_CLI_VERSION => {
            CodexNativeCapabilityProbe.ordinary_session_attach(
                &CodexProtocolVersion::tested(),
                CodexHostPlatform::current(),
            )
        }
        _ => CodexNativeL2Status::NotQualified,
    }
}

fn production_evidence_diagnostics(
    native_l2_status: CodexNativeL2Status,
    authorities: &[DomainAuthority],
) -> ProductionEvidenceDiagnostics {
    let domains = authorities
        .iter()
        .take(MAX_DIAGNOSTIC_AUTHORITY_DOMAINS)
        .map(|authority| AuthorityDomainDiagnostic {
            runtime: authority.domain.runtime.as_str().to_owned(),
            event: authority.domain.event.as_str().to_owned(),
            source_scope: authority.domain.source_scope.as_str().to_owned(),
            native_admission: authority.native_admission,
            ipc_admission: authority.ipc_admission,
            selected_authority: authority.production_authority(),
        })
        .collect();
    ProductionEvidenceDiagnostics {
        runtime: Runtime::Codex,
        native_l2_status,
        native_admission: native_l2_status.native_admission(),
        cooperative_ipc_admission: V031_COOPERATIVE_IPC_ADMISSION.state,
        transparent_shim_admission: V031_TRANSPARENT_SHIM_ADMISSION.state,
        transparent_shim_active: false,
        default_authority: DomainAuthoritySelection::NotAdmitted,
        evidence_transport_count: 2,
        third_transport_present: false,
        shadow_in_denominator: false,
        domains,
        domains_truncated: authorities.len() > MAX_DIAGNOSTIC_AUTHORITY_DOMAINS,
    }
}

fn broker_diagnostics_report(data_root: &Path) -> BrokerDiagnosticsReport {
    let transport_dir = data_root.join("ipc");
    if !data_root.exists() || !transport_dir.exists() {
        return BrokerDiagnosticsReport {
            state: BrokerDiagnosticState::Absent,
            metrics: None,
        };
    }
    let safe_transport = std::fs::symlink_metadata(&transport_dir)
        .map(|metadata| !metadata.file_type().is_symlink() && metadata.is_dir())
        .unwrap_or(false);
    if !safe_transport {
        return BrokerDiagnosticsReport {
            state: BrokerDiagnosticState::UnsafeState,
            metrics: None,
        };
    }
    let endpoint = match LocalEndpoint::from_state_root(data_root) {
        Ok(endpoint) => endpoint,
        Err(IpcError::UnsafeStateObject) => {
            return BrokerDiagnosticsReport {
                state: BrokerDiagnosticState::UnsafeState,
                metrics: None,
            };
        }
        Err(_) => {
            return BrokerDiagnosticsReport {
                state: BrokerDiagnosticState::Unavailable,
                metrics: None,
            };
        }
    };
    match IpcClient::connect(&endpoint, BROKER_DIAGNOSTIC_QUERY_TIMEOUT)
        .and_then(|mut client| client.diagnostics())
    {
        Ok(metrics) => BrokerDiagnosticsReport {
            state: BrokerDiagnosticState::Running,
            metrics: Some(metrics),
        },
        Err(_) => BrokerDiagnosticsReport {
            state: BrokerDiagnosticState::Unavailable,
            metrics: None,
        },
    }
}

enum CodexVersion {
    Present(String),
    Missing,
    Failed,
    TimedOut,
    Malformed,
}

fn codex_version() -> CodexVersion {
    #[cfg(windows)]
    {
        let Some(command) = resolve_windows_codex_command(
            std::env::var_os("PATH").as_deref(),
            std::env::var_os("PATHEXT").as_deref(),
        ) else {
            return CodexVersion::Missing;
        };
        execute_windows_codex_version(command)
    }

    #[cfg(not(windows))]
    {
        let mut command = Command::new("codex");
        command.arg("--version");
        execute_version_command(command)
    }
}

/// Windows PowerShell resolves `.ps1` commands while `CreateProcess` only
/// receives an executable path. Resolve the ordinary `codex` command once,
/// then use a bounded launcher only for the script forms that require one.
/// No Hook configuration or user-provided command text participates here.
#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowsCodexCommandKind {
    Native,
    CmdShim,
    BatShim,
    PowerShellScript,
}

#[cfg(windows)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct WindowsCodexCommand {
    program: PathBuf,
    kind: WindowsCodexCommandKind,
}

#[cfg(windows)]
fn resolve_windows_codex_command(
    path: Option<&OsStr>,
    path_extensions: Option<&OsStr>,
) -> Option<WindowsCodexCommand> {
    let path = path?;
    let extensions = windows_path_extensions(path_extensions);
    for directory in std::env::split_paths(path) {
        // This is the form selected by the Owner's normal PowerShell session.
        // It is intentionally checked before executable PATHEXT candidates in
        // the same directory, matching PowerShell's ExternalScript behavior.
        let power_shell_script = directory.join("codex.ps1");
        if power_shell_script.is_file() {
            return Some(WindowsCodexCommand {
                program: power_shell_script,
                kind: WindowsCodexCommandKind::PowerShellScript,
            });
        }
        for extension in &extensions {
            let candidate = directory.join(format!("codex{extension}"));
            if !candidate.is_file() {
                continue;
            }
            let kind = match extension.as_str() {
                ".cmd" => WindowsCodexCommandKind::CmdShim,
                ".bat" => WindowsCodexCommandKind::BatShim,
                _ => WindowsCodexCommandKind::Native,
            };
            return Some(WindowsCodexCommand {
                program: candidate,
                kind,
            });
        }
    }
    None
}

#[cfg(windows)]
fn windows_path_extensions(path_extensions: Option<&OsStr>) -> Vec<String> {
    let configured = path_extensions
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned());
    let mut result = Vec::new();
    for extension in configured.split(';') {
        let extension = extension.trim().to_ascii_lowercase();
        if !matches!(extension.as_str(), ".com" | ".exe" | ".cmd" | ".bat")
            || result.iter().any(|existing| existing == &extension)
        {
            continue;
        }
        result.push(extension);
    }
    if result.is_empty() {
        [".com", ".exe", ".bat", ".cmd"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else {
        result
    }
}

#[cfg(windows)]
fn execute_windows_codex_version(command: WindowsCodexCommand) -> CodexVersion {
    let process = match command.kind {
        WindowsCodexCommandKind::Native => {
            let mut process = Command::new(command.program);
            process.arg("--version");
            process
        }
        WindowsCodexCommandKind::CmdShim | WindowsCodexCommandKind::BatShim => {
            let Some(command_processor) = system_windows_binary(&["cmd.exe"]) else {
                return CodexVersion::Failed;
            };
            let Some(invocation) = safe_cmd_script_invocation(&command.program) else {
                return CodexVersion::Failed;
            };
            let mut process = Command::new(command_processor);
            // `/d` disables AutoRun. The only command-string input is a
            // validated resolved PATH file plus the literal `--version`.
            process.args(["/d", "/s", "/c"]).arg(invocation);
            process
        }
        WindowsCodexCommandKind::PowerShellScript => {
            let Some(power_shell) =
                system_windows_binary(&["WindowsPowerShell", "v1.0", "powershell.exe"])
            else {
                return CodexVersion::Failed;
            };
            let mut process = Command::new(power_shell);
            // `-File` keeps the script path and its literal argument out of a
            // PowerShell command string. It also preserves normal execution
            // policy instead of bypassing it for a diagnostic probe.
            process
                .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
                .arg(command.program)
                .arg("--version");
            process
        }
    };
    execute_version_command(process)
}

#[cfg(windows)]
fn system_windows_binary(components: &[&str]) -> Option<PathBuf> {
    let root = std::env::var_os("SystemRoot")?;
    let path = components
        .iter()
        .fold(PathBuf::from(root).join("System32"), |path, component| {
            path.join(component)
        });
    path.is_file().then_some(path)
}

#[cfg(windows)]
fn safe_cmd_script_invocation(path: &Path) -> Option<OsString> {
    let path = path.to_str()?;
    if path.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                '"' | '&' | '|' | '<' | '>' | '^' | '%' | '!' | '(' | ')'
            )
    }) {
        return None;
    }
    Some(OsString::from(format!("\"{path}\" --version")))
}

fn execute_version_command(mut command: Command) -> CodexVersion {
    command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return CodexVersion::Missing,
        Err(_) => return CodexVersion::Failed,
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => return CodexVersion::Failed,
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return CodexVersion::TimedOut;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let Ok(output) = child.wait_with_output() else {
        return CodexVersion::Failed;
    };
    classify_version_output(output.status.success(), &output.stdout)
}

fn classify_version_output(success: bool, stdout: &[u8]) -> CodexVersion {
    if !success {
        return CodexVersion::Failed;
    }
    let Ok(output) = std::str::from_utf8(stdout) else {
        return CodexVersion::Malformed;
    };
    let Some(line) = output.lines().next() else {
        return CodexVersion::Malformed;
    };
    let line = line.trim();
    let safe = line.len() <= 80
        && line.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ' ' | '.' | '-' | '_')
        });
    let normalized = line.to_ascii_lowercase();
    let prefix_length = if normalized.starts_with("codex-cli ") {
        "codex-cli".len()
    } else if normalized.starts_with("codex ") {
        "codex".len()
    } else {
        return CodexVersion::Malformed;
    };
    if !safe {
        return CodexVersion::Malformed;
    }
    let value = line[prefix_length..].trim();
    if value.is_empty() {
        CodexVersion::Malformed
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
    use crate::evidence::{CoverageDomain, EventFamily, RuntimeId, SourceScope};
    #[cfg(windows)]
    use std::ffi::OsString;
    #[cfg(windows)]
    use std::fs;
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
        assert_eq!(
            report.production_evidence.default_authority,
            DomainAuthoritySelection::NotAdmitted
        );
        assert_eq!(report.production_evidence.evidence_transport_count, 2);
        assert!(!report.production_evidence.third_transport_present);
        assert!(!report.production_evidence.shadow_in_denominator);
        assert_eq!(report.broker.state, BrokerDiagnosticState::Absent);
    }

    #[test]
    fn authority_diagnostics_preserve_native_ipc_and_not_admitted_per_domain() {
        let authority =
            |event: &str,
             native_admission: NativeAdmissionState,
             ipc_admission: IpcAdmissionState| DomainAuthority {
                domain: CoverageDomain {
                    runtime: RuntimeId::new("codex").unwrap(),
                    event: EventFamily::new(event).unwrap(),
                    source_scope: SourceScope::new("user_hooks").unwrap(),
                },
                native_admission,
                ipc_admission,
            };
        let diagnostics = production_evidence_diagnostics(
            CodexNativeL2Status::UpstreamUnavailable,
            &[
                authority(
                    "session_start",
                    NativeAdmissionState::Admitted,
                    IpcAdmissionState::Admitted,
                ),
                authority(
                    "stop",
                    NativeAdmissionState::Unavailable,
                    IpcAdmissionState::Admitted,
                ),
                authority(
                    "pre_tool_use",
                    NativeAdmissionState::Unavailable,
                    IpcAdmissionState::QualifiedNotAdmittedPerformance,
                ),
            ],
        );
        assert_eq!(diagnostics.domains.len(), 3);
        assert_eq!(
            diagnostics.domains[0].selected_authority,
            DomainAuthoritySelection::Native
        );
        assert_eq!(
            diagnostics.domains[1].selected_authority,
            DomainAuthoritySelection::Ipc
        );
        assert_eq!(
            diagnostics.domains[2].selected_authority,
            DomainAuthoritySelection::NotAdmitted
        );
        assert_eq!(
            diagnostics.native_admission,
            NativeAdmissionState::Unavailable
        );
        assert!(!diagnostics.transparent_shim_active);
        assert!(!diagnostics.domains_truncated);

        let bounded = (0..=MAX_DIAGNOSTIC_AUTHORITY_DOMAINS)
            .map(|index| {
                authority(
                    &format!("event_{index}"),
                    NativeAdmissionState::Unavailable,
                    IpcAdmissionState::Admitted,
                )
            })
            .collect::<Vec<_>>();
        let bounded_diagnostics =
            production_evidence_diagnostics(CodexNativeL2Status::NotQualified, &bounded);
        assert_eq!(
            bounded_diagnostics.domains.len(),
            MAX_DIAGNOSTIC_AUTHORITY_DOMAINS
        );
        assert!(bounded_diagnostics.domains_truncated);
    }

    #[test]
    fn serialized_diagnostics_expose_no_private_operational_fields() {
        let mut report = DiagnosticsReport::empty(1);
        report.overall_status = DiagnosticStatus::Warning;
        report.checks = vec![DiagnosticCheck {
            id: DiagnosticCheckId::ReceiptIntegrity,
            status: DiagnosticStatus::Warning,
            facts: vec![DiagnosticFact::ReceiptIntegrity {
                incomplete: 2,
                malformed: 0,
            }],
        }];
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

    #[test]
    fn codex_probe_classifications_fail_closed_without_false_absence() {
        assert!(matches!(
            classify_version_output(true, b"Codex 0.2.1\n"),
            CodexVersion::Present(value) if value == "0.2.1"
        ));
        assert!(matches!(
            classify_version_output(true, b"codex-cli 0.149.0\n"),
            CodexVersion::Present(value) if value == "0.149.0"
        ));
        assert!(matches!(
            classify_version_output(false, b"Codex 0.2.1\n"),
            CodexVersion::Failed
        ));
        assert!(matches!(
            classify_version_output(true, b"unrecognized"),
            CodexVersion::Malformed
        ));
        assert_eq!(
            codex_binary_check_from(&CodexVersion::Missing).status,
            DiagnosticStatus::Fail
        );
        for version in [
            CodexVersion::Failed,
            CodexVersion::TimedOut,
            CodexVersion::Malformed,
        ] {
            assert_eq!(
                codex_binary_check_from(&version).status,
                DiagnosticStatus::Unknown
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_resolver_honors_pathext_and_classifies_cmd_and_bat_shims() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("codex.exe"), []).unwrap();
        fs::write(directory.path().join("codex.cmd"), []).unwrap();
        let path = std::env::join_paths([directory.path()]).unwrap();
        let native = resolve_windows_codex_command(
            Some(path.as_os_str()),
            Some(OsString::from(".EXE;.CMD").as_os_str()),
        )
        .unwrap();
        assert_eq!(native.kind, WindowsCodexCommandKind::Native);
        let cmd = resolve_windows_codex_command(
            Some(path.as_os_str()),
            Some(OsString::from(".CMD;.EXE").as_os_str()),
        )
        .unwrap();
        assert_eq!(cmd.kind, WindowsCodexCommandKind::CmdShim);

        fs::remove_file(directory.path().join("codex.cmd")).unwrap();
        fs::remove_file(directory.path().join("codex.exe")).unwrap();
        fs::write(directory.path().join("codex.bat"), []).unwrap();
        let bat = resolve_windows_codex_command(
            Some(path.as_os_str()),
            Some(OsString::from(".BAT").as_os_str()),
        )
        .unwrap();
        assert_eq!(bat.kind, WindowsCodexCommandKind::BatShim);
    }

    #[cfg(windows)]
    #[test]
    fn cmd_shim_invocation_is_bounded_and_rejects_shell_metacharacters() {
        let safe = PathBuf::from(r"C:\Program Files\Codex 中文\codex.cmd");
        assert_eq!(
            safe_cmd_script_invocation(&safe),
            Some(OsString::from(
                r#""C:\Program Files\Codex 中文\codex.cmd" --version"#
            ))
        );
        assert!(safe_cmd_script_invocation(Path::new(r"C:\unsafe&shim\codex.cmd")).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn windows_resolver_supports_non_ascii_space_containing_powershell_shim() {
        let temporary = tempdir().unwrap();
        let directory = temporary.path().join("Codex shim 中文");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("codex.ps1"), []).unwrap();
        let path = std::env::join_paths([&directory]).unwrap();
        let command = resolve_windows_codex_command(
            Some(path.as_os_str()),
            Some(OsString::from(".EXE;.CMD").as_os_str()),
        )
        .unwrap();
        assert_eq!(command.kind, WindowsCodexCommandKind::PowerShellScript);
    }

    #[cfg(windows)]
    #[test]
    fn windows_resolver_classifies_missing_command() {
        let directory = tempdir().unwrap();
        let path = std::env::join_paths([directory.path()]).unwrap();
        assert!(
            resolve_windows_codex_command(
                Some(path.as_os_str()),
                Some(OsString::from(".EXE;.CMD;.BAT").as_os_str()),
            )
            .is_none()
        );
    }
}
