//! Runtime-neutral broker and WAL over the shared internal IPC v1 wire.
//!
//! The protocol, local endpoint, bounded client and startup election live in
//! `ipc_client`, which is the sole wire implementation used by both
//! this G35 broker and the G36 producers. This module owns only broker/WAL
//! persistence and G29 CanonicalEvidence conversion.

use crate::domain::TerminalStatus;
use crate::evidence::{
    CanonicalEvidence, CoreIngestOutcome, EventFamily, EvidenceLifecycle, EvidenceTransport,
    InvocationCoverage, InvocationKey, RevisionRef, RuntimeHandlerRef, RuntimeId, RuntimeInstance,
    RuntimeNeutralEvidenceCore, SourceCoverage, SourceScope,
};
use crate::ipc_client::{
    Listener, Stream, checksum, prepare_state_root, read_frame_bounded, write_frame_bounded,
};
use interprocess::local_socket::prelude::*;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub use crate::ipc_client::{
    BROKER_DIAGNOSTICS_SCHEMA_VERSION, BrokerAcknowledgement, BrokerDiagnostics, BrokerStartup,
    Completion, ExitClassification, IPC_FRAME_HEADER_BYTES, IPC_MAGIC, IPC_PROTOCOL_VERSION,
    IpcClient, IpcError, IpcFrame, LifecycleFrame, LocalEndpoint, MAX_IPC_FRAME_BYTES,
    MAX_IPC_REFERENCE_BYTES, ObservationDisposition, ProducerPolicy,
    RECENT_DIAGNOSTIC_SAMPLE_CAPACITY, TerminalOutcome,
};
#[cfg(feature = "performance-harness")]
pub(crate) use crate::ipc_client::{
    CooperativeProducer, QualificationClientStageSample, QualificationSendFailure,
};

pub const WAL_MAGIC: [u8; 4] = *b"HSWL";
pub const WAL_VERSION: u8 = 1;
pub const MAX_WAL_BYTES: u64 = 64 * 1024 * 1024;
// A producer may reuse a connection for at most 25 ms. Keep a bounded
// broker-side scheduling margin so a Windows producer that is briefly
// descheduled after its reuse check does not write to a pipe the broker has
// just retired. The window still bounds an idle server connection and leaves
// lifecycle delivery failure fail-open and no-replay.
const CONNECTION_IDLE_READ_WINDOW: Duration = Duration::from_millis(250);
// Diagnostics are single snapshot queries, not retained producer sessions.
// Their short post-response window preserves the on-demand broker's bounded
// idle shutdown even while lifecycle connections receive the wider margin.
const DIAGNOSTICS_IDLE_READ_WINDOW: Duration = Duration::from_millis(25);
const WAL_HEADER_BYTES: usize = 12;

#[cfg(feature = "performance-harness")]
fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn canonical(frame: &IpcFrame) -> Result<CanonicalEvidence, IpcError> {
    let (lifecycle, lifecycle_state, completion) = match frame {
        IpcFrame::Start(value) => (value, EvidenceLifecycle::Started, None),
        IpcFrame::Complete {
            lifecycle,
            completion,
        } => (lifecycle, EvidenceLifecycle::Completed, Some(completion)),
        IpcFrame::Ack(_) => return Err(IpcError::Invalid("acknowledgement_is_not_evidence")),
        IpcFrame::BrokerDiagnosticsRequest | IpcFrame::BrokerDiagnosticsResponse(_) => {
            return Err(IpcError::Invalid("control_frame_is_not_evidence"));
        }
    };
    lifecycle.validate()?;
    let (terminal_status, duration_ms, invocation_coverage) = match (lifecycle_state, completion) {
        (EvidenceLifecycle::Started, None) => (None, None, InvocationCoverage::Incomplete),
        (EvidenceLifecycle::Completed, Some(completion)) => {
            completion.validate()?;
            (
                Some(match completion.terminal_status {
                    TerminalOutcome::Completed => TerminalStatus::Completed,
                    TerminalOutcome::Failed => TerminalStatus::Failed,
                    TerminalOutcome::Blocked => TerminalStatus::Blocked,
                    TerminalOutcome::Stopped => TerminalStatus::Stopped,
                    TerminalOutcome::TimedOut => TerminalStatus::TimedOut,
                    TerminalOutcome::ProtocolFailure => TerminalStatus::ProtocolFailure,
                }),
                Some(completion.duration_ms),
                InvocationCoverage::Complete,
            )
        }
        _ => return Err(IpcError::Invalid("completion")),
    };
    let evidence = CanonicalEvidence {
        schema_version: 1,
        runtime: RuntimeId::new(lifecycle.runtime.clone())
            .map_err(|_| IpcError::Invalid("runtime"))?,
        runtime_instance: RuntimeInstance::new(lifecycle.runtime_instance.clone())
            .map_err(|_| IpcError::Invalid("runtime_instance"))?,
        invocation_key: InvocationKey::new(lifecycle.invocation.clone())
            .map_err(|_| IpcError::Invalid("invocation"))?,
        runtime_handler_ref: RuntimeHandlerRef::new(lifecycle.handler.clone())
            .map_err(|_| IpcError::Invalid("handler"))?,
        event: EventFamily::new(lifecycle.event.clone()).map_err(|_| IpcError::Invalid("event"))?,
        lifecycle: lifecycle_state,
        occurred_at_unix_ms: lifecycle.occurred_at_unix_ms,
        terminal_status,
        duration_ms,
        source_scope: SourceScope::new(lifecycle.source_scope.clone())
            .map_err(|_| IpcError::Invalid("source_scope"))?,
        revision_ref: lifecycle
            .revision
            .clone()
            .map(RevisionRef::new)
            .transpose()
            .map_err(|_| IpcError::Invalid("revision"))?,
        evidence_transport: EvidenceTransport::Ipc,
        source_coverage: SourceCoverage::Durable,
        invocation_coverage,
    };
    evidence
        .validate()
        .map_err(|_| IpcError::Invalid("canonical_evidence"))?;
    Ok(evidence)
}

/// Append-only compact WAL with grouped `sync_data()` durability. A producer
/// is acknowledged only after broker enqueue/append, never after per-record
/// fsync. Thus an abrupt power loss may lose a bounded final group of
/// observational evidence; it can never manufacture a completion.
pub struct Wal {
    file: std::fs::File,
    path: std::path::PathBuf,
    bytes: u64,
    pending_records: u32,
    pending_bytes: u64,
    append_generation: u64,
    last_group_flush: std::time::Instant,
    policy: GroupDurabilityPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupDurabilityPolicy {
    pub max_records: u32,
    pub max_bytes: u64,
    pub max_interval: std::time::Duration,
}

// A record/byte-triggered request may briefly gather later generations before
// the single physical sync. The deadline is always capped by the 50 ms policy
// interval; time-triggered and shutdown requests are never delayed.
const DURABILITY_COALESCE_WINDOW: std::time::Duration = std::time::Duration::from_millis(2);

impl Default for GroupDurabilityPolicy {
    fn default() -> Self {
        Self {
            max_records: 64,
            max_bytes: 64 * 1024,
            max_interval: std::time::Duration::from_millis(50),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WalFlush {
    pub grouped_records: u32,
    pub grouped_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DurabilityRequest {
    through_generation: u64,
    group: WalFlush,
    coalesce_until: std::time::Instant,
}

impl Wal {
    pub fn open(
        state_root: impl AsRef<std::path::Path>,
        policy: GroupDurabilityPolicy,
    ) -> Result<Self, IpcError> {
        if policy.max_records == 0 || policy.max_bytes == 0 || policy.max_interval.is_zero() {
            return Err(IpcError::Invalid("group_durability_policy"));
        }
        let root = prepare_state_root(state_root.as_ref())?;
        let path = root.join("ipc-evidence-v1.wal");
        if path.exists() {
            let metadata = std::fs::symlink_metadata(&path).map_err(IpcError::Io)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(IpcError::UnsafeStateObject);
            }
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(IpcError::Io)?;
        let bytes = file.metadata().map_err(IpcError::Io)?.len();
        if bytes > MAX_WAL_BYTES {
            return Err(IpcError::WalTooLarge);
        }
        Ok(Self {
            file,
            path,
            bytes,
            pending_records: 0,
            pending_bytes: 0,
            append_generation: 0,
            last_group_flush: std::time::Instant::now(),
            policy,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Appends one complete lifecycle record to the operating system's file
    /// buffer. It deliberately does not decide or perform group durability:
    /// callers must publish `Accepted` only after this succeeds, then run
    /// `flush_if_due` independently of that producer acknowledgement path.
    pub fn append(&mut self, frame: &IpcFrame) -> Result<(), IpcError> {
        if !frame.is_lifecycle() {
            return Err(IpcError::Invalid("wal_record_type"));
        }
        let body = frame.encode()?;
        let body_len = u16::try_from(body.len()).map_err(|_| IpcError::Oversized)?;
        let record_len = WAL_HEADER_BYTES as u64 + u64::from(body_len);
        if self.bytes.saturating_add(record_len) > MAX_WAL_BYTES {
            return Err(IpcError::WalTooLarge);
        }
        let checksum = checksum(&body);
        let mut record =
            Vec::with_capacity(usize::try_from(record_len).map_err(|_| IpcError::Oversized)?);
        record.extend_from_slice(&WAL_MAGIC);
        record.extend_from_slice(&[WAL_VERSION, 0]);
        record.extend_from_slice(&body_len.to_le_bytes());
        record.extend_from_slice(&checksum.to_le_bytes());
        record.extend_from_slice(&body);
        debug_assert_eq!(record.len() as u64, record_len);
        // One serialized OS-buffer append retains complete-record framing and
        // minimizes file-operation collisions with the durability worker.
        self.file.write_all(&record).map_err(IpcError::Io)?;
        self.bytes += record_len;
        self.pending_records += 1;
        self.pending_bytes += record_len;
        self.append_generation += 1;
        Ok(())
    }

    pub fn flush_if_due(&mut self) -> Result<WalFlush, IpcError> {
        self.flush_selected_group(false)
    }

    pub fn flush_group(&mut self) -> Result<WalFlush, IpcError> {
        self.flush_selected_group(true)
    }

    fn flush_selected_group(&mut self, force: bool) -> Result<WalFlush, IpcError> {
        let Some(request) = self.take_durability_request(force) else {
            return Ok(WalFlush::default());
        };
        self.file.sync_data().map_err(IpcError::Io)?;
        Ok(request.group)
    }

    fn take_durability_request(&mut self, force: bool) -> Option<DurabilityRequest> {
        if self.pending_records == 0 {
            return None;
        }
        let now = std::time::Instant::now();
        let record_or_byte_due = self.pending_records >= self.policy.max_records
            || self.pending_bytes >= self.policy.max_bytes;
        let interval_deadline = self.last_group_flush + self.policy.max_interval;
        let interval_due = now >= interval_deadline;
        let due = force || record_or_byte_due || interval_due;
        if !due {
            return None;
        }
        let request = DurabilityRequest {
            through_generation: self.append_generation,
            group: WalFlush {
                grouped_records: self.pending_records,
                grouped_bytes: self.pending_bytes,
            },
            coalesce_until: if force || interval_due {
                now
            } else {
                (now + DURABILITY_COALESCE_WINDOW).min(interval_deadline)
            },
        };
        self.pending_records = 0;
        self.pending_bytes = 0;
        self.last_group_flush = now;
        Some(request)
    }

    fn durability_handle(&self) -> Result<std::fs::File, IpcError> {
        // Use a distinct open-file handle so Windows does not serialize the
        // append owner behind operations on a duplicated handle. The worker is
        // sync-only; append permission is required by FlushFileBuffers.
        let path_metadata = std::fs::symlink_metadata(&self.path).map_err(IpcError::Io)?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
            return Err(IpcError::UnsafeStateObject);
        }
        let durability_handle = std::fs::OpenOptions::new()
            .append(true)
            .read(true)
            .open(&self.path)
            .map_err(IpcError::Io)?;
        let writer_identity =
            same_file::Handle::from_file(self.file.try_clone().map_err(IpcError::Io)?)
                .map_err(IpcError::Io)?;
        let durability_identity =
            same_file::Handle::from_file(durability_handle.try_clone().map_err(IpcError::Io)?)
                .map_err(IpcError::Io)?;
        if writer_identity != durability_identity {
            return Err(IpcError::UnsafeStateObject);
        }
        Ok(durability_handle)
    }

    fn append_generation(&self) -> u64 {
        self.append_generation
    }

    /// Replays whole valid records in deterministic append order. An incomplete
    /// final record is discarded by truncating only the unvalidated tail. A
    /// malformed non-tail record fails closed and is never normalized.
    pub fn recover_and_replay(&mut self) -> Result<WalRecovery, IpcError> {
        self.file.flush().map_err(IpcError::Io)?;
        let mut read = std::fs::File::open(&self.path).map_err(IpcError::Io)?;
        let file_len = read.metadata().map_err(IpcError::Io)?.len();
        let mut offset = 0_u64;
        let mut frames = Vec::new();
        let truncated_tail_bytes = loop {
            let mut header = [0_u8; WAL_HEADER_BYTES];
            match read.read_exact(&mut header) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    break file_len.saturating_sub(offset);
                }
                Err(error) => return Err(IpcError::Io(error)),
            }
            if header[..4] != WAL_MAGIC {
                return Err(IpcError::WalCorrupt("magic"));
            }
            if header[4] != WAL_VERSION || header[5] != 0 {
                return Err(IpcError::WalCorrupt("version_or_flags"));
            }
            let body_len = u16::from_le_bytes([header[6], header[7]]) as usize;
            if body_len > MAX_IPC_FRAME_BYTES {
                return Err(IpcError::WalCorrupt("record_length"));
            }
            let checksum_value = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
            let mut body = vec![0_u8; body_len];
            if let Err(error) = read.read_exact(&mut body) {
                if error.kind() == io::ErrorKind::UnexpectedEof {
                    break file_len.saturating_sub(offset);
                }
                return Err(IpcError::Io(error));
            }
            if checksum(&body) != checksum_value {
                return Err(IpcError::WalCorrupt("checksum"));
            }
            frames.push(IpcFrame::decode(&body).map_err(|_| IpcError::WalCorrupt("frame"))?);
            offset += WAL_HEADER_BYTES as u64 + body_len as u64;
        };
        if truncated_tail_bytes > 0 {
            // Windows append handles carry append-only access, which cannot
            // resize a file. Recovery is broker-startup-only, so reopen a
            // write handle solely for the validated-tail truncation.
            let truncator = std::fs::OpenOptions::new()
                .write(true)
                .open(&self.path)
                .map_err(IpcError::Io)?;
            truncator.set_len(offset).map_err(IpcError::Io)?;
            truncator.sync_data().map_err(IpcError::Io)?;
            self.bytes = offset;
        }
        Ok(WalRecovery {
            frames,
            truncated_tail_bytes,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WalRecovery {
    pub frames: Vec<IpcFrame>,
    pub truncated_tail_bytes: u64,
}

/// Fixed broker operating policy. The 64-record / 64 KiB / 50 ms group is a
/// bounded observational-loss window, never a successful-execution claim.
#[derive(Clone, Debug)]
pub struct BrokerConfig {
    pub state_root: std::path::PathBuf,
    pub queue_capacity: usize,
    pub max_connections: usize,
    pub ack_timeout: Duration,
    pub idle_timeout: Duration,
    pub group_durability: GroupDurabilityPolicy,
}

impl BrokerConfig {
    pub fn for_state_root(root: impl AsRef<std::path::Path>) -> Self {
        Self {
            state_root: root.as_ref().to_path_buf(),
            queue_capacity: 1024,
            max_connections: 128,
            ack_timeout: Duration::from_millis(5),
            idle_timeout: Duration::from_secs(60),
            group_durability: GroupDurabilityPolicy::default(),
        }
    }

    fn validate(&self) -> Result<(), IpcError> {
        if self.queue_capacity == 0
            || self.queue_capacity > 16_384
            || self.max_connections == 0
            || self.max_connections > 128
            || self.ack_timeout.is_zero()
            || self.idle_timeout.is_zero()
        {
            return Err(IpcError::Invalid("broker_config"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BrokerHealth {
    pub accepted: u64,
    pub rejected: u64,
    pub dropped: u64,
    pub malformed: u64,
    pub replayed: u64,
    pub duplicates: u64,
    pub ack_timeouts: u64,
    pub queue_high_water: u64,
    pub durability_requests: u64,
    pub durability_requests_coalesced: u64,
    pub group_flushes: u64,
    /// Failed post-ACK group durability makes the current broker fail closed
    /// for subsequent frames. Already accepted records remain truthful OS-file
    /// appends, with the documented possible observational-loss window.
    pub durability_failures: u64,
}

struct HealthCounters {
    accepted: AtomicU64,
    rejected: AtomicU64,
    dropped: AtomicU64,
    malformed: AtomicU64,
    replayed: AtomicU64,
    duplicates: AtomicU64,
    ack_timeouts: AtomicU64,
    queue_high_water: AtomicU64,
    durability_requests: AtomicU64,
    durability_requests_coalesced: AtomicU64,
    group_flushes: AtomicU64,
    durability_failures: AtomicU64,
    last_group_flush_duration_ns: AtomicU64,
}

impl HealthCounters {
    fn snapshot(&self) -> BrokerHealth {
        BrokerHealth {
            accepted: self.accepted.load(Ordering::Relaxed),
            rejected: self.rejected.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            malformed: self.malformed.load(Ordering::Relaxed),
            replayed: self.replayed.load(Ordering::Relaxed),
            duplicates: self.duplicates.load(Ordering::Relaxed),
            ack_timeouts: self.ack_timeouts.load(Ordering::Relaxed),
            queue_high_water: self.queue_high_water.load(Ordering::Relaxed),
            durability_requests: self.durability_requests.load(Ordering::Relaxed),
            durability_requests_coalesced: self
                .durability_requests_coalesced
                .load(Ordering::Relaxed),
            group_flushes: self.group_flushes.load(Ordering::Relaxed),
            durability_failures: self.durability_failures.load(Ordering::Relaxed),
        }
    }
}

struct RecentDurations {
    cursor: AtomicU64,
    values_ns: [AtomicU64; RECENT_DIAGNOSTIC_SAMPLE_CAPACITY as usize],
}

impl RecentDurations {
    fn new() -> Self {
        Self {
            cursor: AtomicU64::new(0),
            values_ns: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    fn record(&self, duration: Duration) {
        let sequence = self.cursor.fetch_add(1, Ordering::AcqRel);
        let index = (sequence % RECENT_DIAGNOSTIC_SAMPLE_CAPACITY) as usize;
        self.values_ns[index].store(duration_nanos(duration).max(1), Ordering::Release);
    }

    fn percentiles_us(&self) -> (u64, u64, u64, u64) {
        let observed = self.cursor.load(Ordering::Acquire);
        let expected = observed.min(RECENT_DIAGNOSTIC_SAMPLE_CAPACITY) as usize;
        let mut values = Vec::with_capacity(expected);
        let range = if observed <= RECENT_DIAGNOSTIC_SAMPLE_CAPACITY {
            0..expected
        } else {
            0..RECENT_DIAGNOSTIC_SAMPLE_CAPACITY as usize
        };
        for index in range {
            let value = self.values_ns[index].load(Ordering::Acquire);
            if value != 0 {
                values.push(value);
            }
        }
        values.sort_unstable();
        let percentile_us = |percent: usize| -> u64 {
            if values.is_empty() {
                return 0;
            }
            let rank = (values.len() * percent).div_ceil(100).max(1);
            values[rank - 1].div_ceil(1_000)
        };
        (
            values.len() as u64,
            percentile_us(50),
            percentile_us(95),
            percentile_us(99),
        )
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurabilityRequestStatus {
    Scheduled,
    Coalesced,
    Failed,
}

#[derive(Debug, Default)]
struct DurabilityState {
    requested_generation: u64,
    completed_generation: u64,
    unscheduled: Option<PendingDurabilityRange>,
    in_flight: Option<PendingDurabilityRange>,
    queued: Option<PendingDurabilityRange>,
    coalesce_until: Option<Instant>,
    shutting_down: bool,
    failed: bool,
}

/// A bounded oldest-pending marker. At most one range is waiting for the
/// current sync, one is in flight, and one has not yet reached the group
/// durability threshold; that is sufficient to report the exact oldest
/// append which is not yet known durable.
#[derive(Clone, Copy, Debug)]
struct PendingDurabilityRange {
    through_generation: u64,
    oldest_append_at: Instant,
}

#[derive(Debug, Default)]
struct DurabilityCoordinator {
    state: Mutex<DurabilityState>,
    wake: Condvar,
    failure_gate: Mutex<()>,
    failed: AtomicBool,
}

impl DurabilityCoordinator {
    fn record_append(&self, generation: u64, appended_at: Instant) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &mut state.unscheduled {
            Some(range) => range.through_generation = range.through_generation.max(generation),
            None => {
                state.unscheduled = Some(PendingDurabilityRange {
                    through_generation: generation,
                    oldest_append_at: appended_at,
                });
            }
        }
    }

    fn request(&self, through_generation: u64, coalesce_until: Instant) -> DurabilityRequestStatus {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.failed {
            return DurabilityRequestStatus::Failed;
        }
        let status = if state.requested_generation > state.completed_generation {
            DurabilityRequestStatus::Coalesced
        } else {
            DurabilityRequestStatus::Scheduled
        };
        if let Some(range) = state.unscheduled.take() {
            debug_assert!(range.through_generation <= through_generation);
            match &mut state.queued {
                Some(queued) => {
                    queued.through_generation = queued.through_generation.max(through_generation);
                }
                None => {
                    state.queued = Some(PendingDurabilityRange {
                        through_generation,
                        oldest_append_at: range.oldest_append_at,
                    });
                }
            }
        }
        state.requested_generation = state.requested_generation.max(through_generation);
        state.coalesce_until = Some(
            state
                .coalesce_until
                .map_or(coalesce_until, |existing| existing.min(coalesce_until)),
        );
        self.wake.notify_one();
        status
    }

    fn shutdown_and_wait(&self, through_generation: u64) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.requested_generation = state.requested_generation.max(through_generation);
        state.coalesce_until = None;
        state.shutting_down = true;
        self.wake.notify_one();
        while !state.failed && state.completed_generation < through_generation {
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        !state.failed && state.completed_generation >= through_generation
    }

    fn next_request(&self) -> Option<u64> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if state.failed {
                return None;
            }
            if state.requested_generation > state.completed_generation {
                if let Some(deadline) = state.coalesce_until {
                    let now = Instant::now();
                    if now < deadline && !state.shutting_down {
                        let (next_state, _) = self
                            .wake
                            .wait_timeout(state, deadline.duration_since(now))
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        state = next_state;
                        continue;
                    }
                }
                state.coalesce_until = None;
                let range = state
                    .queued
                    .take()
                    .expect("requested durability range is tracked");
                state.in_flight = Some(range);
                return Some(range.through_generation);
            }
            if state.shutting_down {
                return None;
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    fn complete(&self, through_generation: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.completed_generation = state.completed_generation.max(through_generation);
        if let Some(range) = state.in_flight.take() {
            debug_assert!(range.through_generation <= through_generation);
        }
        self.wake.notify_all();
    }

    fn pending_wal_flush_lag_ms(&self) -> Option<u64> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        [state.unscheduled, state.in_flight, state.queued]
            .into_iter()
            .flatten()
            .map(|range| {
                u64::try_from(range.oldest_append_at.elapsed().as_millis()).unwrap_or(u64::MAX)
            })
            .max()
    }

    fn publish_failure(&self) {
        let _failure_gate = self
            .failure_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.failed.store(true, Ordering::Release);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.failed = true;
        self.wake.notify_all();
    }
}

/// Per-request broker stages collected only by the feature-gated development
/// diagnostic. Each field is a duration in nanoseconds and contains no frame
/// or environment data.
#[cfg(feature = "performance-harness")]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct QualificationBrokerStageSample {
    pub broker_read_decode_ns: u64,
    pub activity_bookkeeping_ns: u64,
    pub acknowledgement_channel_allocation_ns: u64,
    pub queue_submission_ns: u64,
    pub queue_wait_ns: u64,
    pub queue_depth_at_dequeue: u64,
    pub queue_wait_group_sync_overlap_ns: u64,
    pub worker_dequeue_handoff_ns: u64,
    pub wal_append_ns: u64,
    pub worker_acknowledgement_handoff_ns: u64,
    pub connection_resume_after_ack_ns: u64,
    pub broker_ack_write_ns: u64,
}

#[cfg(feature = "performance-harness")]
struct QualificationStageCollector {
    samples: Mutex<Vec<QualificationBrokerStageSample>>,
    group_sync_durations_ns: Mutex<Vec<u64>>,
    last_group_sync_interval: Mutex<Option<(Instant, Option<Instant>)>>,
}

#[cfg(feature = "performance-harness")]
impl QualificationStageCollector {
    fn new() -> Self {
        Self {
            samples: Mutex::new(Vec::new()),
            group_sync_durations_ns: Mutex::new(Vec::new()),
            last_group_sync_interval: Mutex::new(None),
        }
    }

    fn record(&self, sample: QualificationBrokerStageSample) {
        if let Ok(mut samples) = self.samples.lock() {
            samples.push(sample);
        }
    }

    fn samples(&self) -> Vec<QualificationBrokerStageSample> {
        self.samples
            .lock()
            .map(|samples| samples.clone())
            .unwrap_or_default()
    }

    fn start_group_sync(&self, started: Instant) {
        if let Ok(mut interval) = self.last_group_sync_interval.lock() {
            *interval = Some((started, None));
        }
    }

    fn record_group_sync(&self, started: Instant, completed: Instant) {
        if let Ok(mut durations) = self.group_sync_durations_ns.lock() {
            durations.push(elapsed_nanos_between(started, completed));
        }
        if let Ok(mut interval) = self.last_group_sync_interval.lock() {
            *interval = Some((started, Some(completed)));
        }
    }

    fn group_sync_durations(&self) -> Vec<u64> {
        self.group_sync_durations_ns
            .lock()
            .map(|durations| durations.clone())
            .unwrap_or_default()
    }

    fn group_sync_overlap_ns(&self, enqueued_at: Instant, dequeued_at: Instant) -> u64 {
        self.last_group_sync_interval
            .lock()
            .ok()
            .and_then(|interval| *interval)
            .map(|(started, completed)| {
                interval_overlap_nanos(
                    enqueued_at,
                    dequeued_at,
                    started,
                    completed.unwrap_or(dequeued_at),
                )
            })
            .unwrap_or_default()
    }
}

#[cfg(feature = "performance-harness")]
struct QualificationRequestTiming {
    origin: Instant,
    broker_read_decode_ns: AtomicU64,
    activity_bookkeeping_ns: AtomicU64,
    acknowledgement_channel_allocation_ns: AtomicU64,
    queue_submission_ns: AtomicU64,
    queue_wait_ns: AtomicU64,
    queue_depth_at_dequeue: AtomicU64,
    queue_wait_group_sync_overlap_ns: AtomicU64,
    worker_dequeue_handoff_ns: AtomicU64,
    wal_append_ns: AtomicU64,
    worker_acknowledgement_handoff_ns: AtomicU64,
    worker_acknowledgement_ready_ns: AtomicU64,
    connection_resume_after_ack_ns: AtomicU64,
    broker_ack_write_ns: AtomicU64,
}

#[cfg(feature = "performance-harness")]
impl QualificationRequestTiming {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            broker_read_decode_ns: AtomicU64::new(0),
            activity_bookkeeping_ns: AtomicU64::new(0),
            acknowledgement_channel_allocation_ns: AtomicU64::new(0),
            queue_submission_ns: AtomicU64::new(0),
            queue_wait_ns: AtomicU64::new(0),
            queue_depth_at_dequeue: AtomicU64::new(0),
            queue_wait_group_sync_overlap_ns: AtomicU64::new(0),
            worker_dequeue_handoff_ns: AtomicU64::new(0),
            wal_append_ns: AtomicU64::new(0),
            worker_acknowledgement_handoff_ns: AtomicU64::new(0),
            worker_acknowledgement_ready_ns: AtomicU64::new(0),
            connection_resume_after_ack_ns: AtomicU64::new(0),
            broker_ack_write_ns: AtomicU64::new(0),
        }
    }

    fn store(slot: &AtomicU64, value: u64) {
        slot.store(value, Ordering::Release);
    }

    fn sample(&self) -> QualificationBrokerStageSample {
        QualificationBrokerStageSample {
            broker_read_decode_ns: self.broker_read_decode_ns.load(Ordering::Acquire),
            activity_bookkeeping_ns: self.activity_bookkeeping_ns.load(Ordering::Acquire),
            acknowledgement_channel_allocation_ns: self
                .acknowledgement_channel_allocation_ns
                .load(Ordering::Acquire),
            queue_submission_ns: self.queue_submission_ns.load(Ordering::Acquire),
            queue_wait_ns: self.queue_wait_ns.load(Ordering::Acquire),
            queue_depth_at_dequeue: self.queue_depth_at_dequeue.load(Ordering::Acquire),
            queue_wait_group_sync_overlap_ns: self
                .queue_wait_group_sync_overlap_ns
                .load(Ordering::Acquire),
            worker_dequeue_handoff_ns: self.worker_dequeue_handoff_ns.load(Ordering::Acquire),
            wal_append_ns: self.wal_append_ns.load(Ordering::Acquire),
            worker_acknowledgement_handoff_ns: self
                .worker_acknowledgement_handoff_ns
                .load(Ordering::Acquire),
            connection_resume_after_ack_ns: self
                .connection_resume_after_ack_ns
                .load(Ordering::Acquire),
            broker_ack_write_ns: self.broker_ack_write_ns.load(Ordering::Acquire),
        }
    }
}

struct QueuedFrame {
    frame: IpcFrame,
    acknowledgement: mpsc::SyncSender<BrokerAcknowledgement>,
    enqueued_at: Instant,
    #[cfg(feature = "performance-harness")]
    timing: Option<Arc<QualificationRequestTiming>>,
}

struct BrokerCore {
    queue: mpsc::SyncSender<QueuedFrame>,
    queue_depth: AtomicUsize,
    active_connections: AtomicUsize,
    ack_timeout: Duration,
    health: Arc<HealthCounters>,
    durability: Arc<DurabilityCoordinator>,
    last_activity: Mutex<Instant>,
    recent_ipc_latency: RecentDurations,
    recent_queue_wait: RecentDurations,
    #[cfg(feature = "performance-harness")]
    qualification_stage_collector: Option<Arc<QualificationStageCollector>>,
}

impl BrokerCore {
    fn diagnostics(&self) -> BrokerDiagnostics {
        let health = self.health.snapshot();
        let (samples, latency_p50_us, latency_p95_us, latency_p99_us) =
            self.recent_ipc_latency.percentiles_us();
        let (_, _, queue_wait_p95_us, _) = self.recent_queue_wait.percentiles_us();
        let last_flush_duration_ns = self
            .health
            .last_group_flush_duration_ns
            .load(Ordering::Acquire);
        BrokerDiagnostics {
            schema_version: BROKER_DIAGNOSTICS_SCHEMA_VERSION,
            accepted: health.accepted,
            rejected: health.rejected,
            dropped: health.dropped,
            malformed: health.malformed,
            replayed: health.replayed,
            duplicates: health.duplicates,
            ack_timeouts: health.ack_timeouts,
            queue_depth: self.queue_depth.load(Ordering::Acquire) as u64,
            active_connections: self.active_connections.load(Ordering::Acquire) as u64,
            queue_high_water: health.queue_high_water,
            durability_requests: health.durability_requests,
            durability_requests_coalesced: health.durability_requests_coalesced,
            group_flushes: health.group_flushes,
            durability_failures: health.durability_failures,
            recent_ipc_latency_samples: samples,
            recent_ipc_latency_p50_us: latency_p50_us,
            recent_ipc_latency_p95_us: latency_p95_us,
            recent_ipc_latency_p99_us: latency_p99_us,
            recent_queue_wait_p95_us: queue_wait_p95_us,
            wal_flush_lag_ms: self.durability.pending_wal_flush_lag_ms(),
            last_wal_flush_duration_us: (last_flush_duration_ns != 0)
                .then(|| last_flush_duration_ns.div_ceil(1_000)),
        }
    }

    #[cfg(not(feature = "performance-harness"))]
    fn submit(&self, frame: IpcFrame) -> BrokerAcknowledgement {
        let (acknowledgement, receiver) = mpsc::sync_channel(1);
        let queued = QueuedFrame {
            frame,
            acknowledgement,
            enqueued_at: Instant::now(),
        };
        // Increment before publishing to the worker: an immediate consumer
        // must never observe/decrement an unaccounted queue entry.
        let depth = self.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        match self.queue.try_send(queued) {
            Ok(()) => {
                self.health
                    .queue_high_water
                    .fetch_max(depth as u64, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.dropped.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::DroppedOverloaded;
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.rejected.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::Rejected;
            }
        }
        match receiver.recv_timeout(self.ack_timeout) {
            Ok(value) => value,
            Err(_) => {
                self.health.ack_timeouts.fetch_add(1, Ordering::Relaxed);
                BrokerAcknowledgement::Busy
            }
        }
    }

    #[cfg(feature = "performance-harness")]
    fn submit(&self, frame: IpcFrame) -> BrokerAcknowledgement {
        let (acknowledgement, receiver) = mpsc::sync_channel(1);
        let queued = QueuedFrame {
            frame,
            acknowledgement,
            enqueued_at: Instant::now(),
            timing: None,
        };
        // Increment before publishing to the worker: an immediate consumer
        // must never observe/decrement an unaccounted queue entry.
        let depth = self.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        match self.queue.try_send(queued) {
            Ok(()) => {
                self.health
                    .queue_high_water
                    .fetch_max(depth as u64, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.dropped.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::DroppedOverloaded;
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.rejected.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::Rejected;
            }
        }
        match receiver.recv_timeout(self.ack_timeout) {
            Ok(value) => value,
            Err(_) => {
                self.health.ack_timeouts.fetch_add(1, Ordering::Relaxed);
                BrokerAcknowledgement::Busy
            }
        }
    }

    #[cfg(feature = "performance-harness")]
    fn submit_for_qualification(
        &self,
        frame: IpcFrame,
        timing: Arc<QualificationRequestTiming>,
    ) -> BrokerAcknowledgement {
        self.submit_with_qualification_timing(frame, Some(timing))
    }

    #[cfg(feature = "performance-harness")]
    fn submit_with_qualification_timing(
        &self,
        frame: IpcFrame,
        timing: Option<Arc<QualificationRequestTiming>>,
    ) -> BrokerAcknowledgement {
        let channel_started = Instant::now();
        let (acknowledgement, receiver) = mpsc::sync_channel(1);
        if let Some(timing) = &timing {
            QualificationRequestTiming::store(
                &timing.acknowledgement_channel_allocation_ns,
                elapsed_nanos(channel_started),
            );
        }
        let queued = QueuedFrame {
            frame,
            acknowledgement,
            enqueued_at: Instant::now(),
            timing: timing.clone(),
        };
        let submission_started = Instant::now();
        // Increment before publishing to the worker: an immediate consumer
        // must never observe/decrement an unaccounted queue entry.
        let depth = self.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        match self.queue.try_send(queued) {
            Ok(()) => {
                if let Some(timing) = &timing {
                    QualificationRequestTiming::store(
                        &timing.queue_submission_ns,
                        elapsed_nanos(submission_started),
                    );
                }
                self.health
                    .queue_high_water
                    .fetch_max(depth as u64, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.dropped.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::DroppedOverloaded;
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.queue_depth.fetch_sub(1, Ordering::Relaxed);
                self.health.rejected.fetch_add(1, Ordering::Relaxed);
                return BrokerAcknowledgement::Rejected;
            }
        }
        match receiver.recv_timeout(self.ack_timeout) {
            Ok(value) => {
                if let Some(timing) = &timing {
                    let ready = timing
                        .worker_acknowledgement_ready_ns
                        .load(Ordering::Acquire);
                    QualificationRequestTiming::store(
                        &timing.connection_resume_after_ack_ns,
                        elapsed_nanos(timing.origin).saturating_sub(ready),
                    );
                }
                value
            }
            Err(_) => {
                self.health.ack_timeouts.fetch_add(1, Ordering::Relaxed);
                BrokerAcknowledgement::Busy
            }
        }
    }

    fn touch(&self) {
        *self.last_activity.lock().expect("broker activity lock") = Instant::now();
    }

    #[cfg(feature = "performance-harness")]
    fn touch_for_qualification(&self, timing: &QualificationRequestTiming) {
        let started = Instant::now();
        self.touch();
        QualificationRequestTiming::store(&timing.activity_bookkeeping_ns, elapsed_nanos(started));
    }
}

/// Safe, bounded broker host. It has no runtime policy, analytics, trust, or
/// process-execution responsibility. Dropped frames are explicit to clients
/// and in `BrokerHealth` rather than fabricated as success.
pub struct BrokerHost {
    endpoint: LocalEndpoint,
    health: Arc<HealthCounters>,
    stopping: Arc<AtomicBool>,
    handles: Vec<thread::JoinHandle<()>>,
    recovery: BrokerRecovery,
    #[cfg(feature = "performance-harness")]
    qualification_stage_collector: Option<Arc<QualificationStageCollector>>,
}

impl BrokerHost {
    pub fn start(config: BrokerConfig) -> Result<Self, IpcError> {
        config.validate()?;
        let wal = Wal::open(&config.state_root, config.group_durability)?;
        Self::start_with_wal(config, wal)
    }

    fn start_with_wal(config: BrokerConfig, wal: Wal) -> Result<Self, IpcError> {
        Self::start_with_wal_with_qualification_timing(
            config,
            wal,
            #[cfg(feature = "performance-harness")]
            None,
            #[cfg(test)]
            None,
        )
    }

    fn start_with_wal_with_qualification_timing(
        config: BrokerConfig,
        mut wal: Wal,
        #[cfg(feature = "performance-harness")] qualification_stage_collector: Option<
            Arc<QualificationStageCollector>,
        >,
        #[cfg(test)] test_group_sync: Option<Arc<TestGroupSync>>,
    ) -> Result<Self, IpcError> {
        let endpoint = LocalEndpoint::from_state_root(&config.state_root)?;
        let recovery = BrokerRecovery::from_wal(wal.recover_and_replay()?);
        let durability_handle = wal.durability_handle()?;
        let listener = endpoint.bind()?;
        let (queue_sender, queue_receiver) = mpsc::sync_channel(config.queue_capacity);
        let health = Arc::new(HealthCounters {
            accepted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            malformed: AtomicU64::new(0),
            replayed: AtomicU64::new(recovery.frames.len() as u64),
            duplicates: AtomicU64::new(0),
            ack_timeouts: AtomicU64::new(0),
            queue_high_water: AtomicU64::new(0),
            durability_requests: AtomicU64::new(0),
            durability_requests_coalesced: AtomicU64::new(0),
            group_flushes: AtomicU64::new(0),
            durability_failures: AtomicU64::new(0),
            last_group_flush_duration_ns: AtomicU64::new(0),
        });
        let durability = Arc::new(DurabilityCoordinator::default());
        let core = Arc::new(BrokerCore {
            queue: queue_sender,
            queue_depth: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
            ack_timeout: config.ack_timeout,
            health: Arc::clone(&health),
            durability: Arc::clone(&durability),
            last_activity: Mutex::new(Instant::now()),
            recent_ipc_latency: RecentDurations::new(),
            recent_queue_wait: RecentDurations::new(),
            #[cfg(feature = "performance-harness")]
            qualification_stage_collector: qualification_stage_collector.clone(),
        });
        let stopping = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        {
            let health = Arc::clone(&health);
            let core = Arc::clone(&core);
            let stopping = Arc::clone(&stopping);
            let durability = Arc::clone(&durability);
            handles.push(thread::spawn(move || {
                wal_worker_loop(&mut wal, queue_receiver, core, health, stopping, durability)
            }));
        }
        {
            let health = Arc::clone(&health);
            let stopping = Arc::clone(&stopping);
            let durability = Arc::clone(&durability);
            #[cfg(feature = "performance-harness")]
            let qualification_stage_collector = qualification_stage_collector.clone();
            handles.push(thread::spawn(move || {
                durability_worker_loop(
                    durability_handle,
                    durability,
                    health,
                    stopping,
                    #[cfg(feature = "performance-harness")]
                    qualification_stage_collector,
                    #[cfg(test)]
                    test_group_sync,
                )
            }));
        }
        {
            let core = Arc::clone(&core);
            let health = Arc::clone(&health);
            let stopping = Arc::clone(&stopping);
            let endpoint_for_drop = endpoint.clone();
            handles.push(thread::spawn(move || {
                accept_loop(
                    listener,
                    core,
                    health,
                    stopping,
                    config.idle_timeout,
                    config.max_connections,
                    endpoint_for_drop,
                )
            }));
        }
        Ok(Self {
            endpoint,
            health,
            stopping,
            handles,
            recovery,
            #[cfg(feature = "performance-harness")]
            qualification_stage_collector,
        })
    }

    #[cfg(feature = "performance-harness")]
    pub(crate) fn start_with_qualification_stage_timing(
        config: BrokerConfig,
    ) -> Result<Self, IpcError> {
        config.validate()?;
        let wal = Wal::open(&config.state_root, config.group_durability)?;
        let collector = Arc::new(QualificationStageCollector::new());
        Self::start_with_wal_with_qualification_timing(
            config,
            wal,
            Some(collector),
            #[cfg(test)]
            None,
        )
    }

    #[cfg(test)]
    fn start_with_test_group_sync(
        config: BrokerConfig,
        sync: Arc<TestGroupSync>,
    ) -> Result<Self, IpcError> {
        config.validate()?;
        let wal = Wal::open(&config.state_root, config.group_durability)?;
        Self::start_with_wal_with_qualification_timing(
            config,
            wal,
            #[cfg(feature = "performance-harness")]
            None,
            Some(sync),
        )
    }

    pub fn endpoint(&self) -> &LocalEndpoint {
        &self.endpoint
    }
    pub fn health(&self) -> BrokerHealth {
        self.health.snapshot()
    }
    pub fn recovery(&self) -> &BrokerRecovery {
        &self.recovery
    }

    #[cfg(feature = "performance-harness")]
    pub(crate) fn qualification_stage_samples(&self) -> Vec<QualificationBrokerStageSample> {
        self.qualification_stage_collector
            .as_ref()
            .map(|collector| collector.samples())
            .unwrap_or_default()
    }

    #[cfg(feature = "performance-harness")]
    pub(crate) fn qualification_group_sync_durations(&self) -> Vec<u64> {
        self.qualification_stage_collector
            .as_ref()
            .map(|collector| collector.group_sync_durations())
            .unwrap_or_default()
    }

    pub fn stop(mut self) -> BrokerHealth {
        self.stopping.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
        #[cfg(unix)]
        self.endpoint.remove_socket_if_owned();
        self.health.snapshot()
    }

    pub fn wait_for_idle(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.stopping.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        if !self.stopping.load(Ordering::Acquire) {
            return false;
        }
        let handles = std::mem::take(&mut self.handles);
        for handle in handles {
            let _ = handle.join();
        }
        #[cfg(unix)]
        self.endpoint.remove_socket_if_owned();
        true
    }
}

impl Drop for BrokerHost {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
        #[cfg(unix)]
        self.endpoint.remove_socket_if_owned();
    }
}

fn wal_worker_loop(
    wal: &mut Wal,
    receiver: mpsc::Receiver<QueuedFrame>,
    core: Arc<BrokerCore>,
    health: Arc<HealthCounters>,
    stopping: Arc<AtomicBool>,
    durability: Arc<DurabilityCoordinator>,
) {
    loop {
        #[cfg(feature = "performance-harness")]
        let worker_available_at = Instant::now();
        match receiver.recv_timeout(Duration::from_millis(2)) {
            Ok(queued) => {
                let dequeued_at = Instant::now();
                let _depth_at_dequeue = core.queue_depth.fetch_sub(1, Ordering::Relaxed);
                core.recent_queue_wait
                    .record(dequeued_at.saturating_duration_since(queued.enqueued_at));
                #[cfg(feature = "performance-harness")]
                if let Some(timing) = &queued.timing {
                    QualificationRequestTiming::store(
                        &timing.queue_wait_ns,
                        elapsed_nanos_between(queued.enqueued_at, dequeued_at),
                    );
                    QualificationRequestTiming::store(
                        &timing.queue_depth_at_dequeue,
                        u64::try_from(_depth_at_dequeue).unwrap_or(u64::MAX),
                    );
                    QualificationRequestTiming::store(
                        &timing.worker_dequeue_handoff_ns,
                        elapsed_nanos_between(
                            worker_available_at.max(queued.enqueued_at),
                            dequeued_at,
                        ),
                    );
                    QualificationRequestTiming::store(
                        &timing.queue_wait_group_sync_overlap_ns,
                        core.qualification_stage_collector
                            .as_ref()
                            .map(|collector| {
                                collector.group_sync_overlap_ns(queued.enqueued_at, dequeued_at)
                            })
                            .unwrap_or_default(),
                    );
                }
                let appended = {
                    let _failure_gate = durability
                        .failure_gate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if durability.failed.load(Ordering::Acquire) {
                        health.rejected.fetch_add(1, Ordering::Relaxed);
                        let _ = queued.acknowledgement.send(BrokerAcknowledgement::Rejected);
                        false
                    } else {
                        #[cfg(feature = "performance-harness")]
                        let append_started = queued.timing.as_ref().map(|_| Instant::now());
                        match wal.append(&queued.frame) {
                            Ok(()) => {
                                durability.record_append(wal.append_generation(), Instant::now());
                                #[cfg(feature = "performance-harness")]
                                if let (Some(timing), Some(append_started)) =
                                    (&queued.timing, append_started)
                                {
                                    QualificationRequestTiming::store(
                                        &timing.wal_append_ns,
                                        elapsed_nanos(append_started),
                                    );
                                }
                                health.accepted.fetch_add(1, Ordering::Relaxed);
                                // The complete record is in the OS file buffer
                                // before this send. Physical group durability
                                // has one separate owner and cannot delay this
                                // or a subsequent append-worker dequeue.
                                #[cfg(feature = "performance-harness")]
                                let handoff_started =
                                    queued.timing.as_ref().map(|_| Instant::now());
                                #[cfg(feature = "performance-harness")]
                                if let Some(timing) = &queued.timing {
                                    QualificationRequestTiming::store(
                                        &timing.worker_acknowledgement_ready_ns,
                                        elapsed_nanos(timing.origin),
                                    );
                                }
                                let _ =
                                    queued.acknowledgement.send(BrokerAcknowledgement::Accepted);
                                #[cfg(feature = "performance-harness")]
                                if let (Some(timing), Some(handoff_started)) =
                                    (&queued.timing, handoff_started)
                                {
                                    QualificationRequestTiming::store(
                                        &timing.worker_acknowledgement_handoff_ns,
                                        elapsed_nanos(handoff_started),
                                    );
                                }
                                true
                            }
                            Err(_) => {
                                health.rejected.fetch_add(1, Ordering::Relaxed);
                                let _ =
                                    queued.acknowledgement.send(BrokerAcknowledgement::Rejected);
                                false
                            }
                        }
                    }
                };
                if appended {
                    schedule_wal_durability(wal, false, &durability, &health);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                schedule_wal_durability(wal, false, &durability, &health);
                if stopping.load(Ordering::Acquire)
                    && core.queue_depth.load(Ordering::Acquire) == 0
                    && core.active_connections.load(Ordering::Acquire) == 0
                {
                    schedule_wal_durability(wal, true, &durability, &health);
                    let _ = durability.shutdown_and_wait(wal.append_generation());
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                schedule_wal_durability(wal, true, &durability, &health);
                let _ = durability.shutdown_and_wait(wal.append_generation());
                break;
            }
        }
    }
}

fn schedule_wal_durability(
    wal: &mut Wal,
    force: bool,
    durability: &DurabilityCoordinator,
    health: &HealthCounters,
) {
    let Some(request) = wal.take_durability_request(force) else {
        return;
    };
    health.durability_requests.fetch_add(1, Ordering::Relaxed);
    if durability.request(request.through_generation, request.coalesce_until)
        == DurabilityRequestStatus::Coalesced
    {
        health
            .durability_requests_coalesced
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn durability_worker_loop(
    durability_handle: std::fs::File,
    durability: Arc<DurabilityCoordinator>,
    health: Arc<HealthCounters>,
    stopping: Arc<AtomicBool>,
    #[cfg(feature = "performance-harness")] qualification_stage_collector: Option<
        Arc<QualificationStageCollector>,
    >,
    #[cfg(test)] test_group_sync: Option<Arc<TestGroupSync>>,
) {
    while let Some(through_generation) = durability.next_request() {
        let sync_started = Instant::now();
        #[cfg(feature = "performance-harness")]
        if let Some(collector) = &qualification_stage_collector {
            collector.start_group_sync(sync_started);
        }
        #[cfg(test)]
        let before_sync = test_group_sync
            .as_ref()
            .map_or(Ok(()), |sync| sync.wait_before_sync());
        #[cfg(not(test))]
        let before_sync: Result<(), IpcError> = Ok(());
        let result = before_sync.and_then(|()| durability_handle.sync_data().map_err(IpcError::Io));
        #[cfg(feature = "performance-harness")]
        if let Some(collector) = &qualification_stage_collector {
            collector.record_group_sync(sync_started, Instant::now());
        }
        match result {
            Ok(()) => {
                health.last_group_flush_duration_ns.store(
                    duration_nanos(sync_started.elapsed()).max(1),
                    Ordering::Release,
                );
                #[cfg(test)]
                if let Some(sync) = &test_group_sync {
                    sync.mark_completed();
                }
                health.group_flushes.fetch_add(1, Ordering::Relaxed);
                durability.complete(through_generation);
            }
            Err(_) => {
                health.durability_failures.fetch_add(1, Ordering::Relaxed);
                durability.publish_failure();
                stopping.store(true, Ordering::Release);
                break;
            }
        }
    }
}

#[cfg(feature = "performance-harness")]
fn elapsed_nanos_between(started: Instant, completed: Instant) -> u64 {
    u64::try_from(completed.saturating_duration_since(started).as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(feature = "performance-harness")]
fn interval_overlap_nanos(
    first_started: Instant,
    first_completed: Instant,
    second_started: Instant,
    second_completed: Instant,
) -> u64 {
    let overlap_started = first_started.max(second_started);
    let overlap_completed = first_completed.min(second_completed);
    elapsed_nanos_between(overlap_started, overlap_completed)
}

fn accept_loop(
    listener: Listener,
    core: Arc<BrokerCore>,
    health: Arc<HealthCounters>,
    stopping: Arc<AtomicBool>,
    idle_timeout: Duration,
    max_connections: usize,
    _endpoint: LocalEndpoint,
) {
    while !stopping.load(Ordering::Acquire) {
        match listener.accept() {
            Ok(stream) => {
                let admitted = core.active_connections.fetch_update(
                    Ordering::AcqRel,
                    Ordering::Acquire,
                    |current| (current < max_connections).then_some(current + 1),
                );
                if admitted.is_err() {
                    health.dropped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                let core = Arc::clone(&core);
                let health = Arc::clone(&health);
                let stopping = Arc::clone(&stopping);
                thread::spawn(move || connection_loop(stream, core, health, stopping));
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1))
            }
            Err(_) => {
                health.rejected.fetch_add(1, Ordering::Relaxed);
                thread::sleep(Duration::from_millis(1));
            }
        }
        if core
            .last_activity
            .lock()
            .expect("broker activity lock")
            .elapsed()
            >= idle_timeout
        {
            stopping.store(true, Ordering::Release);
        }
    }
    #[cfg(unix)]
    _endpoint.remove_socket_if_owned();
}

fn connection_loop(
    mut stream: Stream,
    core: Arc<BrokerCore>,
    health: Arc<HealthCounters>,
    stopping: Arc<AtomicBool>,
) {
    if stream.set_nonblocking(true).is_err() {
        health.rejected.fetch_add(1, Ordering::Relaxed);
        core.active_connections.fetch_sub(1, Ordering::AcqRel);
        return;
    }
    #[cfg(feature = "performance-harness")]
    let stage_collector = core.qualification_stage_collector.as_ref().cloned();
    let mut idle_read_window = CONNECTION_IDLE_READ_WINDOW;
    while !stopping.load(Ordering::Acquire) {
        #[cfg(feature = "performance-harness")]
        let read_started = stage_collector.as_ref().map(|_| Instant::now());
        match read_frame_bounded(&mut stream, idle_read_window) {
            Ok(frame) if frame.is_lifecycle() => {
                let handling_started = Instant::now();
                #[cfg(feature = "performance-harness")]
                let timing = stage_collector
                    .as_ref()
                    .map(|_| Arc::new(QualificationRequestTiming::new()));
                #[cfg(feature = "performance-harness")]
                if let (Some(timing), Some(read_started)) = (&timing, read_started) {
                    QualificationRequestTiming::store(
                        &timing.broker_read_decode_ns,
                        elapsed_nanos(read_started),
                    );
                    core.touch_for_qualification(timing);
                } else {
                    core.touch();
                }
                #[cfg(not(feature = "performance-harness"))]
                core.touch();
                #[cfg(feature = "performance-harness")]
                let acknowledgement = match &timing {
                    Some(timing) => core.submit_for_qualification(frame, Arc::clone(timing)),
                    None => core.submit(frame),
                };
                #[cfg(not(feature = "performance-harness"))]
                let acknowledgement = core.submit(frame);
                #[cfg(feature = "performance-harness")]
                let acknowledgement_write_started = timing.as_ref().map(|_| Instant::now());
                let _ = write_frame_bounded(
                    &mut stream,
                    &IpcFrame::Ack(acknowledgement),
                    Duration::from_millis(5),
                );
                core.recent_ipc_latency.record(handling_started.elapsed());
                #[cfg(feature = "performance-harness")]
                if let (Some(collector), Some(timing), Some(acknowledgement_write_started)) = (
                    stage_collector.as_ref(),
                    timing,
                    acknowledgement_write_started,
                ) {
                    QualificationRequestTiming::store(
                        &timing.broker_ack_write_ns,
                        elapsed_nanos(acknowledgement_write_started),
                    );
                    collector.record(timing.sample());
                }
                idle_read_window = CONNECTION_IDLE_READ_WINDOW;
            }
            Ok(IpcFrame::BrokerDiagnosticsRequest) => {
                // A read-only control query must not keep this on-demand
                // broker alive. Only lifecycle evidence extends the idle
                // lease; otherwise a polling doctor or TUI would turn the
                // broker into an accidental service.
                let response = core.diagnostics();
                let _ = write_frame_bounded(
                    &mut stream,
                    &IpcFrame::BrokerDiagnosticsResponse(response),
                    Duration::from_millis(5),
                );
                idle_read_window = DIAGNOSTICS_IDLE_READ_WINDOW;
            }
            Err(IpcError::Io(error)) if error.kind() == io::ErrorKind::TimedOut => {
                // The producer never reuses an acknowledged connection past
                // 25 ms. A peer that has disappeared can still surface as a
                // timeout on Windows, so release this bounded 250 ms lifecycle
                // slot and let a later lifecycle frame reconnect. No original
                // Hook lifetime is retained by the broker.
                break;
            }
            Err(IpcError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Ok(_) | Err(_) => {
                health.malformed.fetch_add(1, Ordering::Relaxed);
                let _ = write_frame_bounded(
                    &mut stream,
                    &IpcFrame::Ack(BrokerAcknowledgement::Rejected),
                    Duration::from_millis(5),
                );
                break;
            }
        }
    }
    core.active_connections.fetch_sub(1, Ordering::AcqRel);
}

/// A recovered WAL snapshot has no runtime-specific policy. The caller selects
/// the already accepted G29 authority table before replaying it into a core.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BrokerRecovery {
    pub frames: Vec<IpcFrame>,
    pub truncated_tail_bytes: u64,
}

impl BrokerRecovery {
    fn from_wal(recovery: WalRecovery) -> Self {
        Self {
            frames: recovery.frames,
            truncated_tail_bytes: recovery.truncated_tail_bytes,
        }
    }

    pub fn canonical_evidence(&self) -> Result<Vec<CanonicalEvidence>, IpcError> {
        self.frames.iter().map(canonical).collect()
    }

    /// Replays through the accepted G29 core. It intentionally cannot resolve
    /// a runtime handler identity or write a ledger row; that remains a
    /// runtime integration responsibility.
    pub fn ingest_into(
        &self,
        core: &mut RuntimeNeutralEvidenceCore,
    ) -> Result<ReplayIngest, IpcError> {
        let mut result = ReplayIngest::default();
        for evidence in self.canonical_evidence()? {
            match core
                .ingest(evidence)
                .map_err(|_| IpcError::Invalid("canonical_evidence"))?
            {
                CoreIngestOutcome::Produced(_) => result.produced += 1,
                CoreIngestOutcome::Duplicate => result.duplicates += 1,
                CoreIngestOutcome::Shadow => result.shadowed += 1,
                CoreIngestOutcome::NotAdmitted => result.not_admitted += 1,
                CoreIngestOutcome::Unconfigured => result.unconfigured += 1,
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReplayIngest {
    pub produced: u64,
    pub duplicates: u64,
    pub shadowed: u64,
    pub not_admitted: u64,
    pub unconfigured: u64,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestSyncOutcome {
    Succeed,
    Fail,
}

#[cfg(test)]
struct TestGroupSync {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<TestSyncOutcome>>,
    completed: AtomicU64,
}

#[cfg(test)]
impl TestGroupSync {
    fn wait_before_sync(&self) -> Result<(), IpcError> {
        self.entered
            .send(())
            .map_err(|_| IpcError::Io(io::Error::other("test sync observer unavailable")))?;
        match self
            .release
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .recv()
            .map_err(|_| IpcError::Io(io::Error::other("test sync permit unavailable")))?
        {
            TestSyncOutcome::Succeed => Ok(()),
            TestSyncOutcome::Fail => Err(IpcError::Io(io::Error::other(
                "injected durability failure",
            ))),
        }
    }

    fn mark_completed(&self) {
        self.completed.fetch_add(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lifecycle() -> LifecycleFrame {
        LifecycleFrame {
            runtime: "synthetic_runtime".into(),
            runtime_instance: "instance_a".into(),
            invocation: "invocation_a".into(),
            handler: "handler_a".into(),
            event: "event_a".into(),
            source_scope: "scope_a".into(),
            revision: Some("revision_a".into()),
            occurred_at_unix_ms: 1_000,
        }
    }

    fn complete() -> IpcFrame {
        IpcFrame::Complete {
            lifecycle: lifecycle(),
            completion: Completion {
                terminal_status: TerminalOutcome::Failed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(7),
                duration_ms: 12,
            },
        }
    }

    fn controlled_group_sync() -> (
        Arc<TestGroupSync>,
        mpsc::Receiver<()>,
        mpsc::Sender<TestSyncOutcome>,
    ) {
        let (entered_sender, entered_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        (
            Arc::new(TestGroupSync {
                entered: entered_sender,
                release: Mutex::new(release_receiver),
                completed: AtomicU64::new(0),
            }),
            entered_receiver,
            release_sender,
        )
    }

    #[cfg(windows)]
    #[test]
    fn windows_overlapped_client_read_keeps_the_bounded_ack_timeout() {
        let root = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(root.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut stream = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "client did not connect");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("listener failed: {error}"),
                }
            };
            stream.set_nonblocking(true).unwrap();
            read_frame_bounded(&mut stream, Duration::from_millis(100)).unwrap();
            thread::sleep(Duration::from_millis(30));
        });

        let mut client = IpcClient::connect(&endpoint, Duration::from_millis(5)).unwrap();
        let started = Instant::now();
        let error = client.send(&IpcFrame::Start(lifecycle())).unwrap_err();
        assert!(matches!(
            error,
            IpcError::Io(ref value) if value.kind() == io::ErrorKind::TimedOut
        ));
        assert!(started.elapsed() < Duration::from_millis(30));
        server.join().unwrap();
    }

    #[test]
    fn frame_round_trip_is_binary_versioned_and_runtime_neutral() {
        let frame = complete();
        let encoded = frame.encode().unwrap();
        assert!(encoded.starts_with(&IPC_MAGIC));
        assert_eq!(encoded[4], IPC_PROTOCOL_VERSION);
        assert_eq!(IpcFrame::decode(&encoded).unwrap(), frame);
        assert_eq!(
            canonical(&frame).unwrap().evidence_transport,
            EvidenceTransport::Ipc
        );
    }

    #[test]
    fn malformed_magic_version_bounds_and_trailing_payload_are_rejected() {
        let encoded = complete().encode().unwrap();
        let mut bad_magic = encoded.clone();
        bad_magic[0] ^= 1;
        assert!(matches!(
            IpcFrame::decode(&bad_magic),
            Err(IpcError::BadMagic)
        ));
        let mut bad_version = encoded.clone();
        bad_version[4] += 1;
        assert!(matches!(
            IpcFrame::decode(&bad_version),
            Err(IpcError::UnsupportedVersion)
        ));
        assert!(matches!(
            IpcFrame::decode(&encoded[..encoded.len() - 1]),
            Err(IpcError::Invalid("frame_length"))
        ));
        let mut trailing = encoded.clone();
        trailing[8] += 1;
        trailing.push(0);
        assert!(matches!(
            IpcFrame::decode(&trailing),
            Err(IpcError::Invalid("trailing_payload"))
        ));
        assert!(matches!(
            IpcFrame::decode(&vec![0; MAX_IPC_FRAME_BYTES + 1]),
            Err(IpcError::Oversized)
        ));
    }

    #[test]
    fn private_content_has_no_wire_or_wal_field() {
        let encoded = complete().encode().unwrap();
        let forbidden = [
            "prompt",
            "assistant",
            "tool_input",
            "tool_output",
            "stdin",
            "stdout",
            "stderr",
            "command",
            "credential",
            "token",
            "secret",
            "path",
        ];
        let schema = [
            "magic",
            "version",
            "frame_type",
            "flags",
            "runtime",
            "runtime_instance",
            "invocation",
            "handler",
            "event",
            "source_scope",
            "revision",
            "timestamp",
            "terminal_status",
            "exit_classification",
            "exit_value",
            "duration",
        ];
        for field in forbidden {
            assert!(
                !schema.contains(&field),
                "private field entered IPC schema: {field}"
            );
        }
        for private_value in ["raw-prompt-value", "raw-stdout-value", "raw-command-value"] {
            assert!(
                !encoded
                    .windows(private_value.len())
                    .any(|value| value == private_value.as_bytes())
            );
        }
    }

    #[test]
    fn wal_replays_valid_records_and_discards_only_truncated_final_tail() {
        let temp = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(
            temp.path(),
            GroupDurabilityPolicy {
                max_records: 8,
                max_bytes: 4096,
                max_interval: std::time::Duration::from_secs(60),
            },
        )
        .unwrap();
        wal.append(&IpcFrame::Start(lifecycle())).unwrap();
        wal.append(&complete()).unwrap();
        wal.flush_group().unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(wal.path())
            .unwrap();
        file.write_all(&WAL_MAGIC[..2]).unwrap();
        drop(file);
        let recovery = wal.recover_and_replay().unwrap();
        assert_eq!(recovery.frames.len(), 2);
        assert_eq!(recovery.truncated_tail_bytes, 2);
        assert_eq!(std::fs::metadata(wal.path()).unwrap().len(), wal.bytes);
    }

    #[test]
    fn wal_checksum_and_non_tail_corruption_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
        wal.append(&complete()).unwrap();
        wal.flush_group().unwrap();
        let mut bytes = std::fs::read(wal.path()).unwrap();
        let final_byte = bytes.len() - 1;
        bytes[final_byte] ^= 1;
        std::fs::write(wal.path(), bytes).unwrap();
        assert!(matches!(
            wal.recover_and_replay(),
            Err(IpcError::WalCorrupt("checksum"))
        ));
    }

    #[test]
    fn group_durability_flushes_a_bounded_batch_not_each_record() {
        let temp = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(
            temp.path(),
            GroupDurabilityPolicy {
                max_records: 2,
                max_bytes: 4_096,
                max_interval: Duration::from_secs(60),
            },
        )
        .unwrap();
        assert_eq!(wal.append(&IpcFrame::Start(lifecycle())).unwrap(), ());
        wal.append(&complete()).unwrap();
        let flush = wal.flush_if_due().unwrap();
        assert_eq!(flush.grouped_records, 2);
        assert!(flush.grouped_bytes > 0);
    }

    #[test]
    fn default_group_thresholds_and_record_and_byte_triggers_are_exact() {
        let defaults = GroupDurabilityPolicy::default();
        assert_eq!(defaults.max_records, 64);
        assert_eq!(defaults.max_bytes, 65_536);
        assert_eq!(defaults.max_interval, Duration::from_millis(50));

        let records_temp = tempfile::tempdir().unwrap();
        let mut records = Wal::open(
            records_temp.path(),
            GroupDurabilityPolicy {
                max_records: 64,
                max_bytes: u64::MAX,
                max_interval: Duration::from_secs(60),
            },
        )
        .unwrap();
        for _ in 0..63 {
            records.append(&IpcFrame::Start(lifecycle())).unwrap();
        }
        assert!(records.take_durability_request(false).is_none());
        records.append(&IpcFrame::Start(lifecycle())).unwrap();
        let record_request = records.take_durability_request(false).unwrap();
        assert_eq!(record_request.group.grouped_records, 64);

        let bytes_temp = tempfile::tempdir().unwrap();
        let mut bytes = Wal::open(
            bytes_temp.path(),
            GroupDurabilityPolicy {
                max_records: u32::MAX,
                max_bytes: 65_536,
                max_interval: Duration::from_secs(60),
            },
        )
        .unwrap();
        let byte_request = loop {
            bytes.append(&IpcFrame::Start(lifecycle())).unwrap();
            if let Some(request) = bytes.take_durability_request(false) {
                break request;
            }
        };
        assert!(byte_request.group.grouped_bytes >= 65_536);
        assert!(
            byte_request.group.grouped_bytes
                < 65_536 + u64::try_from(MAX_IPC_FRAME_BYTES + WAL_HEADER_BYTES).unwrap()
        );
    }

    #[test]
    fn independently_opened_durability_handle_syncs_the_same_appended_wal() {
        let temp = tempfile::tempdir().unwrap();
        let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
        let durability_handle = wal.durability_handle().unwrap();
        wal.append(&IpcFrame::Start(lifecycle())).unwrap();
        durability_handle.sync_data().unwrap();
        let recovery = wal.recover_and_replay().unwrap();
        assert_eq!(recovery.frames, vec![IpcFrame::Start(lifecycle())]);
    }

    #[test]
    fn async_group_sync_blocks_neither_current_ack_nor_subsequent_appends_and_coalesces() {
        let temp = tempfile::tempdir().unwrap();
        let (sync, entered_receiver, release_sender) = controlled_group_sync();
        let configuration = BrokerConfig {
            state_root: temp.path().to_path_buf(),
            queue_capacity: 2,
            max_connections: 1,
            ack_timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_secs(1),
            group_durability: GroupDurabilityPolicy {
                max_records: 1,
                max_bytes: 4_096,
                max_interval: Duration::from_secs(1),
            },
        };
        let host =
            BrokerHost::start_with_test_group_sync(configuration, Arc::clone(&sync)).unwrap();
        let mut client = IpcClient::connect(host.endpoint(), Duration::from_millis(100)).unwrap();
        for _ in 0..10 {
            assert_eq!(
                client.send(&IpcFrame::Start(lifecycle())).unwrap(),
                BrokerAcknowledgement::Accepted
            );
        }
        entered_receiver
            .recv_timeout(Duration::from_millis(500))
            .unwrap();
        assert_eq!(sync.completed.load(Ordering::Acquire), 0);
        drop(client);

        let (stopped_sender, stopped_receiver) = mpsc::channel();
        let stopper = thread::spawn(move || {
            stopped_sender.send(host.stop()).unwrap();
        });
        assert!(
            stopped_receiver
                .recv_timeout(Duration::from_millis(20))
                .is_err()
        );
        release_sender.send(TestSyncOutcome::Succeed).unwrap();
        let deadline = Instant::now() + Duration::from_millis(500);
        let health = loop {
            if let Ok(health) = stopped_receiver.try_recv() {
                break health;
            }
            if entered_receiver.try_recv().is_ok() {
                assert!(stopped_receiver.try_recv().is_err());
                release_sender.send(TestSyncOutcome::Succeed).unwrap();
                break stopped_receiver
                    .recv_timeout(Duration::from_millis(500))
                    .unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "shutdown or final sync timed out"
            );
            thread::sleep(Duration::from_millis(1));
        };
        stopper.join().unwrap();
        assert!((1..=2).contains(&sync.completed.load(Ordering::Acquire)));
        assert_eq!(health.durability_requests, 10);
        assert_eq!(health.durability_requests_coalesced, 9);
        assert!((1..=2).contains(&health.group_flushes));
        assert_eq!(health.durability_failures, 0);

        let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
        let recovery = wal.recover_and_replay().unwrap();
        assert_eq!(recovery.frames.len(), 10);
    }

    #[test]
    fn record_coalescing_is_bounded_and_time_due_requests_are_immediate() {
        let record_temp = tempfile::tempdir().unwrap();
        let mut record_wal = Wal::open(
            record_temp.path(),
            GroupDurabilityPolicy {
                max_records: 1,
                max_bytes: u64::MAX,
                max_interval: Duration::from_millis(50),
            },
        )
        .unwrap();
        record_wal.append(&IpcFrame::Start(lifecycle())).unwrap();
        let request = record_wal.take_durability_request(false).unwrap();
        assert!(request.coalesce_until >= record_wal.last_group_flush);
        assert!(request.coalesce_until <= record_wal.last_group_flush + DURABILITY_COALESCE_WINDOW);

        let timed_temp = tempfile::tempdir().unwrap();
        let mut timed_wal = Wal::open(
            timed_temp.path(),
            GroupDurabilityPolicy {
                max_records: u32::MAX,
                max_bytes: u64::MAX,
                max_interval: Duration::from_millis(1),
            },
        )
        .unwrap();
        timed_wal.append(&IpcFrame::Start(lifecycle())).unwrap();
        thread::sleep(Duration::from_millis(2));
        let request = timed_wal.take_durability_request(false).unwrap();
        assert_eq!(request.coalesce_until, timed_wal.last_group_flush);
    }

    #[test]
    fn wal_flush_lag_tracks_only_the_oldest_not_yet_durable_append() {
        let durability = DurabilityCoordinator::default();
        let first_append = Instant::now() - Duration::from_millis(50);
        durability.record_append(1, first_append);
        assert!(durability.pending_wal_flush_lag_ms().is_some());
        assert_eq!(
            durability.request(1, Instant::now()),
            DurabilityRequestStatus::Scheduled
        );
        assert_eq!(durability.next_request(), Some(1));

        let later_append = Instant::now();
        durability.record_append(2, later_append);
        assert_eq!(
            durability.request(2, Instant::now()),
            DurabilityRequestStatus::Coalesced
        );
        durability.complete(1);

        let state = durability
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(state.in_flight.is_none());
        assert_eq!(state.queued.unwrap().oldest_append_at, later_append);
        drop(state);

        assert_eq!(durability.next_request(), Some(2));
        durability.complete(2);
        assert_eq!(durability.pending_wal_flush_lag_ms(), None);
    }

    #[test]
    fn fifty_millisecond_low_traffic_trigger_syncs_without_a_later_frame() {
        let temp = tempfile::tempdir().unwrap();
        let (sync, entered_receiver, release_sender) = controlled_group_sync();
        let host = BrokerHost::start_with_test_group_sync(
            BrokerConfig {
                state_root: temp.path().to_path_buf(),
                queue_capacity: 2,
                max_connections: 1,
                ack_timeout: Duration::from_millis(100),
                idle_timeout: Duration::from_secs(1),
                group_durability: GroupDurabilityPolicy {
                    max_records: 64,
                    max_bytes: 65_536,
                    max_interval: Duration::from_millis(50),
                },
            },
            Arc::clone(&sync),
        )
        .unwrap();
        let mut client = IpcClient::connect(host.endpoint(), Duration::from_millis(100)).unwrap();
        assert_eq!(
            client.send(&IpcFrame::Start(lifecycle())).unwrap(),
            BrokerAcknowledgement::Accepted
        );
        entered_receiver
            .recv_timeout(Duration::from_millis(500))
            .unwrap();
        release_sender.send(TestSyncOutcome::Succeed).unwrap();
        drop(client);
        let health = host.stop();
        assert_eq!(sync.completed.load(Ordering::Acquire), 1);
        assert_eq!(health.durability_requests, 1);
        assert_eq!(health.group_flushes, 1);
    }

    #[test]
    fn clean_shutdown_schedules_and_waits_for_a_final_below_threshold_group() {
        let temp = tempfile::tempdir().unwrap();
        let (sync, entered_receiver, release_sender) = controlled_group_sync();
        let host = BrokerHost::start_with_test_group_sync(
            BrokerConfig {
                state_root: temp.path().to_path_buf(),
                queue_capacity: 2,
                max_connections: 1,
                ack_timeout: Duration::from_millis(100),
                idle_timeout: Duration::from_secs(1),
                group_durability: GroupDurabilityPolicy {
                    max_records: 64,
                    max_bytes: 65_536,
                    max_interval: Duration::from_secs(60),
                },
            },
            Arc::clone(&sync),
        )
        .unwrap();
        let mut client = IpcClient::connect(host.endpoint(), Duration::from_millis(100)).unwrap();
        assert_eq!(
            client.send(&IpcFrame::Start(lifecycle())).unwrap(),
            BrokerAcknowledgement::Accepted
        );
        assert!(
            entered_receiver
                .recv_timeout(Duration::from_millis(20))
                .is_err()
        );
        drop(client);

        let (stopped_sender, stopped_receiver) = mpsc::channel();
        let stopper = thread::spawn(move || {
            stopped_sender.send(host.stop()).unwrap();
        });
        entered_receiver
            .recv_timeout(Duration::from_millis(500))
            .unwrap();
        assert!(
            stopped_receiver
                .recv_timeout(Duration::from_millis(20))
                .is_err()
        );
        release_sender.send(TestSyncOutcome::Succeed).unwrap();
        let health = stopped_receiver
            .recv_timeout(Duration::from_millis(500))
            .unwrap();
        stopper.join().unwrap();
        assert_eq!(sync.completed.load(Ordering::Acquire), 1);
        assert_eq!(health.accepted, 1);
        assert_eq!(health.durability_requests, 1);
        assert_eq!(health.group_flushes, 1);

        let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
        assert_eq!(wal.recover_and_replay().unwrap().frames.len(), 1);
    }

    #[test]
    fn asynchronous_durability_failure_fails_closed_for_later_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let (sync, entered_receiver, release_sender) = controlled_group_sync();
        let host = BrokerHost::start_with_test_group_sync(
            BrokerConfig {
                state_root: temp.path().to_path_buf(),
                queue_capacity: 2,
                max_connections: 1,
                ack_timeout: Duration::from_millis(100),
                idle_timeout: Duration::from_secs(1),
                group_durability: GroupDurabilityPolicy {
                    max_records: 1,
                    max_bytes: 4_096,
                    max_interval: Duration::from_secs(1),
                },
            },
            sync,
        )
        .unwrap();
        let mut client = IpcClient::connect(host.endpoint(), Duration::from_millis(100)).unwrap();
        assert_eq!(
            client.send(&IpcFrame::Start(lifecycle())).unwrap(),
            BrokerAcknowledgement::Accepted
        );
        entered_receiver
            .recv_timeout(Duration::from_millis(500))
            .unwrap();
        release_sender.send(TestSyncOutcome::Fail).unwrap();
        let deadline = Instant::now() + Duration::from_millis(500);
        while host.health().durability_failures == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(host.health().durability_failures, 1);
        assert!(!matches!(
            client.send(&IpcFrame::Start(lifecycle())),
            Ok(BrokerAcknowledgement::Accepted)
        ));
        drop(client);
        let health = host.stop();
        assert_eq!(health.accepted, 1);
        assert_eq!(health.durability_failures, 1);

        let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
        assert_eq!(wal.recover_and_replay().unwrap().frames.len(), 1);
    }

    #[test]
    fn unsafe_state_objects_and_unbounded_references_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let blocked = temp.path().join("not-a-directory");
        std::fs::write(&blocked, b"not a state root").unwrap();
        assert!(matches!(
            Wal::open(&blocked, GroupDurabilityPolicy::default()),
            Err(IpcError::UnsafeStateObject)
        ));
        let mut invalid = lifecycle();
        invalid.runtime = "a".repeat(MAX_IPC_REFERENCE_BYTES + 1);
        assert!(matches!(
            IpcFrame::Start(invalid).encode(),
            Err(IpcError::Invalid("runtime"))
        ));
    }

    #[cfg(feature = "performance-harness")]
    #[test]
    fn group_sync_queue_wait_overlap_is_exact_and_never_negative() {
        let origin = Instant::now();
        assert_eq!(
            interval_overlap_nanos(
                origin,
                origin + Duration::from_millis(10),
                origin + Duration::from_millis(4),
                origin + Duration::from_millis(7),
            ),
            3_000_000
        );
        assert_eq!(
            interval_overlap_nanos(
                origin,
                origin + Duration::from_millis(3),
                origin + Duration::from_millis(4),
                origin + Duration::from_millis(7),
            ),
            0
        );
    }

    #[test]
    fn queue_overload_is_bounded_and_visible_without_false_acknowledgement() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let health = Arc::new(HealthCounters {
            accepted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            malformed: AtomicU64::new(0),
            replayed: AtomicU64::new(0),
            duplicates: AtomicU64::new(0),
            ack_timeouts: AtomicU64::new(0),
            queue_high_water: AtomicU64::new(0),
            durability_requests: AtomicU64::new(0),
            durability_requests_coalesced: AtomicU64::new(0),
            group_flushes: AtomicU64::new(0),
            durability_failures: AtomicU64::new(0),
            last_group_flush_duration_ns: AtomicU64::new(0),
        });
        let durability = Arc::new(DurabilityCoordinator::default());
        let broker = BrokerCore {
            queue: sender,
            queue_depth: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
            ack_timeout: Duration::from_millis(1),
            health: Arc::clone(&health),
            durability,
            last_activity: Mutex::new(Instant::now()),
            recent_ipc_latency: RecentDurations::new(),
            recent_queue_wait: RecentDurations::new(),
            #[cfg(feature = "performance-harness")]
            qualification_stage_collector: None,
        };
        assert_eq!(
            broker.submit(IpcFrame::Start(lifecycle())),
            BrokerAcknowledgement::Busy
        );
        assert_eq!(
            broker.submit(IpcFrame::Start(lifecycle())),
            BrokerAcknowledgement::DroppedOverloaded
        );
        assert_eq!(health.snapshot().dropped, 1);
        assert_eq!(health.snapshot().queue_high_water, 1);
    }
}
