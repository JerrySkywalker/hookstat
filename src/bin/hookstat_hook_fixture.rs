//! Dedicated G28 minimal-shim startup and Job Object fixture. It intentionally
//! has no HookStat product dependencies and accepts no payload or command data.

#[cfg(windows)]
use std::time::Instant;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => {}
        [flag, iterations] if flag == "--job-probe" => match iterations.parse::<usize>() {
            Ok(iterations) if (1..=100_000).contains(&iterations) => run_job_probe(iterations),
            _ => std::process::exit(2),
        },
        _ => std::process::exit(2),
    }
}

#[cfg(windows)]
fn run_job_probe(iterations: usize) {
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    let current = match std::env::current_exe() {
        Ok(value) => value,
        Err(_) => std::process::exit(10),
    };
    let handler = match current.parent() {
        Some(parent) => {
            let mut value = PathBuf::from(parent).join("hookstat-g28-handler-fixture");
            value.set_extension(current.extension().unwrap_or_default());
            value
        }
        None => std::process::exit(10),
    };
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        // Child creation and the later wait are deliberately outside the
        // timed region. The child only supplies a valid assign target.
        let mut child = match Command::new(&handler)
            .args(["--sleep-ms", "100"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(value) => value,
            Err(_) => std::process::exit(10),
        };
        let started = Instant::now();
        let mut limits = win32job::ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job = match win32job::Job::create_with_limit_info(&limits) {
            Ok(job) => job,
            Err(_) => std::process::exit(11),
        };
        if job.assign_process(child.as_raw_handle() as isize).is_err() {
            std::process::exit(12);
        }
        let mut released = match job.query_extended_limit_info() {
            Ok(value) => value,
            Err(_) => std::process::exit(13),
        };
        released.clear_limits();
        if job.set_extended_limit_info(&released).is_err() {
            std::process::exit(14);
        }
        drop(job);
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        if child.wait().is_err() {
            std::process::exit(15);
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "schema_version": 1,
            "job_object_cycle_ms": samples,
        })
    );
}

#[cfg(not(windows))]
fn run_job_probe(_iterations: usize) {
    std::process::exit(1);
}
