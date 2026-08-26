#[cfg(windows)]
fn main() -> std::io::Result<()> {
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};
    use serde::Serialize;
    use std::fs;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const WARMUPS_PER_SAMPLE: usize = 25;
    const CONTROL_SAMPLES: usize = 1_000;

    #[derive(Serialize)]
    struct Timing {
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
        architecture: &'static str,
        helper_scope: &'static str,
        helper_idle_expiring: bool,
        evidence_transport_changed: bool,
        private_control_payload: &'static str,
        samples: usize,
        warmups_per_timed_sample: usize,
        frontend_process_plus_control_round_trip: Timing,
        persistent_client_control_round_trip: Timing,
        frontend_binary_size_bytes: u64,
        helper_binary_size_bytes: u64,
        owner_live_codex_config_mutated: bool,
        raw_private_content_captured: bool,
    }

    fn statistics(mut values: Vec<f64>) -> Timing {
        values.sort_by(f64::total_cmp);
        let nearest = |percent: usize| values[(values.len() * percent).div_ceil(100) - 1];
        Timing {
            samples: values.len(),
            p50_ms: nearest(50),
            p95_ms: nearest(95),
            p99_ms: nearest(99),
            max_ms: *values.last().unwrap(),
        }
    }

    fn exchange(endpoint: &str) -> std::io::Result<()> {
        let name = endpoint.to_ns_name::<GenericNamespaced>()?;
        let mut stream = Stream::connect(name)?;
        stream.write_all(b"HSGF\x01\0\0\0")?;
        let mut response = [0_u8; 8];
        stream.read_exact(&mut response)?;
        if response != *b"HSGA\x01\0\0\0" {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        Ok(())
    }

    fn sibling(directory: &Path, name: &str) -> PathBuf {
        directory.join(format!("{name}.exe"))
    }

    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let output = arguments
        .windows(2)
        .find(|values| values[0] == "--output")
        .map(|values| PathBuf::from(&values[1]))
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let samples = arguments
        .windows(2)
        .find(|values| values[0] == "--samples")
        .and_then(|values| values[1].parse::<usize>().ok())
        .filter(|value| (10..=1_000).contains(value))
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    if arguments.len() != 4 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }

    let directory = std::env::current_exe()?
        .parent()
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
        .to_path_buf();
    let frontend = sibling(&directory, "hookstat-g36-helper-floor-frontend");
    let server = sibling(&directory, "hookstat-g36-helper-floor-server");
    let endpoint = format!(
        "hookstat-g36-helper-floor-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let maximum_connections = CONTROL_SAMPLES + samples;
    let mut helper = Command::new(&server)
        .args([
            "--endpoint",
            &endpoint,
            "--max-connections",
            &maximum_connections.to_string(),
            "--idle-expiry-ms",
            "600000",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    let mut ready = String::new();
    BufReader::new(helper.stdout.take().unwrap()).read_line(&mut ready)?;
    if ready.trim() != "READY" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "helper readiness failed",
        ));
    }

    let mut control = Vec::with_capacity(CONTROL_SAMPLES);
    for _ in 0..CONTROL_SAMPLES {
        let started = Instant::now();
        exchange(&endpoint)?;
        control.push(started.elapsed().as_secs_f64() * 1_000.0);
    }

    let mut process = Vec::with_capacity(samples);
    for _ in 0..samples {
        for _ in 0..WARMUPS_PER_SAMPLE {
            let status = Command::new(&frontend)
                .arg("--help")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .status()?;
            if !status.success() {
                return Err(std::io::Error::other("frontend warmup failed"));
            }
        }
        let started = Instant::now();
        let status = Command::new(&frontend)
            .args(["--endpoint", &endpoint])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other("frontend exchange failed"));
        }
        process.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    if !helper.wait()?.success() {
        return Err(std::io::Error::other("owned helper failed"));
    }

    let receipt = Receipt {
        schema_version: 1,
        run_kind: "g36_idle_helper_frontend_floor",
        classification: "DIAGNOSTIC_ONLY",
        acceptance_evidence: false,
        architecture: "tiny_fresh_frontend_plus_persistent_local_helper",
        helper_scope: "per_user_local_prototype",
        helper_idle_expiring: true,
        evidence_transport_changed: false,
        private_control_payload: "fixed_8_byte_probe_only",
        samples,
        warmups_per_timed_sample: WARMUPS_PER_SAMPLE,
        frontend_process_plus_control_round_trip: statistics(process),
        persistent_client_control_round_trip: statistics(control),
        frontend_binary_size_bytes: fs::metadata(frontend)?.len(),
        helper_binary_size_bytes: fs::metadata(server)?.len(),
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };
    fs::write(output, serde_json::to_vec_pretty(&receipt)?)?;
    println!("G36_HELPER_FLOOR_RECEIPT_WRITTEN");
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    std::process::exit(2);
}
