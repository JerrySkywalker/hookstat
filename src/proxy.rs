//! Transparent command-hook proxy.
//!
//! The proxy intentionally does not call `read_to_end`, `output`, or any
//! stream parser. Codex stdin, stdout, and stderr are OS-inherited handles.
//! The only retained data are bounded invocation metadata in `receipt.rs`.

use crate::codex::{CodexError, load_manifest};
use crate::domain::{EvidenceCoverage, TerminalStatus};
use crate::receipt::{ReceiptCompletion, ReceiptSpool, ReceiptStart};
use std::fmt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Keeps the original handler's process tree bound to the proxy lifetime on
/// Windows. The job is armed before the handler is spawned, so a forced proxy
/// termination closes the only job handle and the kernel terminates the whole
/// tree. On a normal root-handler exit, the kill-on-close limit is cleared
/// before the job handle is released so intentionally surviving descendants
/// remain alive.
#[cfg(windows)]
struct ProcessContainment {
    job: win32job::Job,
}

#[cfg(windows)]
impl ProcessContainment {
    fn establish() -> Result<Self, ProxyError> {
        let mut limits = win32job::ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job = win32job::Job::create_with_limit_info(&limits)
            .map_err(|_| ProxyError::ProcessContainment)?;
        // The proxy enters the job before it creates the original shell. Its
        // descendants are therefore contained without a post-spawn race.
        job.assign_current_process()
            .map_err(|_| ProxyError::ProcessContainment)?;
        Ok(Self { job })
    }

    fn release_after_normal_root_exit(&mut self) -> Result<(), ProxyError> {
        let mut limits = self
            .job
            .query_extended_limit_info()
            .map_err(|_| ProxyError::ProcessContainment)?;
        limits.clear_limits();
        self.job
            .set_extended_limit_info(&limits)
            .map_err(|_| ProxyError::ProcessContainment)
    }
}

#[cfg(not(windows))]
struct ProcessContainment;

#[cfg(not(windows))]
impl ProcessContainment {
    fn establish() -> Result<Self, ProxyError> {
        Ok(Self)
    }

    fn release_after_normal_root_exit(&mut self) -> Result<(), ProxyError> {
        Ok(())
    }
}

static INVOCATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum ProxyError {
    Codex(CodexError),
    Receipt(crate::receipt::ReceiptError),
    MissingHandler,
    ProcessContainment,
}
impl fmt::Display for ProxyError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Codex(error) => error.fmt(output),
            Self::Receipt(error) => error.fmt(output),
            Self::MissingHandler => {
                output.write_str("instrumentation manifest has no requested handler")
            }
            Self::ProcessContainment => output
                .write_str("HookStat could not establish the required handler process containment"),
        }
    }
}
impl std::error::Error for ProxyError {}
impl From<CodexError> for ProxyError {
    fn from(value: CodexError) -> Self {
        Self::Codex(value)
    }
}
impl From<crate::receipt::ReceiptError> for ProxyError {
    fn from(value: crate::receipt::ReceiptError) -> Self {
        Self::Receipt(value)
    }
}

/// Executes exactly one saved command handler. Receipt failures are ignored
/// after a best-effort attempt so telemetry cannot intentionally stop a hook.
pub fn run(manifest_path: &Path, handler_key: &str) -> Result<i32, ProxyError> {
    let manifest = load_manifest(manifest_path)?;
    let handler = manifest
        .handlers
        .get(handler_key)
        .ok_or(ProxyError::MissingHandler)?;
    let data_root = manifest_path
        .parent()
        .and_then(Path::parent)
        .ok_or(ProxyError::MissingHandler)?;
    let spool = ReceiptSpool::open(data_root.join("receipts"));
    let started_at = now_unix_ms();
    let invocation_id = format!(
        "i{:016x}{:08x}{:016x}",
        started_at as u64,
        std::process::id(),
        INVOCATION_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let start = ReceiptStart {
        schema_version: 1,
        invocation_id: invocation_id.clone(),
        handler: handler.handler.clone(),
        source: "codex_instrumented_proxy_v1".into(),
        started_at_unix_ms: started_at,
        coverage: EvidenceCoverage::Partial,
    };
    if let Ok(spool) = &spool {
        let _ = spool.write_start(&start);
    }

    let command_line = if cfg!(windows) {
        handler
            .command_windows
            .as_deref()
            .unwrap_or(&handler.command)
    } else {
        &handler.command
    };
    let mut child = platform_shell(command_line);
    // These are direct inheritance operations: bytes pass from Codex to the
    // original command and back without HookStat observing their contents.
    child
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // Establish containment before spawning the handler. On Windows the
    // proxy itself joins a kill-on-close Job Object, which atomically covers
    // every subsequently spawned child and descendant.
    let mut containment = ProcessContainment::establish()?;
    let status = child.status();
    if status.is_ok() {
        containment.release_after_normal_root_exit()?;
    }
    let completed_at = now_unix_ms();
    let (exit_code, terminal_status) = match &status {
        Ok(status) => match status.code() {
            Some(0) => (Some(0), TerminalStatus::Completed),
            // Codex uses exit 2 plus stderr text for several control outcomes.
            // The proxy is prohibited from inspecting stderr, so it remains
            // explicitly unknown rather than a fabricated Blocked/Failed row.
            Some(2) => (Some(2), TerminalStatus::Unknown),
            Some(code) => (Some(code), TerminalStatus::Failed),
            None => (None, TerminalStatus::Unknown),
        },
        Err(_) => (None, TerminalStatus::ProtocolFailure),
    };
    let completion = ReceiptCompletion {
        schema_version: 1,
        invocation_id,
        handler: handler.handler.clone(),
        source: "codex_instrumented_proxy_v1".into(),
        started_at_unix_ms: started_at,
        completed_at_unix_ms: completed_at,
        duration_ms: completed_at.saturating_sub(started_at) as u64,
        exit_code,
        terminal_status,
        coverage: EvidenceCoverage::Partial,
    };
    if let Ok(spool) = &spool {
        let _ = spool.write_completion(&completion);
    }
    Ok(status.ok().and_then(|status| status.code()).unwrap_or(1))
}

#[cfg(windows)]
fn platform_shell(command_line: &str) -> Command {
    // This mirrors Codex's current default-shell contract: COMSPEC /C on
    // Windows and SHELL (or /bin/sh) -lc on Unix. The outer proxy itself is
    // launched by Codex's own configured shell, so the original handler sees
    // the same cwd and inherited environment. A non-default Codex shell is a
    // documented partial-coverage limitation rather than a guessed rewrite.
    let shell = std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(shell);
    command.arg("/C").arg(command_line);
    command
}

#[cfg(not(windows))]
fn platform_shell(command_line: &str) -> Command {
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    let mut command = Command::new(shell);
    command.args(["-lc", command_line]);
    command
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}
