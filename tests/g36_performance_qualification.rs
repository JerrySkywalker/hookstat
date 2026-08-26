//! Explicit Windows-only G36 release-artifact qualification.
//!
//! This ignored test is deliberately invoked only by the release train. It
//! uses a disposable capsule root and state root, never discovers or changes
//! Codex configuration, and serializes no command, path, payload, or host
//! identity into its receipt.

#![cfg(all(windows, feature = "performance-harness"))]

#[allow(dead_code)]
#[path = "../src/hook_shim.rs"]
mod hook_shim;
#[allow(dead_code)]
#[path = "../src/ipc_client.rs"]
mod ipc_client;

use hook_shim::{
    CapsuleStore, ExecutionPlan, HandlerCapsule, InstrumentationEnvelope, OriginalHandlerBudget,
    capsule_file_name, write_key_for_test,
};
use hookstat::g36_host_admission::{
    G28_REFERENCE_WARM_P95_MS, G28_REFERENCE_WARM_P99_MS, HOST_CONTROL_METHODOLOGY,
    HOST_CONTROL_P95_LIMIT_MS, HOST_CONTROL_P99_LIMIT_MS, MAX_COMPARABLE_STARTUP_BIAS_MS,
    PRODUCT_WARM_P95_LIMIT_MS, PRODUCT_WARM_P99_LIMIT_MS, StartupComparabilityDisposition,
    TailLatency, WarmWindowDisposition, classify_startup_comparability,
    classify_warm_window_with_health_and_oracle,
};
use hookstat::ipc::{BrokerConfig, BrokerHost};
use interprocess::local_socket::traits::Listener as _;
use ipc_client::{
    Completion, CooperativeProducer, ExitClassification, LifecycleFrame, Listener, LocalEndpoint,
    ObservationDisposition, TerminalOutcome,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const QUALIFYING_RUNS: usize = 5;
const SAMPLES_PER_RUN: usize = 100;
const WARMUPS_PER_SAMPLE: usize = 25;
const COOPERATIVE_P95_LIMIT_MS: f64 = 1.0;
const COOPERATIVE_P99_LIMIT_MS: f64 = 2.0;
const SHIM_COLD_P95_LIMIT_MS: f64 = 50.0;
const DEFAULT_MAX_WARM_WINDOW_ATTEMPTS: usize = 25;
const DEFAULT_REJECT_RETRY_INTERVAL: Duration = Duration::from_secs(60);
const ORACLE_ROOT_ENV: &str = "HOOKSTAT_G36_ORACLE_ROOT";
const SHIPPING_SHIM_ENV: &str = "HOOKSTAT_G36_SHIPPING_SHIM";
const COOPERATIVE_OUTPUT_ENV: &str = "HOOKSTAT_G36_COOPERATIVE_OUTPUT";
const ORACLE_RECORD_BYTES: usize = 32;

#[derive(Clone, Serialize)]
struct Timing {
    samples: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    max_ms: f64,
}

#[derive(Clone, Serialize)]
struct Series {
    kind: &'static str,
    run: usize,
    timing: Timing,
    observation_gaps: usize,
}

#[derive(Serialize)]
struct CooperativeAcceptanceReceipt {
    schema_version: u8,
    run_kind: &'static str,
    build_profile: &'static str,
    source_git_head: String,
    source_tracked_worktree_clean: bool,
    qualifying_runs: usize,
    samples_per_run: usize,
    percentile_method: &'static str,
    p95_limit_ms: f64,
    p99_limit_ms: f64,
    series: Vec<Series>,
    worst_p95_ms: f64,
    worst_p99_ms: f64,
    observation_gaps: usize,
    outcome: &'static str,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

#[derive(Clone, Serialize)]
struct StartupComparisonSeries {
    run: usize,
    shipping: Timing,
    instrumented: Timing,
    shipping_minus_instrumented_p99_ms: f64,
}

#[derive(Clone, Serialize)]
struct StartupComparabilityAttempt {
    attempt: usize,
    pre_control: Timing,
    startup_comparison_series: Vec<StartupComparisonSeries>,
    shipping_startup_worst_p99_ms: f64,
    instrumented_startup_worst_p99_ms: f64,
    startup_tail_bias_correction_ms: f64,
    post_control: Timing,
    disposition: StartupComparabilityDisposition,
}

#[derive(Serialize)]
struct StartupComparabilityReceipt<'a> {
    schema_version: u8,
    run_kind: &'static str,
    source_git_head: &'a str,
    shipping_binary_size_bytes: u64,
    instrumented_binary_size_bytes: u64,
    shipping_binary_sha256: &'a str,
    instrumented_binary_sha256: &'a str,
    control_methodology: &'static str,
    control_fixture: &'static str,
    control_samples: usize,
    control_warmups_per_timed_sample: usize,
    percentile_method: &'static str,
    control_p95_limit_ms: f64,
    control_p99_limit_ms: f64,
    maximum_comparable_startup_bias_ms: f64,
    comparison_runs: usize,
    samples_per_build_per_run: usize,
    comparison: &'a StartupComparabilityAttempt,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

#[derive(Clone, Serialize)]
struct WarmWindow {
    attempt: usize,
    pre_control: Timing,
    raw_candidate: Timing,
    candidate: Timing,
    candidate_hookstat_induced_timeouts: usize,
    candidate_unexpected_terminal_results: usize,
    candidate_oracle_observation_gaps: usize,
    post_control: Timing,
    disposition: WarmWindowDisposition,
}

#[derive(Serialize)]
struct WarmWindowReceipt<'a> {
    schema_version: u8,
    run_kind: &'static str,
    source_git_head: &'a str,
    shipping_binary_size_bytes: u64,
    instrumented_binary_size_bytes: u64,
    shipping_binary_sha256: &'a str,
    instrumented_binary_sha256: &'a str,
    control_methodology: &'static str,
    control_fixture: &'static str,
    control_samples: usize,
    control_warmups_per_timed_sample: usize,
    percentile_method: &'static str,
    control_p95_limit_ms: f64,
    control_p99_limit_ms: f64,
    g28_reference_warm_p95_ms: f64,
    g28_reference_warm_p99_ms: f64,
    product_p95_limit_ms: f64,
    product_p99_limit_ms: f64,
    further_automatic_budget_relaxation: bool,
    accepted_startup_comparability_attempt: usize,
    startup_tail_bias_correction_ms: f64,
    window: &'a WarmWindow,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u8,
    run_kind: &'static str,
    release_artifacts: bool,
    build_profile: &'static str,
    shim_measurement: &'static str,
    paired_method_identifiable: bool,
    same_invocation_oracle: bool,
    oracle_transport: &'static str,
    oracle_record_bytes: usize,
    observed_overhead_includes_oracle_side_channel: bool,
    host_control_methodology: &'static str,
    host_control_fixture: &'static str,
    host_control_samples_per_phase: usize,
    host_control_warmups_per_timed_sample: usize,
    host_control_percentile_method: &'static str,
    host_control_p95_limit_ms: f64,
    host_control_p99_limit_ms: f64,
    g28_reference_warm_p95_ms: f64,
    g28_reference_warm_p99_ms: f64,
    v031_release_warm_p95_ms: f64,
    v031_release_warm_p99_ms: f64,
    further_automatic_budget_relaxation: bool,
    warm_admitted_runs_required: usize,
    max_warm_window_attempts: usize,
    rejected_window_retry_interval_ms: u64,
    warmup_definition: &'static str,
    warm_harness_self_load: bool,
    qualifying_runs: usize,
    samples_per_run: usize,
    warmups_per_timed_sample: usize,
    collector_model: &'static str,
    elapsed_capture: &'static str,
    source_git_head: String,
    source_tracked_worktree_clean: bool,
    shipping_binary_size_bytes: u64,
    instrumented_binary_size_bytes: u64,
    shipping_binary_sha256: String,
    instrumented_binary_sha256: String,
    startup_comparability_attempts: Vec<StartupComparabilityAttempt>,
    accepted_startup_comparability_attempt: usize,
    startup_comparison_series: Vec<StartupComparisonSeries>,
    shipping_startup_worst_p99_ms: f64,
    instrumented_startup_worst_p99_ms: f64,
    startup_tail_bias_correction_ms: f64,
    startup_bias_material: bool,
    warm_window_attempts: Vec<WarmWindow>,
    host_control_rejected_windows: usize,
    warm_admitted_runs: usize,
    admitted_recalibrated_failure_occurred: bool,
    warm_method_invalidation_occurred: bool,
    admitted_warm_hookstat_induced_timeouts: usize,
    admitted_warm_unexpected_terminal_results: usize,
    admitted_warm_oracle_observation_gaps: usize,
    cold_hookstat_induced_timeouts: usize,
    cold_unexpected_terminal_results: usize,
    cold_oracle_observation_gaps: usize,
    raw_oracle_series: Vec<Series>,
    series: Vec<Series>,
    oracle_primary_record_worst_p95_ms: f64,
    oracle_primary_record_worst_p99_ms: f64,
    cooperative_worst_p95_ms: f64,
    cooperative_worst_p99_ms: f64,
    shim_warm_worst_p95_ms: Option<f64>,
    shim_warm_worst_p99_ms: Option<f64>,
    shim_cold_worst_p95_ms: f64,
    healthy_near_timeout_runs: usize,
    hookstat_induced_timeouts_for_healthy_hook: usize,
    unexpected_terminal_results_for_healthy_hook: usize,
    oracle_observation_gaps: usize,
    outcome: &'static str,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

struct RawOracleRun {
    kind: &'static str,
    run: usize,
    overhead_ms: Vec<f64>,
    oracle_primary_record_ms: Vec<f64>,
    hookstat_induced_timeouts: usize,
    unexpected_terminal_results: usize,
    oracle_observation_gaps: usize,
}

enum OracleReceive {
    Record {
        child_ns: u64,
        oracle_primary_record_ns: u64,
    },
    MissingRecord(ExitStatus),
}

struct OracleSample {
    overhead_ms: Option<f64>,
    oracle_primary_record_ms: Option<f64>,
    hookstat_induced_timeouts: usize,
    unexpected_terminal_results: usize,
    oracle_observation_gaps: usize,
}

struct OracleContext<'a> {
    shim: &'a Path,
    capsule: &'a Path,
    capsule_root: &'a Path,
    state_root: &'a Path,
    oracle_root: &'a Path,
    listener: &'a Listener,
}

struct QualificationArtifacts<'a> {
    shipping_shim: &'a Path,
    instrumented_shim: &'a Path,
    shipping_binary_sha256: &'a str,
    instrumented_binary_sha256: &'a str,
}

fn capsule(plan: ExecutionPlan, budget: Duration) -> HandlerCapsule {
    HandlerCapsule {
        handler_key: "g36_qualification_handler".into(),
        revision: "g36_qualification_revision".into(),
        definition_fingerprint: "sha256:g36_qualification".into(),
        runtime: "controlled_runtime".into(),
        runtime_instance: "controlled_instance".into(),
        event: "controlled_event".into(),
        source_scope: "controlled_scope".into(),
        original_budget: OriginalHandlerBudget(budget),
        instrumentation_envelope: InstrumentationEnvelope(Duration::from_millis(50)),
        execution: plan,
    }
}

fn seal(root: &Path, capsule: &HandlerCapsule) -> PathBuf {
    fs::create_dir(root).unwrap();
    let key = [0x36_u8; 32];
    let store = CapsuleStore::open(root).unwrap();
    write_key_for_test(root, &key).unwrap();
    let name = capsule_file_name(capsule).unwrap();
    store
        .write_for_test(Path::new(&name), capsule, &key)
        .unwrap();
    root.join(name)
}

fn timed_help(shim: &Path) -> f64 {
    let started = Instant::now();
    let status = Command::new(shim)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "timed shim help did not exit zero");
    started.elapsed().as_secs_f64() * 1_000.0
}

fn timing(mut milliseconds: Vec<f64>) -> Timing {
    milliseconds.sort_by(f64::total_cmp);
    let percentile = |percent: f64| {
        milliseconds[((milliseconds.len() as f64 * percent).ceil() as usize).saturating_sub(1)]
    };
    Timing {
        samples: milliseconds.len(),
        p50_ms: percentile(0.50),
        p95_ms: percentile(0.95),
        p99_ms: percentile(0.99),
        max_ms: *milliseconds.last().unwrap(),
    }
}

fn run_silent(executable: &Path) {
    let status = Command::new(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("launch process-start control fixture");
    assert!(status.success(), "process-start control fixture failed");
}

fn host_control_run(fixture: &Path) -> Timing {
    let mut samples = Vec::with_capacity(SAMPLES_PER_RUN);
    for _ in 0..SAMPLES_PER_RUN {
        for _ in 0..WARMUPS_PER_SAMPLE {
            run_silent(fixture);
        }
        let started = Instant::now();
        run_silent(fixture);
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    timing(samples)
}

fn tail(timing: &Timing) -> TailLatency {
    TailLatency::new(timing.p95_ms, timing.p99_ms)
}

fn configured_max_warm_window_attempts() -> usize {
    match std::env::var("HOOKSTAT_G36_MAX_WARM_WINDOWS") {
        Ok(value) => {
            let value = value
                .parse::<usize>()
                .expect("HOOKSTAT_G36_MAX_WARM_WINDOWS must be an integer");
            assert!(
                (QUALIFYING_RUNS..=100).contains(&value),
                "HOOKSTAT_G36_MAX_WARM_WINDOWS must be between 5 and 100"
            );
            value
        }
        Err(std::env::VarError::NotPresent) => DEFAULT_MAX_WARM_WINDOW_ATTEMPTS,
        Err(std::env::VarError::NotUnicode(_)) => {
            panic!("HOOKSTAT_G36_MAX_WARM_WINDOWS must be Unicode")
        }
    }
}

fn configured_reject_retry_interval() -> Duration {
    match std::env::var("HOOKSTAT_G36_REJECT_RETRY_INTERVAL_MS") {
        Ok(value) => {
            let milliseconds = value
                .parse::<u64>()
                .expect("HOOKSTAT_G36_REJECT_RETRY_INTERVAL_MS must be an integer");
            assert!(
                (1_000..=300_000).contains(&milliseconds),
                "HOOKSTAT_G36_REJECT_RETRY_INTERVAL_MS must be between 1000 and 300000"
            );
            Duration::from_millis(milliseconds)
        }
        Err(std::env::VarError::NotPresent) => DEFAULT_REJECT_RETRY_INTERVAL,
        Err(std::env::VarError::NotUnicode(_)) => {
            panic!("HOOKSTAT_G36_REJECT_RETRY_INTERVAL_MS must be Unicode")
        }
    }
}

fn lifecycle(invocation: String) -> LifecycleFrame {
    LifecycleFrame {
        runtime: "controlled_runtime".into(),
        runtime_instance: "controlled_instance".into(),
        invocation,
        handler: "g36_qualification_handler".into(),
        event: "controlled_event".into(),
        source_scope: "controlled_scope".into(),
        revision: Some("g36_qualification_revision".into()),
        occurred_at_unix_ms: 1,
    }
}

fn complete() -> Completion {
    Completion {
        terminal_status: TerminalOutcome::Completed,
        exit_classification: ExitClassification::ExitCode,
        exit_value: Some(0),
        duration_ms: 0,
    }
}

fn emit_cooperative_run(producer: &CooperativeProducer, run: usize) -> (Timing, usize) {
    let mut samples = Vec::with_capacity(SAMPLES_PER_RUN);
    let mut observation_gaps = 0;
    for sample in 0..SAMPLES_PER_RUN {
        let frame = lifecycle(format!("g36-cooperative-{run}-{sample}"));
        let started = Instant::now();
        let disposition = producer.emit_start(frame.clone());
        let elapsed = started.elapsed();
        if disposition == ObservationDisposition::Accepted {
            samples.push(elapsed.as_secs_f64() * 1_000.0);
        } else {
            // A non-accepted observation remains a truthful fail-open gap.
            // It never enters a latency percentile as a successful sample.
            observation_gaps += 1;
        }
    }
    (timing(samples), observation_gaps)
}

fn warm_actual_shipping_shim(shim: &Path) {
    // G28 defines warm as fresh executable launches, not prior end-to-end
    // evidence transactions. `--help` starts the exact shipping shim binary,
    // parses its real arguments, and exits before it can create broker/WAL
    // work. This retains the G28 cache-warmed fresh-start definition without
    // self-loading the timed broker or filesystem path.
    for _ in 0..WARMUPS_PER_SAMPLE {
        let status = Command::new(shim)
            .arg("--help")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "shipping shim warm-up did not exit zero");
    }
}

fn wait_for_broker(producer: &CooperativeProducer) {
    for attempt in 0..50 {
        let frame = lifecycle(format!("g36-readiness-{attempt}"));
        if producer.emit_start(frame.clone()) == ObservationDisposition::Accepted {
            assert_eq!(
                producer.emit_complete(frame, complete()),
                ObservationDisposition::Accepted
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("disposable broker did not become ready before qualification");
}

fn decode_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("fixed u64 field"))
}

fn bounded_missing_oracle_status(child: &mut Child) -> ExitStatus {
    if let Some(status) = child.try_wait().expect("query instrumented shim") {
        return status;
    }
    child
        .kill()
        .expect("terminate exact owned shim after bounded oracle failure");
    child
        .wait()
        .expect("wait for exact owned shim after bounded oracle failure")
}

fn receive_oracle(listener: &Listener, child: &mut Child) -> OracleReceive {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut stream = loop {
        match listener.accept() {
            Ok(stream) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if let Some(status) = child.try_wait().expect("query instrumented shim") {
                    return OracleReceive::MissingRecord(status);
                }
                if Instant::now() >= deadline {
                    return OracleReceive::MissingRecord(bounded_missing_oracle_status(child));
                }
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(error) => panic!("oracle listener failed: {error}"),
        }
    };
    let mut record = [0_u8; ORACLE_RECORD_BYTES];
    if stream.read_exact(&mut record).is_err() {
        return OracleReceive::MissingRecord(bounded_missing_oracle_status(child));
    }
    assert_eq!(&record[..4], b"HSO1");
    assert_eq!(&record[16..20], b"HSO2");
    assert_eq!(record[4], 1);
    assert_eq!(record[20], 1);
    assert!(record[5..8].iter().all(|byte| *byte == 0));
    assert!(record[21..24].iter().all(|byte| *byte == 0));
    OracleReceive::Record {
        child_ns: decode_u64(&record[8..16]),
        oracle_primary_record_ns: decode_u64(&record[24..32]),
    }
}

fn candidate_terminal_observation(status: &ExitStatus) -> (usize, usize) {
    if status.success() {
        (0, 0)
    } else if status.code() == Some(124) {
        (1, 0)
    } else {
        (0, 1)
    }
}

fn launch_with_oracle(context: &OracleContext<'_>) -> OracleSample {
    let started = Instant::now();
    let mut child = Command::new(context.shim)
        .env(ORACLE_ROOT_ENV, context.oracle_root)
        .arg("--capsule")
        .arg(context.capsule)
        .arg("--capsule-root")
        .arg(context.capsule_root)
        .arg("--state-root")
        .arg(context.state_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch instrumented shim");
    match receive_oracle(context.listener, &mut child) {
        OracleReceive::Record {
            child_ns,
            oracle_primary_record_ns,
        } => {
            let status = child.wait().expect("wait for instrumented shim");
            let full_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let (hookstat_induced_timeouts, unexpected_terminal_results) =
                candidate_terminal_observation(&status);
            assert!(
                child_ns <= full_ns,
                "child interval exceeded parent lifetime"
            );
            OracleSample {
                overhead_ms: Some((full_ns - child_ns) as f64 / 1_000_000.0),
                oracle_primary_record_ms: Some(oracle_primary_record_ns as f64 / 1_000_000.0),
                hookstat_induced_timeouts,
                unexpected_terminal_results,
                oracle_observation_gaps: 0,
            }
        }
        OracleReceive::MissingRecord(status) => {
            let (hookstat_induced_timeouts, unexpected_terminal_results) =
                candidate_terminal_observation(&status);
            OracleSample {
                overhead_ms: None,
                oracle_primary_record_ms: None,
                hookstat_induced_timeouts,
                unexpected_terminal_results,
                oracle_observation_gaps: 1,
            }
        }
    }
}

fn emit_oracle_run(
    kind: &'static str,
    run: usize,
    context: &OracleContext<'_>,
    warmed: bool,
) -> RawOracleRun {
    let mut overhead_ms = Vec::with_capacity(SAMPLES_PER_RUN);
    let mut oracle_primary_record_ms = Vec::with_capacity(SAMPLES_PER_RUN);
    let mut hookstat_induced_timeouts = 0;
    let mut unexpected_terminal_results = 0;
    let mut oracle_observation_gaps = 0;
    for _ in 0..SAMPLES_PER_RUN {
        if warmed {
            warm_actual_shipping_shim(context.shim);
        }
        let sample = launch_with_oracle(context);
        overhead_ms.extend(sample.overhead_ms);
        oracle_primary_record_ms.extend(sample.oracle_primary_record_ms);
        hookstat_induced_timeouts += sample.hookstat_induced_timeouts;
        unexpected_terminal_results += sample.unexpected_terminal_results;
        oracle_observation_gaps += sample.oracle_observation_gaps;
    }
    RawOracleRun {
        kind,
        run,
        overhead_ms,
        oracle_primary_record_ms,
        hookstat_induced_timeouts,
        unexpected_terminal_results,
        oracle_observation_gaps,
    }
}

fn startup_comparison_run(
    shipping: &Path,
    instrumented: &Path,
    run: usize,
) -> StartupComparisonSeries {
    let mut shipping_samples = Vec::with_capacity(SAMPLES_PER_RUN);
    let mut instrumented_samples = Vec::with_capacity(SAMPLES_PER_RUN);
    for sample in 0..SAMPLES_PER_RUN {
        warm_actual_shipping_shim(shipping);
        warm_actual_shipping_shim(instrumented);
        if sample % 2 == 0 {
            shipping_samples.push(timed_help(shipping));
            instrumented_samples.push(timed_help(instrumented));
        } else {
            instrumented_samples.push(timed_help(instrumented));
            shipping_samples.push(timed_help(shipping));
        }
    }
    let shipping = timing(shipping_samples);
    let instrumented = timing(instrumented_samples);
    StartupComparisonSeries {
        run,
        shipping_minus_instrumented_p99_ms: (shipping.p99_ms - instrumented.p99_ms).max(0.0),
        shipping,
        instrumented,
    }
}

fn startup_tail_bias(series: &[StartupComparisonSeries]) -> (f64, f64, f64) {
    // The two builds are independently scheduled populations. Compare their
    // complete worst-of-five p99 envelopes; selecting the maximum signed
    // difference from one run pair would select differential scheduler noise.
    let shipping_worst_p99_ms = series
        .iter()
        .map(|series| series.shipping.p99_ms)
        .max_by(f64::total_cmp)
        .unwrap();
    let instrumented_worst_p99_ms = series
        .iter()
        .map(|series| series.instrumented.p99_ms)
        .max_by(f64::total_cmp)
        .unwrap();
    (
        shipping_worst_p99_ms,
        instrumented_worst_p99_ms,
        (shipping_worst_p99_ms - instrumented_worst_p99_ms).max(0.0),
    )
}

fn worst(series: &[Series], kind: &str, percentile: fn(&Timing) -> f64) -> f64 {
    series
        .iter()
        .filter(|series| series.kind == kind)
        .map(|series| percentile(&series.timing))
        .max_by(f64::total_cmp)
        .unwrap()
}

fn worst_optional(series: &[Series], kind: &str, percentile: fn(&Timing) -> f64) -> Option<f64> {
    series
        .iter()
        .filter(|series| series.kind == kind)
        .map(|series| percentile(&series.timing))
        .max_by(f64::total_cmp)
}

fn qualification_output_path() -> PathBuf {
    PathBuf::from(
        std::env::var_os("HOOKSTAT_G36_PERFORMANCE_OUTPUT")
            .expect("HOOKSTAT_G36_PERFORMANCE_OUTPUT is required for a qualifying run"),
    )
}

fn write_json_once(path: &Path, value: &impl Serialize) {
    let parent = path.parent().expect("qualification output has a parent");
    fs::create_dir_all(parent).unwrap();
    assert!(
        !path.exists(),
        "qualification output already exists; historical evidence is immutable"
    );
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("json");
    let temporary = path.with_extension(format!("{extension}.tmp-{}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .expect("create unique qualification receipt staging file");
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .expect("write qualification receipt");
    file.sync_all().expect("sync qualification receipt");
    drop(file);
    fs::rename(&temporary, path).expect("publish qualification receipt atomically");
}

fn warm_window_output_path(output: &Path, attempt: usize) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("qualification output has a Unicode file stem");
    output.with_file_name(format!("{stem}-window-{attempt:03}.json"))
}

fn startup_comparability_output_path(output: &Path, attempt: usize) -> PathBuf {
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("qualification output has a Unicode file stem");
    output.with_file_name(format!("{stem}-startup-comparability-{attempt:03}.json"))
}

fn write_startup_comparability_receipt(
    output: &Path,
    source_git_head: &str,
    artifacts: &QualificationArtifacts<'_>,
    comparison: &StartupComparabilityAttempt,
) {
    let receipt = StartupComparabilityReceipt {
        schema_version: 1,
        run_kind: "g36_startup_build_comparability_admission",
        source_git_head,
        shipping_binary_size_bytes: fs::metadata(artifacts.shipping_shim).unwrap().len(),
        instrumented_binary_size_bytes: fs::metadata(artifacts.instrumented_shim).unwrap().len(),
        shipping_binary_sha256: artifacts.shipping_binary_sha256,
        instrumented_binary_sha256: artifacts.instrumented_binary_sha256,
        control_methodology: HOST_CONTROL_METHODOLOGY,
        control_fixture: "hookstat-hook-fixture",
        control_samples: SAMPLES_PER_RUN,
        control_warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        percentile_method: "nearest_rank",
        control_p95_limit_ms: HOST_CONTROL_P95_LIMIT_MS,
        control_p99_limit_ms: HOST_CONTROL_P99_LIMIT_MS,
        maximum_comparable_startup_bias_ms: MAX_COMPARABLE_STARTUP_BIAS_MS,
        comparison_runs: QUALIFYING_RUNS,
        samples_per_build_per_run: SAMPLES_PER_RUN,
        comparison,
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    write_json_once(
        &startup_comparability_output_path(output, comparison.attempt),
        &receipt,
    );
}

fn write_warm_window_receipt(
    output: &Path,
    source_git_head: &str,
    artifacts: &QualificationArtifacts<'_>,
    accepted_startup_comparability_attempt: usize,
    startup_tail_bias_correction_ms: f64,
    window: &WarmWindow,
) {
    let receipt = WarmWindowReceipt {
        schema_version: 2,
        run_kind: "g36_warm_host_admission_window",
        source_git_head,
        shipping_binary_size_bytes: fs::metadata(artifacts.shipping_shim).unwrap().len(),
        instrumented_binary_size_bytes: fs::metadata(artifacts.instrumented_shim).unwrap().len(),
        shipping_binary_sha256: artifacts.shipping_binary_sha256,
        instrumented_binary_sha256: artifacts.instrumented_binary_sha256,
        control_methodology: HOST_CONTROL_METHODOLOGY,
        control_fixture: "hookstat-hook-fixture",
        control_samples: SAMPLES_PER_RUN,
        control_warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        percentile_method: "nearest_rank",
        control_p95_limit_ms: HOST_CONTROL_P95_LIMIT_MS,
        control_p99_limit_ms: HOST_CONTROL_P99_LIMIT_MS,
        g28_reference_warm_p95_ms: G28_REFERENCE_WARM_P95_MS,
        g28_reference_warm_p99_ms: G28_REFERENCE_WARM_P99_MS,
        product_p95_limit_ms: PRODUCT_WARM_P95_LIMIT_MS,
        product_p99_limit_ms: PRODUCT_WARM_P99_LIMIT_MS,
        further_automatic_budget_relaxation: false,
        accepted_startup_comparability_attempt,
        startup_tail_bias_correction_ms,
        window,
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    write_json_once(&warm_window_output_path(output, window.attempt), &receipt);
}

fn write_receipt(output: &Path, receipt: &Receipt) {
    write_json_once(output, receipt);
}

fn source_git_provenance() -> (String, bool) {
    let root = env!("CARGO_MANIFEST_DIR");
    let head = Command::new("git")
        .args(["-C", root, "rev-parse", "HEAD"])
        .output()
        .expect("read qualification source HEAD");
    assert!(
        head.status.success(),
        "qualification source HEAD unavailable"
    );
    let head = String::from_utf8(head.stdout)
        .expect("qualification source HEAD is UTF-8")
        .trim()
        .to_owned();
    assert!(
        head.len() == 40 && head.bytes().all(|value| value.is_ascii_hexdigit()),
        "qualification source HEAD is not a full Git object id"
    );
    let status = Command::new("git")
        .args([
            "-C",
            root,
            "status",
            "--porcelain=v1",
            "--untracked-files=no",
        ])
        .output()
        .expect("read qualification tracked status");
    assert!(
        status.status.success(),
        "qualification tracked status unavailable"
    );
    (head, status.stdout.is_empty())
}

fn sha256_file(path: &Path) -> String {
    let mut file = fs::File::open(path).expect("qualification artifact is readable");
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).expect("hash qualification artifact");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(debug_assertions)]
fn require_release_profile() {
    panic!("G36 qualification requires cargo test --release; debug artifacts are diagnostic only");
}

#[cfg(not(debug_assertions))]
fn require_release_profile() {}

#[test]
#[ignore = "explicit release-profile cooperative IPC acceptance"]
fn cooperative_ipc_meets_the_v031_production_budget() {
    require_release_profile();
    let (source_git_head, source_tracked_worktree_clean) = source_git_provenance();
    assert!(
        source_tracked_worktree_clean,
        "qualification requires a tracked-clean source head"
    );
    let output = PathBuf::from(
        std::env::var_os(COOPERATIVE_OUTPUT_ENV)
            .expect("HOOKSTAT_G36_COOPERATIVE_OUTPUT is required"),
    );
    let temporary = tempfile::tempdir().unwrap();
    let mut broker_config = BrokerConfig::for_state_root(temporary.path());
    broker_config.ack_timeout = Duration::from_millis(100);
    let host = BrokerHost::start(broker_config).unwrap();
    let producer = CooperativeProducer::for_state_root(temporary.path()).unwrap();
    wait_for_broker(&producer);

    let mut series = Vec::with_capacity(QUALIFYING_RUNS);
    for run in 1..=QUALIFYING_RUNS {
        let (timing, observation_gaps) = emit_cooperative_run(&producer, run);
        series.push(Series {
            kind: "cooperative",
            run,
            timing,
            observation_gaps,
        });
    }
    host.stop();

    let worst_p95_ms = worst(&series, "cooperative", |value| value.p95_ms);
    let worst_p99_ms = worst(&series, "cooperative", |value| value.p99_ms);
    let observation_gaps = series.iter().map(|value| value.observation_gaps).sum();
    let passed = worst_p95_ms <= COOPERATIVE_P95_LIMIT_MS
        && worst_p99_ms <= COOPERATIVE_P99_LIMIT_MS
        && observation_gaps == 0
        && series
            .iter()
            .all(|value| value.timing.samples == SAMPLES_PER_RUN);
    let receipt = CooperativeAcceptanceReceipt {
        schema_version: 1,
        run_kind: "g36_v031_cooperative_ipc_acceptance",
        build_profile: "release",
        source_git_head,
        source_tracked_worktree_clean,
        qualifying_runs: QUALIFYING_RUNS,
        samples_per_run: SAMPLES_PER_RUN,
        percentile_method: "nearest_rank",
        p95_limit_ms: COOPERATIVE_P95_LIMIT_MS,
        p99_limit_ms: COOPERATIVE_P99_LIMIT_MS,
        series,
        worst_p95_ms,
        worst_p99_ms,
        observation_gaps,
        outcome: if passed { "PASS" } else { "FAIL" },
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    write_json_once(&output, &receipt);
    assert!(passed, "cooperative IPC did not satisfy the frozen budget");
}

#[test]
#[ignore = "explicit release-artifact G36 performance qualification"]
fn release_artifact_meets_the_v031_recalibrated_g36_budget() {
    // `CARGO_BIN_EXE_hookstat-hook` inherits the test profile.  A debug test
    // would therefore quietly measure a non-release shipping binary while
    // the receipt claimed otherwise.  Keep the ignored test compilable for
    // ordinary coverage, but reject such an invocation before it can write a
    // qualifying receipt.
    require_release_profile();
    let (source_git_head, source_tracked_worktree_clean) = source_git_provenance();
    assert!(
        source_tracked_worktree_clean,
        "qualification requires a tracked-clean source head"
    );
    let qualification_output = qualification_output_path();
    let max_warm_window_attempts = configured_max_warm_window_attempts();
    let reject_retry_interval = configured_reject_retry_interval();
    let temporary = tempfile::tempdir().unwrap();
    let capsule_root = temporary.path().join("capsules");
    let state_root = temporary.path().join("state");
    let oracle_root = temporary.path().join("oracle");
    let healthy = capsule(
        ExecutionPlan::Direct {
            executable: "cmd.exe".into(),
            arguments: vec!["/D".into(), "/C".into(), "exit /b 0".into()],
        },
        Duration::from_secs(1),
    );
    let healthy_path = seal(&capsule_root, &healthy);
    let mut broker_config = BrokerConfig::for_state_root(&state_root);
    // The release qualification measures the producer's bounded reconnect
    // policy. The broker-side read allowance is deliberately wider than the
    // producer reuse window, so scheduler delay cannot be relabelled as
    // producer latency and a long-running Hook retains no idle slot forever.
    broker_config.ack_timeout = Duration::from_millis(100);
    let host = BrokerHost::start(broker_config).unwrap();
    let producer = CooperativeProducer::for_state_root(&state_root).unwrap();
    let shim = PathBuf::from(env!("CARGO_BIN_EXE_hookstat-hook"));
    let host_control_fixture = PathBuf::from(env!("CARGO_BIN_EXE_hookstat-hook-fixture"));
    assert!(
        host_control_fixture.is_file(),
        "exact G28 minimal-shim control fixture is not a file"
    );
    let shipping_shim = PathBuf::from(
        std::env::var_os(SHIPPING_SHIM_ENV)
            .expect("HOOKSTAT_G36_SHIPPING_SHIM is required for a qualifying run"),
    );
    assert!(
        shipping_shim.is_file(),
        "ordinary shipping shim is not a file"
    );
    let shipping_binary_sha256 = sha256_file(&shipping_shim);
    let instrumented_binary_sha256 = sha256_file(&shim);
    let artifacts = QualificationArtifacts {
        shipping_shim: &shipping_shim,
        instrumented_shim: &shim,
        shipping_binary_sha256: &shipping_binary_sha256,
        instrumented_binary_sha256: &instrumented_binary_sha256,
    };
    let oracle_endpoint = LocalEndpoint::from_state_root(&oracle_root).unwrap();
    let oracle_listener = oracle_endpoint.bind().unwrap();
    let oracle_context = OracleContext {
        shim: &shim,
        capsule: &healthy_path,
        capsule_root: &capsule_root,
        state_root: &state_root,
        oracle_root: &oracle_root,
        listener: &oracle_listener,
    };
    wait_for_broker(&producer);

    let mut series = Vec::with_capacity(QUALIFYING_RUNS * 3);
    for run in 1..=QUALIFYING_RUNS {
        let (timing, observation_gaps) = emit_cooperative_run(&producer, run);
        series.push(Series {
            kind: "cooperative",
            run,
            timing,
            observation_gaps,
        });
    }
    // The product metric uses a feature-gated timing side channel. Its
    // shipping/instrumented startup correction is itself admitted under the
    // exact G28 pre/post host control before it can affect a product window.
    // This prevents a build comparison observed in a rejected host regime
    // from being transferred into a later admitted product result.
    let mut startup_comparability_attempts = Vec::with_capacity(max_warm_window_attempts);
    let mut accepted_startup_comparability = None;
    for attempt in 1..=max_warm_window_attempts {
        let pre_control = host_control_run(&host_control_fixture);
        let startup_comparison_series = (1..=QUALIFYING_RUNS)
            .map(|run| startup_comparison_run(&shipping_shim, &shim, run))
            .collect::<Vec<_>>();
        let (
            shipping_startup_worst_p99_ms,
            instrumented_startup_worst_p99_ms,
            startup_tail_bias_correction_ms,
        ) = startup_tail_bias(&startup_comparison_series);
        let post_control = host_control_run(&host_control_fixture);
        let disposition = classify_startup_comparability(
            tail(&pre_control),
            startup_tail_bias_correction_ms,
            tail(&post_control),
        );
        let comparison = StartupComparabilityAttempt {
            attempt,
            pre_control,
            startup_comparison_series,
            shipping_startup_worst_p99_ms,
            instrumented_startup_worst_p99_ms,
            startup_tail_bias_correction_ms,
            post_control,
            disposition,
        };
        write_startup_comparability_receipt(
            &qualification_output,
            &source_git_head,
            &artifacts,
            &comparison,
        );
        startup_comparability_attempts.push(comparison.clone());
        match disposition {
            StartupComparabilityDisposition::Accepted => {
                accepted_startup_comparability = Some(comparison);
                break;
            }
            StartupComparabilityDisposition::InvalidatedBuildProfile => {
                panic!(
                    "instrumented startup was materially faster in an admitted build-comparability window"
                );
            }
            StartupComparabilityDisposition::RejectedHostSubstrate => {
                if attempt < max_warm_window_attempts {
                    std::thread::sleep(reject_retry_interval);
                }
            }
        }
    }
    let accepted_startup_comparability = accepted_startup_comparability
        .expect("no host-admitted startup build-comparability window was observed");
    let accepted_startup_comparability_attempt = accepted_startup_comparability.attempt;
    let startup_comparison_series = accepted_startup_comparability.startup_comparison_series;
    let shipping_startup_worst_p99_ms =
        accepted_startup_comparability.shipping_startup_worst_p99_ms;
    let instrumented_startup_worst_p99_ms =
        accepted_startup_comparability.instrumented_startup_worst_p99_ms;
    let startup_tail_bias_correction_ms =
        accepted_startup_comparability.startup_tail_bias_correction_ms;
    let startup_bias_material = false;

    let mut raw_oracle_runs = Vec::with_capacity(max_warm_window_attempts + QUALIFYING_RUNS);
    let mut warm_window_attempts = Vec::with_capacity(max_warm_window_attempts);
    let mut warm_admitted_runs = 0;
    let mut admitted_recalibrated_failure_occurred = false;
    let mut warm_method_invalidation_occurred = false;
    let mut admitted_warm_hookstat_induced_timeouts = 0;
    let mut admitted_warm_unexpected_terminal_results = 0;
    let mut admitted_warm_oracle_observation_gaps = 0;
    for attempt in 1..=max_warm_window_attempts {
        let pre_control = host_control_run(&host_control_fixture);
        let raw_run = emit_oracle_run("shim_warm", attempt, &oracle_context, true);
        let raw_candidate = timing(raw_run.overhead_ms.clone());
        let candidate = timing(
            raw_run
                .overhead_ms
                .iter()
                .map(|value| value + startup_tail_bias_correction_ms)
                .collect(),
        );
        let post_control = host_control_run(&host_control_fixture);
        let disposition = classify_warm_window_with_health_and_oracle(
            tail(&pre_control),
            tail(&candidate),
            tail(&post_control),
            raw_run.hookstat_induced_timeouts,
            raw_run.unexpected_terminal_results,
            raw_run.oracle_observation_gaps,
        );
        let window = WarmWindow {
            attempt,
            pre_control,
            raw_candidate,
            candidate,
            candidate_hookstat_induced_timeouts: raw_run.hookstat_induced_timeouts,
            candidate_unexpected_terminal_results: raw_run.unexpected_terminal_results,
            candidate_oracle_observation_gaps: raw_run.oracle_observation_gaps,
            post_control,
            disposition,
        };
        write_warm_window_receipt(
            &qualification_output,
            &source_git_head,
            &artifacts,
            accepted_startup_comparability_attempt,
            startup_tail_bias_correction_ms,
            &window,
        );
        raw_oracle_runs.push(raw_run);
        warm_window_attempts.push(window.clone());

        if matches!(
            disposition,
            WarmWindowDisposition::AdmittedPass | WarmWindowDisposition::FailRecalibratedBudget
        ) {
            admitted_warm_hookstat_induced_timeouts += window.candidate_hookstat_induced_timeouts;
            admitted_warm_unexpected_terminal_results +=
                window.candidate_unexpected_terminal_results;
        }

        match disposition {
            WarmWindowDisposition::AdmittedPass => {
                warm_admitted_runs += 1;
                series.push(Series {
                    kind: "shim_warm",
                    run: attempt,
                    timing: window.candidate,
                    observation_gaps: 0,
                });
                if warm_admitted_runs == QUALIFYING_RUNS {
                    break;
                }
            }
            WarmWindowDisposition::FailRecalibratedBudget => {
                admitted_recalibrated_failure_occurred = true;
                series.push(Series {
                    kind: "shim_warm",
                    run: attempt,
                    timing: window.candidate,
                    observation_gaps: 0,
                });
                break;
            }
            WarmWindowDisposition::RejectedHostSubstrate => {
                if attempt < max_warm_window_attempts {
                    std::thread::sleep(reject_retry_interval);
                }
            }
            WarmWindowDisposition::InvalidatedByMethod => {
                warm_method_invalidation_occurred = true;
                admitted_warm_oracle_observation_gaps += window.candidate_oracle_observation_gaps;
                series.push(Series {
                    kind: "shim_warm",
                    run: attempt,
                    timing: window.candidate,
                    observation_gaps: window.candidate_oracle_observation_gaps,
                });
                break;
            }
        }
    }
    let host_control_rejected_windows = warm_window_attempts
        .iter()
        .filter(|window| window.disposition == WarmWindowDisposition::RejectedHostSubstrate)
        .count();

    let mut cold_hookstat_induced_timeouts = 0;
    let mut cold_unexpected_terminal_results = 0;
    let mut cold_oracle_observation_gaps = 0;
    for run in 1..=QUALIFYING_RUNS {
        let raw_run = emit_oracle_run("shim_cold", run, &oracle_context, false);
        cold_hookstat_induced_timeouts += raw_run.hookstat_induced_timeouts;
        cold_unexpected_terminal_results += raw_run.unexpected_terminal_results;
        cold_oracle_observation_gaps += raw_run.oracle_observation_gaps;
        series.push(Series {
            kind: "shim_cold",
            run,
            timing: timing(
                raw_run
                    .overhead_ms
                    .iter()
                    .map(|value| value + startup_tail_bias_correction_ms)
                    .collect(),
            ),
            observation_gaps: raw_run.oracle_observation_gaps,
        });
        raw_oracle_runs.push(raw_run);
    }

    let near_timeout = capsule(
        ExecutionPlan::Direct {
            executable: "cmd.exe".into(),
            arguments: vec!["/D".into(), "/C".into(), "ping -n 2 127.0.0.1 >nul".into()],
        },
        Duration::from_millis(1_250),
    );
    let near_timeout_root = temporary.path().join("near-timeout-capsules");
    let near_timeout_path = seal(&near_timeout_root, &near_timeout);
    let healthy_near_timeout_runs = 5;
    let mut near_timeout_hookstat_induced_timeouts = 0;
    let mut near_timeout_unexpected_terminal_results = 0;
    for _ in 0..healthy_near_timeout_runs {
        let status = Command::new(&shim)
            .arg("--capsule")
            .arg(&near_timeout_path)
            .arg("--capsule-root")
            .arg(&near_timeout_root)
            .arg("--state-root")
            .arg(&state_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        if status.code() == Some(124) {
            near_timeout_hookstat_induced_timeouts += 1;
        } else if !status.success() {
            near_timeout_unexpected_terminal_results += 1;
        }
    }
    let hookstat_induced_timeouts_for_healthy_hook = admitted_warm_hookstat_induced_timeouts
        + cold_hookstat_induced_timeouts
        + near_timeout_hookstat_induced_timeouts;
    let unexpected_terminal_results_for_healthy_hook = admitted_warm_unexpected_terminal_results
        + cold_unexpected_terminal_results
        + near_timeout_unexpected_terminal_results;
    let oracle_observation_gaps =
        admitted_warm_oracle_observation_gaps + cold_oracle_observation_gaps;

    let raw_oracle_series = raw_oracle_runs
        .iter()
        .map(|run| Series {
            kind: match run.kind {
                "shim_warm" => "shim_warm_raw_oracle",
                "shim_cold" => "shim_cold_raw_oracle",
                _ => unreachable!("bounded raw oracle kind"),
            },
            run: run.run,
            timing: timing(run.overhead_ms.clone()),
            observation_gaps: run.oracle_observation_gaps,
        })
        .collect::<Vec<_>>();
    let oracle_primary_record_series = raw_oracle_runs
        .iter()
        .map(|run| Series {
            kind: "oracle_primary_record",
            run: run.run,
            timing: timing(run.oracle_primary_record_ms.clone()),
            observation_gaps: run.oracle_observation_gaps,
        })
        .collect::<Vec<_>>();
    let cooperative_worst_p95_ms = worst(&series, "cooperative", |value| value.p95_ms);
    let cooperative_worst_p99_ms = worst(&series, "cooperative", |value| value.p99_ms);
    let shim_warm_worst_p95_ms = worst_optional(&series, "shim_warm", |value| value.p95_ms);
    let shim_warm_worst_p99_ms = worst_optional(&series, "shim_warm", |value| value.p99_ms);
    let shim_cold_worst_p95_ms = worst(&series, "shim_cold", |value| value.p95_ms);
    let oracle_primary_record_worst_p95_ms = worst(
        &oracle_primary_record_series,
        "oracle_primary_record",
        |value| value.p95_ms,
    );
    let oracle_primary_record_worst_p99_ms = worst(
        &oracle_primary_record_series,
        "oracle_primary_record",
        |value| value.p99_ms,
    );
    let passed = cooperative_worst_p95_ms <= COOPERATIVE_P95_LIMIT_MS
        && cooperative_worst_p99_ms <= COOPERATIVE_P99_LIMIT_MS
        && warm_admitted_runs == QUALIFYING_RUNS
        && !admitted_recalibrated_failure_occurred
        && !warm_method_invalidation_occurred
        && shim_warm_worst_p95_ms.is_some_and(|value| value <= PRODUCT_WARM_P95_LIMIT_MS)
        && shim_warm_worst_p99_ms.is_some_and(|value| value <= PRODUCT_WARM_P99_LIMIT_MS)
        && shim_cold_worst_p95_ms <= SHIM_COLD_P95_LIMIT_MS
        && !startup_bias_material
        && hookstat_induced_timeouts_for_healthy_hook == 0
        && unexpected_terminal_results_for_healthy_hook == 0
        && oracle_observation_gaps == 0;
    let outcome = if passed {
        "PASS"
    } else if admitted_recalibrated_failure_occurred {
        "FAIL_RECALIBRATED_BUDGET"
    } else if warm_method_invalidation_occurred || oracle_observation_gaps > 0 {
        "INVALIDATED_BY_METHOD"
    } else if warm_admitted_runs < QUALIFYING_RUNS {
        "INSUFFICIENT_ADMITTED_WINDOWS"
    } else if startup_bias_material {
        "INVALIDATED_BUILD_COMPARABILITY"
    } else {
        "FAIL_RECALIBRATED_BUDGET"
    };
    let receipt = Receipt {
        schema_version: 2,
        run_kind: "g36_release_artifact_performance_qualification",
        release_artifacts: true,
        build_profile: "release",
        shim_measurement: "same_invocation_parent_lifetime_minus_child_spawn_wait_with_conservative_shipping_startup_tail_correction",
        paired_method_identifiable: false,
        same_invocation_oracle: true,
        oracle_transport: "feature_gated_local_fixed_32_byte_timing_side_channel",
        oracle_record_bytes: ORACLE_RECORD_BYTES,
        observed_overhead_includes_oracle_side_channel: true,
        host_control_methodology: HOST_CONTROL_METHODOLOGY,
        host_control_fixture: "hookstat-hook-fixture",
        host_control_samples_per_phase: SAMPLES_PER_RUN,
        host_control_warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        host_control_percentile_method: "nearest_rank",
        host_control_p95_limit_ms: HOST_CONTROL_P95_LIMIT_MS,
        host_control_p99_limit_ms: HOST_CONTROL_P99_LIMIT_MS,
        g28_reference_warm_p95_ms: G28_REFERENCE_WARM_P95_MS,
        g28_reference_warm_p99_ms: G28_REFERENCE_WARM_P99_MS,
        v031_release_warm_p95_ms: PRODUCT_WARM_P95_LIMIT_MS,
        v031_release_warm_p99_ms: PRODUCT_WARM_P99_LIMIT_MS,
        further_automatic_budget_relaxation: false,
        warm_admitted_runs_required: QUALIFYING_RUNS,
        max_warm_window_attempts,
        rejected_window_retry_interval_ms: u64::try_from(reject_retry_interval.as_millis())
            .unwrap_or(u64::MAX),
        warmup_definition: "25_unmeasured_fresh_actual_instrumented_hookstat_hook_help_launches_before_each_timed_invocation",
        warm_harness_self_load: false,
        qualifying_runs: QUALIFYING_RUNS,
        samples_per_run: SAMPLES_PER_RUN,
        warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        collector_model: "per_thread_local_samples",
        elapsed_capture: "immediately_after_operation",
        source_git_head,
        source_tracked_worktree_clean,
        shipping_binary_size_bytes: fs::metadata(&shipping_shim).unwrap().len(),
        instrumented_binary_size_bytes: fs::metadata(&shim).unwrap().len(),
        shipping_binary_sha256,
        instrumented_binary_sha256,
        startup_comparability_attempts,
        accepted_startup_comparability_attempt,
        startup_comparison_series,
        shipping_startup_worst_p99_ms,
        instrumented_startup_worst_p99_ms,
        startup_tail_bias_correction_ms,
        startup_bias_material,
        warm_window_attempts,
        host_control_rejected_windows,
        warm_admitted_runs,
        admitted_recalibrated_failure_occurred,
        warm_method_invalidation_occurred,
        admitted_warm_hookstat_induced_timeouts,
        admitted_warm_unexpected_terminal_results,
        admitted_warm_oracle_observation_gaps,
        cold_hookstat_induced_timeouts,
        cold_unexpected_terminal_results,
        cold_oracle_observation_gaps,
        raw_oracle_series,
        series,
        oracle_primary_record_worst_p95_ms,
        oracle_primary_record_worst_p99_ms,
        cooperative_worst_p95_ms,
        cooperative_worst_p99_ms,
        shim_warm_worst_p95_ms,
        shim_warm_worst_p99_ms,
        shim_cold_worst_p95_ms,
        healthy_near_timeout_runs,
        hookstat_induced_timeouts_for_healthy_hook,
        unexpected_terminal_results_for_healthy_hook,
        oracle_observation_gaps,
        outcome,
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    write_receipt(&qualification_output, &receipt);
    drop(host);
    assert!(passed, "G36 qualification outcome was {outcome}");
}

#[test]
fn startup_bias_uses_complete_build_envelopes_not_one_noisy_pair() {
    let timing_with_p99 = |p99_ms| Timing {
        samples: 100,
        p50_ms: 10.0,
        p95_ms: 12.0,
        p99_ms,
        max_ms: p99_ms,
    };
    let shipping_values = [13.4711, 15.4546, 12.4124, 16.1891, 13.7764];
    let instrumented_values = [13.4585, 15.3259, 14.5795, 13.9797, 13.2878];
    let series = shipping_values
        .into_iter()
        .zip(instrumented_values)
        .enumerate()
        .map(
            |(index, (shipping, instrumented))| StartupComparisonSeries {
                run: index + 1,
                shipping: timing_with_p99(shipping),
                instrumented: timing_with_p99(instrumented),
                shipping_minus_instrumented_p99_ms: (shipping - instrumented).max(0.0),
            },
        )
        .collect::<Vec<_>>();
    let (shipping, instrumented, bias) = startup_tail_bias(&series);
    assert_eq!(shipping, 16.1891);
    assert_eq!(instrumented, 15.3259);
    assert!((bias - 0.8632).abs() < 0.000_000_1);
}

#[test]
fn candidate_timeout_is_retained_instead_of_panicking_before_post_control() {
    let status = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 124"])
        .status()
        .expect("launch bounded exit fixture");
    assert_eq!(candidate_terminal_observation(&status), (1, 0));
}

#[test]
fn pre_oracle_child_exit_is_retained_instead_of_panicking() {
    let temporary = tempfile::tempdir().unwrap();
    let endpoint = LocalEndpoint::from_state_root(temporary.path().join("oracle")).unwrap();
    let listener = endpoint.bind().unwrap();
    let mut child = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 7"])
        .spawn()
        .expect("launch bounded pre-oracle exit fixture");
    match receive_oracle(&listener, &mut child) {
        OracleReceive::MissingRecord(status) => assert_eq!(status.code(), Some(7)),
        OracleReceive::Record { .. } => panic!("exit fixture unexpectedly emitted an oracle"),
    }
}
