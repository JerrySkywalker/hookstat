//! Runtime-neutral, local-only IPC evidence frames and broker-side WAL.
//!
//! This module deliberately ends at `CanonicalEvidence`. Runtime-specific
//! integration and identity resolution remain outside the broker. The only
//! supported transports are Windows Named Pipes and Unix Domain Sockets through
//! `interprocess::local_socket`; TCP and HTTP are not part of this subsystem.

use crate::domain::TerminalStatus;
use crate::evidence::{
    CanonicalEvidence, CoreIngestOutcome, EventFamily, EvidenceLifecycle, EvidenceTransport,
    InvocationCoverage, InvocationKey, RevisionRef, RuntimeHandlerRef, RuntimeId, RuntimeInstance,
    RuntimeNeutralEvidenceCore, SourceCoverage, SourceScope,
};
use interprocess::ConnectWaitMode;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::{
    ConnectOptions, Listener, ListenerNonblockingMode, ListenerOptions, Stream, prelude::*,
};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// The first and only G35 wire version. New readers must reject unknown
/// versions instead of guessing field meaning.
pub const IPC_PROTOCOL_VERSION: u8 = 1;
pub const IPC_MAGIC: [u8; 4] = *b"HSIP";
pub const WAL_MAGIC: [u8; 4] = *b"HSWL";
pub const WAL_VERSION: u8 = 1;
pub const MAX_IPC_FRAME_BYTES: usize = 1024;
pub const MAX_IPC_REFERENCE_BYTES: usize = 128;
pub const MAX_WAL_BYTES: u64 = 64 * 1024 * 1024;

const FRAME_HEADER_BYTES: usize = 10;
const WAL_HEADER_BYTES: usize = 12;

/// A bounded runtime-neutral lifecycle envelope. Every string is an opaque
/// identifier, not a path, command, payload, or stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleFrame {
    pub runtime: String,
    pub runtime_instance: String,
    pub invocation: String,
    pub handler: String,
    pub event: String,
    pub source_scope: String,
    pub revision: Option<String>,
    pub occurred_at_unix_ms: i64,
}

impl LifecycleFrame {
    pub fn validate(&self) -> Result<(), IpcError> {
        for (field, value) in [
            ("runtime", self.runtime.as_str()),
            ("runtime_instance", self.runtime_instance.as_str()),
            ("invocation", self.invocation.as_str()),
            ("handler", self.handler.as_str()),
            ("event", self.event.as_str()),
            ("source_scope", self.source_scope.as_str()),
        ] {
            validate_reference(field, value)?;
        }
        if let Some(revision) = &self.revision {
            validate_reference("revision", revision)?;
        }
        if self.occurred_at_unix_ms < 0 {
            return Err(IpcError::Invalid("occurred_at_unix_ms"));
        }
        Ok(())
    }

    fn encode_into(&self, output: &mut Vec<u8>) -> Result<(), IpcError> {
        self.validate()?;
        for value in [
            self.runtime.as_str(),
            self.runtime_instance.as_str(),
            self.invocation.as_str(),
            self.handler.as_str(),
            self.event.as_str(),
            self.source_scope.as_str(),
        ] {
            encode_reference(output, value)?;
        }
        match &self.revision {
            Some(value) => encode_reference(output, value)?,
            None => output.push(0),
        }
        output.extend_from_slice(&self.occurred_at_unix_ms.to_le_bytes());
        Ok(())
    }

    fn decode_from(input: &mut Cursor<'_>) -> Result<Self, IpcError> {
        let value = Self {
            runtime: input.reference("runtime")?,
            runtime_instance: input.reference("runtime_instance")?,
            invocation: input.reference("invocation")?,
            handler: input.reference("handler")?,
            event: input.reference("event")?,
            source_scope: input.reference("source_scope")?,
            revision: input.optional_reference("revision")?,
            occurred_at_unix_ms: input.i64()?,
        };
        value.validate()?;
        Ok(value)
    }

    /// Normalization stays runtime-neutral. IPC evidence is deliberately
    /// `Durable` source coverage, not a claim of complete runtime coverage.
    pub fn canonical(
        &self,
        lifecycle: EvidenceLifecycle,
        completion: Option<&Completion>,
    ) -> Result<CanonicalEvidence, IpcError> {
        self.validate()?;
        let (terminal_status, duration_ms, invocation_coverage) = match lifecycle {
            EvidenceLifecycle::Started => (None, None, InvocationCoverage::Incomplete),
            EvidenceLifecycle::Completed => {
                let completion = completion.ok_or(IpcError::Invalid("completion"))?;
                completion.validate()?;
                (
                    Some(completion.terminal_status),
                    Some(completion.duration_ms),
                    InvocationCoverage::Complete,
                )
            }
        };
        let evidence = CanonicalEvidence {
            schema_version: 1,
            runtime: RuntimeId::new(self.runtime.clone()).map_err(IpcError::Evidence)?,
            runtime_instance: RuntimeInstance::new(self.runtime_instance.clone())
                .map_err(IpcError::Evidence)?,
            invocation_key: InvocationKey::new(self.invocation.clone())
                .map_err(IpcError::Evidence)?,
            runtime_handler_ref: RuntimeHandlerRef::new(self.handler.clone())
                .map_err(IpcError::Evidence)?,
            event: EventFamily::new(self.event.clone()).map_err(IpcError::Evidence)?,
            lifecycle,
            occurred_at_unix_ms: self.occurred_at_unix_ms,
            terminal_status,
            duration_ms,
            source_scope: SourceScope::new(self.source_scope.clone())
                .map_err(IpcError::Evidence)?,
            revision_ref: self
                .revision
                .clone()
                .map(RevisionRef::new)
                .transpose()
                .map_err(IpcError::Evidence)?,
            evidence_transport: EvidenceTransport::Ipc,
            source_coverage: SourceCoverage::Durable,
            invocation_coverage,
        };
        evidence.validate().map_err(IpcError::Evidence)?;
        Ok(evidence)
    }
}

/// Bounded classification for a runtime-provided process outcome. The broker
/// never executes a process and therefore cannot infer this value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExitClassification {
    NotApplicable = 0,
    ExitCode = 1,
    Signal = 2,
    RuntimeControlled = 3,
}

impl ExitClassification {
    fn decode(value: u8) -> Result<Self, IpcError> {
        match value {
            0 => Ok(Self::NotApplicable),
            1 => Ok(Self::ExitCode),
            2 => Ok(Self::Signal),
            3 => Ok(Self::RuntimeControlled),
            _ => Err(IpcError::Invalid("exit_classification")),
        }
    }
}

/// Completion-only fields. `exit_value` is optional only for classifications
/// where no platform process result exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Completion {
    pub terminal_status: TerminalStatus,
    pub exit_classification: ExitClassification,
    pub exit_value: Option<i32>,
    pub duration_ms: u64,
}

impl Completion {
    pub fn validate(&self) -> Result<(), IpcError> {
        if matches!(
            self.terminal_status,
            TerminalStatus::Incomplete | TerminalStatus::Unknown
        ) {
            return Err(IpcError::Invalid("terminal_status"));
        }
        match (self.exit_classification, self.exit_value) {
            (ExitClassification::NotApplicable, None) => Ok(()),
            (ExitClassification::NotApplicable, Some(_)) => Err(IpcError::Invalid("exit_value")),
            (_, Some(_)) => Ok(()),
            (_, None) => Err(IpcError::Invalid("exit_value")),
        }
    }

    fn encode_into(&self, output: &mut Vec<u8>) -> Result<(), IpcError> {
        self.validate()?;
        output.push(encode_terminal_status(self.terminal_status));
        output.push(self.exit_classification as u8);
        match self.exit_value {
            Some(value) => {
                output.push(1);
                output.extend_from_slice(&value.to_le_bytes());
            }
            None => output.push(0),
        }
        output.extend_from_slice(&self.duration_ms.to_le_bytes());
        Ok(())
    }

    fn decode_from(input: &mut Cursor<'_>) -> Result<Self, IpcError> {
        let terminal_status = decode_terminal_status(input.u8()?)?;
        let exit_classification = ExitClassification::decode(input.u8()?)?;
        let exit_value = match input.u8()? {
            0 => None,
            1 => Some(input.i32()?),
            _ => return Err(IpcError::Invalid("exit_value_presence")),
        };
        let completion = Self {
            terminal_status,
            exit_classification,
            exit_value,
            duration_ms: input.u64()?,
        };
        completion.validate()?;
        Ok(completion)
    }
}

/// The only producer lifecycle frames accepted by G35.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IpcFrame {
    Start(LifecycleFrame),
    Complete {
        lifecycle: LifecycleFrame,
        completion: Completion,
    },
    /// Broker acknowledgement. This control frame never enters the WAL.
    Ack(BrokerAcknowledgement),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BrokerAcknowledgement {
    Accepted = 1,
    DroppedOverloaded = 2,
    Rejected = 3,
    Busy = 4,
}

impl BrokerAcknowledgement {
    fn decode(value: u8) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::Accepted),
            2 => Ok(Self::DroppedOverloaded),
            3 => Ok(Self::Rejected),
            4 => Ok(Self::Busy),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
    }
}

impl IpcFrame {
    pub fn encode(&self) -> Result<Vec<u8>, IpcError> {
        let mut payload = Vec::with_capacity(256);
        let frame_type = match self {
            Self::Start(value) => {
                value.encode_into(&mut payload)?;
                1_u8
            }
            Self::Complete {
                lifecycle,
                completion,
            } => {
                lifecycle.encode_into(&mut payload)?;
                completion.encode_into(&mut payload)?;
                2_u8
            }
            Self::Ack(value) => {
                payload.push(*value as u8);
                3_u8
            }
        };
        if payload.len() > MAX_IPC_FRAME_BYTES - FRAME_HEADER_BYTES
            || payload.len() > u16::MAX as usize
        {
            return Err(IpcError::Oversized);
        }
        let mut output = Vec::with_capacity(FRAME_HEADER_BYTES + payload.len());
        output.extend_from_slice(&IPC_MAGIC);
        output.push(IPC_PROTOCOL_VERSION);
        output.push(frame_type);
        output.extend_from_slice(&0_u16.to_le_bytes()); // flags are reserved and must be zero.
        output.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        output.extend_from_slice(&payload);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, IpcError> {
        if input.len() < FRAME_HEADER_BYTES {
            return Err(IpcError::Truncated);
        }
        if input.len() > MAX_IPC_FRAME_BYTES {
            return Err(IpcError::Oversized);
        }
        if input[..4] != IPC_MAGIC {
            return Err(IpcError::BadMagic);
        }
        if input[4] != IPC_PROTOCOL_VERSION {
            return Err(IpcError::UnsupportedVersion);
        }
        let frame_type = input[5];
        if u16::from_le_bytes([input[6], input[7]]) != 0 {
            return Err(IpcError::Invalid("flags"));
        }
        let payload_len = u16::from_le_bytes([input[8], input[9]]) as usize;
        if payload_len > MAX_IPC_FRAME_BYTES - FRAME_HEADER_BYTES
            || input.len() != FRAME_HEADER_BYTES + payload_len
        {
            return Err(IpcError::Invalid("frame_length"));
        }
        let mut cursor = Cursor::new(&input[FRAME_HEADER_BYTES..]);
        let frame = match frame_type {
            1 => Self::Start(LifecycleFrame::decode_from(&mut cursor)?),
            2 => Self::Complete {
                lifecycle: LifecycleFrame::decode_from(&mut cursor)?,
                completion: Completion::decode_from(&mut cursor)?,
            },
            3 => Self::Ack(BrokerAcknowledgement::decode(cursor.u8()?)?),
            _ => return Err(IpcError::Invalid("frame_type")),
        };
        if !cursor.is_empty() {
            return Err(IpcError::Invalid("trailing_payload"));
        }
        Ok(frame)
    }

    pub fn canonical(&self) -> Result<CanonicalEvidence, IpcError> {
        match self {
            Self::Start(value) => value.canonical(EvidenceLifecycle::Started, None),
            Self::Complete {
                lifecycle,
                completion,
            } => lifecycle.canonical(EvidenceLifecycle::Completed, Some(completion)),
            Self::Ack(_) => Err(IpcError::Invalid("acknowledgement_is_not_evidence")),
        }
    }

    fn is_lifecycle(&self) -> bool {
        matches!(self, Self::Start(_) | Self::Complete { .. })
    }
}

/// Reads exactly one bounded binary frame from a local stream. This is used by
/// both Windows Named Pipe and Unix Domain Socket implementations.
pub fn read_frame(mut input: impl Read) -> Result<IpcFrame, IpcError> {
    let mut header = [0_u8; FRAME_HEADER_BYTES];
    input.read_exact(&mut header).map_err(IpcError::Io)?;
    let length = u16::from_le_bytes([header[8], header[9]]) as usize;
    if length > MAX_IPC_FRAME_BYTES - FRAME_HEADER_BYTES {
        return Err(IpcError::Oversized);
    }
    let mut encoded = Vec::with_capacity(FRAME_HEADER_BYTES + length);
    encoded.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    input.read_exact(&mut payload).map_err(IpcError::Io)?;
    encoded.extend_from_slice(&payload);
    IpcFrame::decode(&encoded)
}

pub fn write_frame(mut output: impl Write, frame: &IpcFrame) -> Result<(), IpcError> {
    let encoded = frame.encode()?;
    output.write_all(&encoded).map_err(IpcError::Io)
}

fn read_frame_bounded(input: &mut Stream, timeout: Duration) -> Result<IpcFrame, IpcError> {
    let deadline = Instant::now() + timeout;
    let mut header = [0_u8; FRAME_HEADER_BYTES];
    read_exact_bounded(input, &mut header, deadline)?;
    let length = u16::from_le_bytes([header[8], header[9]]) as usize;
    if length > MAX_IPC_FRAME_BYTES - FRAME_HEADER_BYTES {
        return Err(IpcError::Oversized);
    }
    let mut encoded = Vec::with_capacity(FRAME_HEADER_BYTES + length);
    encoded.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    read_exact_bounded(input, &mut payload, deadline)?;
    encoded.extend_from_slice(&payload);
    IpcFrame::decode(&encoded)
}

fn write_frame_bounded(
    input: &mut Stream,
    frame: &IpcFrame,
    timeout: Duration,
) -> Result<(), IpcError> {
    let encoded = frame.encode()?;
    write_all_bounded(input, &encoded, Instant::now() + timeout)
}

fn read_exact_bounded(
    input: &mut Stream,
    mut buffer: &mut [u8],
    deadline: Instant,
) -> Result<(), IpcError> {
    let mut spins = 0_u32;
    while !buffer.is_empty() {
        match input.read(buffer) {
            // A nonblocking Windows Named Pipe reports a zero-byte read while
            // a peer has connected but has not written yet. Treat it like
            // `WouldBlock` and keep the same bounded deadline.
            Ok(0) => {
                if Instant::now() >= deadline {
                    return Err(IpcError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "bounded IPC read",
                    )));
                }
                spins += 1;
                if spins < 8 {
                    std::hint::spin_loop();
                } else {
                    spins = 0;
                    thread::yield_now();
                }
            }
            Ok(read) => {
                let (_, rest) = buffer.split_at_mut(read);
                buffer = rest;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                if Instant::now() >= deadline {
                    return Err(IpcError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "bounded IPC read",
                    )));
                }
                thread::sleep(Duration::from_micros(100));
            }
            Err(error) => return Err(IpcError::Io(error)),
        }
    }
    Ok(())
}

fn write_all_bounded(
    input: &mut Stream,
    mut buffer: &[u8],
    deadline: Instant,
) -> Result<(), IpcError> {
    let mut spins = 0_u32;
    while !buffer.is_empty() {
        match input.write(buffer) {
            Ok(0) => {
                return Err(IpcError::Io(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "bounded IPC write",
                )));
            }
            Ok(written) => buffer = &buffer[written..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                if Instant::now() >= deadline {
                    return Err(IpcError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "bounded IPC write",
                    )));
                }
                spins += 1;
                if spins < 8 {
                    std::hint::spin_loop();
                } else {
                    spins = 0;
                    thread::yield_now();
                }
            }
            Err(error) => return Err(IpcError::Io(error)),
        }
    }
    Ok(())
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
    last_group_flush: std::time::Instant,
    policy: GroupDurabilityPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroupDurabilityPolicy {
    pub max_records: u32,
    pub max_bytes: u64,
    pub max_interval: std::time::Duration,
}

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
            last_group_flush: std::time::Instant::now(),
            policy,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn append(&mut self, frame: &IpcFrame) -> Result<WalFlush, IpcError> {
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
        self.file.write_all(&WAL_MAGIC).map_err(IpcError::Io)?;
        self.file
            .write_all(&[WAL_VERSION, 0])
            .map_err(IpcError::Io)?;
        self.file
            .write_all(&body_len.to_le_bytes())
            .map_err(IpcError::Io)?;
        self.file
            .write_all(&checksum.to_le_bytes())
            .map_err(IpcError::Io)?;
        self.file.write_all(&body).map_err(IpcError::Io)?;
        self.bytes += record_len;
        self.pending_records += 1;
        self.pending_bytes += record_len;
        self.flush_if_due()
    }

    pub fn flush_if_due(&mut self) -> Result<WalFlush, IpcError> {
        let due = self.pending_records >= self.policy.max_records
            || self.pending_bytes >= self.policy.max_bytes
            || self.last_group_flush.elapsed() >= self.policy.max_interval;
        if due {
            self.flush_group()
        } else {
            Ok(WalFlush::default())
        }
    }

    pub fn flush_group(&mut self) -> Result<WalFlush, IpcError> {
        if self.pending_records == 0 {
            return Ok(WalFlush::default());
        }
        self.file.sync_data().map_err(IpcError::Io)?;
        let flushed = WalFlush {
            grouped_records: self.pending_records,
            grouped_bytes: self.pending_bytes,
        };
        self.pending_records = 0;
        self.pending_bytes = 0;
        self.last_group_flush = std::time::Instant::now();
        Ok(flushed)
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

#[derive(Debug)]
pub enum IpcError {
    Io(io::Error),
    Evidence(crate::evidence::EvidenceError),
    BadMagic,
    UnsupportedVersion,
    Oversized,
    Truncated,
    Invalid(&'static str),
    UnsafeStateObject,
    EndpointInUse,
    StartupTimedOut,
    WalTooLarge,
    WalCorrupt(&'static str),
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io(_) => "local IPC I/O failed",
            Self::Evidence(_) => "IPC evidence did not satisfy the canonical boundary",
            Self::BadMagic => "IPC frame magic was invalid",
            Self::UnsupportedVersion => "IPC protocol version is not supported",
            Self::Oversized => "IPC frame exceeded a bounded limit",
            Self::Truncated => "IPC frame was truncated",
            Self::Invalid(_) => "IPC frame structure was invalid",
            Self::UnsafeStateObject => "IPC state root contained an unsafe object",
            Self::EndpointInUse => "IPC endpoint is already owned by a healthy broker",
            Self::StartupTimedOut => {
                "IPC broker startup did not become ready within its bounded timeout"
            }
            Self::WalTooLarge => "IPC WAL exceeded its bounded size",
            Self::WalCorrupt(_) => "IPC WAL contained a malformed record",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for IpcError {}

fn checksum(value: &[u8]) -> u32 {
    let digest = Sha256::digest(value);
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

fn validate_reference(field: &'static str, value: &str) -> Result<(), IpcError> {
    if value.is_empty() || value.len() > MAX_IPC_REFERENCE_BYTES || value.chars().any(|character| !matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':')) {
        return Err(IpcError::Invalid(field));
    }
    Ok(())
}

fn encode_reference(output: &mut Vec<u8>, value: &str) -> Result<(), IpcError> {
    validate_reference("reference", value)?;
    output.push(u8::try_from(value.len()).map_err(|_| IpcError::Oversized)?);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_terminal_status(value: TerminalStatus) -> u8 {
    match value {
        TerminalStatus::Completed => 1,
        TerminalStatus::Failed => 2,
        TerminalStatus::Blocked => 3,
        TerminalStatus::Stopped => 4,
        TerminalStatus::TimedOut => 5,
        TerminalStatus::ProtocolFailure => 6,
        TerminalStatus::Incomplete | TerminalStatus::Unknown => 0,
    }
}

fn decode_terminal_status(value: u8) -> Result<TerminalStatus, IpcError> {
    match value {
        1 => Ok(TerminalStatus::Completed),
        2 => Ok(TerminalStatus::Failed),
        3 => Ok(TerminalStatus::Blocked),
        4 => Ok(TerminalStatus::Stopped),
        5 => Ok(TerminalStatus::TimedOut),
        6 => Ok(TerminalStatus::ProtocolFailure),
        _ => Err(IpcError::Invalid("terminal_status")),
    }
}

fn prepare_state_root(root: &std::path::Path) -> Result<std::path::PathBuf, IpcError> {
    if root.exists() {
        let metadata = std::fs::symlink_metadata(root).map_err(IpcError::Io)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(IpcError::UnsafeStateObject);
        }
    } else {
        std::fs::create_dir_all(root).map_err(IpcError::Io)?;
    }
    let root = std::fs::canonicalize(root).map_err(IpcError::Io)?;
    let metadata = std::fs::symlink_metadata(&root).map_err(IpcError::Io)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || state_metadata_is_unsafe(&metadata)
    {
        return Err(IpcError::UnsafeStateObject);
    }
    Ok(root)
}

fn state_metadata_is_unsafe(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o022 != 0 {
            return true;
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Collision-safe local endpoint derived only from HookStat's user-local state
/// root. A producer cannot select a filesystem path through a wire frame.
#[derive(Clone, Debug)]
pub struct LocalEndpoint {
    state_root: std::path::PathBuf,
    endpoint_id: String,
}

impl LocalEndpoint {
    pub fn from_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> {
        let state_root = prepare_state_root(root.as_ref())?;
        let mut hasher = Sha256::new();
        hasher.update(b"hookstat-g35-local-endpoint-v1\0");
        hasher.update(state_root.as_os_str().as_encoded_bytes());
        hasher.update(b"\0");
        hasher.update(
            std::env::var_os("USERNAME")
                .or_else(|| std::env::var_os("USER"))
                .unwrap_or_default()
                .as_encoded_bytes(),
        );
        let digest = hasher.finalize();
        let endpoint_id = digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let endpoint = Self {
            state_root,
            endpoint_id,
        };
        endpoint.transport_dir()?;
        Ok(endpoint)
    }

    pub fn state_root(&self) -> &std::path::Path {
        &self.state_root
    }

    pub fn endpoint_id(&self) -> &str {
        &self.endpoint_id
    }

    #[cfg(unix)]
    pub fn unix_socket_path(&self) -> Result<std::path::PathBuf, IpcError> {
        let path = self
            .transport_dir()?
            .join(format!("g35-{}.sock", self.endpoint_id));
        if path.as_os_str().as_encoded_bytes().len() > 96 {
            return Err(IpcError::Invalid("unix_socket_path_length"));
        }
        Ok(path)
    }

    #[cfg(windows)]
    pub fn named_pipe_name(&self) -> String {
        format!("hookstat-g35-{}", self.endpoint_id)
    }

    fn transport_dir(&self) -> Result<std::path::PathBuf, IpcError> {
        let dir = self.state_root.join("ipc");
        if dir.exists() {
            let metadata = std::fs::symlink_metadata(&dir).map_err(IpcError::Io)?;
            if metadata.file_type().is_symlink()
                || !metadata.is_dir()
                || state_metadata_is_unsafe(&metadata)
            {
                return Err(IpcError::UnsafeStateObject);
            }
        } else {
            std::fs::create_dir(&dir).map_err(IpcError::Io)?;
        }
        let canonical = std::fs::canonicalize(&dir).map_err(IpcError::Io)?;
        if canonical.parent() != Some(self.state_root.as_path()) {
            return Err(IpcError::UnsafeStateObject);
        }
        Ok(canonical)
    }

    fn connect_stream(&self, timeout: Duration) -> Result<Stream, IpcError> {
        #[cfg(unix)]
        let name = {
            use interprocess::local_socket::{GenericFilePath, ToFsName};
            self.unix_socket_path()?
                .to_fs_name::<GenericFilePath>()
                .map_err(IpcError::Io)?
        };
        #[cfg(windows)]
        let name = self
            .named_pipe_name()
            .to_ns_name::<GenericNamespaced>()
            .map_err(IpcError::Io)?;
        let stream = ConnectOptions::new()
            .name(name)
            .wait_mode(ConnectWaitMode::Timeout(timeout))
            .connect_sync()
            .map_err(IpcError::Io)?;
        stream.set_nonblocking(true).map_err(IpcError::Io)?;
        Ok(stream)
    }

    fn bind(&self) -> Result<Listener, IpcError> {
        #[cfg(unix)]
        {
            use interprocess::local_socket::{GenericFilePath, ToFsName};
            use interprocess::os::unix::local_socket::ListenerOptionsExt;
            use std::os::unix::fs::FileTypeExt;
            let path = self.unix_socket_path()?;
            if path.exists() {
                let metadata = std::fs::symlink_metadata(&path).map_err(IpcError::Io)?;
                if metadata.file_type().is_symlink() || !metadata.file_type().is_socket() {
                    return Err(IpcError::UnsafeStateObject);
                }
                if self.connect_stream(Duration::from_millis(5)).is_ok() {
                    return Err(IpcError::EndpointInUse);
                }
                std::fs::remove_file(&path).map_err(IpcError::Io)?;
            }
            let name = path.to_fs_name::<GenericFilePath>().map_err(IpcError::Io)?;
            ListenerOptions::new()
                .name(name)
                .nonblocking(ListenerNonblockingMode::Accept)
                .reclaim_name(false)
                .max_spin_time(Duration::ZERO)
                .mode(0o600)
                .create_sync()
                .map_err(IpcError::Io)
        }
        #[cfg(windows)]
        {
            use interprocess::os::windows::local_socket::ListenerOptionsExt;
            // `GenericNamespaced` maps to a local Windows Named Pipe. No
            // network address is present in this endpoint name or API. The
            // protected owner-rights DACL grants the creating user only the
            // read/write access needed by a local producer.
            let name = self
                .named_pipe_name()
                .to_ns_name::<GenericNamespaced>()
                .map_err(IpcError::Io)?;
            ListenerOptions::new()
                .name(name)
                .nonblocking(ListenerNonblockingMode::Accept)
                .reclaim_name(false)
                .security_descriptor(owner_only_pipe_security_descriptor()?)
                .create_sync()
                .map_err(|error| {
                    if error.kind() == io::ErrorKind::AddrInUse {
                        IpcError::EndpointInUse
                    } else {
                        IpcError::Io(error)
                    }
                })
        }
    }

    #[cfg(unix)]
    fn remove_socket_if_owned(&self) {
        if let Ok(path) = self.unix_socket_path()
            && let Ok(metadata) = std::fs::symlink_metadata(&path)
        {
            use std::os::unix::fs::FileTypeExt;
            if metadata.file_type().is_socket() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

#[cfg(windows)]
fn owner_only_pipe_security_descriptor()
-> Result<interprocess::os::windows::security_descriptor::SecurityDescriptor, IpcError> {
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;
    let sddl = widestring::U16CString::from_str("D:P(A;;GRGW;;;OW)")
        .map_err(|_| IpcError::Invalid("pipe_security_descriptor"))?;
    SecurityDescriptor::deserialize(sddl.as_ucstr()).map_err(IpcError::Io)
}

/// A connected generic producer. G36 may supply a cooperative producer or a
/// tiny shim, but neither is implemented here.
pub struct IpcClient {
    stream: Stream,
    timeout: Duration,
}

impl IpcClient {
    pub fn connect(endpoint: &LocalEndpoint, timeout: Duration) -> Result<Self, IpcError> {
        Ok(Self {
            stream: endpoint.connect_stream(timeout)?,
            timeout,
        })
    }

    pub fn send(&mut self, frame: &IpcFrame) -> Result<BrokerAcknowledgement, IpcError> {
        if !frame.is_lifecycle() {
            return Err(IpcError::Invalid("producer_frame_type"));
        }
        write_frame_bounded(&mut self.stream, frame, self.timeout)?;
        match read_frame_bounded(&mut self.stream, self.timeout)? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
    }
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
    pub group_flushes: u64,
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
    group_flushes: AtomicU64,
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
            group_flushes: self.group_flushes.load(Ordering::Relaxed),
        }
    }
}

struct QueuedFrame {
    frame: IpcFrame,
    acknowledgement: mpsc::SyncSender<BrokerAcknowledgement>,
}

struct BrokerCore {
    queue: mpsc::SyncSender<QueuedFrame>,
    queue_depth: AtomicUsize,
    active_connections: AtomicUsize,
    ack_timeout: Duration,
    health: Arc<HealthCounters>,
    last_activity: Mutex<Instant>,
}

impl BrokerCore {
    fn submit(&self, frame: IpcFrame) -> BrokerAcknowledgement {
        let (acknowledgement, receiver) = mpsc::sync_channel(1);
        let queued = QueuedFrame {
            frame,
            acknowledgement,
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

    fn touch(&self) {
        *self.last_activity.lock().expect("broker activity lock") = Instant::now();
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
}

impl BrokerHost {
    pub fn start(config: BrokerConfig) -> Result<Self, IpcError> {
        config.validate()?;
        let endpoint = LocalEndpoint::from_state_root(&config.state_root)?;
        let mut wal = Wal::open(&config.state_root, config.group_durability)?;
        let recovery = BrokerRecovery::from_wal(wal.recover_and_replay()?);
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
            group_flushes: AtomicU64::new(0),
        });
        let core = Arc::new(BrokerCore {
            queue: queue_sender,
            queue_depth: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
            ack_timeout: config.ack_timeout,
            health: Arc::clone(&health),
            last_activity: Mutex::new(Instant::now()),
        });
        let stopping = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        {
            let health = Arc::clone(&health);
            let core = Arc::clone(&core);
            let stopping = Arc::clone(&stopping);
            handles.push(thread::spawn(move || {
                wal_worker_loop(&mut wal, queue_receiver, core, health, stopping)
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
        })
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

    pub fn stop(mut self) {
        self.stopping.store(true, Ordering::Release);
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
        #[cfg(unix)]
        self.endpoint.remove_socket_if_owned();
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
) {
    loop {
        match receiver.recv_timeout(Duration::from_millis(2)) {
            Ok(queued) => {
                core.queue_depth.fetch_sub(1, Ordering::Relaxed);
                let acknowledgement = match wal.append(&queued.frame) {
                    Ok(flush) => {
                        health.accepted.fetch_add(1, Ordering::Relaxed);
                        if flush.grouped_records > 0 {
                            health.group_flushes.fetch_add(1, Ordering::Relaxed);
                        }
                        BrokerAcknowledgement::Accepted
                    }
                    Err(_) => {
                        health.rejected.fetch_add(1, Ordering::Relaxed);
                        BrokerAcknowledgement::Rejected
                    }
                };
                let _ = queued.acknowledgement.send(acknowledgement);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(flush) = wal.flush_if_due()
                    && flush.grouped_records > 0
                {
                    health.group_flushes.fetch_add(1, Ordering::Relaxed);
                }
                if stopping.load(Ordering::Acquire)
                    && core.queue_depth.load(Ordering::Acquire) == 0
                    && core.active_connections.load(Ordering::Acquire) == 0
                {
                    let _ = wal.flush_group();
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = wal.flush_group();
                break;
            }
        }
    }
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
    while !stopping.load(Ordering::Acquire) {
        match read_frame_bounded(&mut stream, Duration::from_millis(10)) {
            Ok(frame) if frame.is_lifecycle() => {
                core.touch();
                let acknowledgement = core.submit(frame);
                let _ = write_frame_bounded(
                    &mut stream,
                    &IpcFrame::Ack(acknowledgement),
                    Duration::from_millis(5),
                );
            }
            Err(IpcError::Io(error)) if error.kind() == io::ErrorKind::TimedOut => {
                thread::sleep(Duration::from_millis(1));
            }
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
        self.frames.iter().map(IpcFrame::canonical).collect()
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
            match core.ingest(evidence).map_err(IpcError::Evidence)? {
                CoreIngestOutcome::Produced(_) => result.produced += 1,
                CoreIngestOutcome::Duplicate => result.duplicates += 1,
                CoreIngestOutcome::Shadow => result.shadowed += 1,
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
    pub unconfigured: u64,
}

/// A bounded startup handoff. It elects one starter using a state-root-local
/// lease and makes all other producers wait only until the configured deadline.
#[derive(Clone, Debug)]
pub struct BrokerStartup {
    endpoint: LocalEndpoint,
    timeout: Duration,
    stale_lease_after: Duration,
}

impl BrokerStartup {
    pub fn new(endpoint: LocalEndpoint, timeout: Duration) -> Result<Self, IpcError> {
        if timeout.is_zero() {
            return Err(IpcError::Invalid("startup_timeout"));
        }
        Ok(Self {
            endpoint,
            timeout,
            stale_lease_after: timeout,
        })
    }

    pub fn connect_or_start(
        &self,
        mut start: impl FnMut() -> Result<(), IpcError>,
    ) -> Result<IpcClient, IpcError> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Ok(client) = IpcClient::connect(&self.endpoint, Duration::from_millis(2)) {
                return Ok(client);
            }
            if let Some(_lease) = StartupLease::try_acquire(&self.endpoint, self.stale_lease_after)?
            {
                start()?;
                // The elected starter retains its lease until the new endpoint
                // is actually connectable. Without this, a just-started pipe
                // can briefly look absent and trigger a startup storm.
                loop {
                    if let Ok(client) = IpcClient::connect(&self.endpoint, Duration::from_millis(2))
                    {
                        // A Windows Named Pipe listener creates its next
                        // accept instance after the first connection is
                        // observed. Keep the election lease through this
                        // bounded stabilization interval so followers do not
                        // mistake that transition for a missing broker.
                        thread::sleep(Duration::from_millis(25));
                        return Ok(client);
                    }
                    if Instant::now() >= deadline {
                        return Err(IpcError::StartupTimedOut);
                    }
                    thread::sleep(Duration::from_millis(2));
                }
            }
            if Instant::now() >= deadline {
                return Err(IpcError::StartupTimedOut);
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

struct StartupLease {
    path: std::path::PathBuf,
}

impl StartupLease {
    fn try_acquire(
        endpoint: &LocalEndpoint,
        stale_after: Duration,
    ) -> Result<Option<Self>, IpcError> {
        let path = endpoint.transport_dir()?.join("broker-start-v1.lock");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(Some(Self { path })),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = std::fs::symlink_metadata(&path).map_err(IpcError::Io)?;
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(IpcError::UnsafeStateObject);
                }
                if metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|elapsed| elapsed >= stale_after)
                {
                    std::fs::remove_file(&path).map_err(IpcError::Io)?;
                }
                Ok(None)
            }
            Err(error) => Err(IpcError::Io(error)),
        }
    }
}

impl Drop for StartupLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct Cursor<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }
    fn is_empty(&self) -> bool {
        self.offset == self.input.len()
    }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], IpcError> {
        let end = self.offset.checked_add(length).ok_or(IpcError::Truncated)?;
        let value = self
            .input
            .get(self.offset..end)
            .ok_or(IpcError::Truncated)?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, IpcError> {
        Ok(self.bytes(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, IpcError> {
        Ok(i32::from_le_bytes(
            self.bytes(4)?.try_into().map_err(|_| IpcError::Truncated)?,
        ))
    }
    fn i64(&mut self) -> Result<i64, IpcError> {
        Ok(i64::from_le_bytes(
            self.bytes(8)?.try_into().map_err(|_| IpcError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, IpcError> {
        Ok(u64::from_le_bytes(
            self.bytes(8)?.try_into().map_err(|_| IpcError::Truncated)?,
        ))
    }
    fn reference(&mut self, field: &'static str) -> Result<String, IpcError> {
        let length = self.u8()? as usize;
        if length == 0 || length > MAX_IPC_REFERENCE_BYTES {
            return Err(IpcError::Invalid(field));
        }
        let value = std::str::from_utf8(self.bytes(length)?)
            .map_err(|_| IpcError::Invalid(field))?
            .to_owned();
        validate_reference(field, &value)?;
        Ok(value)
    }
    fn optional_reference(&mut self, field: &'static str) -> Result<Option<String>, IpcError> {
        let length = self.u8()? as usize;
        if length == 0 {
            return Ok(None);
        }
        if length > MAX_IPC_REFERENCE_BYTES {
            return Err(IpcError::Invalid(field));
        }
        let value = std::str::from_utf8(self.bytes(length)?)
            .map_err(|_| IpcError::Invalid(field))?
            .to_owned();
        validate_reference(field, &value)?;
        Ok(Some(value))
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
                terminal_status: TerminalStatus::Failed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(7),
                duration_ms: 12,
            },
        }
    }

    #[test]
    fn frame_round_trip_is_binary_versioned_and_runtime_neutral() {
        let frame = complete();
        let encoded = frame.encode().unwrap();
        assert!(encoded.starts_with(&IPC_MAGIC));
        assert_eq!(encoded[4], IPC_PROTOCOL_VERSION);
        assert_eq!(IpcFrame::decode(&encoded).unwrap(), frame);
        assert_eq!(
            frame.canonical().unwrap().evidence_transport,
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
        assert_eq!(
            wal.append(&IpcFrame::Start(lifecycle())).unwrap(),
            WalFlush::default()
        );
        let flush = wal.append(&complete()).unwrap();
        assert_eq!(flush.grouped_records, 2);
        assert!(flush.grouped_bytes > 0);
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
            group_flushes: AtomicU64::new(0),
        });
        let broker = BrokerCore {
            queue: sender,
            queue_depth: AtomicUsize::new(0),
            active_connections: AtomicUsize::new(0),
            ack_timeout: Duration::from_millis(1),
            health: Arc::clone(&health),
            last_activity: Mutex::new(Instant::now()),
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
