use std::process::ExitCode;

use hookstat::render::render_home;
use hookstat::{blocked_report, synthetic_fixture_report};

fn print_help() {
    println!(
        "HookStat {version}\n\nReliability analytics for hooks across coding-agent runtimes.\n\nUsage:\n  hookstat [status]\n  hookstat preview-fixture [--json]\n  hookstat [--help] [--version]\n\nCodex historical ingestion is currently blocked pending a durable, per-handler evidence source. preview-fixture is deterministic synthetic development evidence only; it never reads Codex data.",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.first().map(String::as_str) {
        Some("-h" | "--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("hookstat {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        None | Some("status") => {
            let report = blocked_report(now_unix_ms());
            print!("{}", render_home(&report, 80));
            ExitCode::from(3)
        }
        Some("preview-fixture") => {
            let report = synthetic_fixture_report(now_unix_ms());
            if arguments
                .get(1)
                .is_some_and(|argument| argument == "--json")
            {
                match report.to_pretty_json() {
                    Ok(json) => println!("{json}"),
                    Err(error) => {
                        eprintln!("hookstat: failed to serialize synthetic report: {error}");
                        return ExitCode::from(1);
                    }
                }
            } else {
                print!("{}", render_home(&report, 80));
            }
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!(
                "hookstat: command '{command}' is not available in the repository-foundation build"
            );
            ExitCode::from(2)
        }
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
