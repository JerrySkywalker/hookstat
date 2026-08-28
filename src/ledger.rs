//! HookStat-owned SQLite storage for source-neutral canonical metadata only.

use crate::analytics::{PeriodMetrics, RevisionMetrics, TimeBounds, TimeWindow};
use crate::domain::{
    EvidenceCoverage, EvidenceGeneration, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent,
    HookInvocation, Runtime, TerminalStatus, ValidationError,
};
use crate::evidence::CORRELATION_CONFLICT_FINGERPRINT;
use crate::identity::{generated_label, sanitize_display_name};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 5;

// Historical rows remain byte-for-byte durable even when a future or malformed
// taxonomy value cannot be represented by the current domain model. Canonical
// readers use this predicate to quarantine those rows from reliability counts
// without rewriting or deleting them. The generation clause is separate so a
// read-only v0.3 ledger can still be interpreted before the additive column is
// migrated.
const CANONICAL_TAXONOMY_WITHOUT_GENERATION: &str = "runtime IN ('codex', 'deepseek_harness', 'opencode')
    AND evidence_kind IN ('codex_session_jsonl', 'codex_state_database', 'codex_app_server_live', 'codex_instrumented_receipt', 'runtime_neutral_ipc', 'open_telemetry', 'synthetic_fixture')
    AND coverage IN ('complete', 'partial', 'sync_only', 'best_effort', 'unknown', 'not_admitted', 'synthetic_fixture')
    AND handler_event IN ('session_start', 'session_end', 'user_prompt_submit', 'pre_tool_use', 'post_tool_use', 'permission_request', 'pre_compact', 'post_compact', 'stop', 'subagent_start', 'subagent_stop')
    AND handler_execution_mode IN ('sync', 'async', 'unknown')
    AND terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure', 'incomplete', 'unknown')";

const CANONICAL_TAXONOMY_WITH_GENERATION: &str = "runtime IN ('codex', 'deepseek_harness', 'opencode')
    AND evidence_kind IN ('codex_session_jsonl', 'codex_state_database', 'codex_app_server_live', 'codex_instrumented_receipt', 'runtime_neutral_ipc', 'open_telemetry', 'synthetic_fixture')
    AND evidence_generation IN ('legacy_v03_proxy', 'v031_native', 'v031_cooperative_ipc', 'synthetic_fixture')
    AND coverage IN ('complete', 'partial', 'sync_only', 'best_effort', 'unknown', 'not_admitted', 'synthetic_fixture')
    AND handler_event IN ('session_start', 'session_end', 'user_prompt_submit', 'pre_tool_use', 'post_tool_use', 'permission_request', 'pre_compact', 'post_compact', 'stop', 'subagent_start', 'subagent_stop')
    AND handler_execution_mode IN ('sync', 'async', 'unknown')
    AND terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure', 'incomplete', 'unknown')";

pub struct Ledger {
    connection: Connection,
    has_evidence_generation: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IngestReceipt {
    pub inserted: u64,
    pub upgraded: u64,
    pub duplicates: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerAlias {
    pub runtime: Runtime,
    pub handler_key: String,
    pub display_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasSaveOutcome {
    Saved,
    Conflict,
}

/// Durable state for the append-only receipt reconciliation journal. The
/// cursor is committed in the same SQLite transaction as its accepted rows,
/// so interruption can only replay idempotent evidence on restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiptReconciliationState {
    pub journal_offset: u64,
    pub malformed_receipts: u64,
}

/// Read-only, bounded receipt facts retained by the reconciliation catalog.
///
/// These values are valid only when the caller has independently confirmed
/// that the durable receipt journal has not advanced past `journal_offset`.
/// They deliberately do not attempt to revalidate every canonical receipt
/// file: that is an explicit reconciliation operation, not a diagnostics
/// control-plane query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReceiptCatalogDiagnostics {
    pub journal_offset: u64,
    pub record_count: u64,
    pub incomplete: u64,
    pub malformed: u64,
    pub latest_occurred_at_unix_ms: Option<i64>,
}

/// Work returned by the normal analysis query. Its row count is a real count
/// of materialized canonical rows, not a wall-clock estimate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerQuery {
    pub invocations: Vec<HookInvocation>,
    pub rows_materialized: u64,
    pub bounds: TimeBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RevisionEpochMetrics {
    pub current: RevisionMetrics,
    pub previous: Option<RevisionMetrics>,
}

#[derive(Debug)]
pub enum LedgerError {
    Database(rusqlite::Error),
    Validation(ValidationError),
}
impl fmt::Display for LedgerError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => output.write_str("HookStat ledger operation failed"),
            Self::Validation(error) => error.fmt(output),
        }
    }
}
impl std::error::Error for LedgerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Validation(error) => Some(error),
        }
    }
}
impl From<rusqlite::Error> for LedgerError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value)
    }
}
impl From<ValidationError> for LedgerError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl Ledger {
    fn canonical_taxonomy_predicate(&self) -> &'static str {
        if self.has_evidence_generation {
            CANONICAL_TAXONOMY_WITH_GENERATION
        } else {
            CANONICAL_TAXONOMY_WITHOUT_GENERATION
        }
    }

    /// Callers select a HookStat-owned user-data location. This never derives,
    /// locks, or writes a Codex-owned path.
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, LedgerError> {
        Self::from_connection(Connection::open(path)?)
    }
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Opens an existing ledger without running migrations or creating files.
    /// Diagnostics uses this path so inspecting health cannot mutate state.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, LedgerError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let has_evidence_generation = invocation_column_exists(&connection, "evidence_generation")?;
        Ok(Self {
            connection,
            has_evidence_generation,
        })
    }

    fn from_connection(connection: Connection) -> Result<Self, LedgerError> {
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS hookstat_schema (version INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS hook_invocations (
                source_key TEXT NOT NULL, source_record_id TEXT NOT NULL,
                runtime TEXT NOT NULL, evidence_kind TEXT NOT NULL,
                evidence_generation TEXT NOT NULL DEFAULT 'legacy_v03_proxy', coverage TEXT NOT NULL,
                handler_key TEXT NOT NULL, handler_revision TEXT NOT NULL DEFAULT 'legacy',
                handler_label TEXT NOT NULL, handler_source_kind TEXT NOT NULL DEFAULT 'legacy',
                handler_event TEXT NOT NULL, handler_matcher_identity TEXT NOT NULL DEFAULT 'legacy',
                handler_structural_identity TEXT NOT NULL DEFAULT 'legacy', handler_execution_mode TEXT NOT NULL DEFAULT 'sync',
                occurred_at_unix_ms INTEGER NOT NULL, terminal_status TEXT NOT NULL,
                duration_ms INTEGER, error_fingerprint TEXT,
                PRIMARY KEY (source_key, source_record_id)
            );
            CREATE INDEX IF NOT EXISTS hook_invocations_handler_time ON hook_invocations (handler_key, occurred_at_unix_ms);
            CREATE INDEX IF NOT EXISTS hook_invocations_time ON hook_invocations (occurred_at_unix_ms, handler_key);
            CREATE TABLE IF NOT EXISTS source_cursors (source_key TEXT PRIMARY KEY, cursor TEXT NOT NULL, updated_at_unix_ms INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS handler_catalog (
                runtime TEXT NOT NULL,
                handler_key TEXT NOT NULL,
                latest_revision TEXT NOT NULL,
                explicit_name TEXT,
                script_filename TEXT,
                command_basename TEXT,
                source_label_key TEXT NOT NULL,
                resolver_version INTEGER NOT NULL,
                observed_at_unix_ms INTEGER NOT NULL,
                PRIMARY KEY (runtime, handler_key)
            );
            CREATE TABLE IF NOT EXISTS handler_annotations (
                runtime TEXT NOT NULL,
                handler_key TEXT NOT NULL,
                display_name TEXT NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL,
                PRIMARY KEY (runtime, handler_key)
            );
            CREATE TABLE IF NOT EXISTS receipt_reconciliation (
                source_key TEXT PRIMARY KEY,
                journal_offset INTEGER NOT NULL,
                malformed_receipts INTEGER NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL,
                catalog_version INTEGER NOT NULL DEFAULT 0,
                catalog_record_count INTEGER,
                catalog_incomplete INTEGER,
                catalog_latest_occurred_at_unix_ms INTEGER
            );
            CREATE TABLE IF NOT EXISTS hookstat_migration_issues (
                issue_kind TEXT PRIMARY KEY,
                rows_affected INTEGER NOT NULL
            );
            ",
        )?;
        // Additive migrations for earlier ledgers. Column presence is checked
        // explicitly so a real SQLite failure is never mistaken for the
        // harmless already-migrated case.
        for (column, statement) in [
            (
                "handler_revision",
                "ALTER TABLE hook_invocations ADD COLUMN handler_revision TEXT NOT NULL DEFAULT 'legacy'",
            ),
            (
                "handler_source_kind",
                "ALTER TABLE hook_invocations ADD COLUMN handler_source_kind TEXT NOT NULL DEFAULT 'legacy'",
            ),
            (
                "handler_matcher_identity",
                "ALTER TABLE hook_invocations ADD COLUMN handler_matcher_identity TEXT NOT NULL DEFAULT 'legacy'",
            ),
            (
                "handler_structural_identity",
                "ALTER TABLE hook_invocations ADD COLUMN handler_structural_identity TEXT NOT NULL DEFAULT 'legacy'",
            ),
            (
                "handler_execution_mode",
                "ALTER TABLE hook_invocations ADD COLUMN handler_execution_mode TEXT NOT NULL DEFAULT 'sync'",
            ),
            (
                "evidence_generation",
                "ALTER TABLE hook_invocations ADD COLUMN evidence_generation TEXT NOT NULL DEFAULT 'legacy_v03_proxy'",
            ),
        ] {
            if !invocation_column_exists(&connection, column)? {
                connection.execute(statement, [])?;
            }
        }
        for (column, statement) in [
            (
                "catalog_version",
                "ALTER TABLE receipt_reconciliation ADD COLUMN catalog_version INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "catalog_record_count",
                "ALTER TABLE receipt_reconciliation ADD COLUMN catalog_record_count INTEGER",
            ),
            (
                "catalog_incomplete",
                "ALTER TABLE receipt_reconciliation ADD COLUMN catalog_incomplete INTEGER",
            ),
            (
                "catalog_latest_occurred_at_unix_ms",
                "ALTER TABLE receipt_reconciliation ADD COLUMN catalog_latest_occurred_at_unix_ms INTEGER",
            ),
        ] {
            if !table_column_exists(&connection, "receipt_reconciliation", column)? {
                connection.execute(statement, [])?;
            }
        }
        connection.execute(
            "DELETE FROM hookstat_migration_issues WHERE issue_kind = 'invalid_legacy_taxonomy'",
            [],
        )?;
        connection.execute(
            "INSERT INTO hookstat_migration_issues (issue_kind, rows_affected)
             SELECT 'invalid_legacy_taxonomy', count(*) FROM hook_invocations
             WHERE runtime NOT IN ('codex', 'deepseek_harness', 'opencode')
                OR evidence_kind NOT IN ('codex_session_jsonl', 'codex_state_database', 'codex_app_server_live', 'codex_instrumented_receipt', 'runtime_neutral_ipc', 'open_telemetry', 'synthetic_fixture')
                OR evidence_generation NOT IN ('legacy_v03_proxy', 'v031_native', 'v031_cooperative_ipc', 'synthetic_fixture')
                OR coverage NOT IN ('complete', 'partial', 'sync_only', 'best_effort', 'unknown', 'not_admitted', 'synthetic_fixture')
                OR handler_event NOT IN ('session_start', 'session_end', 'user_prompt_submit', 'pre_tool_use', 'post_tool_use', 'permission_request', 'pre_compact', 'post_compact', 'stop', 'subagent_start', 'subagent_stop')
                OR handler_execution_mode NOT IN ('sync', 'async', 'unknown')
                OR terminal_status NOT IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure', 'incomplete', 'unknown')
             HAVING count(*) > 0
             ON CONFLICT(issue_kind) DO UPDATE SET rows_affected = excluded.rows_affected",
            [],
        )?;
        // Publish the new schema version only after every additive migration
        // and validation-index step has completed. A partial open can safely
        // retry without claiming a schema it did not finish.
        connection.execute_batch(
            "DELETE FROM hookstat_schema;
             INSERT INTO hookstat_schema (version) VALUES (5);",
        )?;
        Ok(Self {
            connection,
            has_evidence_generation: true,
        })
    }

    /// A duplicate is harmless. A later lifecycle result may refine an
    /// `incomplete` or `best_effort` record for the exact same receipt id.
    /// A correlator-proven conflict is the sole conservative downgrade: it
    /// replaces a terminal row with `Unknown` so a disproven result leaves the
    /// reliability denominator without a schema migration.
    pub fn ingest(&mut self, values: &[HookInvocation]) -> Result<IngestReceipt, LedgerError> {
        for value in values {
            value.validate()?;
        }
        let transaction = self.connection.transaction()?;
        let receipt = Self::ingest_transaction(&transaction, values)?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Atomically accepts a batch of receipt evidence and advances the durable
    /// journal cursor. A process interruption leaves the prior cursor intact,
    /// which makes replay safe through the normal duplicate/upgrading ingest
    /// semantics rather than silently skipping evidence.
    pub fn ingest_receipt_reconciliation(
        &mut self,
        source_key: &str,
        receipt_source_key: &str,
        journal_offset: u64,
        malformed_receipts: u64,
        updated_at_unix_ms: i64,
        values: &[HookInvocation],
    ) -> Result<IngestReceipt, LedgerError> {
        if source_key.trim().is_empty() || source_key.len() > 256 {
            return Err(ValidationError::new("source_key").into());
        }
        for value in values {
            value.validate()?;
        }
        let transaction = self.connection.transaction()?;
        let receipt = Self::ingest_transaction(&transaction, values)?;
        // This aggregation runs only while reconciliation is already mutating
        // the catalog. The resulting fixed facts make routine diagnostics a
        // single-row lookup rather than an unbounded historical scan.
        let (record_count, incomplete, latest_occurred_at_unix_ms): (i64, i64, Option<i64>) =
            transaction.query_row(
                "SELECT count(*),
                        COALESCE(SUM(CASE WHEN terminal_status = 'incomplete' THEN 1 ELSE 0 END), 0),
                        MAX(occurred_at_unix_ms)
                 FROM hook_invocations WHERE source_key = ?1",
                [receipt_source_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
        transaction.execute(
            "INSERT INTO receipt_reconciliation (
                source_key, journal_offset, malformed_receipts, updated_at_unix_ms,
                catalog_version, catalog_record_count, catalog_incomplete,
                catalog_latest_occurred_at_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7)
             ON CONFLICT(source_key) DO UPDATE SET
                journal_offset = excluded.journal_offset,
                malformed_receipts = excluded.malformed_receipts,
                updated_at_unix_ms = excluded.updated_at_unix_ms,
                catalog_version = excluded.catalog_version,
                catalog_record_count = excluded.catalog_record_count,
                catalog_incomplete = excluded.catalog_incomplete,
                catalog_latest_occurred_at_unix_ms = excluded.catalog_latest_occurred_at_unix_ms",
            params![
                source_key,
                i64::try_from(journal_offset)
                    .map_err(|_| ValidationError::new("journal_offset"))?,
                i64::try_from(malformed_receipts)
                    .map_err(|_| ValidationError::new("malformed_receipts"))?,
                updated_at_unix_ms,
                record_count,
                incomplete,
                latest_occurred_at_unix_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub fn receipt_reconciliation_state(
        &self,
        source_key: &str,
    ) -> Result<Option<ReceiptReconciliationState>, LedgerError> {
        self.connection
            .query_row(
                "SELECT journal_offset, malformed_receipts
                 FROM receipt_reconciliation WHERE source_key = ?1",
                [source_key],
                |row| {
                    let offset: i64 = row.get(0)?;
                    let malformed: i64 = row.get(1)?;
                    Ok(ReceiptReconciliationState {
                        journal_offset: u64::try_from(offset)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        malformed_receipts: u64::try_from(malformed)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Read-only compatibility helper for v0.2 ledgers that predate the
    /// receipt catalog. Absence is reported as unobserved rather than being
    /// fabricated into a clean zero-integrity claim.
    pub fn receipt_reconciliation_state_if_present(
        &self,
        source_key: &str,
    ) -> Result<Option<ReceiptReconciliationState>, LedgerError> {
        let present: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'receipt_reconciliation')",
            [],
            |row| row.get(0),
        )?;
        if present {
            self.receipt_reconciliation_state(source_key)
        } else {
            Ok(None)
        }
    }

    pub fn incomplete_receipt_count(&self) -> Result<u64, LedgerError> {
        let sql = format!(
            "SELECT count(*) FROM hook_invocations
             WHERE {}
               AND source_key = 'codex_instrumented_receipts_v1'
               AND terminal_status = 'incomplete'",
            self.canonical_taxonomy_predicate()
        );
        self.connection
            .query_row(&sql, [], |row| row.get::<_, i64>(0))
            .map(|value| value as u64)
            .map_err(Into::into)
    }

    /// Returns the bounded receipt catalog facts when the ledger has a
    /// reconciliation cursor. A missing catalog remains unobserved rather
    /// than being fabricated into a zero-integrity result.
    pub(crate) fn receipt_catalog_diagnostics_if_present(
        &self,
        reconciliation_source_key: &str,
    ) -> Result<Option<ReceiptCatalogDiagnostics>, LedgerError> {
        if !table_column_exists(
            &self.connection,
            "receipt_reconciliation",
            "catalog_version",
        )? {
            return Ok(None);
        }
        self.connection
            .query_row(
                "SELECT journal_offset, catalog_record_count, catalog_incomplete,
                        malformed_receipts, catalog_latest_occurred_at_unix_ms
                 FROM receipt_reconciliation
                 WHERE source_key = ?1 AND catalog_version = 1",
                [reconciliation_source_key],
                |row| {
                    let offset: i64 = row.get(0)?;
                    let record_count: i64 = row.get(1)?;
                    let incomplete: i64 = row.get(2)?;
                    let malformed: i64 = row.get(3)?;
                    Ok(ReceiptCatalogDiagnostics {
                        journal_offset: u64::try_from(offset)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        record_count: u64::try_from(record_count)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        incomplete: u64::try_from(incomplete)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        malformed: u64::try_from(malformed)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        latest_occurred_at_unix_ms: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Reads the bounded working set needed for every finite reliability view.
    /// The maximum rolling intelligence comparison is 30d + its predecessor,
    /// therefore finite requests materialize at most the most recent 60d.
    /// `All` remains the explicit full-history mode.
    pub fn invocations_for_reliability(
        &self,
        now_unix_ms: i64,
        selected_window: TimeWindow,
    ) -> Result<LedgerQuery, LedgerError> {
        let bounds = match selected_window {
            TimeWindow::All => TimeWindow::All.bounds_at(now_unix_ms),
            TimeWindow::Today
            | TimeWindow::Last24Hours
            | TimeWindow::Last7Days
            | TimeWindow::Last30Days => {
                let widest = TimeWindow::Last30Days.bounds_at(now_unix_ms);
                TimeBounds {
                    current_start_unix_ms: widest.previous_start_unix_ms,
                    current_end_unix_ms: now_unix_ms,
                    previous_start_unix_ms: None,
                    previous_end_unix_ms: None,
                }
            }
        };
        let invocations = self.invocations_in_bounds(bounds)?;
        Ok(LedgerQuery {
            rows_materialized: invocations.len() as u64,
            invocations,
            bounds,
        })
    }

    /// Reads the canonical historical snapshot for the lazy Changes
    /// workbench. This is intentionally separate from the bounded normal
    /// reliability query: navigating to Changes may request long history, but
    /// a period switch on Overview/Hooks must never do so implicitly.
    pub fn invocations_for_workbench(&self, now_unix_ms: i64) -> Result<LedgerQuery, LedgerError> {
        let bounds = TimeBounds {
            current_start_unix_ms: None,
            current_end_unix_ms: now_unix_ms,
            previous_start_unix_ms: None,
            previous_end_unix_ms: None,
        };
        let invocations = self.invocations_in_bounds(bounds)?;
        Ok(LedgerQuery {
            rows_materialized: invocations.len() as u64,
            invocations,
            bounds,
        })
    }

    /// Database-side `All` aggregates for the visible finite-window handlers.
    /// This returns one compact row per handler rather than historical
    /// invocations, preserving the released All-time trend semantics.
    pub fn all_time_period_metrics(
        &self,
        now_unix_ms: i64,
    ) -> Result<BTreeMap<String, PeriodMetrics>, LedgerError> {
        let sql = format!(
            "SELECT handler_key, count(*),
                    COALESCE(SUM(CASE WHEN terminal_status IN
                        ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure')
                        THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN terminal_status IN
                        ('failed', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0)
             FROM hook_invocations
             WHERE {}
               AND occurred_at_unix_ms <= ?1
            GROUP BY handler_key",
            self.canonical_taxonomy_predicate()
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map([now_unix_ms], |row| {
            let metrics = period_metrics_from_counts(row.get(1)?, row.get(2)?, row.get(3)?)?;
            Ok((row.get::<_, String>(0)?, metrics))
        })?;
        rows.collect::<Result<BTreeMap<_, _>, _>>()
            .map_err(Into::into)
    }

    /// Returns the current and immediately preceding contiguous revision epoch
    /// through indexed timeline boundary lookups and SQL aggregates. It never
    /// materializes the handler's historical invocation rows in Rust.
    pub fn revision_epoch_metrics(
        &self,
        handler_keys: &[String],
    ) -> Result<BTreeMap<String, RevisionEpochMetrics>, LedgerError> {
        let mut result = BTreeMap::new();
        for key in handler_keys {
            let Some(latest) = self.latest_timeline_point(key)? else {
                continue;
            };
            let current_boundary = self.last_different_revision(key, &latest.revision, None)?;
            let current_metrics = self.metrics_after(key, current_boundary.as_ref())?;
            let current = RevisionMetrics {
                revision: latest.revision.clone(),
                runs: current_metrics.runs,
                failure_sample_count: current_metrics.failure_sample_count,
                failed_runs: current_metrics.failed_runs,
                failure_rate_percent: current_metrics.failure_rate_percent,
            };
            let previous = if let Some(current_boundary) = current_boundary {
                let prior_boundary = self.last_different_revision(
                    key,
                    &current_boundary.revision,
                    Some(&current_boundary),
                )?;
                let previous_metrics =
                    self.metrics_between(key, prior_boundary.as_ref(), &current_boundary)?;
                Some(RevisionMetrics {
                    revision: current_boundary.revision,
                    runs: previous_metrics.runs,
                    failure_sample_count: previous_metrics.failure_sample_count,
                    failed_runs: previous_metrics.failed_runs,
                    failure_rate_percent: previous_metrics.failure_rate_percent,
                })
            } else {
                None
            };
            result.insert(key.clone(), RevisionEpochMetrics { current, previous });
        }
        Ok(result)
    }

    fn ingest_transaction(
        transaction: &rusqlite::Transaction<'_>,
        values: &[HookInvocation],
    ) -> Result<IngestReceipt, LedgerError> {
        let mut receipt = IngestReceipt::default();
        for value in values {
            let prior: Option<String> = transaction.query_row("SELECT terminal_status FROM hook_invocations WHERE source_key = ?1 AND source_record_id = ?2", [&value.source_key, &value.source_record_id], |row| row.get(0)).optional()?;
            let changed = transaction.execute(
                "INSERT INTO hook_invocations (
                    source_key, source_record_id, runtime, evidence_kind, evidence_generation, coverage,
                    handler_key, handler_revision, handler_label, handler_source_kind, handler_event,
                    handler_matcher_identity, handler_structural_identity, handler_execution_mode,
                    occurred_at_unix_ms, terminal_status, duration_ms, error_fingerprint
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                ON CONFLICT(source_key, source_record_id) DO UPDATE SET
                    coverage = excluded.coverage, handler_revision = excluded.handler_revision,
                    handler_label = excluded.handler_label, handler_source_kind = excluded.handler_source_kind,
                    handler_matcher_identity = excluded.handler_matcher_identity,
                    handler_structural_identity = excluded.handler_structural_identity,
                    handler_execution_mode = excluded.handler_execution_mode, terminal_status = excluded.terminal_status,
                    occurred_at_unix_ms = excluded.occurred_at_unix_ms,
                    duration_ms = excluded.duration_ms, error_fingerprint = excluded.error_fingerprint
                WHERE (hook_invocations.terminal_status = 'incomplete' AND excluded.terminal_status != 'incomplete')
                   OR (hook_invocations.coverage = 'best_effort' AND excluded.coverage != 'best_effort')
                   OR (excluded.error_fingerprint = ?19
                       AND excluded.terminal_status = 'unknown'
                       AND excluded.coverage = 'unknown')",
                params![
                    &value.source_key, &value.source_record_id, value.runtime.as_storage(), value.evidence_kind.as_storage(), value.evidence_generation.as_storage(), value.coverage.as_storage(),
                    &value.handler.key, &value.handler.revision, &value.handler.label, &value.handler.source_kind, value.handler.event.as_storage(),
                    &value.handler.matcher_identity, &value.handler.structural_identity, value.handler.execution_mode.as_storage(),
                    value.occurred_at_unix_ms, value.terminal_status.as_storage(), value.duration_ms.map(|duration| duration as i64), &value.error_fingerprint,
                    CORRELATION_CONFLICT_FINGERPRINT,
                ],
            )?;
            if changed == 0 {
                receipt.duplicates += 1;
            } else if prior.is_some() {
                receipt.upgraded += 1;
            } else {
                receipt.inserted += 1;
            }
            let safe_label = (!generated_label(&value.handler.label))
                .then(|| sanitize_display_name(&value.handler.label))
                .flatten();
            transaction.execute(
                "INSERT INTO handler_catalog (
                    runtime, handler_key, latest_revision, explicit_name,
                    script_filename, command_basename, source_label_key,
                    resolver_version, observed_at_unix_ms
                ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, 1, ?6)
                ON CONFLICT(runtime, handler_key) DO UPDATE SET
                    latest_revision = excluded.latest_revision,
                    explicit_name = excluded.explicit_name,
                    source_label_key = excluded.source_label_key,
                    resolver_version = excluded.resolver_version,
                    observed_at_unix_ms = excluded.observed_at_unix_ms",
                params![
                    value.runtime.as_storage(),
                    &value.handler.key,
                    &value.handler.revision,
                    safe_label,
                    &value.handler.source_kind,
                    value.occurred_at_unix_ms,
                ],
            )?;
        }
        Ok(receipt)
    }

    pub fn advance_cursor(
        &mut self,
        source_key: &str,
        cursor: &str,
        updated_at_unix_ms: i64,
    ) -> Result<(), LedgerError> {
        if source_key.trim().is_empty() || source_key.len() > 256 {
            return Err(ValidationError::new("source_key").into());
        }
        if cursor.is_empty() || cursor.len() > 256 {
            return Err(ValidationError::new("cursor").into());
        }
        self.connection.execute("INSERT INTO source_cursors (source_key, cursor, updated_at_unix_ms) VALUES (?1, ?2, ?3) ON CONFLICT(source_key) DO UPDATE SET cursor = excluded.cursor, updated_at_unix_ms = excluded.updated_at_unix_ms", params![source_key, cursor, updated_at_unix_ms])?;
        Ok(())
    }
    pub fn cursor(&self, source_key: &str) -> Result<Option<String>, LedgerError> {
        self.connection
            .query_row(
                "SELECT cursor FROM source_cursors WHERE source_key = ?1",
                [source_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }
    pub fn invocation_count(&self) -> Result<u64, LedgerError> {
        let sql = format!(
            "SELECT count(*) FROM hook_invocations WHERE {}",
            self.canonical_taxonomy_predicate()
        );
        self.connection
            .query_row(&sql, [], |row| row.get::<_, i64>(0))
            .map(|count| count as u64)
            .map_err(Into::into)
    }

    /// Sanitized count of preserved legacy rows whose taxonomy cannot be
    /// interpreted by the current release. The rows themselves remain
    /// untouched and never masquerade as v0.3.1 Native or IPC evidence.
    pub fn migration_issue_count(&self) -> Result<u64, LedgerError> {
        self.connection
            .query_row(
                "SELECT COALESCE(SUM(rows_affected), 0) FROM hookstat_migration_issues",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u64)
            .map_err(Into::into)
    }

    /// Stores an explicit, bounded user alias in HookStat-owned state. It has
    /// no effect on invocation attribution, proxy routing, or trust.
    pub fn set_handler_alias(
        &mut self,
        runtime: Runtime,
        handler_key: &str,
        display_name: &str,
        updated_at_unix_ms: i64,
    ) -> Result<(), LedgerError> {
        if handler_key.trim().is_empty() || handler_key.len() > 128 {
            return Err(ValidationError::new("handler_key").into());
        }
        let Some(display_name) = sanitize_display_name(display_name) else {
            return Err(ValidationError::new("handler_alias").into());
        };
        self.connection.execute(
            "INSERT INTO handler_annotations (runtime, handler_key, display_name, updated_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(runtime, handler_key) DO UPDATE SET
                display_name = excluded.display_name,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![runtime.as_storage(), handler_key, display_name, updated_at_unix_ms],
        )?;
        Ok(())
    }

    /// Applies a presentation-only alias only when the stored alias still
    /// equals the snapshot that the editor opened. This protects a Human
    /// draft from overwriting another local HookStat presentation edit. It
    /// never touches Codex configuration or canonical invocation evidence.
    pub fn set_handler_alias_if_unchanged(
        &mut self,
        runtime: Runtime,
        handler_key: &str,
        display_name: &str,
        expected_alias: Option<&str>,
        updated_at_unix_ms: i64,
    ) -> Result<AliasSaveOutcome, LedgerError> {
        if handler_key.trim().is_empty() || handler_key.len() > 128 {
            return Err(ValidationError::new("handler_key").into());
        }
        // `sanitize_display_name` compactly normalizes imported metadata.
        // Interactive aliases have a stricter contract: control text is an
        // invalid draft, never an invisible rewrite of what the Human typed.
        if display_name.chars().any(char::is_control) {
            return Err(ValidationError::new("handler_alias").into());
        }
        let Some(display_name) = sanitize_display_name(display_name) else {
            return Err(ValidationError::new("handler_alias").into());
        };
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let stored = transaction
            .query_row(
                "SELECT display_name FROM handler_annotations WHERE runtime = ?1 AND handler_key = ?2",
                params![runtime.as_storage(), handler_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if stored.as_deref() != expected_alias {
            return Ok(AliasSaveOutcome::Conflict);
        }
        transaction.execute(
            "INSERT INTO handler_annotations (runtime, handler_key, display_name, updated_at_unix_ms)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(runtime, handler_key) DO UPDATE SET
                display_name = excluded.display_name,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![runtime.as_storage(), handler_key, display_name, updated_at_unix_ms],
        )?;
        transaction.commit()?;
        Ok(AliasSaveOutcome::Saved)
    }

    /// Returns only the sanitized user-facing aliases, never raw commands or
    /// private configuration material.
    pub fn handler_aliases(&self) -> Result<Vec<HandlerAlias>, LedgerError> {
        let mut statement = self.connection.prepare(
            "SELECT runtime, handler_key, display_name
             FROM handler_annotations ORDER BY runtime, handler_key",
        )?;
        let rows = statement.query_map([], |row| {
            let invalid = || rusqlite::Error::InvalidQuery;
            let runtime = Runtime::from_storage(&row.get::<_, String>(0)?).ok_or_else(invalid)?;
            Ok(HandlerAlias {
                runtime,
                handler_key: row.get(1)?,
                display_name: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Reads aliases without assuming that a pre-v0.2 ledger has received the
    /// additive annotations-table migration. This is intentionally useful to
    /// read-only consumers: an absent optional presentation table is not a
    /// reason to migrate or reject valid historical invocation evidence.
    pub fn handler_aliases_if_present(&self) -> Result<Vec<HandlerAlias>, LedgerError> {
        let present: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'handler_annotations')",
            [],
            |row| row.get(0),
        )?;
        if present {
            self.handler_aliases()
        } else {
            Ok(Vec::new())
        }
    }

    pub fn invocations(&self) -> Result<Vec<HookInvocation>, LedgerError> {
        self.invocations_in_bounds(TimeBounds {
            current_start_unix_ms: None,
            current_end_unix_ms: i64::MAX,
            previous_start_unix_ms: None,
            previous_end_unix_ms: None,
        })
    }

    fn invocations_in_bounds(
        &self,
        bounds: TimeBounds,
    ) -> Result<Vec<HookInvocation>, LedgerError> {
        let generation = if self.has_evidence_generation {
            "evidence_generation"
        } else {
            "'legacy_v03_proxy'"
        };
        let canonical = self.canonical_taxonomy_predicate();
        let (sql, parameters): (String, Vec<i64>) = match bounds.current_start_unix_ms {
            Some(start) => (
                format!("SELECT source_key, source_record_id, runtime, evidence_kind, {generation}, coverage, handler_key, handler_revision,
                        handler_label, handler_source_kind, handler_event, handler_matcher_identity,
                        handler_structural_identity, handler_execution_mode, occurred_at_unix_ms, terminal_status,
                        duration_ms, error_fingerprint FROM hook_invocations
                 WHERE {canonical}
                   AND occurred_at_unix_ms >= ?1 AND occurred_at_unix_ms <= ?2
                 ORDER BY occurred_at_unix_ms, source_key, source_record_id"),
                vec![start, bounds.current_end_unix_ms],
            ),
            None => (
                format!("SELECT source_key, source_record_id, runtime, evidence_kind, {generation}, coverage, handler_key, handler_revision,
                        handler_label, handler_source_kind, handler_event, handler_matcher_identity,
                        handler_structural_identity, handler_execution_mode, occurred_at_unix_ms, terminal_status,
                        duration_ms, error_fingerprint FROM hook_invocations
                 WHERE {canonical}
                   AND occurred_at_unix_ms <= ?1
                 ORDER BY occurred_at_unix_ms, source_key, source_record_id"),
                vec![bounds.current_end_unix_ms],
            ),
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(parameters), |row| {
            let invalid = || rusqlite::Error::InvalidQuery;
            let runtime = Runtime::from_storage(&row.get::<_, String>(2)?).ok_or_else(invalid)?;
            let evidence_kind =
                EvidenceKind::from_storage(&row.get::<_, String>(3)?).ok_or_else(invalid)?;
            let evidence_generation =
                EvidenceGeneration::from_storage(&row.get::<_, String>(4)?).ok_or_else(invalid)?;
            let coverage =
                EvidenceCoverage::from_storage(&row.get::<_, String>(5)?).ok_or_else(invalid)?;
            let event = HookEvent::from_storage(&row.get::<_, String>(10)?).ok_or_else(invalid)?;
            let execution_mode =
                ExecutionMode::from_storage(&row.get::<_, String>(13)?).ok_or_else(invalid)?;
            let terminal_status =
                TerminalStatus::from_storage(&row.get::<_, String>(15)?).ok_or_else(invalid)?;
            let duration: Option<i64> = row.get(16)?;
            Ok(HookInvocation {
                source_key: row.get(0)?,
                source_record_id: row.get(1)?,
                runtime,
                evidence_kind,
                evidence_generation,
                coverage,
                handler: HandlerIdentity {
                    key: row.get(6)?,
                    revision: row.get(7)?,
                    label: row.get(8)?,
                    source_kind: row.get(9)?,
                    event,
                    matcher_identity: row.get(11)?,
                    structural_identity: row.get(12)?,
                    execution_mode,
                },
                occurred_at_unix_ms: row.get(14)?,
                terminal_status,
                duration_ms: duration.map(|value| value as u64),
                error_fingerprint: row.get(17)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn latest_timeline_point(
        &self,
        handler_key: &str,
    ) -> Result<Option<TimelinePoint>, LedgerError> {
        let sql = format!(
            "SELECT occurred_at_unix_ms, source_key, source_record_id, handler_revision
             FROM hook_invocations WHERE {}
               AND handler_key = ?1
             ORDER BY occurred_at_unix_ms DESC, source_key DESC, source_record_id DESC LIMIT 1",
            self.canonical_taxonomy_predicate()
        );
        self.connection
            .query_row(&sql, [handler_key], timeline_point_from_row)
            .optional()
            .map_err(Into::into)
    }

    fn last_different_revision(
        &self,
        handler_key: &str,
        revision: &str,
        before: Option<&TimelinePoint>,
    ) -> Result<Option<TimelinePoint>, LedgerError> {
        let value = match before {
            Some(before) => {
                let sql = format!(
                    "SELECT occurred_at_unix_ms, source_key, source_record_id, handler_revision
                 FROM hook_invocations
                 WHERE {}
                   AND handler_key = ?1 AND handler_revision != ?2
                   AND (occurred_at_unix_ms < ?3
                        OR (occurred_at_unix_ms = ?3 AND (source_key < ?4
                            OR (source_key = ?4 AND source_record_id < ?5))))
                 ORDER BY occurred_at_unix_ms DESC, source_key DESC, source_record_id DESC LIMIT 1",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(
                        &sql,
                        params![
                            handler_key,
                            revision,
                            before.occurred_at_unix_ms,
                            &before.source_key,
                            &before.source_record_id
                        ],
                        timeline_point_from_row,
                    )
                    .optional()?
            }
            None => {
                let sql = format!(
                    "SELECT occurred_at_unix_ms, source_key, source_record_id, handler_revision
                 FROM hook_invocations WHERE {}
                   AND handler_key = ?1 AND handler_revision != ?2
                 ORDER BY occurred_at_unix_ms DESC, source_key DESC, source_record_id DESC LIMIT 1",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(
                        &sql,
                        params![handler_key, revision],
                        timeline_point_from_row,
                    )
                    .optional()?
            }
        };
        Ok(value)
    }

    fn metrics_after(
        &self,
        handler_key: &str,
        boundary: Option<&TimelinePoint>,
    ) -> Result<PeriodMetrics, LedgerError> {
        match boundary {
            Some(boundary) => {
                let sql = format!(
                    "SELECT count(*),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('failed', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0)
                 FROM hook_invocations WHERE {}
                   AND handler_key = ?1
                   AND (occurred_at_unix_ms > ?2
                        OR (occurred_at_unix_ms = ?2 AND (source_key > ?3
                            OR (source_key = ?3 AND source_record_id > ?4))))",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(
                        &sql,
                        params![
                            handler_key,
                            boundary.occurred_at_unix_ms,
                            &boundary.source_key,
                            &boundary.source_record_id
                        ],
                        period_metrics_from_row,
                    )
                    .map_err(Into::into)
            }
            None => {
                let sql = format!(
                    "SELECT count(*),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('failed', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0)
                 FROM hook_invocations WHERE {}
                   AND handler_key = ?1",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(&sql, [handler_key], period_metrics_from_row)
                    .map_err(Into::into)
            }
        }
    }

    fn metrics_between(
        &self,
        handler_key: &str,
        lower: Option<&TimelinePoint>,
        upper: &TimelinePoint,
    ) -> Result<PeriodMetrics, LedgerError> {
        match lower {
            Some(lower) => {
                let sql = format!(
                    "SELECT count(*),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('failed', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0)
                 FROM hook_invocations WHERE {}
                   AND handler_key = ?1
                   AND (occurred_at_unix_ms > ?2
                        OR (occurred_at_unix_ms = ?2 AND (source_key > ?3
                            OR (source_key = ?3 AND source_record_id > ?4))))
                   AND (occurred_at_unix_ms < ?5
                        OR (occurred_at_unix_ms = ?5 AND (source_key < ?6
                            OR (source_key = ?6 AND source_record_id <= ?7))))",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(
                        &sql,
                        params![
                            handler_key,
                            lower.occurred_at_unix_ms,
                            &lower.source_key,
                            &lower.source_record_id,
                            upper.occurred_at_unix_ms,
                            &upper.source_key,
                            &upper.source_record_id
                        ],
                        period_metrics_from_row,
                    )
                    .map_err(Into::into)
            }
            None => {
                let sql = format!(
                    "SELECT count(*),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('completed', 'failed', 'blocked', 'stopped', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0),
                        COALESCE(SUM(CASE WHEN terminal_status IN ('failed', 'timed_out', 'protocol_failure') THEN 1 ELSE 0 END), 0)
                 FROM hook_invocations WHERE {}
                   AND handler_key = ?1
                   AND (occurred_at_unix_ms < ?2
                        OR (occurred_at_unix_ms = ?2 AND (source_key < ?3
                            OR (source_key = ?3 AND source_record_id <= ?4))))",
                    self.canonical_taxonomy_predicate()
                );
                self.connection
                    .query_row(
                        &sql,
                        params![
                            handler_key,
                            upper.occurred_at_unix_ms,
                            &upper.source_key,
                            &upper.source_record_id
                        ],
                        period_metrics_from_row,
                    )
                    .map_err(Into::into)
            }
        }
    }

    #[cfg(test)]
    fn invocation_columns(&self) -> Result<Vec<String>, LedgerError> {
        let mut statement = self
            .connection
            .prepare("PRAGMA table_info(hook_invocations)")?;
        statement
            .query_map([], |row| row.get(1))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn table_exists(&self, table: &str) -> Result<bool, LedgerError> {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}

fn invocation_column_exists(connection: &Connection, column: &str) -> Result<bool, LedgerError> {
    table_column_exists(connection, "hook_invocations", column)
}

fn table_column_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, LedgerError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for value in columns {
        if value? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Clone, Debug)]
struct TimelinePoint {
    occurred_at_unix_ms: i64,
    source_key: String,
    source_record_id: String,
    revision: String,
}

fn timeline_point_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimelinePoint> {
    Ok(TimelinePoint {
        occurred_at_unix_ms: row.get(0)?,
        source_key: row.get(1)?,
        source_record_id: row.get(2)?,
        revision: row.get(3)?,
    })
}

fn period_metrics_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PeriodMetrics> {
    period_metrics_from_counts(row.get(0)?, row.get(1)?, row.get(2)?)
}

fn period_metrics_from_counts(
    runs: i64,
    terminal_samples: i64,
    failures: i64,
) -> rusqlite::Result<PeriodMetrics> {
    let runs = u64::try_from(runs).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let failure_sample_count =
        u64::try_from(terminal_samples).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let failed_runs = u64::try_from(failures).map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(PeriodMetrics {
        runs,
        failure_sample_count,
        failed_runs,
        failure_rate_percent: if failure_sample_count == 0 {
            0.0
        } else {
            failed_runs as f64 * 100.0 / failure_sample_count as f64
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(id: &str) -> HookInvocation {
        HookInvocation {
            source_key: "fixture-source".into(),
            source_record_id: id.into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            evidence_generation: EvidenceGeneration::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: HandlerIdentity {
                key: "fixture-handler".into(),
                revision: "fixture-revision".into(),
                label: "fixture handler".into(),
                source_kind: "fixture".into(),
                event: HookEvent::Stop,
                matcher_identity: "any".into(),
                structural_identity: "g0:h0".into(),
                execution_mode: ExecutionMode::Sync,
            },
            occurred_at_unix_ms: 1_000,
            terminal_status: TerminalStatus::Completed,
            duration_ms: None,
            error_fingerprint: None,
        }
    }
    #[test]
    fn repeated_ingest_is_idempotent_and_incremental() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        assert_eq!(ledger.ingest(&[fixture("one")]).unwrap().inserted, 1);
        assert_eq!(ledger.ingest(&[fixture("one")]).unwrap().duplicates, 1);
        assert_eq!(ledger.ingest(&[fixture("two")]).unwrap().inserted, 1);
        assert_eq!(ledger.invocation_count().unwrap(), 2);
    }
    #[test]
    fn incomplete_is_upgraded_once_by_completion() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut start = fixture("one");
        start.terminal_status = TerminalStatus::Incomplete;
        ledger.ingest(&[start]).unwrap();
        let receipt = ledger.ingest(&[fixture("one")]).unwrap();
        assert_eq!(receipt.upgraded, 1);
        assert_eq!(
            ledger.invocations().unwrap()[0].terminal_status,
            TerminalStatus::Completed
        );
    }
    #[test]
    fn malformed_batch_does_not_corrupt_accepted_history() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger.ingest(&[fixture("one")]).unwrap();
        let mut malformed = fixture("two");
        malformed.handler.key.clear();
        assert!(ledger.ingest(&[malformed]).is_err());
        assert_eq!(ledger.invocation_count().unwrap(), 1);
    }
    #[test]
    fn schema_never_contains_raw_payload_columns() {
        let ledger = Ledger::open_in_memory().unwrap();
        let columns = ledger.invocation_columns().unwrap();
        for banned in ["prompt", "payload", "stdin", "stdout", "stderr", "command"] {
            assert!(!columns.iter().any(|column| column.contains(banned)));
        }
    }

    #[test]
    fn alias_migration_is_additive_and_does_not_change_invocation_identity() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger.ingest(&[fixture("one")]).unwrap();
        ledger
            .set_handler_alias(
                Runtime::Codex,
                "fixture-handler",
                "HAPI Session Hook",
                1_000,
            )
            .unwrap();
        assert!(ledger.table_exists("handler_catalog").unwrap());
        assert!(ledger.table_exists("handler_annotations").unwrap());
        assert_eq!(
            ledger.invocations().unwrap()[0].handler.key,
            "fixture-handler"
        );
        assert_eq!(
            ledger.handler_aliases().unwrap(),
            vec![HandlerAlias {
                runtime: Runtime::Codex,
                handler_key: "fixture-handler".into(),
                display_name: "HAPI Session Hook".into(),
            }]
        );
    }

    #[test]
    fn alias_apply_is_conflict_safe_and_rejects_unsafe_text() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        assert_eq!(
            ledger
                .set_handler_alias_if_unchanged(
                    Runtime::Codex,
                    "fixture-handler",
                    "Readable Alias",
                    None,
                    1_000,
                )
                .unwrap(),
            AliasSaveOutcome::Saved
        );
        assert_eq!(
            ledger
                .set_handler_alias_if_unchanged(
                    Runtime::Codex,
                    "fixture-handler",
                    "Lost Update",
                    None,
                    1_001,
                )
                .unwrap(),
            AliasSaveOutcome::Conflict
        );
        assert!(
            ledger
                .set_handler_alias_if_unchanged(
                    Runtime::Codex,
                    "fixture-handler",
                    "unsafe\nname",
                    Some("Readable Alias"),
                    1_002,
                )
                .is_err()
        );
        assert_eq!(
            ledger.handler_aliases().unwrap()[0].display_name,
            "Readable Alias"
        );
    }

    #[test]
    fn optional_alias_read_keeps_a_legacy_ledger_readable_without_migration() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("ledger.sqlite3");
        let ledger = Ledger::open_path(&path).unwrap();
        ledger
            .connection
            .execute("DROP TABLE handler_annotations", [])
            .unwrap();
        drop(ledger);
        let read_only = Ledger::open_read_only(&path).unwrap();
        assert!(read_only.handler_aliases_if_present().unwrap().is_empty());
    }

    #[test]
    fn finite_reliability_queries_materialize_only_the_recent_bounded_range() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let now = 90_i64 * 24 * 60 * 60 * 1_000;
        let mut values = Vec::new();
        for index in 0..200 {
            let mut value = fixture(&format!("old-{index}"));
            value.occurred_at_unix_ms = now - 70_i64 * 24 * 60 * 60 * 1_000 - index;
            values.push(value);
        }
        for index in 0..4 {
            let mut value = fixture(&format!("recent-{index}"));
            value.occurred_at_unix_ms = now - index;
            values.push(value);
        }
        ledger.ingest(&values).unwrap();

        let finite = ledger
            .invocations_for_reliability(now, TimeWindow::Last7Days)
            .unwrap();
        assert_eq!(finite.rows_materialized, 4);
        assert!(finite.bounds.current_start_unix_ms.is_some());
        let all = ledger
            .invocations_for_reliability(now, TimeWindow::All)
            .unwrap();
        assert_eq!(all.rows_materialized, 204);
    }

    #[test]
    fn receipt_reconciliation_cursor_and_rows_commit_together() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut first = fixture("receipt-one");
        first.source_key = "codex_instrumented_receipts_v1".into();
        first.occurred_at_unix_ms = 1_234;
        let receipt = ledger
            .ingest_receipt_reconciliation(
                "receipt_catalog_journal_v1",
                "codex_instrumented_receipts_v1",
                42,
                1,
                1_000,
                std::slice::from_ref(&first),
            )
            .unwrap();
        assert_eq!(receipt.inserted, 1);
        assert_eq!(
            ledger
                .receipt_reconciliation_state("receipt_catalog_journal_v1")
                .unwrap(),
            Some(ReceiptReconciliationState {
                journal_offset: 42,
                malformed_receipts: 1,
            })
        );
        let replay = ledger
            .ingest_receipt_reconciliation(
                "receipt_catalog_journal_v1",
                "codex_instrumented_receipts_v1",
                42,
                1,
                1_001,
                &[first],
            )
            .unwrap();
        assert_eq!(replay.duplicates, 1);
        assert_eq!(ledger.invocation_count().unwrap(), 1);
        assert_eq!(
            ledger
                .receipt_catalog_diagnostics_if_present("receipt_catalog_journal_v1")
                .unwrap(),
            Some(ReceiptCatalogDiagnostics {
                journal_offset: 42,
                record_count: 1,
                incomplete: 0,
                malformed: 1,
                latest_occurred_at_unix_ms: Some(1_234),
            })
        );
    }

    #[test]
    fn bounded_query_plus_specialized_aggregates_matches_full_v02_analytics_for_all_periods() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let now = 90_i64 * 24 * 60 * 60 * 1_000;
        let mut values = Vec::new();
        for index in 0..8 {
            let mut value = fixture(&format!("old-r1-{index}"));
            value.occurred_at_unix_ms = now - 75_i64 * 24 * 60 * 60 * 1_000 + index;
            value.handler.revision = "r1".into();
            value.terminal_status = TerminalStatus::Completed;
            values.push(value);
        }
        for index in 0..8 {
            let mut value = fixture(&format!("recent-r2-{index}"));
            value.occurred_at_unix_ms = now - 2_i64 * 24 * 60 * 60 * 1_000 + index;
            value.handler.revision = "r2".into();
            value.terminal_status = if index % 2 == 0 {
                TerminalStatus::Failed
            } else {
                TerminalStatus::Completed
            };
            values.push(value);
        }
        ledger.ingest(&values).unwrap();
        for window in [
            TimeWindow::Today,
            TimeWindow::Last24Hours,
            TimeWindow::Last7Days,
            TimeWindow::Last30Days,
            TimeWindow::All,
        ] {
            let full = crate::report::instrumented_report(&values, now, window, 0, 0);
            let bounded = ledger.invocations_for_reliability(now, window).unwrap();
            assert_eq!(
                bounded.rows_materialized,
                if window == TimeWindow::All { 16 } else { 8 }
            );
            let mut optimized =
                crate::report::instrumented_report(&bounded.invocations, now, window, 0, 0);
            let all_time = ledger.all_time_period_metrics(now).unwrap();
            let keys = optimized
                .intelligence
                .iter()
                .map(|item| item.handler_key.clone())
                .collect::<Vec<_>>();
            let revisions = ledger.revision_epoch_metrics(&keys).unwrap();
            for item in &mut optimized.intelligence {
                let trend = item
                    .trends
                    .iter_mut()
                    .find(|trend| trend.window == TimeWindow::All)
                    .unwrap();
                *trend = crate::analytics::all_time_trend(
                    all_time.get(&item.handler_key).unwrap().clone(),
                    optimized.qualification.coverage,
                );
                let epochs = revisions.get(&item.handler_key).unwrap();
                item.revision_comparison = crate::analytics::revision_comparison_from_epochs(
                    epochs.current.clone(),
                    epochs.previous.clone(),
                    optimized.qualification.coverage,
                );
            }
            assert_eq!(optimized.handlers, full.handlers, "{window:?}");
            assert_eq!(
                optimized.recent_failures, full.recent_failures,
                "{window:?}"
            );
            assert_eq!(optimized.intelligence, full.intelligence, "{window:?}");
        }
    }
}
