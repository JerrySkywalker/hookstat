//! HookStat-owned SQLite storage for source-neutral canonical metadata only.

use crate::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus, ValidationError,
};
use crate::identity::{generated_label, sanitize_display_name};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::fmt;
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 2;

pub struct Ledger {
    connection: Connection,
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
        Ok(Self {
            connection: Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?,
        })
    }

    fn from_connection(connection: Connection) -> Result<Self, LedgerError> {
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS hookstat_schema (version INTEGER PRIMARY KEY);
            INSERT OR IGNORE INTO hookstat_schema (version) VALUES (2);
            CREATE TABLE IF NOT EXISTS hook_invocations (
                source_key TEXT NOT NULL, source_record_id TEXT NOT NULL,
                runtime TEXT NOT NULL, evidence_kind TEXT NOT NULL, coverage TEXT NOT NULL,
                handler_key TEXT NOT NULL, handler_revision TEXT NOT NULL DEFAULT 'legacy',
                handler_label TEXT NOT NULL, handler_source_kind TEXT NOT NULL DEFAULT 'legacy',
                handler_event TEXT NOT NULL, handler_matcher_identity TEXT NOT NULL DEFAULT 'legacy',
                handler_structural_identity TEXT NOT NULL DEFAULT 'legacy', handler_execution_mode TEXT NOT NULL DEFAULT 'sync',
                occurred_at_unix_ms INTEGER NOT NULL, terminal_status TEXT NOT NULL,
                duration_ms INTEGER, error_fingerprint TEXT,
                PRIMARY KEY (source_key, source_record_id)
            );
            CREATE INDEX IF NOT EXISTS hook_invocations_handler_time ON hook_invocations (handler_key, occurred_at_unix_ms);
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
            ",
        )?;
        // Additive migration for an early development ledger. Failures mean the
        // column already exists, not a change to any runtime-owned data.
        for statement in [
            "ALTER TABLE hook_invocations ADD COLUMN handler_revision TEXT NOT NULL DEFAULT 'legacy'",
            "ALTER TABLE hook_invocations ADD COLUMN handler_source_kind TEXT NOT NULL DEFAULT 'legacy'",
            "ALTER TABLE hook_invocations ADD COLUMN handler_matcher_identity TEXT NOT NULL DEFAULT 'legacy'",
            "ALTER TABLE hook_invocations ADD COLUMN handler_structural_identity TEXT NOT NULL DEFAULT 'legacy'",
            "ALTER TABLE hook_invocations ADD COLUMN handler_execution_mode TEXT NOT NULL DEFAULT 'sync'",
        ] {
            let _ = connection.execute(statement, []);
        }
        Ok(Self { connection })
    }

    /// A duplicate is harmless. A completion may only replace an earlier
    /// explicit `incomplete` start record for the exact same receipt id.
    pub fn ingest(&mut self, values: &[HookInvocation]) -> Result<IngestReceipt, LedgerError> {
        for value in values {
            value.validate()?;
        }
        let transaction = self.connection.transaction()?;
        let mut receipt = IngestReceipt::default();
        for value in values {
            let prior: Option<String> = transaction.query_row("SELECT terminal_status FROM hook_invocations WHERE source_key = ?1 AND source_record_id = ?2", [&value.source_key, &value.source_record_id], |row| row.get(0)).optional()?;
            let changed = transaction.execute(
                "INSERT INTO hook_invocations (
                    source_key, source_record_id, runtime, evidence_kind, coverage,
                    handler_key, handler_revision, handler_label, handler_source_kind, handler_event,
                    handler_matcher_identity, handler_structural_identity, handler_execution_mode,
                    occurred_at_unix_ms, terminal_status, duration_ms, error_fingerprint
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(source_key, source_record_id) DO UPDATE SET
                    coverage = excluded.coverage, handler_revision = excluded.handler_revision,
                    handler_label = excluded.handler_label, handler_source_kind = excluded.handler_source_kind,
                    handler_matcher_identity = excluded.handler_matcher_identity,
                    handler_structural_identity = excluded.handler_structural_identity,
                    handler_execution_mode = excluded.handler_execution_mode, terminal_status = excluded.terminal_status,
                    duration_ms = excluded.duration_ms, error_fingerprint = excluded.error_fingerprint
                WHERE hook_invocations.terminal_status = 'incomplete' AND excluded.terminal_status != 'incomplete'",
                params![
                    &value.source_key, &value.source_record_id, value.runtime.as_storage(), value.evidence_kind.as_storage(), value.coverage.as_storage(),
                    &value.handler.key, &value.handler.revision, &value.handler.label, &value.handler.source_kind, value.handler.event.as_storage(),
                    &value.handler.matcher_identity, &value.handler.structural_identity, value.handler.execution_mode.as_storage(),
                    value.occurred_at_unix_ms, value.terminal_status.as_storage(), value.duration_ms.map(|duration| duration as i64), &value.error_fingerprint,
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
        transaction.commit()?;
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
        self.connection
            .query_row("SELECT count(*) FROM hook_invocations", [], |row| {
                row.get::<_, i64>(0)
            })
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

    pub fn invocations(&self) -> Result<Vec<HookInvocation>, LedgerError> {
        let mut statement = self.connection.prepare(
            "SELECT source_key, source_record_id, runtime, evidence_kind, coverage, handler_key, handler_revision,
                    handler_label, handler_source_kind, handler_event, handler_matcher_identity,
                    handler_structural_identity, handler_execution_mode, occurred_at_unix_ms, terminal_status,
                    duration_ms, error_fingerprint FROM hook_invocations ORDER BY occurred_at_unix_ms, source_record_id",
        )?;
        let rows = statement.query_map([], |row| {
            let invalid = || rusqlite::Error::InvalidQuery;
            let runtime = Runtime::from_storage(&row.get::<_, String>(2)?).ok_or_else(invalid)?;
            let evidence_kind =
                EvidenceKind::from_storage(&row.get::<_, String>(3)?).ok_or_else(invalid)?;
            let coverage =
                EvidenceCoverage::from_storage(&row.get::<_, String>(4)?).ok_or_else(invalid)?;
            let event = HookEvent::from_storage(&row.get::<_, String>(9)?).ok_or_else(invalid)?;
            let execution_mode =
                ExecutionMode::from_storage(&row.get::<_, String>(12)?).ok_or_else(invalid)?;
            let terminal_status =
                TerminalStatus::from_storage(&row.get::<_, String>(14)?).ok_or_else(invalid)?;
            let duration: Option<i64> = row.get(15)?;
            Ok(HookInvocation {
                source_key: row.get(0)?,
                source_record_id: row.get(1)?,
                runtime,
                evidence_kind,
                coverage,
                handler: HandlerIdentity {
                    key: row.get(5)?,
                    revision: row.get(6)?,
                    label: row.get(7)?,
                    source_kind: row.get(8)?,
                    event,
                    matcher_identity: row.get(10)?,
                    structural_identity: row.get(11)?,
                    execution_mode,
                },
                occurred_at_unix_ms: row.get(13)?,
                terminal_status,
                duration_ms: duration.map(|value| value as u64),
                error_fingerprint: row.get(16)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
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

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(id: &str) -> HookInvocation {
        HookInvocation {
            source_key: "fixture-source".into(),
            source_record_id: id.into(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
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
}
