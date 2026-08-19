//! HookStat-owned SQLite storage for canonical metadata only.
//!
//! This module is source-agnostic. It does not open or write any Codex file and
//! no CLI command creates a ledger while HS-G01 remains blocked.

use crate::domain::{HookInvocation, ValidationError};
use rusqlite::{Connection, OptionalExtension, params};
use std::fmt;
use std::path::Path;

/// Schema version for the HookStat-owned ledger.
pub const SCHEMA_VERSION: i64 = 1;

pub struct Ledger {
    connection: Connection,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IngestReceipt {
    pub inserted: u64,
    pub duplicates: u64,
}

#[derive(Debug)]
pub enum LedgerError {
    Database(rusqlite::Error),
    Validation(ValidationError),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(_) => formatter.write_str("HookStat ledger operation failed"),
            Self::Validation(error) => error.fmt(formatter),
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
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<ValidationError> for LedgerError {
    fn from(error: ValidationError) -> Self {
        Self::Validation(error)
    }
}

impl Ledger {
    /// Opens a HookStat-owned path. Callers choose the platform data directory;
    /// this method never derives or mutates a Codex-owned path.
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, LedgerError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    /// In-memory construction keeps the default test suite private and
    /// filesystem-independent.
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, LedgerError> {
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS hookstat_schema (
                version INTEGER PRIMARY KEY
            );
            INSERT OR IGNORE INTO hookstat_schema (version) VALUES (1);

            CREATE TABLE IF NOT EXISTS hook_invocations (
                source_key TEXT NOT NULL,
                source_record_id TEXT NOT NULL,
                runtime TEXT NOT NULL,
                evidence_kind TEXT NOT NULL,
                coverage TEXT NOT NULL,
                handler_key TEXT NOT NULL,
                handler_label TEXT NOT NULL,
                handler_event TEXT NOT NULL,
                occurred_at_unix_ms INTEGER NOT NULL,
                terminal_status TEXT NOT NULL,
                duration_ms INTEGER,
                error_fingerprint TEXT,
                PRIMARY KEY (source_key, source_record_id)
            );
            CREATE INDEX IF NOT EXISTS hook_invocations_handler_time
                ON hook_invocations (handler_key, occurred_at_unix_ms);

            CREATE TABLE IF NOT EXISTS source_cursors (
                source_key TEXT PRIMARY KEY,
                cursor TEXT NOT NULL,
                updated_at_unix_ms INTEGER NOT NULL
            );
            ",
        )?;
        Ok(Self { connection })
    }

    /// Inserts a batch atomically. A duplicate source record is harmless and a
    /// malformed record is rejected before any row in its batch is persisted.
    pub fn ingest(&mut self, invocations: &[HookInvocation]) -> Result<IngestReceipt, LedgerError> {
        for invocation in invocations {
            invocation.validate()?;
        }

        let transaction = self.connection.transaction()?;
        let mut receipt = IngestReceipt::default();
        for invocation in invocations {
            let duration_ms = invocation.duration_ms.map(|duration_ms| duration_ms as i64);
            let changed = transaction.execute(
                "
                INSERT INTO hook_invocations (
                    source_key, source_record_id, runtime, evidence_kind, coverage,
                    handler_key, handler_label, handler_event, occurred_at_unix_ms,
                    terminal_status, duration_ms, error_fingerprint
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(source_key, source_record_id) DO NOTHING
                ",
                params![
                    &invocation.source_key,
                    &invocation.source_record_id,
                    invocation.runtime.as_storage(),
                    invocation.evidence_kind.as_storage(),
                    invocation.coverage.as_storage(),
                    &invocation.handler.key,
                    &invocation.handler.label,
                    invocation.handler.event.as_storage(),
                    invocation.occurred_at_unix_ms,
                    invocation.terminal_status.as_storage(),
                    duration_ms,
                    &invocation.error_fingerprint,
                ],
            )?;
            if changed == 0 {
                receipt.duplicates += 1;
            } else {
                receipt.inserted += 1;
            }
        }
        transaction.commit()?;
        Ok(receipt)
    }

    /// Stores an opaque source cursor after successful ingestion. Source paths
    /// should be normalized or hashed by an admitted adapter before this call.
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
        self.connection.execute(
            "
            INSERT INTO source_cursors (source_key, cursor, updated_at_unix_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(source_key) DO UPDATE SET
                cursor = excluded.cursor,
                updated_at_unix_ms = excluded.updated_at_unix_ms
            ",
            params![source_key, cursor, updated_at_unix_ms],
        )?;
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

    #[cfg(test)]
    fn invocation_columns(&self) -> Result<Vec<String>, LedgerError> {
        let mut statement = self
            .connection
            .prepare("PRAGMA table_info(hook_invocations)")?;
        let columns = statement
            .query_map([], |row| row.get(1))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        EvidenceCoverage, EvidenceKind, HandlerIdentity, HookEvent, Runtime, TerminalStatus,
    };

    fn fixture(id: &str) -> HookInvocation {
        HookInvocation {
            source_key: "fixture-source".to_owned(),
            source_record_id: id.to_owned(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::SyntheticFixture,
            coverage: EvidenceCoverage::SyntheticFixture,
            handler: HandlerIdentity {
                key: "fixture-handler".to_owned(),
                label: "fixture handler".to_owned(),
                event: HookEvent::Stop,
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
        let repeated = ledger.ingest(&[fixture("one")]).unwrap();
        assert_eq!(repeated.inserted, 0);
        assert_eq!(repeated.duplicates, 1);
        assert_eq!(ledger.ingest(&[fixture("two")]).unwrap().inserted, 1);
        assert_eq!(ledger.invocation_count().unwrap(), 2);
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
    fn cursor_is_upserted_without_storing_raw_payload_columns() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger
            .advance_cursor("fixture-source", "cursor-1", 1)
            .unwrap();
        ledger
            .advance_cursor("fixture-source", "cursor-2", 2)
            .unwrap();
        assert_eq!(
            ledger.cursor("fixture-source").unwrap().as_deref(),
            Some("cursor-2")
        );
        let columns = ledger.invocation_columns().unwrap();
        assert!(!columns.iter().any(|column| column.contains("prompt")));
        assert!(!columns.iter().any(|column| column.contains("payload")));
    }
}
