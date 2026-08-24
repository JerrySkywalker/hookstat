//! Disposable process-tree fixture for HookStat shim integration tests.
//!
//! It accepts only explicit temporary paths and writes fixed marker bytes. It
//! is never called by the shipped shim and carries no Hook configuration or
//! private command material into HookStat evidence.

use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    match arguments.next().as_deref() {
        Some(value) if value == "--parent" => parent(arguments),
        Some(value) if value == "--child" => child(arguments),
        _ => ExitCode::from(2),
    }
}

fn parent(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> ExitCode {
    let (Some(started), Some(leaked), Some(delay_ms)) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        return ExitCode::from(2);
    }
    let Ok(current) = std::env::current_exe() else {
        return ExitCode::from(3);
    };
    if Command::new(current)
        .arg("--child")
        .arg(started)
        .arg(leaked)
        .arg(delay_ms)
        .spawn()
        .is_err()
    {
        return ExitCode::from(4);
    }
    // The parent stays alive until the shim timeout or external termination.
    thread::sleep(Duration::from_secs(10));
    ExitCode::SUCCESS
}

fn child(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> ExitCode {
    let (Some(started), Some(leaked), Some(delay_ms)) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        return ExitCode::from(2);
    }
    let Ok(delay_ms) = delay_ms.to_string_lossy().parse::<u64>() else {
        return ExitCode::from(2);
    };
    if std::fs::write(PathBuf::from(started), b"started").is_err() {
        return ExitCode::from(5);
    }
    thread::sleep(Duration::from_millis(delay_ms));
    if std::fs::write(PathBuf::from(leaked), b"leaked").is_err() {
        return ExitCode::from(6);
    }
    ExitCode::SUCCESS
}
