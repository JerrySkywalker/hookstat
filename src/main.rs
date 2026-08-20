use hookstat::analytics::TimeWindow;
use hookstat::codex::{apply, default_data_root, default_dry_run, discover_paths, restore, trust};
use hookstat::ledger::Ledger;
use hookstat::proxy;
use hookstat::receipt::ReceiptSpool;
use hookstat::render::render_home;
use hookstat::report::{MachineReport, instrumented_report, synthetic_fixture_report};
use hookstat::tui;
use std::path::PathBuf;
use std::process::ExitCode;

fn print_help() {
    println!(
        "HookStat {version}\n\nReliability analytics for hooks across coding-agent runtimes.\n\nUsage:\n  hookstat [tui]\n  hookstat report [--json]\n  hookstat preview-fixture [--json]\n  hookstat codex instrument --dry-run [--config-root <path>]\n  hookstat codex instrument --apply --config-root <path> [--data-root <path>]\n  hookstat codex instrument --trust [--dry-run] --config-root <path> [--data-root <path>]\n  hookstat codex instrument --restore --config-root <path> [--data-root <path>]\n\nNormal Codex launch remains `codex`. Instrumentation is opt-in and wraps individual command handlers only after an explicit apply. `--apply` never approves trust. `--trust` is a separate explicit action that uses Codex's official App Server only after HookStat proves the current manifest, journal, and effective handlers are exact supported targets.",
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
        Some("preview-fixture") => preview_fixture(&arguments[1..]),
        Some("report") => report_command(&arguments[1..]),
        Some("codex") => codex_command(&arguments[1..]),
        None | Some("tui") => tui_command(),
        Some(value) => {
            eprintln!("hookstat: unknown command '{value}'");
            ExitCode::from(2)
        }
    }
}

fn preview_fixture(arguments: &[String]) -> ExitCode {
    let report = synthetic_fixture_report(now_unix_ms());
    if arguments.iter().any(|value| value == "--json") {
        print_json(&report)
    } else {
        print!("{}", render_home(&report, 80));
        ExitCode::SUCCESS
    }
}
fn report_command(arguments: &[String]) -> ExitCode {
    match load_current_report() {
        Ok((report, _, _, _)) if arguments.iter().any(|value| value == "--json") => {
            print_json(&report)
        }
        Ok((report, _, _, _)) => {
            print!("{}", render_home(&report, 80));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("hookstat: {error}");
            ExitCode::from(1)
        }
    }
}
fn tui_command() -> ExitCode {
    match load_current_report() {
        Ok((_, values, malformed, incomplete)) => {
            match tui::run_with_refresh(values, malformed, incomplete, now_unix_ms(), || {
                load_current_report().map(|(_, values, malformed, incomplete)| {
                    tui::RefreshSnapshot {
                        values,
                        malformed,
                        incomplete,
                        now: now_unix_ms(),
                    }
                })
            }) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => {
                    eprintln!("hookstat: interactive terminal operation failed");
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("hookstat: {error}");
            ExitCode::from(1)
        }
    }
}

fn codex_command(arguments: &[String]) -> ExitCode {
    match arguments.first().map(String::as_str) {
        Some("instrument") => instrument_command(&arguments[1..]),
        Some("proxy") => proxy_command(&arguments[1..]),
        _ => {
            eprintln!("hookstat: expected `codex instrument` or internal `codex proxy`");
            ExitCode::from(2)
        }
    }
}

fn instrument_command(arguments: &[String]) -> ExitCode {
    let dry_run = arguments.iter().any(|value| value == "--dry-run");
    let apply_requested = arguments.iter().any(|value| value == "--apply");
    let restore_requested = arguments.iter().any(|value| value == "--restore");
    let trust_requested = arguments.iter().any(|value| value == "--trust");
    if [apply_requested, restore_requested, trust_requested]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
        || (!dry_run && !apply_requested && !restore_requested && !trust_requested)
        || (dry_run && (apply_requested || restore_requested))
    {
        eprintln!("hookstat: choose --dry-run, --apply, --trust [--dry-run], or --restore");
        return ExitCode::from(2);
    }
    let config_root = option_path(arguments, "--config-root");
    let data_root = option_path(arguments, "--data-root")
        .map(Ok)
        .unwrap_or_else(default_data_root);
    let data_root = match data_root {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    if restore_requested {
        let Some(root) = config_root else {
            eprintln!("hookstat: --restore requires explicit --config-root");
            return ExitCode::from(2);
        };
        return match restore(&root.join("hooks.json"), &data_root) {
            Ok(summary) => {
                print_safe_json(&summary);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("hookstat: {error}");
                ExitCode::from(1)
            }
        };
    }
    if trust_requested {
        let Some(root) = config_root else {
            eprintln!("hookstat: --trust requires explicit --config-root");
            return ExitCode::from(2);
        };
        let cwd = match std::env::current_dir() {
            Ok(path) => path,
            Err(_) => {
                eprintln!("hookstat: cannot resolve current working directory");
                return ExitCode::from(1);
            }
        };
        return match trust(&root.join("hooks.json"), &data_root, &cwd, dry_run) {
            Ok(summary) => {
                print_safe_json(&summary);
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("hookstat: {error}");
                ExitCode::from(1)
            }
        };
    }
    let discovery = match config_root {
        Some(root) => discover_paths(&[root.join("hooks.json"), root.join("config.toml")]),
        None if dry_run => {
            return match default_dry_run() {
                Ok(report) => {
                    print_safe_json(&report);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("hookstat: {error}");
                    ExitCode::from(1)
                }
            };
        }
        None => {
            eprintln!(
                "hookstat: --apply requires explicit --config-root; no live configuration is selected implicitly"
            );
            return ExitCode::from(2);
        }
    };
    let discovery = match discovery {
        Ok(value) => value,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    if dry_run {
        print_safe_json(&discovery.summary);
        return ExitCode::SUCCESS;
    }
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            eprintln!("hookstat: cannot locate HookStat executable");
            return ExitCode::from(1);
        }
    };
    match apply(&discovery, &data_root, &executable) {
        Ok(summary) => {
            print_safe_json(&summary);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("hookstat: {error}");
            ExitCode::from(1)
        }
    }
}

fn proxy_command(arguments: &[String]) -> ExitCode {
    let Some(manifest) = option_path(arguments, "--manifest") else {
        eprintln!("hookstat: internal proxy requires --manifest");
        return ExitCode::from(2);
    };
    let Some(handler) = option_value(arguments, "--handler") else {
        eprintln!("hookstat: internal proxy requires --handler");
        return ExitCode::from(2);
    };
    match proxy::run(&manifest, handler) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("hookstat: proxy setup failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn load_current_report() -> Result<
    (
        MachineReport,
        Vec<hookstat::domain::HookInvocation>,
        u64,
        u64,
    ),
    String,
> {
    let root = default_data_root().map_err(|error| error.to_string())?;
    let spool = ReceiptSpool::open(root.join("receipts")).map_err(|error| error.to_string())?;
    let mut ledger =
        Ledger::open_path(root.join("ledger.sqlite3")).map_err(|error| error.to_string())?;
    let (scan, _) = spool
        .ingest_into(&mut ledger)
        .map_err(|error| error.to_string())?;
    let values = ledger.invocations().map_err(|error| error.to_string())?;
    let report = instrumented_report(
        &values,
        now_unix_ms(),
        TimeWindow::Last7Days,
        scan.malformed,
        scan.starts_without_completion,
    );
    Ok((
        report,
        values,
        scan.malformed,
        scan.starts_without_completion,
    ))
}
fn print_json(report: &MachineReport) -> ExitCode {
    match report.to_pretty_json() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("hookstat: JSON serialization failed");
            ExitCode::from(1)
        }
    }
}
fn print_safe_json(value: &impl serde::Serialize) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(_) => eprintln!("hookstat: JSON serialization failed"),
    }
}
fn option_value<'a>(arguments: &'a [String], flag: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|value| value == flag)
        .and_then(|index| arguments.get(index + 1))
        .map(String::as_str)
}
fn option_path(arguments: &[String], flag: &str) -> Option<PathBuf> {
    option_value(arguments, flag).map(PathBuf::from)
}
fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}
