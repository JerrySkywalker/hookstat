//! HookStat-owned SQLite storage for source-neutral canonical metadata only.

use crate::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus, ValidationError,
};
use rusqlite::{Connection, OptionalExtension, params};
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
}
