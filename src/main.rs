use hookstat::analytics::TimeWindow;
#[cfg(windows)]
use hookstat::codex::require_windows_path_identity;
use hookstat::codex::{
    apply, default_data_root, default_dry_run, discover_paths, discover_runtime_presentation,
    is_safe_handler_key, manifest_path_from_token, restore, trust,
};
use hookstat::ledger::Ledger;
use hookstat::observability::{StartupObservatory, StartupPhase, WorkCounters};
use hookstat::proxy;
use hookstat::receipt::ReceiptSpool;
use hookstat::render::render_home;
use hookstat::report::{MachineReport, instrumented_report, synthetic_fixture_report};
use hookstat::tui;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

fn print_help() {
    println!(
        "HookStat {version}\n\nReliability analytics for hooks across coding-agent runtimes.\n\nUsage:\n  hookstat [tui] [--lang <auto|en-US|zh-CN>] [--timing-output]\n  hookstat report [--json] [--read-only] [--data-root <path>]\n  hookstat doctor [--json] [--data-root <path>]\n  hookstat diagnostics export --output <path> --apply [--data-root <path>]\n  hookstat preview-fixture [--json]\n  hookstat identity alias --handler <hk_...> --name <display-name> [--data-root <path>]\n  hookstat codex instrument --dry-run [--config-root <path>]\n  hookstat codex instrument --apply --config-root <path> [--data-root <path>]\n  hookstat codex instrument --trust [--dry-run] --config-root <path> [--data-root <path>]\n  hookstat codex instrument --restore --config-root <path> [--data-root <path>]\n\nNormal Codex launch remains `codex`. Instrumentation is opt-in and wraps individual command handlers only after an explicit apply. `--apply` never approves trust. `--trust` is a separate explicit action that uses Codex's official App Server only after HookStat proves the current manifest, journal, and effective handlers are exact supported targets.",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(arguments) = tui_arguments(&arguments) {
        return tui_command(arguments);
    }
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
        Some("doctor") => doctor_command(&arguments[1..]),
        Some("diagnostics") => diagnostics_command(&arguments[1..]),
        Some("identity") => identity_command(&arguments[1..]),
        Some("codex") => codex_command(&arguments[1..]),
        None => tui_command(&[]),
        Some(value) => {
            eprintln!("hookstat: unknown command '{value}'");
            ExitCode::from(2)
        }
    }
}

fn tui_arguments(arguments: &[String]) -> Option<&[String]> {
    match arguments.first().map(String::as_str) {
        None => Some(arguments),
        Some("tui") => Some(&arguments[1..]),
        Some("--lang") => Some(arguments),
        Some(_) => None,
    }
}

fn identity_command(arguments: &[String]) -> ExitCode {
    if arguments.first().map(String::as_str) != Some("alias") {
        eprintln!("hookstat: expected `identity alias`");
        return ExitCode::from(2);
    }
    let Some(handler) = option_value(arguments, "--handler") else {
        eprintln!("hookstat: identity alias requires --handler");
        return ExitCode::from(2);
    };
    let Some(name) = option_value(arguments, "--name") else {
        eprintln!("hookstat: identity alias requires --name");
        return ExitCode::from(2);
    };
    if !is_safe_handler_key(handler) {
        eprintln!("hookstat: identity alias requires a safe handler key");
        return ExitCode::from(2);
    }
    let root = match option_path(arguments, "--data-root")
        .map(Ok)
        .unwrap_or_else(default_data_root)
    {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let mut ledger = match Ledger::open_path(root.join("ledger.sqlite3")) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    match ledger.set_handler_alias(
        hookstat::domain::Runtime::Codex,
        handler,
        name,
        now_unix_ms(),
    ) {
        Ok(()) => {
            println!("alias_saved=true");
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("hookstat: display name is not safe to store");
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
    let root = match option_path(arguments, "--data-root")
        .map(Ok)
        .unwrap_or_else(default_data_root)
    {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let result = if arguments.iter().any(|argument| argument == "--read-only") {
        load_read_only_report(&root)
    } else {
        load_current_report_at(&root)
    };
    match result {
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

fn doctor_command(arguments: &[String]) -> ExitCode {
    let root = match option_path(arguments, "--data-root")
        .map(Ok)
        .unwrap_or_else(default_data_root)
    {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let report = hookstat::diagnostics::collect(&root, now_unix_ms());
    if arguments.iter().any(|argument| argument == "--json") {
        return print_diagnostics_json(&report);
    }
    println!("HookStat diagnostics (read-only)");
    println!("overall={:?}", report.overall_status);
    for check in report.checks {
        println!("{:?}={:?}", check.id, check.status);
    }
    ExitCode::SUCCESS
}

fn diagnostics_command(arguments: &[String]) -> ExitCode {
    if arguments.first().map(String::as_str) != Some("export") {
        eprintln!("hookstat: expected `diagnostics export`");
        return ExitCode::from(2);
    }
    let Some(output) = option_path(arguments, "--output") else {
        eprintln!("hookstat: diagnostics export requires --output");
        return ExitCode::from(2);
    };
    let root = match option_path(arguments, "--data-root")
        .map(Ok)
        .unwrap_or_else(default_data_root)
    {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let report = hookstat::diagnostics::collect(&root, now_unix_ms());
    if !arguments.iter().any(|argument| argument == "--apply") {
        println!("export_preview=true");
        return print_diagnostics_json(&report);
    }
    let json = match serde_json::to_vec_pretty(&report) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("hookstat: diagnostics serialization failed");
            return ExitCode::from(1);
        }
    };
    let Some(parent) = output.parent() else {
        eprintln!("hookstat: diagnostics export output is invalid");
        return ExitCode::from(2);
    };
    if !parent.is_dir() {
        eprintln!("hookstat: diagnostics export parent must already exist");
        return ExitCode::from(2);
    }
    let temporary = parent.join(format!(".hookstat-diagnostics-{}.tmp", std::process::id()));
    if std::fs::write(&temporary, json).is_err() || std::fs::rename(&temporary, &output).is_err() {
        let _ = std::fs::remove_file(&temporary);
        eprintln!("hookstat: diagnostics export failed");
        return ExitCode::from(1);
    }
    println!("diagnostics_exported=true");
    ExitCode::SUCCESS
}
fn tui_command(arguments: &[String]) -> ExitCode {
    let explicit_language = match option_value(arguments, "--lang") {
        Some(value) => match hookstat::tui::localization::InterfaceLanguage::parse(value) {
            Some(language) => Some(language),
            None => {
                eprintln!("hookstat: --lang must be auto, en-US, or zh-CN");
                return ExitCode::from(2);
            }
        },
        None if arguments.iter().any(|argument| argument == "--lang") => {
            eprintln!("hookstat: --lang requires a locale value");
            return ExitCode::from(2);
        }
        None => None,
    };
    let root = match default_data_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let observatory = StartupObservatory::start();
    let timing_output = arguments
        .iter()
        .any(|argument| argument == "--timing-output");
    let reliability_root = root.clone();
    let reliability_observatory = observatory.clone();
    let diagnostics_root = root.clone();
    let diagnostics_observatory = observatory.clone();
    let changes_root = root.clone();
    let changes_observatory = observatory.clone();
    let runtime_catalog_cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
            return ExitCode::from(1);
        }
    };
    let alias_root = root;
    match tui::run_loading_with_refreshes_language(
        TimeWindow::Last7Days,
        explicit_language,
        move |request| {
            let started = Instant::now();
            let result = load_reliability_snapshot_at(
                &reliability_root,
                request.reason.window(),
                &reliability_observatory,
            );
            if result.is_ok() {
                let name = match request.reason {
                    tui::RefreshReason::Manual(_) => "warm_manual_refresh",
                    tui::RefreshReason::Window(_) => "period_request_to_snapshot",
                };
                reliability_observatory.record_latency(name, started.elapsed().as_millis());
            }
            result
        },
        move |_| {
            let started = Instant::now();
            let report = hookstat::diagnostics::collect(&diagnostics_root, now_unix_ms());
            diagnostics_observatory
                .record_latency("diagnostics_refresh", started.elapsed().as_millis());
            Ok(report)
        },
        move |request| {
            let started = Instant::now();
            let result = load_changes_snapshot_at(&changes_root, request.reason.window());
            if result.is_ok() {
                changes_observatory
                    .record_latency("changes_workbench_snapshot", started.elapsed().as_millis());
            }
            result
        },
        move |_| {
            discover_runtime_presentation(&runtime_catalog_cwd, now_unix_ms())
                .map_err(|error| error.to_string())
        },
        move |request| {
            let mut ledger = match Ledger::open_path(alias_root.join("ledger.sqlite3")) {
                Ok(ledger) => ledger,
                Err(_) => return tui::AliasApplyOutcome::Failed,
            };
            match ledger.set_handler_alias_if_unchanged(
                request.runtime,
                &request.handler_key,
                &request.draft,
                request.expected_alias.as_deref(),
                now_unix_ms(),
            ) {
                Ok(hookstat::ledger::AliasSaveOutcome::Saved) => tui::AliasApplyOutcome::Saved,
                Ok(hookstat::ledger::AliasSaveOutcome::Conflict) => {
                    tui::AliasApplyOutcome::Conflict
                }
                Err(_) => tui::AliasApplyOutcome::Failed,
            }
        },
        observatory.clone(),
    ) {
        Ok(()) => {
            if timing_output {
                println!("{}", observatory.sanitized_output());
            }
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("hookstat: interactive terminal operation failed");
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
    #[cfg(windows)]
    let executable = match require_windows_path_identity(&executable) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("hookstat: {error}");
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
    let manifest = match (
        option_path(arguments, "--manifest"),
        option_value(arguments, "--manifest-token"),
    ) {
        (Some(path), None) => path,
        (None, Some(token)) => match manifest_path_from_token(token) {
            Ok(path) => path,
            Err(_) => {
                eprintln!("hookstat: internal proxy requires a valid manifest token");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("hookstat: internal proxy requires exactly one manifest selector");
            return ExitCode::from(2);
        }
    };
    let Some(handler) = option_value(arguments, "--handler") else {
        eprintln!("hookstat: internal proxy requires --handler");
        return ExitCode::from(2);
    };
    if !is_safe_handler_key(handler) {
        eprintln!("hookstat: internal proxy requires a safe handler key");
        return ExitCode::from(2);
    }
    match proxy::run(&manifest, handler) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("hookstat: proxy setup failed: {error}");
            ExitCode::from(1)
        }
    }
}

fn load_current_report_at(
    root: &std::path::Path,
) -> Result<
    (
        MachineReport,
        Vec<hookstat::domain::HookInvocation>,
        u64,
        u64,
    ),
    String,
> {
    let spool = ReceiptSpool::open(root.join("receipts")).map_err(|error| error.to_string())?;
    let mut ledger =
        Ledger::open_path(root.join("ledger.sqlite3")).map_err(|error| error.to_string())?;
    let reconciled = spool
        .reconcile_incremental(&mut ledger, now_unix_ms())
        .map_err(|error| error.to_string())?;
    let now = now_unix_ms();
    let mut values = ledger
        .invocations_for_reliability(now, TimeWindow::Last7Days)
        .map_err(|error| error.to_string())?
        .invocations;
    let aliases = ledger
        .handler_aliases()
        .map_err(|error| error.to_string())?;
    for value in &mut values {
        if let Some(alias) = aliases
            .iter()
            .find(|alias| alias.runtime == value.runtime && alias.handler_key == value.handler.key)
        {
            value.handler.label.clone_from(&alias.display_name);
        }
    }
    let mut report = instrumented_report(
        &values,
        now,
        TimeWindow::Last7Days,
        reconciled.malformed,
        reconciled.incomplete,
    );
    enrich_report_from_ledger(&ledger, &mut report, now)?;
    Ok((report, values, reconciled.malformed, reconciled.incomplete))
}

fn load_read_only_report(
    root: &std::path::Path,
) -> Result<
    (
        MachineReport,
        Vec<hookstat::domain::HookInvocation>,
        u64,
        u64,
    ),
    String,
> {
    let ledger =
        Ledger::open_read_only(root.join("ledger.sqlite3")).map_err(|error| error.to_string())?;
    let reconciliation = ledger
        .receipt_reconciliation_state_if_present("receipt_catalog_journal_v1")
        .map_err(|error| error.to_string())?;
    let (malformed, incomplete, receipt_integrity_observed) = match reconciliation {
        Some(state) => (
            state.malformed_receipts,
            ledger
                .incomplete_receipt_count()
                .map_err(|error| error.to_string())?,
            true,
        ),
        None => (0, 0, false),
    };
    let now = now_unix_ms();
    let mut values = ledger
        .invocations_for_reliability(now, TimeWindow::Last7Days)
        .map_err(|error| error.to_string())?
        .invocations;
    let aliases = ledger
        .handler_aliases_if_present()
        .map_err(|error| error.to_string())?;
    for value in &mut values {
        if let Some(alias) = aliases
            .iter()
            .find(|alias| alias.runtime == value.runtime && alias.handler_key == value.handler.key)
        {
            value.handler.label.clone_from(&alias.display_name);
        }
    }
    let mut report = hookstat::report::instrumented_report_with_receipt_integrity(
        &values,
        now,
        TimeWindow::Last7Days,
        malformed,
        incomplete,
        receipt_integrity_observed,
    );
    enrich_report_from_ledger(&ledger, &mut report, now)?;
    Ok((report, values, malformed, incomplete))
}

fn load_reliability_snapshot_at(
    root: &std::path::Path,
    window: TimeWindow,
    observatory: &StartupObservatory,
) -> Result<tui::RefreshSnapshot, String> {
    let spool = ReceiptSpool::open(root.join("receipts")).map_err(|error| error.to_string())?;
    let mut ledger =
        Ledger::open_path(root.join("ledger.sqlite3")).map_err(|error| error.to_string())?;
    let reconciled = spool
        .reconcile_incremental(&mut ledger, now_unix_ms())
        .map_err(|error| error.to_string())?;
    observatory.mark(StartupPhase::ReceiptIngestReady);
    let now = now_unix_ms();
    let query = ledger
        .invocations_for_reliability(now, window)
        .map_err(|error| error.to_string())?;
    observatory.mark(StartupPhase::LedgerQueryReady);
    let mut values = query.invocations;
    let aliases = ledger
        .handler_aliases()
        .map_err(|error| error.to_string())?;
    for value in &mut values {
        if let Some(alias) = aliases
            .iter()
            .find(|alias| alias.runtime == value.runtime && alias.handler_key == value.handler.key)
        {
            value.handler.label.clone_from(&alias.display_name);
        }
    }
    observatory.record_work(WorkCounters {
        receipt_files_inspected: reconciled.work.files_inspected,
        receipt_files_parsed: reconciled.work.files_parsed,
        ledger_rows_materialized: query.rows_materialized,
        selected_query_range: query
            .bounds
            .current_start_unix_ms
            .map(|start| format!("[{start}, {}]", query.bounds.current_end_unix_ms)),
        requested_generation: None,
        accepted_generation: None,
    });
    let mut report = instrumented_report(
        &values,
        now,
        window,
        reconciled.malformed,
        reconciled.incomplete,
    );
    enrich_report_from_ledger(&ledger, &mut report, now)?;
    let alias_annotations = aliases
        .into_iter()
        .map(|alias| tui::AliasAnnotation {
            runtime: alias.runtime,
            handler_key: alias.handler_key,
            display_name: alias.display_name,
        })
        .collect();
    Ok(tui::RefreshSnapshot::from_report_with_aliases(
        report,
        alias_annotations,
    ))
}

/// Loads the historical workbench only when its page is requested. This path
/// deliberately remains separate from the bounded reliability refresh: a
/// normal Today/24h/7d/30d view must never materialize the complete ledger.
fn load_changes_snapshot_at(
    root: &std::path::Path,
    window: TimeWindow,
) -> Result<tui::ChangesSnapshot, String> {
    let spool = ReceiptSpool::open(root.join("receipts")).map_err(|error| error.to_string())?;
    let mut ledger =
        Ledger::open_path(root.join("ledger.sqlite3")).map_err(|error| error.to_string())?;
    let _reconciled = spool
        .reconcile_incremental(&mut ledger, now_unix_ms())
        .map_err(|error| error.to_string())?;
    let now = now_unix_ms();
    let query = ledger
        .invocations_for_workbench(now)
        .map_err(|error| error.to_string())?;
    let mut values = query.invocations;
    let aliases = ledger
        .handler_aliases()
        .map_err(|error| error.to_string())?;
    for value in &mut values {
        if let Some(alias) = aliases
            .iter()
            .find(|alias| alias.runtime == value.runtime && alias.handler_key == value.handler.key)
        {
            value.handler.label.clone_from(&alias.display_name);
        }
    }
    Ok(tui::ChangesSnapshot::from_values(
        values,
        now,
        window,
        hookstat::domain::SourceQualification::instrumented().coverage,
    ))
}

/// Finite-window raw rows cover the largest rolling comparison (60 days).
/// These specialized aggregate paths restore exact All-time and adjacent
/// revision semantics without loading full invocation history.
fn enrich_report_from_ledger(
    ledger: &Ledger,
    report: &mut MachineReport,
    now: i64,
) -> Result<(), String> {
    let all_time = ledger
        .all_time_period_metrics(now)
        .map_err(|error| error.to_string())?;
    let handler_keys = report
        .intelligence
        .iter()
        .map(|value| value.handler_key.clone())
        .collect::<Vec<_>>();
    let revisions = ledger
        .revision_epoch_metrics(&handler_keys)
        .map_err(|error| error.to_string())?;
    let coverage = report.qualification.coverage;
    for intelligence in &mut report.intelligence {
        if let Some(metrics) = all_time.get(&intelligence.handler_key)
            && let Some(trend) = intelligence
                .trends
                .iter_mut()
                .find(|trend| trend.window == TimeWindow::All)
        {
            *trend = hookstat::analytics::all_time_trend(metrics.clone(), coverage);
        }
        if let Some(epochs) = revisions.get(&intelligence.handler_key) {
            intelligence.revision_comparison = hookstat::analytics::revision_comparison_from_epochs(
                epochs.current.clone(),
                epochs.previous.clone(),
                coverage,
            );
        }
    }
    Ok(())
}

fn print_diagnostics_json(report: &hookstat::diagnostics::DiagnosticsReport) -> ExitCode {
    match serde_json::to_string_pretty(report) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(_) => {
            eprintln!("hookstat: diagnostics serialization failed");
            ExitCode::from(1)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_language_flag_matches_the_documented_invocation_forms() {
        let implicit = vec!["--lang".to_owned(), "zh-CN".to_owned()];
        assert_eq!(tui_arguments(&implicit), Some(implicit.as_slice()));

        let explicit = vec!["tui".to_owned(), "--lang".to_owned(), "en-US".to_owned()];
        assert_eq!(tui_arguments(&explicit), Some(&explicit[1..]));

        let report = vec!["report".to_owned(), "--json".to_owned()];
        assert_eq!(tui_arguments(&report), None);
    }
}
