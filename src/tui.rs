//! The frozen Ratatui/crossterm v0.1 interaction baseline.

use crate::analytics::TimeWindow;
use crate::domain::HookInvocation;
use crate::report::{MachineReport, instrumented_report};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};
use std::io;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Home,
    Detail,
}

#[derive(Clone, Debug)]
pub struct App {
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
    window: TimeWindow,
    selected: usize,
    screen: Screen,
    ingest_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RefreshSnapshot {
    pub values: Vec<HookInvocation>,
    pub malformed: u64,
    pub incomplete: u64,
    pub now: i64,
}

type Refresh = Box<dyn FnMut() -> Result<RefreshSnapshot, String>>;

impl App {
    pub fn new(values: Vec<HookInvocation>, malformed: u64, incomplete: u64, now: i64) -> Self {
        Self {
            values,
            malformed,
            incomplete,
            now,
            window: TimeWindow::Last7Days,
            selected: 0,
            screen: Screen::Home,
            ingest_error: None,
        }
    }
    pub fn with_ingest_error(mut self) -> Self {
        self.ingest_error = Some("Receipt refresh failed; accepted history retained.".into());
        self
    }
    fn report(&self) -> MachineReport {
        instrumented_report(
            &self.values,
            self.now,
            self.window,
            self.malformed,
            self.incomplete,
        )
    }
    fn move_selection(&mut self, delta: isize) {
        let length = self.report().handlers.len();
        if length == 0 {
            self.selected = 0;
        } else {
            self.selected = (self.selected as isize + delta).rem_euclid(length as isize) as usize;
        }
    }
    fn choose_window(&mut self, window: TimeWindow) {
        self.window = window;
        self.selected = 0;
    }
    fn replace_snapshot(&mut self, snapshot: RefreshSnapshot) {
        self.values = snapshot.values;
        self.malformed = snapshot.malformed;
        self.incomplete = snapshot.incomplete;
        self.now = snapshot.now;
        self.ingest_error = None;
        let length = self.report().handlers.len();
        if length == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(length - 1);
        }
    }
}

pub fn run(
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
) -> io::Result<()> {
    run_with_optional_refresh(values, malformed, incomplete, now, None)
}

pub fn run_with_refresh(
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
    refresh: impl FnMut() -> Result<RefreshSnapshot, String> + 'static,
) -> io::Result<()> {
    run_with_optional_refresh(values, malformed, incomplete, now, Some(Box::new(refresh)))
}

fn run_with_optional_refresh(
    values: Vec<HookInvocation>,
    malformed: u64,
    incomplete: u64,
    now: i64,
    mut refresh: Option<Refresh>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut output = io::stdout();
    if let Err(error) = execute!(output, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error);
    }
    let backend = CrosstermBackend::new(output);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let mut recovery = io::stdout();
            let _ = execute!(recovery, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = run_loop(
        &mut terminal,
        App::new(values, malformed, incomplete, now),
        &mut refresh,
    );
    let cleanup = terminal
        .show_cursor()
        .and_then(|_| execute!(terminal.backend_mut(), LeaveAlternateScreen))
        .and_then(|_| disable_raw_mode());
    result.and(cleanup)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    refresh: &mut Option<Refresh>,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            KeyCode::Enter if !app.report().handlers.is_empty() => app.screen = Screen::Detail,
            KeyCode::Esc | KeyCode::Backspace => app.screen = Screen::Home,
            KeyCode::Char('1') => app.choose_window(TimeWindow::Last24Hours),
            KeyCode::Char('7') => app.choose_window(TimeWindow::Last7Days),
            KeyCode::Char('3') => app.choose_window(TimeWindow::Last30Days),
            KeyCode::Char('a') => app.choose_window(TimeWindow::All),
            KeyCode::Char('r') => match refresh.as_mut() {
                Some(refresh) => match refresh() {
                    Ok(snapshot) => app.replace_snapshot(snapshot),
                    Err(_) => app = app.with_ingest_error(),
                },
                None => app.now = now_unix_ms(),
            },
            _ => {}
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Home => draw_home(frame, app),
        Screen::Detail => draw_detail(frame, app),
    }
}

fn draw_home(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let report = app.report();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(format!(
            "Hook Reliability                          {}",
            report.window.label()
        ))
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::BOTTOM)),
        chunks[0],
    );
    let mut coverage = format!(
        "Codex · instrumented receipts · {:?} coverage · incomplete={} malformed={}",
        report.qualification.coverage, report.incomplete_receipts, report.malformed_receipts
    );
    if let Some(error) = &app.ingest_error {
        coverage.push_str(&format!("\nError: {error}"));
    }
    frame.render_widget(
        Paragraph::new(coverage).style(Style::default().fg(Color::Yellow)),
        chunks[1],
    );
    if report.handlers.is_empty() {
        frame.render_widget(Paragraph::new("No admitted receipt rows yet. This is not 0.00% healthy.\nRun `hookstat codex instrument --dry-run` to inspect opt-in coverage.").block(Block::default().title("Most unreliable hooks").borders(Borders::ALL)), chunks[2]);
    } else if area.width < 52 {
        let lines = report
            .handlers
            .iter()
            .enumerate()
            .map(|(index, item)| {
                format!(
                    "{} {} / {}  {:.2}% (n={})",
                    if index == app.selected { ">" } else { " " },
                    item.handler.event.label(),
                    &item.handler.key[3..],
                    item.failure_rate_percent,
                    item.failure_sample_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .title("Most unreliable hooks")
                    .borders(Borders::ALL),
            ),
            chunks[2],
        );
    } else {
        let header = Row::new(["Handler", "Runs", "Failed", "Failure"]);
        let rows = report.handlers.iter().enumerate().map(|(index, item)| {
            Row::new(vec![
                Cell::from(format!(
                    "{} / {}",
                    item.handler.event.label(),
                    &item.handler.key[3..]
                )),
                Cell::from(item.runs.to_string()),
                Cell::from(item.failed_runs.to_string()),
                Cell::from(format!(
                    "{:.2}% (n={})",
                    item.failure_rate_percent, item.failure_sample_count
                )),
            ])
            .style(if index == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
        });
        let table = Table::new(
            rows,
            [
                Constraint::Min(24),
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Length(15),
            ],
        )
        .header(header.style(Style::default().add_modifier(Modifier::BOLD)))
        .block(
            Block::default()
                .title("Most unreliable hooks")
                .borders(Borders::ALL),
        );
        frame.render_widget(table, chunks[2]);
    }
    let help = "j/k select · Enter detail · 1/7/3/a range · r refresh · q quit";
    frame.render_widget(Paragraph::new(help), chunks[3]);
}

fn draw_detail(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let report = app.report();
    let Some(item) = report.handlers.get(app.selected) else {
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new("No handler selected. Esc to return."), area);
        return;
    };
    let mut text = format!(
        "{} · {} · Codex\n\n24h / 7d / 30d / All selection: {}\nFailed {} / samples {} · {:.2}% (n={})\n\nCompleted {}  Failed {}  Blocked {}  Stopped {}\nTimedOut {}  ProtocolFailure {}\n",
        item.handler.key,
        item.handler.event.label(),
        report.window.label(),
        item.failed_runs,
        item.failure_sample_count,
        item.failure_rate_percent,
        item.failure_sample_count,
        item.terminal.completed,
        item.terminal.failed,
        item.terminal.blocked,
        item.terminal.stopped,
        item.terminal.timed_out,
        item.terminal.protocol_failure
    );
    if item.terminal.incomplete > 0 || item.terminal.unknown > 0 {
        text.push_str(&format!(
            "\nCoverage warning: incomplete={} unknown={}; never treated as healthy.\n",
            item.terminal.incomplete, item.terminal.unknown
        ));
    }
    if let Some(p50) = item.p50_duration_ms {
        text.push_str(&format!(
            "\np50 {p50} ms · p95 {} ms · p99 {} ms\n",
            item.p95_duration_ms.unwrap_or(p50),
            item.p99_duration_ms.unwrap_or(p50)
        ));
    }
    text.push_str("\nEsc/Backspace back · j/k select · 1/7/3/a range · r refresh · q quit");
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Hook detail")),
        area,
    );
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
    use ratatui::backend::TestBackend;
    fn rendered(app: App, width: u16) -> String {
        let backend = TestBackend::new(width, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }
    #[test]
    fn normal_and_small_tui_keep_failure_sample_count() {
        let app = App::new(vec![], 0, 0, 1_000);
        assert!(rendered(app, 80).contains("not 0.00% healthy"));
        let app = App::new(
            crate::report::synthetic_fixture_invocations_for_tui(1_000),
            0,
            0,
            1_000,
        );
        for width in [80, 44] {
            assert!(rendered(app.clone(), width).contains("n="));
        }
    }
    #[test]
    fn ingest_error_and_detail_are_renderable() {
        let mut app = App::new(
            crate::report::synthetic_fixture_invocations_for_tui(1_000),
            0,
            0,
            1_000,
        )
        .with_ingest_error();
        assert!(rendered(app.clone(), 80).contains("accepted history retained"));
        app.screen = Screen::Detail;
        assert!(rendered(app, 80).contains("Hook detail"));
    }
    #[test]
    fn refresh_snapshot_replaces_data_and_preserves_small_terminal_invariants() {
        let mut app = App::new(vec![], 4, 2, 1_000).with_ingest_error();
        app.replace_snapshot(RefreshSnapshot {
            values: crate::report::synthetic_fixture_invocations_for_tui(2_000),
            malformed: 0,
            incomplete: 1,
            now: 2_000,
        });
        assert!(app.ingest_error.is_none());
        assert!(rendered(app.clone(), 44).contains("n="));
        assert!(rendered(app, 80).contains("incomplete=1"));
    }
}
