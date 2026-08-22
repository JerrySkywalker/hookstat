//! Developer-only G28 Windows hot-path measurement laboratory.
//!
//! The harness writes only a bounded, schema-checked performance receipt. Its
//! disposable proxy manifest contains a synthetic local fixture command, and
//! neither that command nor any process output is included in the receipt.

use crate::codex::{ProxyHandler, ProxyManifest};
use crate::domain::{EvidenceCoverage, ExecutionMode, HandlerIdentity, HookEvent, TerminalStatus};
use crate::receipt::{ReceiptCompletion, ReceiptSpool, ReceiptStart};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const SCHEMA_VERSION: u8 = 1;
const PIPE_FRAME_BYTES: usize = 64;
const DEFAULT_PROCESS_ITERATIONS: usize = 100;
const DEFAULT_IO_ITERATIONS: usize = 10_000;
const DEFAULT_PIPE_ITERATIONS: usize = 1_000;
const PIPE_WARMUP_ITERATIONS: usize = 25;
const PROCESS_WARMUP_ITERATIONS: usize = 25;
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

const REQUIRED_OPERATION_CLASSIFICATIONS: [(&str, &str); 16] = [
    (
        "direct_original_fixture",
        "repeated_fresh_windows_shell_handler_fixture_not_cache_cold",
    ),
    (
        "current_v030_proxy",
        "repeated_fresh_process_manifest_receipts_journal_shell_job_not_cache_cold",
    ),
    (
        "hookstat_executable_startup",
        "repeated_fresh_process_startup_help_path_not_cache_cold",
    ),
    (
        "cmd_shell_overhead",
        "repeated_fresh_process_cmd_exe_c_not_cache_cold",
    ),
    (
        "rust_createprocess_direct_spawn",
        "repeated_fresh_rust_createprocess_not_cache_cold",
    ),
    (
        "windows_job_object_cycle",
        "fresh_child_job_create_assign_release",
    ),
    (
        "current_receipt_start_write",
        "warm_atomic_json_start_record",
    ),
    (
        "current_receipt_completion_write",
        "warm_atomic_json_completion_record",
    ),
    ("current_journal_append", "warm_ndjson_append_without_sync"),
    (
        "current_sync_data",
        "warm_journal_file_sync_data_after_dirty_append",
    ),
    (
        "minimal_shim_process_start_fixture",
        "repeated_fresh_minimal_fixture_process_start_not_cache_cold",
    ),
    (
        "minimal_shim_cache_warmed_process_start_fixture",
        "cache_warmed_fresh_minimal_fixture_process_start",
    ),
    (
        "windows_named_pipe_cold_connection",
        "cold_local_named_pipe_connect",
    ),
    (
        "windows_named_pipe_warm_connection",
        "warm_local_named_pipe_connect_after_warmup",
    ),
    (
        "windows_named_pipe_one_way_bounded_frame_write",
        "warm_local_named_pipe_64_byte_write",
    ),
    (
        "windows_named_pipe_bounded_frame_ack_round_trip",
        "warm_local_named_pipe_64_byte_write_ack",
    ),
];

#[derive(Clone, Debug)]
pub struct HarnessConfig {
    pub process_iterations: usize,
    pub io_iterations: usize,
    pub pipe_iterations: usize,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            process_iterations: DEFAULT_PROCESS_ITERATIONS,
            io_iterations: DEFAULT_IO_ITERATIONS,
            pipe_iterations: DEFAULT_PIPE_ITERATIONS,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HarnessPaths {
    pub hookstat_executable: PathBuf,
    pub handler_fixture_executable: PathBuf,
    pub shim_fixture_executable: PathBuf,
    pub named_pipe_probe: PathBuf,
}

impl HarnessPaths {
    pub fn from_current_executable() -> Result<Self, PerformanceError> {
        let current = std::env::current_exe().map_err(PerformanceError::Io)?;
        let parent = current.parent().ok_or(PerformanceError::Prerequisite(
            "benchmark executable has no parent directory",
        ))?;
        let extension = current.extension().unwrap_or_default();
        Ok(Self {
            hookstat_executable: parent.join("hookstat").with_extension(extension),
            handler_fixture_executable: parent
                .join("hookstat-g28-handler-fixture")
                .with_extension(extension),
            shim_fixture_executable: parent
                .join("hookstat-hook-fixture")
                .with_extension(extension),
            named_pipe_probe: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join("performance")
                .join("named-pipe-probe.ps1"),
        })
    }

    fn validate(&self) -> Result<(), PerformanceError> {
        for (path, name) in [
            (&self.hookstat_executable, "current HookStat executable"),
            (
                &self.handler_fixture_executable,
                "synthetic handler fixture",
            ),
            (&self.shim_fixture_executable, "minimal shim fixture"),
            (&self.named_pipe_probe, "Named Pipe probe"),
        ] {
            if !path.is_file() {
                return Err(PerformanceError::Prerequisite(name));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerformanceReceipt {
    pub schema_version: u8,
    pub run_kind: String,
    pub platform: PlatformMetadata,
    pub sample_plan: SamplePlan,
    pub samples: Vec<PerformanceSample>,
    pub one_second_timeout_reproduction: TimeoutReproduction,
    pub privacy: PrivacyReceipt,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlatformMetadata {
    pub operating_system: String,
    pub architecture: String,
    pub windows_version_build: String,
    pub logical_processors: usize,
    pub benchmark_machine_fingerprint: String,
    pub rustc: String,
    pub cargo: String,
    pub build_profile: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SamplePlan {
    pub process_iterations: usize,
    pub io_iterations: usize,
    pub pipe_iterations: usize,
    pub pipe_warmup_iterations: usize,
    pub bounded_frame_bytes: usize,
    pub percentile_method: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerformanceSample {
    pub operation: String,
    pub classification: String,
    pub iterations: usize,
    pub statistics_ms: LatencyStatistics,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LatencyStatistics {
    pub min: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
    pub mean: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TimeoutReproduction {
    pub declaration_timeout_ms: u64,
    pub start_evidence_emitted: bool,
    pub completion_evidence_missing: bool,
    pub terminal_status: String,
    pub isolated_disposable_fixture: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrivacyReceipt {
    pub owner_live_codex_config_mutated: bool,
    pub raw_private_content_captured: bool,
}

#[derive(Debug)]
pub enum PerformanceError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Receipt(crate::receipt::ReceiptError),
    Proxy(crate::proxy::ProxyError),
    Prerequisite(&'static str),
    Invalid(&'static str),
    ProcessFailed(&'static str),
}

impl fmt::Display for PerformanceError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => output.write_str("G28 performance laboratory I/O failed"),
            Self::Json(_) => output.write_str("G28 performance laboratory received invalid JSON"),
            Self::Receipt(error) => error.fmt(output),
            Self::Proxy(error) => error.fmt(output),
            Self::Prerequisite(name) => write!(output, "G28 prerequisite unavailable: {name}"),
            Self::Invalid(field) => write!(output, "invalid G28 performance value: {field}"),
            Self::ProcessFailed(name) => write!(output, "G28 fixture process failed: {name}"),
        }
    }
}

impl std::error::Error for PerformanceError {}
impl From<std::io::Error> for PerformanceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for PerformanceError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl From<crate::receipt::ReceiptError> for PerformanceError {
    fn from(value: crate::receipt::ReceiptError) -> Self {
        Self::Receipt(value)
    }
}
impl From<crate::proxy::ProxyError> for PerformanceError {
    fn from(value: crate::proxy::ProxyError) -> Self {
        Self::Proxy(value)
    }
}

/// Runs the Windows-only G28 laboratory. The caller chooses where to save the
/// resulting sanitized receipt; the function itself never writes a repository
/// artifact or inspects the Owner's Codex configuration.
pub fn run(
    config: &HarnessConfig,
    paths: &HarnessPaths,
) -> Result<PerformanceReceipt, PerformanceError> {
    if !cfg!(windows) {
        return Err(PerformanceError::Prerequisite(
            "the Windows performance laboratory cannot run on this platform",
        ));
    }
    validate_config(config)?;
    paths.validate()?;
    let fixture = DisposableFixture::create(&paths.handler_fixture_executable)?;
    let spool = ReceiptSpool::open(fixture.root.join("receipt-micro"))?;
    let mut samples = vec![
        measure_process(
            "direct_original_fixture",
            "repeated_fresh_windows_shell_handler_fixture_not_cache_cold",
            config.process_iterations,
            || run_cmd_shell_handler(&paths.handler_fixture_executable),
        )?,
        measure_process(
            "current_v030_proxy",
            "repeated_fresh_process_manifest_receipts_journal_shell_job_not_cache_cold",
            config.process_iterations,
            || fixture.run_current_proxy(&paths.hookstat_executable),
        )?,
        measure_process(
            "hookstat_executable_startup",
            "repeated_fresh_process_startup_help_path_not_cache_cold",
            config.process_iterations,
            || run_silent(&paths.hookstat_executable, &["--help"]),
        )?,
        measure_process(
            "cmd_shell_overhead",
            "repeated_fresh_process_cmd_exe_c_not_cache_cold",
            config.process_iterations,
            run_cmd_shell_fixture,
        )?,
        measure_process(
            "rust_createprocess_direct_spawn",
            "repeated_fresh_rust_createprocess_not_cache_cold",
            config.process_iterations,
            || run_silent(&paths.handler_fixture_executable, &[]),
        )?,
        measure_job_object_cycles(&paths.shim_fixture_executable, config.process_iterations)?,
    ];

    let handler = fixture_handler();
    let mut receipt_id = 0_u64;
    samples.push(measure_process(
        "current_receipt_start_write",
        "warm_atomic_json_start_record",
        config.io_iterations,
        || {
            receipt_id += 1;
            spool
                .performance_write_start_record(&receipt_start(&handler, receipt_id))
                .map_err(PerformanceError::from)
        },
    )?);
    samples.push(measure_process(
        "current_receipt_completion_write",
        "warm_atomic_json_completion_record",
        config.io_iterations,
        || {
            receipt_id += 1;
            spool
                .performance_write_completion_record(&receipt_completion(&handler, receipt_id))
                .map_err(PerformanceError::from)
        },
    )?);
    samples.push(measure_process(
        "current_journal_append",
        "warm_ndjson_append_without_sync",
        config.io_iterations,
        || {
            receipt_id += 1;
            spool
                .performance_append_journal_unflushed(&format!("g28j{receipt_id:016x}"))
                .map_err(PerformanceError::from)
        },
    )?);
    samples.push(measure_process_with_setup(
        "current_sync_data",
        "warm_journal_file_sync_data_after_dirty_append",
        config.io_iterations,
        || {
            receipt_id += 1;
            spool
                .performance_append_journal_unflushed(&format!("g28d{receipt_id:016x}"))
                .map_err(PerformanceError::from)
        },
        || {
            spool
                .performance_sync_journal_data()
                .map_err(PerformanceError::from)
        },
    )?);
    samples.push(measure_process(
        "minimal_shim_process_start_fixture",
        "repeated_fresh_minimal_fixture_process_start_not_cache_cold",
        config.process_iterations,
        || run_silent(&paths.shim_fixture_executable, &[]),
    )?);
    samples.push(measure_process_with_setup(
        "minimal_shim_cache_warmed_process_start_fixture",
        "cache_warmed_fresh_minimal_fixture_process_start",
        config.process_iterations,
        || {
            for _ in 0..PROCESS_WARMUP_ITERATIONS {
                run_silent(&paths.shim_fixture_executable, &[])?;
            }
            Ok(())
        },
        || run_silent(&paths.shim_fixture_executable, &[]),
    )?);

    let pipe = measure_named_pipes(paths, config.pipe_iterations)?;
    samples.extend(pipe.into_samples()?);
    let timeout = reproduce_one_second_timeout(&fixture, &paths.hookstat_executable)?;

    let windows_version_build = required_windows_platform_value(
        "$v=[Environment]::OSVersion.Version; '{0}.{1}.{2}' -f $v.Major,$v.Minor,$v.Build",
        "Windows version/build",
    )?;
    let processor_model = required_windows_platform_value(
        "(Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1 -ExpandProperty Name).Trim()",
        "Windows processor model",
    )?;
    let benchmark_machine_fingerprint = machine_fingerprint(
        &windows_version_build,
        std::env::consts::OS,
        std::env::consts::ARCH,
        thread::available_parallelism().map_or(1, usize::from),
        &processor_model,
    );

    let receipt = PerformanceReceipt {
        schema_version: SCHEMA_VERSION,
        run_kind: "hs_g28_windows_hot_path_baseline".into(),
        platform: PlatformMetadata {
            operating_system: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            windows_version_build,
            logical_processors: thread::available_parallelism().map_or(1, usize::from),
            benchmark_machine_fingerprint,
            rustc: command_version("rustc"),
            cargo: command_version("cargo"),
            build_profile: if cfg!(debug_assertions) {
                "debug".into()
            } else {
                "release".into()
            },
        },
        sample_plan: SamplePlan {
            process_iterations: config.process_iterations,
            io_iterations: config.io_iterations,
            pipe_iterations: config.pipe_iterations,
            pipe_warmup_iterations: PIPE_WARMUP_ITERATIONS,
            bounded_frame_bytes: PIPE_FRAME_BYTES,
            percentile_method: "nearest_rank".into(),
        },
        samples,
        one_second_timeout_reproduction: timeout,
        privacy: PrivacyReceipt {
            owner_live_codex_config_mutated: false,
            raw_private_content_captured: false,
        },
    };
    receipt.validate()?;
    fixture.cleanup()?;
    Ok(receipt)
}

impl PerformanceReceipt {
    pub fn validate(&self) -> Result<(), PerformanceError> {
        if self.schema_version != SCHEMA_VERSION
            || self.run_kind != "hs_g28_windows_hot_path_baseline"
            || self.samples.len() != REQUIRED_OPERATION_CLASSIFICATIONS.len()
            || self.platform.operating_system.is_empty()
            || self.platform.operating_system.len() > 32
            || self.platform.architecture.is_empty()
            || self.platform.architecture.len() > 32
            || self.platform.windows_version_build.is_empty()
            || self.platform.windows_version_build.len() > 32
            || self.platform.logical_processors == 0
            || self.platform.benchmark_machine_fingerprint.len() != 64
            || !self
                .platform
                .benchmark_machine_fingerprint
                .chars()
                .all(|value| value.is_ascii_hexdigit())
            || self.platform.rustc.is_empty()
            || self.platform.rustc.len() > 120
            || self.platform.cargo.is_empty()
            || self.platform.cargo.len() > 120
            || !matches!(self.platform.build_profile.as_str(), "debug" | "release")
            || !(10..=10_000).contains(&self.sample_plan.process_iterations)
            || !(100..=100_000).contains(&self.sample_plan.io_iterations)
            || !(100..=10_000).contains(&self.sample_plan.pipe_iterations)
            || self.sample_plan.pipe_warmup_iterations > 1_000
            || self.sample_plan.bounded_frame_bytes != PIPE_FRAME_BYTES
            || self.sample_plan.percentile_method != "nearest_rank"
            || self.one_second_timeout_reproduction.declaration_timeout_ms != 1_000
            || !self.one_second_timeout_reproduction.start_evidence_emitted
            || !self
                .one_second_timeout_reproduction
                .completion_evidence_missing
            || self.one_second_timeout_reproduction.terminal_status != "incomplete"
            || !self
                .one_second_timeout_reproduction
                .isolated_disposable_fixture
            || self.privacy.owner_live_codex_config_mutated
            || self.privacy.raw_private_content_captured
        {
            return Err(PerformanceError::Invalid("receipt"));
        }
        for sample in &self.samples {
            if sample.operation.len() > 96
                || sample.classification.len() > 128
                || sample.iterations == 0
                || !sample.statistics_ms.is_valid()
            {
                return Err(PerformanceError::Invalid("sample"));
            }
        }
        if !REQUIRED_OPERATION_CLASSIFICATIONS
            .iter()
            .all(|(operation, classification)| {
                self.samples.iter().any(|sample| {
                    sample.operation == *operation && sample.classification == *classification
                })
            })
        {
            return Err(PerformanceError::Invalid("sample classification"));
        }
        Ok(())
    }
}

impl LatencyStatistics {
    fn is_valid(&self) -> bool {
        let values = [self.min, self.p50, self.p95, self.p99, self.max, self.mean];
        values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
            && self.min <= self.p50
            && self.p50 <= self.p95
            && self.p95 <= self.p99
            && self.p99 <= self.max
    }
}

fn validate_config(config: &HarnessConfig) -> Result<(), PerformanceError> {
    if !(10..=10_000).contains(&config.process_iterations)
        || !(100..=100_000).contains(&config.io_iterations)
        || !(100..=10_000).contains(&config.pipe_iterations)
    {
        return Err(PerformanceError::Invalid("iteration_count"));
    }
    Ok(())
}

fn measure_process<F>(
    operation: &str,
    classification: &str,
    iterations: usize,
    operation_body: F,
) -> Result<PerformanceSample, PerformanceError>
where
    F: FnMut() -> Result<(), PerformanceError>,
{
    Ok(PerformanceSample {
        operation: operation.into(),
        classification: classification.into(),
        iterations,
        statistics_ms: measure_region_ms(iterations, operation_body)?,
    })
}

/// Runs bounded setup before each sample, then times only the supplied region.
/// This is used when the operation must begin from a known state, such as a
/// dirty journal file or an explicitly cache-warmed fixture binary.
fn measure_process_with_setup<Setup, Operation>(
    operation: &str,
    classification: &str,
    iterations: usize,
    setup: Setup,
    measured_operation: Operation,
) -> Result<PerformanceSample, PerformanceError>
where
    Setup: FnMut() -> Result<(), PerformanceError>,
    Operation: FnMut() -> Result<(), PerformanceError>,
{
    Ok(PerformanceSample {
        operation: operation.into(),
        classification: classification.into(),
        iterations,
        statistics_ms: measure_region_after_setup_ms(iterations, setup, measured_operation)?,
    })
}

/// Times only the supplied region. Fixture creation, warm-up, output parsing,
/// and result serialization are deliberately outside the measured interval.
fn measure_region_ms<F>(
    iterations: usize,
    mut operation: F,
) -> Result<LatencyStatistics, PerformanceError>
where
    F: FnMut() -> Result<(), PerformanceError>,
{
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        operation()?;
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    latency_statistics(&mut samples)
}

fn measure_region_after_setup_ms<Setup, Operation>(
    iterations: usize,
    mut setup: Setup,
    mut operation: Operation,
) -> Result<LatencyStatistics, PerformanceError>
where
    Setup: FnMut() -> Result<(), PerformanceError>,
    Operation: FnMut() -> Result<(), PerformanceError>,
{
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        setup()?;
        let started = Instant::now();
        operation()?;
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    latency_statistics(&mut samples)
}

fn latency_statistics(samples: &mut [f64]) -> Result<LatencyStatistics, PerformanceError> {
    if samples.is_empty()
        || samples
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(PerformanceError::Invalid("latency_samples"));
    }
    let sum = samples.iter().sum::<f64>();
    samples.sort_by(f64::total_cmp);
    let percentile = |percent: usize| samples[(samples.len() * percent).div_ceil(100) - 1];
    Ok(LatencyStatistics {
        min: samples[0],
        p50: percentile(50),
        p95: percentile(95),
        p99: percentile(99),
        max: samples[samples.len() - 1],
        mean: sum / samples.len() as f64,
    })
}

fn run_silent(executable: &Path, arguments: &[&str]) -> Result<(), PerformanceError> {
    let status = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(PerformanceError::ProcessFailed("direct fixture"))
    }
}

fn run_cmd_shell_fixture() -> Result<(), PerformanceError> {
    let status = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()))
        .args(["/D", "/C", "exit 0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(PerformanceError::ProcessFailed("cmd fixture"))
    }
}

fn run_cmd_shell_handler(executable: &Path) -> Result<(), PerformanceError> {
    let handler = executable.to_string_lossy();
    if handler.is_empty()
        || handler.chars().any(|value| {
            !(value.is_ascii_alphanumeric() || matches!(value, ':' | '\\' | '.' | '-' | '_'))
        })
    {
        return Err(PerformanceError::Prerequisite(
            "safe bare synthetic handler fixture path",
        ));
    }
    let status = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()))
        .args(["/D", "/C", handler.as_ref()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(PerformanceError::ProcessFailed("original handler fixture"))
    }
}

fn command_version(program: &str) -> String {
    Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|value| String::from_utf8(value.stdout).ok())
        .map(|value| value.trim().chars().take(120).collect())
        .filter(|value: &String| !value.is_empty())
        .unwrap_or_else(|| "unavailable".into())
}

fn required_windows_platform_value(
    script: &str,
    prerequisite: &'static str,
) -> Result<String, PerformanceError> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(PerformanceError::Prerequisite(prerequisite));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() || value.len() > 160 || value.contains(['\r', '\n', '\0']) {
        return Err(PerformanceError::Prerequisite(prerequisite));
    }
    Ok(value)
}

/// Keeps the benchmark receipt attributable without persisting a hostname,
/// path, username, serial number, or raw CPU model.
fn machine_fingerprint(
    windows_version_build: &str,
    operating_system: &str,
    architecture: &str,
    logical_processors: usize,
    processor_model: &str,
) -> String {
    let mut hasher = Sha256::new();
    for value in [
        windows_version_build,
        operating_system,
        architecture,
        &logical_processors.to_string(),
        processor_model,
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[derive(Clone, Debug, Deserialize)]
struct NamedPipeProbe {
    schema_version: u8,
    cold_connection_ms: Vec<f64>,
    warm_connection_ms: Vec<f64>,
    warm_one_way_write_ms: Vec<f64>,
    warm_ack_round_trip_ms: Vec<f64>,
}

impl NamedPipeProbe {
    fn into_samples(self) -> Result<Vec<PerformanceSample>, PerformanceError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(PerformanceError::Invalid("named_pipe_schema"));
        }
        let values = [
            (
                "windows_named_pipe_cold_connection",
                "cold_local_named_pipe_connect",
                self.cold_connection_ms,
            ),
            (
                "windows_named_pipe_warm_connection",
                "warm_local_named_pipe_connect_after_warmup",
                self.warm_connection_ms,
            ),
            (
                "windows_named_pipe_one_way_bounded_frame_write",
                "warm_local_named_pipe_64_byte_write",
                self.warm_one_way_write_ms,
            ),
            (
                "windows_named_pipe_bounded_frame_ack_round_trip",
                "warm_local_named_pipe_64_byte_write_ack",
                self.warm_ack_round_trip_ms,
            ),
        ];
        values
            .into_iter()
            .map(|(operation, classification, mut samples)| {
                let iterations = samples.len();
                Ok(PerformanceSample {
                    operation: operation.into(),
                    classification: classification.into(),
                    iterations,
                    statistics_ms: latency_statistics(&mut samples)?,
                })
            })
            .collect()
    }
}

fn measure_named_pipes(
    paths: &HarnessPaths,
    iterations: usize,
) -> Result<NamedPipeProbe, PerformanceError> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&paths.named_pipe_probe)
        .args([
            "-Iterations",
            &iterations.to_string(),
            "-Warmup",
            &PIPE_WARMUP_ITERATIONS.to_string(),
        ])
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(PerformanceError::ProcessFailed("Named Pipe probe"));
    }
    serde_json::from_slice(&output.stdout).map_err(PerformanceError::from)
}

#[derive(Clone, Debug, Deserialize)]
struct JobObjectProbe {
    schema_version: u8,
    job_object_cycle_ms: Vec<f64>,
}

fn measure_job_object_cycles(
    fixture: &Path,
    iterations: usize,
) -> Result<PerformanceSample, PerformanceError> {
    let output = Command::new(fixture)
        .args(["--job-probe", &iterations.to_string()])
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(PerformanceError::ProcessFailed("Windows Job Object probe"));
    }
    let mut value: JobObjectProbe = serde_json::from_slice(&output.stdout)?;
    if value.schema_version != SCHEMA_VERSION || value.job_object_cycle_ms.len() != iterations {
        return Err(PerformanceError::Invalid("job_object_probe"));
    }
    Ok(PerformanceSample {
        operation: "windows_job_object_cycle".into(),
        classification: "fresh_child_job_create_assign_release".into(),
        iterations,
        statistics_ms: latency_statistics(&mut value.job_object_cycle_ms)?,
    })
}

struct DisposableFixture {
    root: PathBuf,
    manifest: PathBuf,
}

impl DisposableFixture {
    fn create(handler_executable: &Path) -> Result<Self, PerformanceError> {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "hookstat-g28-{:016x}-{:016x}",
            std::process::id(),
            sequence
        ));
        if root.exists() {
            return Err(PerformanceError::Prerequisite(
                "unique disposable fixture root",
            ));
        }
        let manifest = root.join("manifests").join("g28-fixture.json");
        fs::create_dir_all(
            manifest
                .parent()
                .ok_or(PerformanceError::Invalid("manifest"))?,
        )?;
        // The disposable helper is built at a no-whitespace sibling path.
        // Pass it in the same bare-command form that the existing Windows
        // `cmd /C` proxy accepts, rather than testing Rust argv escaping.
        let command = handler_executable.display().to_string();
        let mut handlers = std::collections::BTreeMap::new();
        handlers.insert(
            "hk_g28_fixture".into(),
            ProxyHandler {
                handler: fixture_handler(),
                command: command.clone(),
                command_windows: Some(command),
            },
        );
        let manifest_data = ProxyManifest {
            schema_version: 1,
            config_path_fingerprint: "g28_fixture_config".into(),
            original_config_sha256: "g28_fixture_original".into(),
            handlers,
        };
        fs::write(&manifest, serde_json::to_vec(&manifest_data)?)?;
        Ok(Self { root, manifest })
    }

    fn run_current_proxy(&self, executable: &Path) -> Result<(), PerformanceError> {
        let manifest = self.manifest.to_string_lossy().into_owned();
        let status = Command::new(executable)
            .args([
                "codex",
                "proxy",
                "--manifest",
                manifest.as_str(),
                "--handler",
                "hk_g28_fixture",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(PerformanceError::ProcessFailed("current v0.3 proxy"))
        }
    }

    fn cleanup(&self) -> Result<(), PerformanceError> {
        if self.root.is_dir() {
            fs::remove_dir_all(&self.root)?;
        }
        Ok(())
    }
}

fn fixture_handler() -> HandlerIdentity {
    HandlerIdentity {
        key: "hk_g28_fixture".into(),
        revision: "hr_g28_fixture".into(),
        label: "g28_fixture".into(),
        source_kind: "synthetic_fixture".into(),
        event: HookEvent::PreToolUse,
        matcher_identity: "g28_any".into(),
        structural_identity: "g28:0:0".into(),
        execution_mode: ExecutionMode::Sync,
    }
}

fn receipt_start(handler: &HandlerIdentity, sequence: u64) -> ReceiptStart {
    ReceiptStart {
        schema_version: 1,
        invocation_id: format!("g28s{sequence:016x}"),
        handler: handler.clone(),
        source: "g28_fixture".into(),
        started_at_unix_ms: 1_000,
        coverage: EvidenceCoverage::Partial,
    }
}

fn receipt_completion(handler: &HandlerIdentity, sequence: u64) -> ReceiptCompletion {
    ReceiptCompletion {
        schema_version: 1,
        invocation_id: format!("g28c{sequence:016x}"),
        handler: handler.clone(),
        source: "g28_fixture".into(),
        started_at_unix_ms: 1_000,
        completed_at_unix_ms: 1_001,
        duration_ms: 1,
        exit_code: Some(0),
        terminal_status: TerminalStatus::Completed,
        coverage: EvidenceCoverage::Partial,
    }
}

fn reproduce_one_second_timeout(
    fixture: &DisposableFixture,
    executable: &Path,
) -> Result<TimeoutReproduction, PerformanceError> {
    let mut manifest: ProxyManifest = serde_json::from_slice(&fs::read(&fixture.manifest)?)?;
    let slow_command = format!(
        "{} --sleep-ms 2000",
        fixture_handler_executable_from_manifest(&fixture.manifest, &manifest)?
    );
    let handler = manifest
        .handlers
        .get_mut("hk_g28_fixture")
        .ok_or(PerformanceError::Invalid("fixture handler"))?;
    handler.command = slow_command.clone();
    handler.command_windows = Some(slow_command);
    // Keep timeout evidence in its own disposable data root. The main fixture
    // already contains completed proxy benchmark receipts and must not alter
    // the start-only assertion for this independent failure reproduction.
    let timeout_root = fixture.root.join("timeout-reproduction");
    let slow_manifest = timeout_root.join("manifests").join("g28-timeout.json");
    fs::create_dir_all(
        slow_manifest
            .parent()
            .ok_or(PerformanceError::Invalid("timeout manifest"))?,
    )?;
    fs::write(&slow_manifest, serde_json::to_vec(&manifest)?)?;
    let slow_manifest_argument = slow_manifest.to_string_lossy().into_owned();

    let mut child = Command::new(executable)
        .args([
            "codex",
            "proxy",
            "--manifest",
            slow_manifest_argument.as_str(),
            "--handler",
            "hk_g28_fixture",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Err(PerformanceError::ProcessFailed("one-second proxy fixture"));
        }
        thread::sleep(Duration::from_millis(5));
    }
    child.kill()?;
    let _ = child.wait()?;
    let scan = ReceiptSpool::open_existing(timeout_root.join("receipts"))?.scan();
    let start_evidence_emitted = scan.invocations.len() == 1;
    let completion_evidence_missing = scan.starts_without_completion == 1;
    let terminal_status = scan
        .invocations
        .first()
        .map(|value| value.terminal_status)
        .ok_or(PerformanceError::Invalid("timeout evidence"))?;
    if !start_evidence_emitted
        || !completion_evidence_missing
        || terminal_status != TerminalStatus::Incomplete
    {
        return Err(PerformanceError::Invalid("timeout reproduction"));
    }
    Ok(TimeoutReproduction {
        declaration_timeout_ms: 1_000,
        start_evidence_emitted,
        completion_evidence_missing,
        terminal_status: terminal_status.as_storage().into(),
        isolated_disposable_fixture: true,
    })
}

fn fixture_handler_executable_from_manifest(
    manifest_path: &Path,
    manifest: &ProxyManifest,
) -> Result<String, PerformanceError> {
    let handler = manifest
        .handlers
        .get("hk_g28_fixture")
        .ok_or(PerformanceError::Invalid("fixture handler"))?;
    let command = handler
        .command_windows
        .as_deref()
        .ok_or(PerformanceError::Invalid("fixture command"))?;
    let trimmed = command.trim().trim_matches('"');
    if trimmed.is_empty() || manifest_path.as_os_str().is_empty() {
        return Err(PerformanceError::Invalid("fixture command"));
    }
    Ok(trimmed.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_statistics_use_nearest_rank_and_reject_empty_input() {
        let mut samples = vec![4.0, 1.0, 2.0, 3.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let statistics = latency_statistics(&mut samples).unwrap();
        assert_eq!(statistics.min, 1.0);
        assert_eq!(statistics.p50, 5.0);
        assert_eq!(statistics.p95, 10.0);
        assert_eq!(statistics.p99, 10.0);
        assert!(latency_statistics(&mut []).is_err());
    }

    #[test]
    fn measured_region_excludes_per_sample_setup_work() {
        let statistics = measure_region_after_setup_ms(
            10,
            || {
                thread::sleep(Duration::from_millis(15));
                Ok(())
            },
            || Ok(()),
        )
        .unwrap();
        assert!(statistics.max < 5.0);
    }

    #[test]
    fn receipt_contract_is_parseable_bounded_and_has_no_private_fields() {
        let mut receipt = PerformanceReceipt {
            schema_version: SCHEMA_VERSION,
            run_kind: "hs_g28_windows_hot_path_baseline".into(),
            platform: PlatformMetadata {
                operating_system: "windows".into(),
                architecture: "x86_64".into(),
                windows_version_build: "10.0.12345".into(),
                logical_processors: 1,
                benchmark_machine_fingerprint: "a".repeat(64),
                rustc: "rustc test".into(),
                cargo: "cargo test".into(),
                build_profile: "release".into(),
            },
            sample_plan: SamplePlan {
                process_iterations: 100,
                io_iterations: 10_000,
                pipe_iterations: 1_000,
                pipe_warmup_iterations: 25,
                bounded_frame_bytes: PIPE_FRAME_BYTES,
                percentile_method: "nearest_rank".into(),
            },
            samples: REQUIRED_OPERATION_CLASSIFICATIONS
                .iter()
                .map(|(operation, classification)| PerformanceSample {
                    operation: (*operation).into(),
                    classification: (*classification).into(),
                    iterations: 1,
                    statistics_ms: LatencyStatistics {
                        min: 0.0,
                        p50: 0.0,
                        p95: 0.0,
                        p99: 0.0,
                        max: 0.0,
                        mean: 0.0,
                    },
                })
                .collect(),
            one_second_timeout_reproduction: TimeoutReproduction {
                declaration_timeout_ms: 1_000,
                start_evidence_emitted: true,
                completion_evidence_missing: true,
                terminal_status: "incomplete".into(),
                isolated_disposable_fixture: true,
            },
            privacy: PrivacyReceipt {
                owner_live_codex_config_mutated: false,
                raw_private_content_captured: false,
            },
        };
        receipt.validate().unwrap();
        let encoded = serde_json::to_string(&receipt).unwrap();
        let decoded: PerformanceReceipt = serde_json::from_str(&encoded).unwrap();
        decoded.validate().unwrap();
        let document: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_no_private_content_keys(&document);
        receipt.sample_plan.bounded_frame_bytes = PIPE_FRAME_BYTES + 1;
        assert!(receipt.validate().is_err());
    }

    #[test]
    fn receipt_rejects_an_ambiguous_warm_or_cold_classification() {
        let mut receipt = PerformanceReceipt {
            schema_version: SCHEMA_VERSION,
            run_kind: "hs_g28_windows_hot_path_baseline".into(),
            platform: PlatformMetadata {
                operating_system: "windows".into(),
                architecture: "x86_64".into(),
                windows_version_build: "10.0.12345".into(),
                logical_processors: 1,
                benchmark_machine_fingerprint: "a".repeat(64),
                rustc: "rustc test".into(),
                cargo: "cargo test".into(),
                build_profile: "release".into(),
            },
            sample_plan: SamplePlan {
                process_iterations: 100,
                io_iterations: 10_000,
                pipe_iterations: 1_000,
                pipe_warmup_iterations: 25,
                bounded_frame_bytes: PIPE_FRAME_BYTES,
                percentile_method: "nearest_rank".into(),
            },
            samples: REQUIRED_OPERATION_CLASSIFICATIONS
                .iter()
                .map(|(operation, classification)| PerformanceSample {
                    operation: (*operation).into(),
                    classification: (*classification).into(),
                    iterations: 1,
                    statistics_ms: LatencyStatistics {
                        min: 0.0,
                        p50: 0.0,
                        p95: 0.0,
                        p99: 0.0,
                        max: 0.0,
                        mean: 0.0,
                    },
                })
                .collect(),
            one_second_timeout_reproduction: TimeoutReproduction {
                declaration_timeout_ms: 1_000,
                start_evidence_emitted: true,
                completion_evidence_missing: true,
                terminal_status: "incomplete".into(),
                isolated_disposable_fixture: true,
            },
            privacy: PrivacyReceipt {
                owner_live_codex_config_mutated: false,
                raw_private_content_captured: false,
            },
        };
        receipt.samples[0].classification = "cold_process".into();
        assert!(receipt.validate().is_err());
    }

    fn assert_no_private_content_keys(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(entries) => {
                for (key, value) in entries {
                    assert!(
                        !matches!(
                            key.as_str(),
                            "prompt"
                                | "assistant_content"
                                | "tool_input"
                                | "tool_output"
                                | "stdout"
                                | "stderr"
                                | "raw_command"
                                | "file_path"
                        ),
                        "receipt unexpectedly serializes private field {key}"
                    );
                    assert_no_private_content_keys(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    assert_no_private_content_keys(value);
                }
            }
            _ => {}
        }
    }
}
