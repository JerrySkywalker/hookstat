#[allow(dead_code)]
#[path = "../hook_shim.rs"]
mod hook_shim;
#[allow(dead_code)]
#[path = "../ipc_client.rs"]
mod ipc_client;

use hook_shim::{CapsuleStore, run_capsule};
use std::path::PathBuf;
use std::process::ExitCode;

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
        Ok(outcome) => ExitCode::from(outcome.exit_code as u8),
        Err(_) => {
            eprintln!("hookstat-hook: private control-plane or execution setup failed");
            ExitCode::from(1)
        }
    }
}
