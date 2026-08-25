//! Explicit Windows-only G36 release-artifact qualification.
//!
//! This ignored test is deliberately invoked only by the release train. It
//! uses a disposable capsule root and state root, never discovers or changes
//! Codex configuration, and serializes no command, path, payload, or host
//! identity into its receipt.

#![cfg(windows)]

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
use hookstat::ipc::{BrokerConfig, BrokerHost};
use interprocess::local_socket::traits::Listener as _;
use ipc_client::{
    Completion, CooperativeProducer, ExitClassification, LifecycleFrame, Listener, LocalEndpoint,
    ObservationDisposition, TerminalOutcome,
};
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const QUALIFYING_RUNS: usize = 5;
const SAMPLES_PER_RUN: usize = 100;
const WARMUPS_PER_SAMPLE: usize = 25;
const COOPERATIVE_P95_LIMIT_MS: f64 = 1.0;
const COOPERATIVE_P99_LIMIT_MS: f64 = 2.0;
const SHIM_WARM_P95_LIMIT_MS: f64 = 20.0;
const SHIM_WARM_P99_LIMIT_MS: f64 = 25.0;
const SHIM_COLD_P95_LIMIT_MS: f64 = 50.0;
const MAX_COMPARABLE_STARTUP_BIAS_MS: f64 = 2.0;
const ORACLE_ROOT_ENV: &str = "HOOKSTAT_G36_ORACLE_ROOT";
const SHIPPING_SHIM_ENV: &str = "HOOKSTAT_G36_SHIPPING_SHIM";
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
struct StartupComparisonSeries {
    run: usize,
    shipping: Timing,
    instrumented: Timing,
    shipping_minus_instrumented_p99_ms: f64,
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
    warmup_definition: &'static str,
    warm_harness_self_load: bool,
    qualifying_runs: usize,
    samples_per_run: usize,
    warmups_per_timed_sample: usize,
    collector_model: &'static str,
    elapsed_capture: &'static str,
    shipping_binary_size_bytes: u64,
    instrumented_binary_size_bytes: u64,
    startup_comparison_series: Vec<StartupComparisonSeries>,
    shipping_startup_worst_p99_ms: f64,
    instrumented_startup_worst_p99_ms: f64,
    startup_tail_bias_correction_ms: f64,
    startup_bias_material: bool,
    raw_oracle_series: Vec<Series>,
    series: Vec<Series>,
    oracle_primary_record_worst_p95_ms: f64,
    oracle_primary_record_worst_p99_ms: f64,
    cooperative_worst_p95_ms: f64,
    cooperative_worst_p99_ms: f64,
    shim_warm_worst_p95_ms: f64,
    shim_warm_worst_p99_ms: f64,
    shim_cold_worst_p95_ms: f64,
    healthy_near_timeout_runs: usize,
    hookstat_induced_timeouts_for_healthy_hook: usize,
    outcome: &'static str,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

struct RawOracleRun {
    kind: &'static str,
    run: usize,
    overhead_ms: Vec<f64>,
    oracle_primary_record_ms: Vec<f64>,
}

struct OracleContext<'a> {
    shim: &'a Path,
    capsule: &'a Path,
    capsule_root: &'a Path,
    state_root: &'a Path,
    oracle_root: &'a Path,
    listener: &'a Listener,
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

fn receive_oracle(listener: &Listener, child: &mut Child) -> (u64, u64) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut stream = loop {
        match listener.accept() {
            Ok(stream) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(
                    child.try_wait().expect("query instrumented shim").is_none(),
                    "instrumented shim exited before its oracle record"
                );
                assert!(
                    Instant::now() < deadline,
                    "instrumented shim did not connect to the bounded oracle"
                );
                std::thread::sleep(Duration::from_micros(100));
            }
            Err(error) => panic!("oracle listener failed: {error}"),
        }
    };
    let mut record = [0_u8; ORACLE_RECORD_BYTES];
    stream
        .read_exact(&mut record)
        .expect("read fixed-size oracle record");
    assert_eq!(&record[..4], b"HSO1");
    assert_eq!(&record[16..20], b"HSO2");
    assert_eq!(record[4], 1);
    assert_eq!(record[20], 1);
    assert!(record[5..8].iter().all(|byte| *byte == 0));
    assert!(record[21..24].iter().all(|byte| *byte == 0));
    (decode_u64(&record[8..16]), decode_u64(&record[24..32]))
}

fn launch_with_oracle(context: &OracleContext<'_>) -> (f64, f64) {
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
    let (child_ns, oracle_primary_record_ns) = receive_oracle(context.listener, &mut child);
    let status = child.wait().expect("wait for instrumented shim");
    let full_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    assert!(
        status.success(),
        "instrumented healthy shim changed exit zero"
    );
    assert!(
        child_ns <= full_ns,
        "child interval exceeded parent lifetime"
    );
    (
        (full_ns - child_ns) as f64 / 1_000_000.0,
        oracle_primary_record_ns as f64 / 1_000_000.0,
    )
}

fn emit_oracle_run(
    kind: &'static str,
    run: usize,
    context: &OracleContext<'_>,
    warmed: bool,
) -> RawOracleRun {
    let mut overhead_ms = Vec::with_capacity(SAMPLES_PER_RUN);
    let mut oracle_primary_record_ms = Vec::with_capacity(SAMPLES_PER_RUN);
    for _ in 0..SAMPLES_PER_RUN {
        if warmed {
            warm_actual_shipping_shim(context.shim);
        }
        let (overhead, oracle_primary) = launch_with_oracle(context);
        overhead_ms.push(overhead);
        oracle_primary_record_ms.push(oracle_primary);
    }
    RawOracleRun {
        kind,
        run,
        overhead_ms,
        oracle_primary_record_ms,
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

fn write_receipt(receipt: &Receipt) {
    let output = std::env::var_os("HOOKSTAT_G36_PERFORMANCE_OUTPUT")
        .expect("HOOKSTAT_G36_PERFORMANCE_OUTPUT is required for a qualifying run");
    let output = PathBuf::from(output);
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(output, serde_json::to_vec_pretty(receipt).unwrap()).unwrap();
}

#[cfg(debug_assertions)]
fn require_release_profile() {
    panic!("G36 qualification requires cargo test --release; debug artifacts are diagnostic only");
}

#[cfg(not(debug_assertions))]
fn require_release_profile() {}

#[test]
#[ignore = "explicit release-artifact G36 performance qualification"]
fn release_artifact_meets_the_frozen_g36_budget() {
    // `CARGO_BIN_EXE_hookstat-hook` inherits the test profile.  A debug test
    // would therefore quietly measure a non-release shipping binary while
    // the receipt claimed otherwise.  Keep the ignored test compilable for
    // ordinary coverage, but reject such an invocation before it can write a
    // qualifying receipt.
    require_release_profile();
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
    let shipping_shim = PathBuf::from(
        std::env::var_os(SHIPPING_SHIM_ENV)
            .expect("HOOKSTAT_G36_SHIPPING_SHIM is required for a qualifying run"),
    );
    assert!(
        shipping_shim.is_file(),
        "ordinary shipping shim is not a file"
    );
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
    let mut raw_oracle_runs = Vec::with_capacity(QUALIFYING_RUNS * 2);
    for run in 1..=QUALIFYING_RUNS {
        raw_oracle_runs.push(emit_oracle_run("shim_warm", run, &oracle_context, true));
    }
    for run in 1..=QUALIFYING_RUNS {
        raw_oracle_runs.push(emit_oracle_run("shim_cold", run, &oracle_context, false));
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
    let mut hookstat_induced_timeouts_for_healthy_hook = 0;
    for _ in 0..healthy_near_timeout_runs {
        let started = Instant::now();
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
        let _elapsed = started.elapsed();
        if status.code() == Some(124) {
            hookstat_induced_timeouts_for_healthy_hook += 1;
        }
        assert!(
            status.success(),
            "near-timeout healthy fixture did not exit zero"
        );
    }

    // Compare the feature-gated oracle binary with the ordinary shipping
    // release artifact only after the broker-dependent samples. The broker's
    // production idle expiry therefore cannot change a warm/cold result.
    let startup_comparison_series = (1..=QUALIFYING_RUNS)
        .map(|run| startup_comparison_run(&shipping_shim, &shim, run))
        .collect::<Vec<_>>();
    let (
        shipping_startup_worst_p99_ms,
        instrumented_startup_worst_p99_ms,
        startup_tail_bias_correction_ms,
    ) = startup_tail_bias(&startup_comparison_series);
    let startup_bias_material = startup_tail_bias_correction_ms >= MAX_COMPARABLE_STARTUP_BIAS_MS;

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
            observation_gaps: 0,
        })
        .collect::<Vec<_>>();
    let oracle_primary_record_series = raw_oracle_runs
        .iter()
        .map(|run| Series {
            kind: "oracle_primary_record",
            run: run.run,
            timing: timing(run.oracle_primary_record_ms.clone()),
            observation_gaps: 0,
        })
        .collect::<Vec<_>>();
    for run in &raw_oracle_runs {
        series.push(Series {
            kind: run.kind,
            run: run.run,
            timing: timing(
                run.overhead_ms
                    .iter()
                    .map(|value| value + startup_tail_bias_correction_ms)
                    .collect(),
            ),
            observation_gaps: 0,
        });
    }

    let cooperative_worst_p95_ms = worst(&series, "cooperative", |value| value.p95_ms);
    let cooperative_worst_p99_ms = worst(&series, "cooperative", |value| value.p99_ms);
    let shim_warm_worst_p95_ms = worst(&series, "shim_warm", |value| value.p95_ms);
    let shim_warm_worst_p99_ms = worst(&series, "shim_warm", |value| value.p99_ms);
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
        && shim_warm_worst_p95_ms <= SHIM_WARM_P95_LIMIT_MS
        && shim_warm_worst_p99_ms <= SHIM_WARM_P99_LIMIT_MS
        && shim_cold_worst_p95_ms <= SHIM_COLD_P95_LIMIT_MS
        && !startup_bias_material
        && hookstat_induced_timeouts_for_healthy_hook == 0;
    let receipt = Receipt {
        schema_version: 1,
        run_kind: "g36_release_artifact_performance_qualification",
        release_artifacts: true,
        build_profile: "release",
        shim_measurement: "same_invocation_parent_lifetime_minus_child_spawn_wait_with_conservative_shipping_startup_tail_correction",
        paired_method_identifiable: false,
        same_invocation_oracle: true,
        oracle_transport: "feature_gated_local_fixed_32_byte_timing_side_channel",
        oracle_record_bytes: ORACLE_RECORD_BYTES,
        observed_overhead_includes_oracle_side_channel: true,
        warmup_definition: "25_unmeasured_fresh_actual_instrumented_hookstat_hook_help_launches_before_each_timed_invocation",
        warm_harness_self_load: false,
        qualifying_runs: QUALIFYING_RUNS,
        samples_per_run: SAMPLES_PER_RUN,
        warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        collector_model: "per_thread_local_samples",
        elapsed_capture: "immediately_after_operation",
        shipping_binary_size_bytes: fs::metadata(&shipping_shim).unwrap().len(),
        instrumented_binary_size_bytes: fs::metadata(&shim).unwrap().len(),
        startup_comparison_series,
        shipping_startup_worst_p99_ms,
        instrumented_startup_worst_p99_ms,
        startup_tail_bias_correction_ms,
        startup_bias_material,
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
        outcome: if passed { "PASS" } else { "FAIL" },
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    write_receipt(&receipt);
    drop(host);
    assert!(passed, "G36 frozen performance budget was exceeded");
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
