//! Narrow local broker entrypoint for the G36 on-demand handoff.
//!
//! It owns no runtime integration, manifest, analytics, TUI, report, SQLite
//! operation, network listener, or configuration mutation. It exits on the
//! accepted G35 idle policy.

use hookstat::ipc::{BrokerConfig, BrokerHost};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let mut values = std::env::args().skip(1);
    if values.next().as_deref() != Some("--state-root") { return ExitCode::from(2); }
    let Some(root) = values.next() else { return ExitCode::from(2); };
    if values.next().is_some() { return ExitCode::from(2); }
    match BrokerHost::start(BrokerConfig::for_state_root(PathBuf::from(root))) {
        Ok(mut host) => {
            let _ = host.wait_for_idle(Duration::from_secs(65));
            ExitCode::SUCCESS
        }
        // A concurrent shim may have started a healthy local broker first.
        Err(hookstat_ipc_client::IpcError::EndpointInUse) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
