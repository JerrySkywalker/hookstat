//! Runs a disposable G36 test process outside an inherited non-nestable Job.

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

fn main() {
    let mut program = None;
    let mut receipt_output = None;
    let mut shipping_shim = None;
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while index < arguments.len() {
        let flag = &arguments[index];
        let value = arguments.get(index + 1).map(String::as_str);
        match (flag.as_str(), value) {
            ("--program", Some(value)) => program = Some(PathBuf::from(value)),
            ("--receipt-output", Some(value)) => receipt_output = Some(PathBuf::from(value)),
            ("--shipping-shim", Some(value)) => shipping_shim = Some(PathBuf::from(value)),
            _ => {
                eprintln!(
                    "usage: hookstat-g36-breakaway-launcher --program <g36-test-exe> --receipt-output <sanitized-json> [--shipping-shim <ordinary-hookstat-hook>]"
                );
                std::process::exit(2);
            }
        }
        index += 2;
    }
    let (Some(program), Some(receipt_output)) = (program, receipt_output) else {
        eprintln!("hookstat-g36-breakaway-launcher requires both bounded inputs");
        std::process::exit(2);
    };
    if !program.is_file() {
        eprintln!("hookstat-g36-breakaway-launcher received no test executable");
        std::process::exit(2);
    }
    if shipping_shim.as_ref().is_some_and(|path| !path.is_file()) {
        eprintln!("hookstat-g36-breakaway-launcher received no shipping shim");
        std::process::exit(2);
    }
    #[cfg(windows)]
    let result = {
        let mut command = Command::new(program);
        command
            .arg("--ignored")
            .env("HOOKSTAT_G36_PERFORMANCE_OUTPUT", receipt_output)
            .creation_flags(CREATE_BREAKAWAY_FROM_JOB);
        if let Some(shipping_shim) = shipping_shim {
            command.env("HOOKSTAT_G36_SHIPPING_SHIM", shipping_shim);
        }
        command.status()
    };
    #[cfg(not(windows))]
    let result: Result<std::process::ExitStatus, std::io::Error> = Err(std::io::Error::other(
        "G36 breakaway launcher is Windows-only",
    ));
    match result {
        Ok(status) if status.success() => println!("G36_BREAKAWAY_TEST_PASSED"),
        Ok(_) | Err(_) => {
            eprintln!("G36_BREAKAWAY_TEST_FAILED");
            std::process::exit(1);
        }
    }
}
