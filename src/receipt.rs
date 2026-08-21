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
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SOURCE_KEY: &str = "codex_instrumented_receipts_v1";
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
        atomic_json(&self.path_for(&value.invocation_id, "start"), value)
    }

    pub fn write_completion(&self, value: &ReceiptCompletion) -> Result<(), ReceiptError> {
        validate_completion(value)?;
        atomic_json(&self.path_for(&value.invocation_id, "complete"), value)
    }

    /// Scans records independently of the ledger. An orphan start becomes an
    /// explicit `incomplete` row; a later completion upgrades that row through
    /// the ledger's guarded upsert instead of fabricating an outcome.
    pub fn scan(&self) -> ReceiptScan {
        let mut starts = BTreeMap::new();
        let mut completions = BTreeMap::new();
        let mut malformed = 0;
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
            let loaded = fs::read(&path).ok();
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
}
