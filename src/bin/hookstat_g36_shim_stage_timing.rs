//! Feature-gated, sanitized G36 transparent-shim attribution probe.

#[allow(dead_code)]
#[path = "../hook_shim.rs"]
mod hook_shim;
#[allow(dead_code)]
#[path = "../ipc_client.rs"]
mod ipc_client;

use hook_shim::{
    CapsuleStore, ExecutionPlan, HandlerCapsule, InstrumentationEnvelope, OriginalHandlerBudget,
    capsule_file_name, run_capsule_for_qualification_timed, write_key_for_test,
};
use hookstat::ipc::{BrokerConfig, BrokerHost};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

#[derive(Serialize)]
struct Statistics {
    samples: usize,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
}

#[derive(Serialize)]
struct Receipt {
    schema_version: u8,
    run_kind: &'static str,
    acceptance_evidence: bool,
    samples: usize,
    process_startup_baseline: Statistics,
    argument_parsing: Statistics,
    capsule_directory_file_validation: Statistics,
    capsule_read: Statistics,
    key_read: Statistics,
    hmac_verification_and_capsule_validation: Statistics,
    local_endpoint_and_producer_construction: Statistics,
    tokio_runtime_construction_where_applicable: &'static str,
    start_ipc: Statistics,
    job_object_establish: Statistics,
    original_child_spawn: Statistics,
    post_child_wait_poll: Statistics,
    job_object_release: Statistics,
    complete_ipc: Statistics,
    execution_total: Statistics,
    final_process_exit_remainder: &'static str,
    owner_live_codex_config_mutated: bool,
    raw_private_content_captured: bool,
}

fn statistics(mut samples: Vec<u64>) -> Statistics {
    samples.sort_unstable();
    let value =
        |percent: usize| samples[(samples.len() * percent).div_ceil(100) - 1] as f64 / 1_000_000.0;
    Statistics {
        samples: samples.len(),
        p50_ms: value(50),
        p95_ms: value(95),
        p99_ms: value(99),
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn capsule() -> HandlerCapsule {
    HandlerCapsule {
        handler_key: "g36_stage_handler".into(),
        revision: "g36_stage_revision".into(),
        definition_fingerprint: "sha256:g36_stage".into(),
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
    fs::create_dir_all(root).unwrap();
    let store = CapsuleStore::open(root).unwrap();
    let key = [0x36_u8; 32];
    write_key_for_test(root, &key).unwrap();
    let name = capsule_file_name(capsule).unwrap();
    store
        .write_for_test(Path::new(&name), capsule, &key)
        .unwrap();
    root.join(name)
}

fn help_launch(shim: &Path) -> u64 {
    let started = Instant::now();
    let status = Command::new(shim)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "shipping shim help path failed");
    elapsed_ns(started)
}

fn parse_shipping_argument_shape() {
    // Mirrors the shipping shim's fixed three-option parser without retaining
    // a real capsule path or control-plane value in the diagnostic process.
    let arguments = [
        "--capsule",
        "opaque-capsule",
        "--capsule-root",
        "opaque-root",
        "--state-root",
        "opaque-state",
    ];
    let mut values = arguments.into_iter();
    let mut capsule = false;
    let mut capsule_root = false;
    let mut state_root = false;
    while let Some(flag) = values.next() {
        let target = match flag {
            "--capsule" => &mut capsule,
            "--capsule-root" => &mut capsule_root,
            "--state-root" => &mut state_root,
            _ => unreachable!("fixed parser input"),
        };
        *target = values.next().is_some();
    }
    std::hint::black_box((capsule, capsule_root, state_root));
}

fn main() {
    let mut output = None;
    let mut shim = None;
    let mut samples = 100_usize;
    let mut breakaway_worker = false;
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        if flag == "--breakaway-worker" {
            breakaway_worker = true;
            index += 1;
            continue;
        }
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--output", Some(value)) => output = Some(PathBuf::from(value)),
            ("--shim", Some(value)) => shim = Some(PathBuf::from(value)),
            ("--samples", Some(value)) => samples = value.parse().unwrap_or(0),
            _ => {
                eprintln!(
                    "usage: hookstat-g36-shim-stage-timing --output <sanitized-json> --shim <shipping-hookstat-hook> [--samples <10..1000>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let (Some(output), Some(shim)) = (output, shim) else {
        eprintln!("hookstat-g36-shim-stage-timing requires --output and --shim");
        std::process::exit(2);
    };
    if !(10..=1_000).contains(&samples) || !shim.is_file() {
        eprintln!("hookstat-g36-shim-stage-timing received invalid bounded input");
        std::process::exit(2);
    }
    #[cfg(windows)]
    if !breakaway_worker {
        // The Codex parent can already be inside a non-nestable Job. The
        // shipping shim assigns *itself* to its containment Job, so a timing
        // worker needs the same breakaway admission as a standalone Hook.
        // This is a disposable local diagnostic process, not a user process.
        let current = std::env::current_exe().expect("stage binary path");
        let status = Command::new(current)
            .arg("--breakaway-worker")
            .arg("--output")
            .arg(&output)
            .arg("--shim")
            .arg(&shim)
            .arg("--samples")
            .arg(samples.to_string())
            .creation_flags(CREATE_BREAKAWAY_FROM_JOB)
            .status();
        match status {
            Ok(status) if status.success() => return,
            Ok(_) | Err(_) => {
                eprintln!("hookstat-g36-shim-stage-timing could not establish breakaway admission");
                std::process::exit(1);
            }
        }
    }
    let root = std::env::temp_dir().join(format!(
        "hookstat-g36-shim-stage-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |value| value.as_nanos())
    ));
    let capsule_root = root.join("capsules");
    let state_root = root.join("state");
    let sealed = seal(&capsule_root, &capsule());
    let host = BrokerHost::start(BrokerConfig::for_state_root(&state_root)).unwrap();
    let store = CapsuleStore::open(&capsule_root).unwrap();
    let mut startup = Vec::with_capacity(samples);
    let mut arguments_parse = Vec::with_capacity(samples);
    let mut directory_validation = Vec::with_capacity(samples);
    let mut capsule_read = Vec::with_capacity(samples);
    let mut key_read = Vec::with_capacity(samples);
    let mut hmac = Vec::with_capacity(samples);
    let mut producer = Vec::with_capacity(samples);
    let mut start_ipc = Vec::with_capacity(samples);
    let mut job_establish = Vec::with_capacity(samples);
    let mut spawn = Vec::with_capacity(samples);
    let mut wait = Vec::with_capacity(samples);
    let mut job_release = Vec::with_capacity(samples);
    let mut complete_ipc = Vec::with_capacity(samples);
    let mut execution_total = Vec::with_capacity(samples);
    for _ in 0..samples {
        startup.push(help_launch(&shim));
        let argument_started = Instant::now();
        parse_shipping_argument_shape();
        arguments_parse.push(elapsed_ns(argument_started));
        let (loaded, load) = store.load_for_qualification_timed(&sealed).unwrap();
        directory_validation.push(load.capsule_directory_file_validation_ns);
        capsule_read.push(load.capsule_read_ns);
        key_read.push(load.key_read_ns);
        hmac.push(load.hmac_and_capsule_validation_ns);
        let (outcome, stages) = run_capsule_for_qualification_timed(&loaded, &state_root).unwrap();
        assert_eq!(
            outcome.exit_code, 0,
            "controlled direct handler changed exit"
        );
        producer.push(stages.producer_construction_ns);
        start_ipc.push(stages.start_ipc_ns);
        job_establish.push(stages.job_object_establish_ns);
        spawn.push(stages.original_child_spawn_ns);
        wait.push(stages.child_wait_poll_ns);
        job_release.push(stages.job_object_release_ns);
        complete_ipc.push(stages.complete_ipc_ns);
        execution_total.push(stages.total_execution_ns);
    }
    host.stop();
    let receipt = Receipt {
        schema_version: 1,
        run_kind: "hs_g36_transparent_shim_stage_diagnostic",
        acceptance_evidence: false,
        samples,
        process_startup_baseline: statistics(startup),
        argument_parsing: statistics(arguments_parse),
        capsule_directory_file_validation: statistics(directory_validation),
        capsule_read: statistics(capsule_read),
        key_read: statistics(key_read),
        hmac_verification_and_capsule_validation: statistics(hmac),
        local_endpoint_and_producer_construction: statistics(producer),
        tokio_runtime_construction_where_applicable: "included_in_local_endpoint_and_producer_construction",
        start_ipc: statistics(start_ipc),
        job_object_establish: statistics(job_establish),
        original_child_spawn: statistics(spawn),
        post_child_wait_poll: statistics(wait),
        job_object_release: statistics(job_release),
        complete_ipc: statistics(complete_ipc),
        execution_total: statistics(execution_total),
        final_process_exit_remainder: "not separately measurable inside a live diagnostic process; shipping process startup is reported independently",
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    let written = serde_json::to_vec_pretty(&receipt)
        .map_err(|_| std::io::Error::other("G36 shim timing serialization"))
        .and_then(|bytes| std::fs::write(output, bytes));
    let _ = fs::remove_dir_all(&root);
    if written.is_err() {
        eprintln!("hookstat-g36-shim-stage-timing could not write the receipt");
        std::process::exit(1);
    }
    println!("G36_SHIM_STAGE_TIMING_RECEIPT_WRITTEN");
}
