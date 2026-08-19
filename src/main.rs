use std::process::ExitCode;

fn print_help() {
    println!(
        "HookStat {version}\n\nReliability analytics for hooks across coding-agent runtimes.\n\nUsage:\n  hookstat [--help] [--version]\n\nThe repository-foundation build does not yet ingest runtime evidence.",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("-h" | "--help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("hookstat {}", env!("CARGO_PKG_VERSION"));
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
