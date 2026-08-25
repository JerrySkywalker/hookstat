#[allow(dead_code)]
#[path = "../hook_shim.rs"]
mod hook_shim;
#[allow(dead_code)]
#[path = "../ipc_client.rs"]
mod ipc_client;

use hook_shim::{CapsuleStore, run_capsule};
use std::path::PathBuf;
use std::process::ExitCode;

#[cfg(all(feature = "performance-harness", unix))]
use interprocess::local_socket::{GenericFilePath, ToFsName};
#[cfg(all(feature = "performance-harness", windows))]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
#[cfg(feature = "performance-harness")]
use interprocess::{ConnectWaitMode, local_socket::ConnectOptions};
#[cfg(feature = "performance-harness")]
use std::{io::Write, path::Path, time::Duration};

#[cfg(feature = "performance-harness")]
const G36_ORACLE_ROOT_ENV: &str = "HOOKSTAT_G36_ORACLE_ROOT";

/// Writes two fixed-size developer-only records to a local timing side
/// channel. The first contains only the same-invocation child interval. The
/// second contains the connect plus first-record write duration, which lets
/// the harness bound its primary observation cost. No command, capsule, path,
/// stream, prompt, tool data, or credential is serialized.
#[cfg(feature = "performance-harness")]
fn emit_g36_same_invocation_oracle(root: &Path, child_interval_ns: u64) -> Option<()> {
    let endpoint = ipc_client::LocalEndpoint::from_state_root(root).ok()?;
    #[cfg(windows)]
    let name = endpoint
        .named_pipe_name()
        .to_ns_name::<GenericNamespaced>()
        .ok()?;
    #[cfg(unix)]
    let name = endpoint
        .unix_socket_path()
        .ok()?
        .to_fs_name::<GenericFilePath>()
        .ok()?;
    let started = std::time::Instant::now();
    let mut stream = ConnectOptions::new()
        .name(name)
        .wait_mode(ConnectWaitMode::Timeout(Duration::from_millis(10)))
        .connect_sync()
        .ok()?;
    let mut child_record = [0_u8; 16];
    child_record[..4].copy_from_slice(b"HSO1");
    child_record[4] = 1;
    child_record[8..].copy_from_slice(&child_interval_ns.to_le_bytes());
    stream.write_all(&child_record).ok()?;
    let primary_record_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let mut overhead_record = [0_u8; 16];
    overhead_record[..4].copy_from_slice(b"HSO2");
    overhead_record[4] = 1;
    overhead_record[8..].copy_from_slice(&primary_record_ns.to_le_bytes());
    stream.write_all(&overhead_record).ok()?;
    stream.flush().ok()
}

fn main() -> ExitCode {
    let mut capsule = None;
    let mut capsule_root = None;
    let mut state_root = None;
    let mut values = std::env::args().skip(1);
    while let Some(value) = values.next() {
        let target = match value.as_str() {
            "--capsule" => &mut capsule,
            "--capsule-root" => &mut capsule_root,
            "--state-root" => &mut state_root,
            "-h" | "--help" => {
                println!(
                    "usage: hookstat-hook --capsule <private-file> --capsule-root <private-dir> --state-root <hookstat-state>"
                );
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("hookstat-hook: invalid arguments");
                return ExitCode::from(2);
            }
        };
        let Some(value) = values.next() else {
            eprintln!("hookstat-hook: invalid arguments");
            return ExitCode::from(2);
        };
        *target = Some(PathBuf::from(value));
    }
    let (Some(capsule), Some(capsule_root), Some(state_root)) = (capsule, capsule_root, state_root)
    else {
        eprintln!("hookstat-hook: --capsule, --capsule-root, and --state-root are required");
        return ExitCode::from(2);
    };
    let result = CapsuleStore::open(capsule_root)
        .and_then(|store| store.load(capsule))
        .and_then(|capsule| {
            run_capsule(&capsule, &state_root).map_err(|_| hook_shim::CapsuleError::Io)
        });
    match result {
        Ok(outcome) => {
            #[cfg(feature = "performance-harness")]
            if let Some(root) = std::env::var_os(G36_ORACLE_ROOT_ENV) {
                let _ = emit_g36_same_invocation_oracle(
                    Path::new(&root),
                    outcome.original_child_interval_ns,
                );
            }
            ExitCode::from(outcome.exit_code as u8)
        }
        Err(_) => {
            eprintln!("hookstat-hook: private control-plane or execution setup failed");
            ExitCode::from(1)
        }
    }
}
