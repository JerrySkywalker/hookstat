//! Atomic, HookStat-owned receipt spool for instrumented handlers.
//!
//! A receipt is intentionally metadata-only. The proxy never reads its stdin
//! and uses inherited stdout/stderr; no payload bytes can enter this module.

use crate::domain::{
    EvidenceCoverage, EvidenceKind, HandlerIdentity, HookInvocation, Runtime, TerminalStatus,
};
use crate::ledger::{IngestReceipt, Ledger, LedgerError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE_KEY: &str = "codex_instrumented_receipts_v1";
const RECONCILIATION_SOURCE_KEY: &str = "receipt_catalog_journal_v1";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiptStart {
    pub schema_version: u8,
    pub invocation_id: String,
    pub handler: HandlerIdentity,
    pub source: String,
    pub started_at_unix_ms: i64,
    pub coverage: EvidenceCoverage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiptCompletion {
    pub schema_version: u8,
    pub invocation_id: String,
    pub handler: HandlerIdentity,
    pub source: String,
    pub started_at_unix_ms: i64,
    pub completed_at_unix_ms: i64,
    pub duration_ms: u64,
    /// The process exit code only. It reveals no stdout/stderr content.
    pub exit_code: Option<i32>,
    pub terminal_status: TerminalStatus,
    pub coverage: EvidenceCoverage,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReceiptScan {
    pub invocations: Vec<HookInvocation>,
    pub malformed: u64,
    pub starts_without_completion: u64,
    pub files_inspected: u64,
    pub files_parsed: u64,
}

/// Deterministic evidence-work counters for the local performance contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReceiptWork {
    pub files_inspected: u64,
    pub files_parsed: u64,
    pub full_reconciliation: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReceiptReconciliation {
    pub malformed: u64,
    pub incomplete: u64,
    pub ingest: IngestReceipt,
    pub work: ReceiptWork,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JournalEntry {
    schema_version: u8,
    invocation_id: String,
    stage: JournalStage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalStage {
    Start,
    Complete,
}

#[derive(Debug)]
pub enum ReceiptError {
    Io(io::Error),
    Json(serde_json::Error),
    Ledger(LedgerError),
    Invalid(&'static str),
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => output.write_str("HookStat receipt spool operation failed"),
            Self::Json(_) => output.write_str("HookStat receipt metadata is malformed"),
            Self::Ledger(error) => error.fmt(output),
            Self::Invalid(field) => write!(output, "invalid HookStat receipt metadata in {field}"),
        }
    }
}
impl std::error::Error for ReceiptError {}
impl From<io::Error> for ReceiptError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for ReceiptError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
impl From<LedgerError> for ReceiptError {
    fn from(value: LedgerError) -> Self {
        Self::Ledger(value)
    }
}

#[derive(Clone, Debug)]
pub struct ReceiptSpool {
    root: PathBuf,
}

impl ReceiptSpool {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ReceiptError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("records"))?;
        Ok(Self { root })
    }

    /// Opens an already-created spool without creating its directory. This is
    /// deliberately separate from `open`: operational diagnostics must be
    /// able to inspect a missing or damaged spool without repairing it.
    pub fn open_existing(root: impl AsRef<Path>) -> Result<Self, ReceiptError> {
        let root = root.as_ref().to_path_buf();
        if !root.join("records").is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "receipt records").into());
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write_start(&self, value: &ReceiptStart) -> Result<(), ReceiptError> {
        validate_start(value)?;
        atomic_json(&self.path_for(&value.invocation_id, "start"), value)?;
        self.append_journal(&value.invocation_id, JournalStage::Start)
    }

    pub fn write_completion(&self, value: &ReceiptCompletion) -> Result<(), ReceiptError> {
        validate_completion(value)?;
        atomic_json(&self.path_for(&value.invocation_id, "complete"), value)?;
        self.append_journal(&value.invocation_id, JournalStage::Complete)
    }

    /// Scans records independently of the ledger. An orphan start becomes an
    /// explicit `incomplete` row; a later completion upgrades that row through
    /// the ledger's guarded upsert instead of fabricating an outcome.
    pub fn scan(&self) -> ReceiptScan {
        let mut starts = BTreeMap::new();
        let mut completions = BTreeMap::new();
        let mut malformed = 0;
        let mut files_inspected = 0;
        let mut files_parsed = 0;
        let Ok(entries) = fs::read_dir(self.root.join("records")) else {
            return ReceiptScan::default();
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            // A process interruption may leave a same-directory temporary
            // file. It was never atomically published as a receipt and must
            // not fabricate malformed coverage or corrupt accepted records.
            if name.starts_with(".hookstat-") && name.ends_with(".tmp") {
                continue;
            }
            files_inspected += 1;
            let loaded = fs::read(&path).ok();
            if loaded.is_some() {
                files_parsed += 1;
            }
            match (
                name.ends_with(".start.json"),
                name.ends_with(".complete.json"),
                loaded,
            ) {
                (true, _, Some(bytes)) => match serde_json::from_slice::<ReceiptStart>(&bytes)
                    .and_then(|value| {
                        validate_start(&value).map_err(|error| {
                            serde_json::Error::io(io::Error::new(
                                io::ErrorKind::InvalidData,
                                error.to_string(),
                            ))
                        })?;
                        Ok(value)
                    }) {
                    Ok(value) => {
                        starts.insert(value.invocation_id.clone(), value);
                    }
                    Err(_) => malformed += 1,
                },
                (_, true, Some(bytes)) => match serde_json::from_slice::<ReceiptCompletion>(&bytes)
                    .and_then(|value| {
                        validate_completion(&value).map_err(|error| {
                            serde_json::Error::io(io::Error::new(
                                io::ErrorKind::InvalidData,
                                error.to_string(),
                            ))
                        })?;
                        Ok(value)
                    }) {
                    Ok(value) => {
                        completions.insert(value.invocation_id.clone(), value);
                    }
                    Err(_) => malformed += 1,
                },
                _ => malformed += 1,
            }
        }
        let mut result = ReceiptScan {
            invocations: Vec::new(),
            malformed,
            starts_without_completion: 0,
            files_inspected,
            files_parsed,
        };
        for (id, start) in &starts {
            if let Some(completion) = completions.get(id) {
                result.invocations.push(completion.to_invocation());
            } else {
                result.starts_without_completion += 1;
                result.invocations.push(HookInvocation {
                    source_key: SOURCE_KEY.to_owned(),
                    source_record_id: id.clone(),
                    runtime: Runtime::Codex,
                    evidence_kind: EvidenceKind::CodexInstrumentedReceipt,
                    coverage: EvidenceCoverage::Unknown,
                    handler: start.handler.clone(),
                    occurred_at_unix_ms: start.started_at_unix_ms,
                    terminal_status: TerminalStatus::Incomplete,
                    duration_ms: None,
                    error_fingerprint: None,
                });
            }
        }
        // A completion surviving a cleanup/crash without its start is useful
        // terminal evidence, but its coverage is explicitly best-effort.
        for (id, completion) in completions {
            if !starts.contains_key(&id) {
                let mut value = completion.to_invocation();
                value.coverage = EvidenceCoverage::BestEffort;
                result.invocations.push(value);
            }
        }
        result
            .invocations
            .sort_by(|left, right| left.source_record_id.cmp(&right.source_record_id));
        result
    }

    pub fn ingest_into(
        &self,
        ledger: &mut Ledger,
    ) -> Result<(ReceiptScan, IngestReceipt), ReceiptError> {
        let scan = self.scan();
        let receipt = ledger.ingest(&scan.invocations)?;
        Ok((scan, receipt))
    }

    /// Reconciles the normal warm path from an append-only, HookStat-owned
    /// journal. Existing v0.2 spools automatically receive one full scan;
    /// later warm starts read only bytes appended after the durable cursor.
    /// The receipt files remain canonical evidence and `reconcile_full` is
    /// available for bounded/full integrity checks outside the hot path.
    pub fn reconcile_incremental(
        &self,
        ledger: &mut Ledger,
        now_unix_ms: i64,
    ) -> Result<ReceiptReconciliation, ReceiptError> {
        let state = ledger.receipt_reconciliation_state(RECONCILIATION_SOURCE_KEY)?;
        let journal_length = self.journal_length()?;
        let Some(state) = state else {
            return self.reconcile_full(ledger, now_unix_ms);
        };
        if state.journal_offset > journal_length {
            // A damaged/replaced journal must not make a cursor skip evidence.
            // Recover by scanning canonical files rather than guessing an order.
            return self.reconcile_full(ledger, now_unix_ms);
        }
        let (entries, next_offset, journal_malformed) =
            self.read_journal_from(state.journal_offset)?;
        let mut values = Vec::new();
        let mut work = ReceiptWork::default();
        let mut malformed_delta = journal_malformed;
        for entry in entries {
            let (value, malformed, inspected, parsed) = self.invocation_for(&entry.invocation_id);
            work.files_inspected += inspected;
            work.files_parsed += parsed;
            malformed_delta += malformed;
            if let Some(value) = value {
                values.push(value);
            }
        }
        let ingest = ledger.ingest_receipt_reconciliation(
            RECONCILIATION_SOURCE_KEY,
            next_offset,
            state.malformed_receipts.saturating_add(malformed_delta),
            now_unix_ms,
            &values,
        )?;
        Ok(ReceiptReconciliation {
            malformed: state.malformed_receipts.saturating_add(malformed_delta),
            incomplete: ledger.incomplete_receipt_count()?,
            ingest,
            work,
        })
    }

    /// Explicit integrity reconciliation. This path may inspect every receipt
    /// and is intentionally not used during an unchanged warm startup.
    pub fn reconcile_full(
        &self,
        ledger: &mut Ledger,
        now_unix_ms: i64,
    ) -> Result<ReceiptReconciliation, ReceiptError> {
        let scan = self.scan();
        let journal_length = self.journal_length()?;
        let ingest = ledger.ingest_receipt_reconciliation(
            RECONCILIATION_SOURCE_KEY,
            journal_length,
            scan.malformed,
            now_unix_ms,
            &scan.invocations,
        )?;
        Ok(ReceiptReconciliation {
            malformed: scan.malformed,
            incomplete: ledger.incomplete_receipt_count()?,
            ingest,
            work: ReceiptWork {
                files_inspected: scan.files_inspected,
                files_parsed: scan.files_parsed,
                full_reconciliation: true,
            },
        })
    }

    fn journal_path(&self) -> PathBuf {
        self.root.join("receipt-journal-v1.ndjson")
    }

    fn append_journal(&self, invocation_id: &str, stage: JournalStage) -> Result<(), ReceiptError> {
        let entry = JournalEntry {
            schema_version: 1,
            invocation_id: invocation_id.to_owned(),
            stage,
        };
        let mut bytes = serde_json::to_vec(&entry)?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.journal_path())?;
        file.write_all(&bytes)?;
        file.sync_data()?;
        Ok(())
    }

    fn journal_length(&self) -> Result<u64, ReceiptError> {
        let path = self.journal_path();
        if !path.exists() {
            OpenOptions::new().create(true).append(true).open(&path)?;
        }
        Ok(fs::metadata(path)?.len())
    }

    fn read_journal_from(
        &self,
        offset: u64,
    ) -> Result<(Vec<JournalEntry>, u64, u64), ReceiptError> {
        let mut file = OpenOptions::new().read(true).open(self.journal_path())?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let next_offset = offset.saturating_add(bytes.len() as u64);
        let mut entries = Vec::new();
        let mut malformed = 0;
        for line in bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            match serde_json::from_slice::<JournalEntry>(line) {
                Ok(entry) if entry.schema_version == 1 && valid_id(&entry.invocation_id) => {
                    entries.push(entry)
                }
                _ => malformed += 1,
            }
        }
        Ok((entries, next_offset, malformed))
    }

    fn invocation_for(&self, invocation_id: &str) -> (Option<HookInvocation>, u64, u64, u64) {
        let mut malformed = 0;
        let mut inspected = 0;
        let mut parsed = 0;
        let start = self.read_start(invocation_id, &mut malformed, &mut inspected, &mut parsed);
        let completion =
            self.read_completion(invocation_id, &mut malformed, &mut inspected, &mut parsed);
        let value = match (start, completion) {
            (_, Some(completion)) => Some(completion.to_invocation()),
            (Some(start), None) => Some(HookInvocation {
                source_key: SOURCE_KEY.to_owned(),
                source_record_id: invocation_id.to_owned(),
                runtime: Runtime::Codex,
                evidence_kind: EvidenceKind::CodexInstrumentedReceipt,
                coverage: EvidenceCoverage::Unknown,
                handler: start.handler,
                occurred_at_unix_ms: start.started_at_unix_ms,
                terminal_status: TerminalStatus::Incomplete,
                duration_ms: None,
                error_fingerprint: None,
            }),
            (None, None) => None,
        };
        (value, malformed, inspected, parsed)
    }

    fn read_start(
        &self,
        invocation_id: &str,
        malformed: &mut u64,
        inspected: &mut u64,
        parsed: &mut u64,
    ) -> Option<ReceiptStart> {
        let path = self.path_for(invocation_id, "start");
        if !path.is_file() {
            return None;
        }
        *inspected += 1;
        let Ok(bytes) = fs::read(path) else {
            *malformed += 1;
            return None;
        };
        *parsed += 1;
        match serde_json::from_slice::<ReceiptStart>(&bytes).and_then(|value| {
            validate_start(&value).map_err(|error| {
                serde_json::Error::io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    error.to_string(),
                ))
            })?;
            Ok(value)
        }) {
            Ok(value) => Some(value),
            Err(_) => {
                *malformed += 1;
                None
            }
        }
    }

    fn read_completion(
        &self,
        invocation_id: &str,
        malformed: &mut u64,
        inspected: &mut u64,
        parsed: &mut u64,
    ) -> Option<ReceiptCompletion> {
        let path = self.path_for(invocation_id, "complete");
        if !path.is_file() {
            return None;
        }
        *inspected += 1;
        let Ok(bytes) = fs::read(path) else {
            *malformed += 1;
            return None;
        };
        *parsed += 1;
        match serde_json::from_slice::<ReceiptCompletion>(&bytes).and_then(|value| {
            validate_completion(&value).map_err(|error| {
                serde_json::Error::io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    error.to_string(),
                ))
            })?;
            Ok(value)
        }) {
            Ok(value) => Some(value),
            Err(_) => {
                *malformed += 1;
                None
            }
        }
    }

    fn path_for(&self, id: &str, stage: &str) -> PathBuf {
        self.root.join("records").join(format!("{id}.{stage}.json"))
    }
}

impl ReceiptCompletion {
    pub fn to_invocation(&self) -> HookInvocation {
        HookInvocation {
            source_key: SOURCE_KEY.to_owned(),
            source_record_id: self.invocation_id.clone(),
            runtime: Runtime::Codex,
            evidence_kind: EvidenceKind::CodexInstrumentedReceipt,
            coverage: self.coverage,
            handler: self.handler.clone(),
            occurred_at_unix_ms: self.started_at_unix_ms,
            terminal_status: self.terminal_status,
            duration_ms: Some(self.duration_ms),
            error_fingerprint: self
                .terminal_status
                .is_execution_failure()
                .then_some("exit_nonzero".to_owned()),
        }
    }
}

fn atomic_json(path: &Path, value: &impl Serialize) -> Result<(), ReceiptError> {
    let bytes = serde_json::to_vec(value)?;
    let parent = path.parent().ok_or(ReceiptError::Invalid("path"))?;
    fs::create_dir_all(parent)?;
    let suffix = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".hookstat-{}-{suffix}.tmp", std::process::id()));
    fs::write(&temporary, bytes)?;
    // Same-directory rename gives an atomic replacement on supported local
    // filesystems. A crash leaves either a complete prior file or a temp file.
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        // Windows refuses replacement by rename. A simultaneous duplicate
        // receipt has already been durably published, which is the desired
        // idempotent state; discard only our unpublished temp.
        Err(_) if path.exists() => {
            let _ = fs::remove_file(&temporary);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(ReceiptError::Io(error))
        }
    }
}

fn validate_start(value: &ReceiptStart) -> Result<(), ReceiptError> {
    if value.schema_version != 1
        || !valid_id(&value.invocation_id)
        || !valid_id(&value.source)
        || value.started_at_unix_ms < 0
    {
        return Err(ReceiptError::Invalid("start"));
    }
    validate_handler(&value.handler)
}
fn validate_completion(value: &ReceiptCompletion) -> Result<(), ReceiptError> {
    if value.schema_version != 1
        || !valid_id(&value.invocation_id)
        || !valid_id(&value.source)
        || value.started_at_unix_ms < 0
        || value.completed_at_unix_ms < value.started_at_unix_ms
    {
        return Err(ReceiptError::Invalid("completion"));
    }
    validate_handler(&value.handler)
}
fn validate_handler(value: &HandlerIdentity) -> Result<(), ReceiptError> {
    for field in [
        &value.key,
        &value.revision,
        &value.label,
        &value.source_kind,
        &value.matcher_identity,
        &value.structural_identity,
    ] {
        if !valid_id(field) {
            return Err(ReceiptError::Invalid("handler"));
        }
    }
    Ok(())
}
fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ExecutionMode, HookEvent};
    use tempfile::tempdir;
    fn handler() -> HandlerIdentity {
        HandlerIdentity {
            key: "hk_123".into(),
            revision: "hr_456".into(),
            label: "handler hk_123".into(),
            source_kind: "user_hooks_json".into(),
            event: HookEvent::Stop,
            matcher_identity: "any".into(),
            structural_identity: "g0:h0".into(),
            execution_mode: ExecutionMode::Sync,
        }
    }
    fn start(id: &str) -> ReceiptStart {
        ReceiptStart {
            schema_version: 1,
            invocation_id: id.into(),
            handler: handler(),
            source: "codex".into(),
            started_at_unix_ms: 1_000,
            coverage: EvidenceCoverage::Partial,
        }
    }
    fn complete(id: &str) -> ReceiptCompletion {
        ReceiptCompletion {
            schema_version: 1,
            invocation_id: id.into(),
            handler: handler(),
            source: "codex".into(),
            started_at_unix_ms: 1_000,
            completed_at_unix_ms: 1_050,
            duration_ms: 50,
            exit_code: Some(0),
            terminal_status: TerminalStatus::Completed,
            coverage: EvidenceCoverage::Partial,
        }
    }
    #[test]
    fn atomic_receipts_are_idempotent_and_start_only_is_incomplete() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        spool.write_start(&start("one")).unwrap();
        let before = spool.scan();
        assert_eq!(before.starts_without_completion, 1);
        assert_eq!(
            before.invocations[0].terminal_status,
            TerminalStatus::Incomplete
        );
        spool.write_completion(&complete("one")).unwrap();
        let mut ledger = Ledger::open_in_memory().unwrap();
        let (scan, first) = spool.ingest_into(&mut ledger).unwrap();
        assert_eq!(scan.malformed, 0);
        assert_eq!(first.inserted, 1);
        let (_, second) = spool.ingest_into(&mut ledger).unwrap();
        assert_eq!(second.duplicates, 1);
        assert_eq!(ledger.invocation_count().unwrap(), 1);
    }
    #[test]
    fn malformed_receipt_is_isolated_and_never_becomes_success() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        fs::write(temp.path().join("records/bad.complete.json"), b"not json").unwrap();
        let scan = spool.scan();
        assert_eq!(scan.malformed, 1);
        assert!(scan.invocations.is_empty());
    }
    #[test]
    fn receipt_json_never_has_payload_stream_fields() {
        let encoded = serde_json::to_string(&complete("one")).unwrap();
        for banned in ["stdin", "stdout", "stderr", "prompt", "payload", "command"] {
            assert!(!encoded.contains(banned));
        }
    }
    #[test]
    fn duplicate_and_interrupted_receipts_are_idempotent_and_never_corrupt_coverage() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        spool.write_start(&start("duplicate")).unwrap();
        spool.write_start(&start("duplicate")).unwrap();
        spool.write_completion(&complete("duplicate")).unwrap();
        spool.write_completion(&complete("duplicate")).unwrap();
        fs::write(
            temp.path().join("records/.hookstat-interrupted.tmp"),
            b"partial",
        )
        .unwrap();
        let scan = spool.scan();
        assert_eq!(scan.malformed, 0);
        assert_eq!(scan.invocations.len(), 1);
        assert_eq!(
            scan.invocations[0].terminal_status,
            TerminalStatus::Completed
        );
    }
    #[test]
    fn concurrent_writers_and_out_of_order_completion_remain_idempotent() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        let mut workers = Vec::new();
        for index in 0..64 {
            let spool = spool.clone();
            workers.push(std::thread::spawn(move || {
                let id = format!("concurrent-{index}");
                spool.write_start(&start(&id)).unwrap();
                spool.write_completion(&complete(&id)).unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        spool
            .write_completion(&complete("completion-first"))
            .unwrap();
        let before_start = spool.scan();
        assert!(
            before_start
                .invocations
                .iter()
                .any(|value| value.source_record_id == "completion-first"
                    && value.coverage == EvidenceCoverage::BestEffort)
        );
        spool.write_start(&start("completion-first")).unwrap();
        let mut ledger = Ledger::open_in_memory().unwrap();
        let (scan, first) = spool.ingest_into(&mut ledger).unwrap();
        assert_eq!(scan.malformed, 0);
        assert_eq!(scan.invocations.len(), 65);
        assert_eq!(first.inserted, 65);
        let (_, repeated) = spool.ingest_into(&mut ledger).unwrap();
        assert_eq!(repeated.duplicates, 65);
        assert_eq!(ledger.invocation_count().unwrap(), 65);
    }

    fn assert_incremental_reconciliation_work(scale: usize) {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        // Model a v0.2 spool: canonical historical receipt files with no
        // journal yet. The first reconciliation is deliberately full.
        for index in 0..scale {
            let id = format!("historic-{index:05}");
            // This is a synthetic legacy fixture, so it intentionally uses
            // direct writes rather than measuring the proxy's atomic writer.
            // The reconciliation must tolerate independently published files.
            fs::write(
                spool.path_for(&id, "complete"),
                serde_json::to_vec(&complete(&id)).unwrap(),
            )
            .unwrap();
        }
        let mut ledger = Ledger::open_in_memory().unwrap();
        let cold = spool.reconcile_incremental(&mut ledger, 1_000).unwrap();
        assert!(cold.work.full_reconciliation);
        assert_eq!(cold.work.files_inspected, scale as u64);
        assert_eq!(cold.work.files_parsed, scale as u64);
        assert_eq!(ledger.invocation_count().unwrap(), scale as u64);

        let warm = spool.reconcile_incremental(&mut ledger, 1_001).unwrap();
        assert!(!warm.work.full_reconciliation);
        assert_eq!(warm.work.files_inspected, 0);
        assert_eq!(warm.work.files_parsed, 0);

        spool.write_start(&start("new-record")).unwrap();
        let start_only = spool.reconcile_incremental(&mut ledger, 1_002).unwrap();
        assert_eq!(start_only.work.files_inspected, 1);
        assert_eq!(start_only.work.files_parsed, 1);
        assert_eq!(start_only.incomplete, 1);

        spool.write_completion(&complete("new-record")).unwrap();
        let upgraded = spool.reconcile_incremental(&mut ledger, 1_003).unwrap();
        assert_eq!(upgraded.work.files_inspected, 2);
        assert_eq!(upgraded.work.files_parsed, 2);
        assert_eq!(upgraded.ingest.upgraded, 1);
        assert_eq!(upgraded.incomplete, 0);
        assert_eq!(ledger.invocation_count().unwrap(), scale as u64 + 1);
    }

    #[test]
    fn incremental_reconciliation_keeps_warm_work_constant() {
        assert_incremental_reconciliation_work(32);
    }

    /// Explicit deterministic scale harness for the owned baseline. It stays
    /// separate from the ordinary unit suite because Windows endpoint scanning
    /// can make thousands of synthetic file creations unsuitable for a fast
    /// per-commit correctness loop.
    #[test]
    #[ignore = "explicit 6769-receipt scale harness"]
    fn incremental_reconciliation_migrates_6769_receipt_files() {
        assert_incremental_reconciliation_work(6_769);
    }

    #[test]
    fn incremental_journal_replay_is_idempotent_and_malformed_evidence_stays_visible() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        let mut ledger = Ledger::open_in_memory().unwrap();
        let _ = spool.reconcile_incremental(&mut ledger, 1_000).unwrap();
        spool.write_start(&start("recoverable")).unwrap();

        // Simulate interruption after the durable receipt write but before a
        // caller observes its successful reconciliation. Replaying the same
        // journal range cannot corrupt or duplicate accepted evidence.
        let first = spool.reconcile_incremental(&mut ledger, 1_001).unwrap();
        assert_eq!(first.ingest.inserted, 1);
        let second = spool.reconcile_incremental(&mut ledger, 1_002).unwrap();
        assert_eq!(second.ingest.inserted, 0);
        assert_eq!(ledger.invocation_count().unwrap(), 1);

        fs::write(temp.path().join("records/bad.complete.json"), b"malformed").unwrap();
        spool.append_journal("bad", JournalStage::Complete).unwrap();
        let malformed = spool.reconcile_incremental(&mut ledger, 1_003).unwrap();
        assert_eq!(malformed.malformed, 1);
        assert_eq!(ledger.invocation_count().unwrap(), 1);
    }

    #[test]
    fn concurrent_journal_writers_reconcile_without_full_rescan() {
        let temp = tempdir().unwrap();
        let spool = ReceiptSpool::open(temp.path()).unwrap();
        let mut ledger = Ledger::open_in_memory().unwrap();
        let cold = spool.reconcile_incremental(&mut ledger, 1_000).unwrap();
        assert!(cold.work.full_reconciliation);
        let mut workers = Vec::new();
        for index in 0..16 {
            let spool = spool.clone();
            workers.push(std::thread::spawn(move || {
                let id = format!("journal-{index}");
                spool.write_start(&start(&id)).unwrap();
                spool.write_completion(&complete(&id)).unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
        let warm = spool.reconcile_incremental(&mut ledger, 1_001).unwrap();
        assert!(!warm.work.full_reconciliation);
        assert_eq!(warm.malformed, 0);
        assert_eq!(ledger.invocation_count().unwrap(), 16);
    }
}
