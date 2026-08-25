//! Developer-only G36 same-invocation transparent-overhead oracle.
//!
//! The receipt contains only bounded stage durations, binary sizes, and
//! privacy booleans. It never serializes the disposable command, capsule,
//! endpoint, filesystem path, stdout/stderr, or host identity.

#[allow(dead_code)]
#[path = "../hook_shim.rs"]
mod hook_shim;
#[allow(dead_code)]
#[path = "../ipc_client.rs"]
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
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
const ORACLE_ROOT_ENV: &str = "HOOKSTAT_G36_ORACLE_ROOT";
const WARMUPS_PER_SAMPLE: usize = 25;
const ORACLE_RECORD_BYTES: usize = 32;

#[derive(Clone, Copy, Serialize)]
struct Statistics {
    samples: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    max_ms: f64,
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u8,
    run_kind: &'static str,
    classification: &'static str,
    acceptance_evidence: bool,
    samples: usize,
    warmups_per_timed_sample: usize,
    warmup_definition: &'static str,
    oracle_metric: &'static str,
    oracle_transport: &'static str,
    oracle_record_bytes: usize,
    observed_overhead_includes_oracle_side_channel: bool,
    shipping_binary_size_bytes: u64,
    instrumented_binary_size_bytes: u64,
    shipping_startup: Statistics,
    instrumented_startup: Statistics,
    instrumented_minus_shipping_startup_p95_ms: f64,
    instrumented_startup_not_faster_than_shipping: bool,
    full_transparent_invocation: Statistics,
    same_invocation_original_child_spawn_wait: Statistics,
    same_invocation_observed_overhead: Statistics,
    oracle_primary_record_observation: Statistics,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

#[derive(Clone, Copy)]
struct OracleSample {
    full_ns: u64,
    child_ns: u64,
    observed_overhead_ns: u64,
    oracle_primary_record_ns: u64,
}

fn statistics(mut values: Vec<u64>) -> Statistics {
    values.sort_unstable();
    let value =
        |percent: usize| values[(values.len() * percent).div_ceil(100) - 1] as f64 / 1_000_000.0;
    Statistics {
        samples: values.len(),
        p50_ms: value(50),
        p95_ms: value(95),
        p99_ms: value(99),
        max_ms: *values.last().expect("nonempty timing series") as f64 / 1_000_000.0,
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn capsule() -> HandlerCapsule {
    HandlerCapsule {
        handler_key: "g36_same_invocation_handler".into(),
        revision: "g36_same_invocation_revision".into(),
        definition_fingerprint: "sha256:g36_same_invocation".into(),
        runtime: "controlled_runtime".into(),
        runtime_instance: "controlled_instance".into(),
        event: "controlled_event".into(),
        source_scope: "controlled_scope".into(),
        original_budget: OriginalHandlerBudget(Duration::from_secs(1)),
        instrumentation_envelope: InstrumentationEnvelope(Duration::from_millis(50)),
        execution: ExecutionPlan::Direct {
            executable: "cmd.exe".into(),
            arguments: vec!["/D".into(), "/C".into(), "exit /b 0".into()],
        },
    }
}

fn seal(root: &Path, capsule: &HandlerCapsule) -> PathBuf {
    fs::create_dir(root).expect("create disposable capsule root");
    let store = CapsuleStore::open(root).expect("open disposable capsule root");
    let key = [0x36_u8; 32];
    write_key_for_test(root, &key).expect("write disposable capsule key");
    let name = capsule_file_name(capsule).expect("derive capsule name");
    store
        .write_for_test(Path::new(&name), capsule, &key)
        .expect("seal disposable capsule");
    root.join(name)
}

fn warm(binary: &Path) {
    for _ in 0..WARMUPS_PER_SAMPLE {
        let status = Command::new(binary)
            .arg("--help")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("launch cache-warm shim");
        assert!(status.success(), "cache-warm shim changed exit status");
    }
}

fn timed_help(binary: &Path) -> u64 {
    let started = Instant::now();
    let status = Command::new(binary)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("launch timed shim help");
    assert!(status.success(), "timed shim help changed exit status");
    elapsed_ns(started)
}

fn startup_pairs(shipping: &Path, instrumented: &Path, samples: usize) -> (Statistics, Statistics) {
    let mut shipping_samples = Vec::with_capacity(samples);
    let mut instrumented_samples = Vec::with_capacity(samples);
    for sample in 0..samples {
        warm(shipping);
        warm(instrumented);
        if sample % 2 == 0 {
            shipping_samples.push(timed_help(shipping));
            instrumented_samples.push(timed_help(instrumented));
        } else {
            instrumented_samples.push(timed_help(instrumented));
            shipping_samples.push(timed_help(shipping));
        }
    }
    (
        statistics(shipping_samples),
        statistics(instrumented_samples),
    )
}

fn wait_for_broker(state_root: &Path) {
    let producer = CooperativeProducer::for_state_root(state_root)
        .expect("construct disposable readiness producer");
    for attempt in 0..50 {
        let lifecycle = LifecycleFrame {
            runtime: "controlled_runtime".into(),
            runtime_instance: "controlled_instance".into(),
            invocation: format!("g36-oracle-readiness-{attempt}"),
            handler: "g36_same_invocation_handler".into(),
            event: "controlled_event".into(),
            source_scope: "controlled_scope".into(),
            revision: Some("g36_same_invocation_revision".into()),
            occurred_at_unix_ms: 1,
        };
        if producer.emit_start(lifecycle.clone()) == ObservationDisposition::Accepted {
            assert_eq!(
                producer.emit_complete(
                    lifecycle,
                    Completion {
                        terminal_status: TerminalOutcome::Completed,
                        exit_classification: ExitClassification::ExitCode,
                        exit_value: Some(0),
                        duration_ms: 0,
                    },
                ),
                ObservationDisposition::Accepted
            );
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("disposable broker did not become ready before the oracle");
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
                thread::sleep(Duration::from_micros(100));
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

fn oracle_sample(
    instrumented: &Path,
    capsule: &Path,
    capsule_root: &Path,
    state_root: &Path,
    oracle_root: &Path,
    listener: &Listener,
) -> OracleSample {
    let started = Instant::now();
    let mut child = Command::new(instrumented)
        .env(ORACLE_ROOT_ENV, oracle_root)
        .arg("--capsule")
        .arg(capsule)
        .arg("--capsule-root")
        .arg(capsule_root)
        .arg("--state-root")
        .arg(state_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("launch instrumented shim");
    let (child_ns, oracle_primary_record_ns) = receive_oracle(listener, &mut child);
    let status = child.wait().expect("wait for instrumented shim");
    let full_ns = elapsed_ns(started);
    assert!(
        status.success(),
        "instrumented healthy shim changed exit zero"
    );
    assert!(
        child_ns <= full_ns,
        "child interval exceeded parent lifetime"
    );
    OracleSample {
        full_ns,
        child_ns,
        observed_overhead_ns: full_ns - child_ns,
        oracle_primary_record_ns,
    }
}

fn parse_arguments() -> (PathBuf, PathBuf, PathBuf, usize, bool) {
    let mut output = None;
    let mut shipping = None;
    let mut instrumented = None;
    let mut samples = 100_usize;
    let mut breakaway_worker = false;
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--breakaway-worker" {
            breakaway_worker = true;
            index += 1;
            continue;
        }
        let value = arguments.get(index + 1).map(String::as_str);
        match (arguments[index].as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--shipping-shim", Some(value)) => shipping = Some(PathBuf::from(value)),
            ("--instrumented-shim", Some(value)) => instrumented = Some(PathBuf::from(value)),
            ("--samples", Some(value)) => samples = value.parse().unwrap_or(0),
            _ => {
                eprintln!(
                    "usage: hookstat-g36-same-invocation-oracle --output <sanitized-json> --shipping-shim <ordinary-hookstat-hook> --instrumented-shim <performance-hookstat-hook> [--samples <10..1000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let (Some(output), Some(shipping), Some(instrumented)) = (output, shipping, instrumented)
    else {
        eprintln!("hookstat-g36-same-invocation-oracle requires all paths");
        std::process::exit(2);
    };
    if !(10..=1_000).contains(&samples) || !shipping.is_file() || !instrumented.is_file() {
        eprintln!("hookstat-g36-same-invocation-oracle received invalid bounded input");
        std::process::exit(2);
    }
    (output, shipping, instrumented, samples, breakaway_worker)
}

#[cfg(windows)]
fn relaunch_breakaway(output: &Path, shipping: &Path, instrumented: &Path, samples: usize) -> bool {
    let current = std::env::current_exe().expect("oracle executable path");
    let status = Command::new(current)
        .arg("--breakaway-worker")
        .arg("--output")
        .arg(output)
        .arg("--shipping-shim")
        .arg(shipping)
        .arg("--instrumented-shim")
        .arg(instrumented)
        .arg("--samples")
        .arg(samples.to_string())
        .creation_flags(CREATE_BREAKAWAY_FROM_JOB)
        .status();
    matches!(status, Ok(status) if status.success())
}

fn run(output: PathBuf, shipping: PathBuf, instrumented: PathBuf, samples: usize) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "hookstat-g36-same-invocation-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("create disposable oracle root");
    let capsule_root = root.join("capsules");
    let state_root = root.join("state");
    let oracle_root = root.join("oracle");
    let sealed = seal(&capsule_root, &capsule());
    let host = BrokerHost::start(BrokerConfig::for_state_root(&state_root))
        .expect("start disposable broker");
    let endpoint = LocalEndpoint::from_state_root(&oracle_root).expect("create oracle endpoint");
    let listener = endpoint.bind().expect("bind oracle endpoint");

    wait_for_broker(&state_root);
    let mut oracle_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        warm(&instrumented);
        oracle_samples.push(oracle_sample(
            &instrumented,
            &sealed,
            &capsule_root,
            &state_root,
            &oracle_root,
            &listener,
        ));
    }
    // Startup comparison runs after the oracle so the disposable broker's
    // production idle expiry cannot change a same-invocation sample.
    let (shipping_startup, instrumented_startup) = startup_pairs(&shipping, &instrumented, samples);

    host.stop();
    let full_transparent_invocation =
        statistics(oracle_samples.iter().map(|sample| sample.full_ns).collect());
    let same_invocation_original_child_spawn_wait = statistics(
        oracle_samples
            .iter()
            .map(|sample| sample.child_ns)
            .collect(),
    );
    let same_invocation_observed_overhead = statistics(
        oracle_samples
            .iter()
            .map(|sample| sample.observed_overhead_ns)
            .collect(),
    );
    let oracle_primary_record_observation = statistics(
        oracle_samples
            .iter()
            .map(|sample| sample.oracle_primary_record_ns)
            .collect(),
    );
    let startup_delta = instrumented_startup.p95_ms - shipping_startup.p95_ms;
    let receipt = Receipt {
        schema_version: 1,
        run_kind: "hs_g36_same_invocation_transparent_overhead_oracle",
        classification: "DIAGNOSTIC_ONLY",
        acceptance_evidence: false,
        samples,
        warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        warmup_definition: "25_unmeasured_fresh_help_launches_before_each_timed_sample",
        oracle_metric: "parent_observed_full_shim_lifetime_minus_same_invocation_child_spawn_wait",
        oracle_transport: "feature_gated_local_fixed_32_byte_timing_side_channel",
        oracle_record_bytes: ORACLE_RECORD_BYTES,
        observed_overhead_includes_oracle_side_channel: true,
        shipping_binary_size_bytes: fs::metadata(&shipping).expect("shipping metadata").len(),
        instrumented_binary_size_bytes: fs::metadata(&instrumented)
            .expect("instrumented metadata")
            .len(),
        shipping_startup,
        instrumented_startup,
        instrumented_minus_shipping_startup_p95_ms: startup_delta,
        instrumented_startup_not_faster_than_shipping: startup_delta >= 0.0,
        full_transparent_invocation,
        same_invocation_original_child_spawn_wait,
        same_invocation_observed_overhead,
        oracle_primary_record_observation,
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    let written = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| std::io::Error::other("G36 oracle receipt serialization"))
        .and_then(|bytes| fs::write(&output, bytes));
    let _ = fs::remove_dir_all(&root);
    written.expect("write sanitized G36 oracle receipt");
    println!("G36_SAME_INVOCATION_ORACLE_RECEIPT_WRITTEN");
}

fn main() {
    #[cfg(not(windows))]
    {
        eprintln!("hookstat-g36-same-invocation-oracle is Windows-only");
        std::process::exit(2);
    }
    #[cfg(windows)]
    {
        let (output, shipping, instrumented, samples, breakaway_worker) = parse_arguments();
        if !breakaway_worker {
            if relaunch_breakaway(&output, &shipping, &instrumented, samples) {
                return;
            }
            eprintln!(
                "hookstat-g36-same-invocation-oracle could not establish breakaway admission"
            );
            std::process::exit(1);
        }
        run(output, shipping, instrumented, samples);
    }
}
