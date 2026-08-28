//! Private, bounded local IPC protocol and fail-open producer surface.
//!
//! This crate is intentionally the only definition of the HookStat IPC v1
//! frame. It is usable by a cooperative Hook or the `hookstat-hook` shim
//! without bringing the HookStat application, SQLite, reporting, analytics,
//! or terminal UI into the hot path.

use interprocess::ConnectWaitMode;
#[cfg(unix)]
use interprocess::local_socket::ConnectOptions;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
#[cfg(windows)]
use interprocess::local_socket::tokio::Stream as TokioStream;
use interprocess::local_socket::{ListenerNonblockingMode, ListenerOptions, prelude::*};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub use interprocess::local_socket::{Listener, Stream};

/// The first and only production wire version.
pub const IPC_PROTOCOL_VERSION: u8 = 1;
pub const IPC_MAGIC: [u8; 4] = *b"HSIP";
pub const MAX_IPC_FRAME_BYTES: usize = 1024;
pub const MAX_IPC_REFERENCE_BYTES: usize = 128;
pub const BROKER_DIAGNOSTICS_SCHEMA_VERSION: u8 = 1;
pub const RECENT_DIAGNOSTIC_SAMPLE_CAPACITY: u64 = 128;

/// Fixed HSIP v1 frame header size: magic, protocol version, frame kind,
/// flags, and payload length. Reference conformance fixtures use this public
/// protocol constant rather than duplicating a private wire-layout guess.
pub const IPC_FRAME_HEADER_BYTES: usize = 10;
// The broker releases an idle server-side connection after a 50 ms bounded
// read window. Reconnect before half that window so a long-running Hook never
// sends a lifecycle frame over a connection whose delivery is ambiguous.
const PRODUCER_CONNECTION_REUSE_WINDOW: Duration = Duration::from_millis(25);

/// Bounded, opaque runtime-neutral lifecycle metadata. No command, stream,
/// prompt, tool payload, credential, or filesystem path is a protocol field.
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
}

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

/// Closed terminal representation accepted by the broker. The `u8` values
/// intentionally match the G35 terminal mapping without depending on the
/// product domain crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TerminalOutcome {
    Completed = 1,
    Failed = 2,
    Blocked = 3,
    Stopped = 4,
    TimedOut = 5,
    ProtocolFailure = 6,
}

impl TerminalOutcome {
    fn decode(value: u8) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::Completed),
            2 => Ok(Self::Failed),
            3 => Ok(Self::Blocked),
            4 => Ok(Self::Stopped),
            5 => Ok(Self::TimedOut),
            6 => Ok(Self::ProtocolFailure),
            _ => Err(IpcError::Invalid("terminal_status")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Completion {
    pub terminal_status: TerminalOutcome,
    pub exit_classification: ExitClassification,
    pub exit_value: Option<i32>,
    pub duration_ms: u64,
}

impl Completion {
    pub fn validate(&self) -> Result<(), IpcError> {
        match (self.exit_classification, self.exit_value) {
            (ExitClassification::NotApplicable, None) | (_, Some(_)) => Ok(()),
            _ => Err(IpcError::Invalid("exit_value")),
        }
    }

    fn encode_into(&self, output: &mut Vec<u8>) -> Result<(), IpcError> {
        self.validate()?;
        output.push(self.terminal_status as u8);
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
        let completion = Self {
            terminal_status: TerminalOutcome::decode(input.u8()?)?,
            exit_classification: ExitClassification::decode(input.u8()?)?,
            exit_value: match input.u8()? {
                0 => None,
                1 => Some(input.i32()?),
                _ => return Err(IpcError::Invalid("exit_value_presence")),
            },
            duration_ms: input.u64()?,
        };
        completion.validate()?;
        Ok(completion)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IpcFrame {
    Start(LifecycleFrame),
    Complete {
        lifecycle: LifecycleFrame,
        completion: Completion,
    },
    Ack(BrokerAcknowledgement),
    BrokerDiagnosticsRequest,
    BrokerDiagnosticsResponse(BrokerDiagnostics),
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

/// Bounded numeric broker self-observability returned only to an explicit
/// local diagnostics query. This control-plane snapshot is never a lifecycle
/// evidence frame and contains no runtime, handler, command, path, payload, or
/// stream content.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct BrokerDiagnostics {
    pub schema_version: u8,
    pub accepted: u64,
    pub rejected: u64,
    pub dropped: u64,
    pub malformed: u64,
    pub replayed: u64,
    pub duplicates: u64,
    pub ack_timeouts: u64,
    pub queue_depth: u64,
    pub active_connections: u64,
    pub queue_high_water: u64,
    pub durability_requests: u64,
    pub durability_requests_coalesced: u64,
    pub group_flushes: u64,
    pub durability_failures: u64,
    pub recent_ipc_latency_samples: u64,
    pub recent_ipc_latency_p50_us: u64,
    pub recent_ipc_latency_p95_us: u64,
    pub recent_ipc_latency_p99_us: u64,
    pub recent_queue_wait_p95_us: u64,
    pub wal_flush_lag_ms: Option<u64>,
    pub last_wal_flush_duration_us: Option<u64>,
}

impl BrokerDiagnostics {
    pub fn validate(&self) -> Result<(), IpcError> {
        if self.schema_version != BROKER_DIAGNOSTICS_SCHEMA_VERSION
            || self.queue_depth > 16_384
            || self.active_connections > 128
            || self.queue_high_water > 16_384
            || self.recent_ipc_latency_samples > RECENT_DIAGNOSTIC_SAMPLE_CAPACITY
            || self.recent_ipc_latency_p50_us > self.recent_ipc_latency_p95_us
            || self.recent_ipc_latency_p95_us > self.recent_ipc_latency_p99_us
        {
            return Err(IpcError::Invalid("broker_diagnostics"));
        }
        Ok(())
    }

    fn encode_into(&self, output: &mut Vec<u8>) -> Result<(), IpcError> {
        self.validate()?;
        output.push(self.schema_version);
        for value in [
            self.accepted,
            self.rejected,
            self.dropped,
            self.malformed,
            self.replayed,
            self.duplicates,
            self.ack_timeouts,
            self.queue_depth,
            self.active_connections,
            self.queue_high_water,
            self.durability_requests,
            self.durability_requests_coalesced,
            self.group_flushes,
            self.durability_failures,
            self.recent_ipc_latency_samples,
            self.recent_ipc_latency_p50_us,
            self.recent_ipc_latency_p95_us,
            self.recent_ipc_latency_p99_us,
            self.recent_queue_wait_p95_us,
        ] {
            output.extend_from_slice(&value.to_le_bytes());
        }
        encode_optional_u64(output, self.wal_flush_lag_ms);
        encode_optional_u64(output, self.last_wal_flush_duration_us);
        Ok(())
    }

    fn decode_from(input: &mut Cursor<'_>) -> Result<Self, IpcError> {
        let value = Self {
            schema_version: input.u8()?,
            accepted: input.u64()?,
            rejected: input.u64()?,
            dropped: input.u64()?,
            malformed: input.u64()?,
            replayed: input.u64()?,
            duplicates: input.u64()?,
            ack_timeouts: input.u64()?,
            queue_depth: input.u64()?,
            active_connections: input.u64()?,
            queue_high_water: input.u64()?,
            durability_requests: input.u64()?,
            durability_requests_coalesced: input.u64()?,
            group_flushes: input.u64()?,
            durability_failures: input.u64()?,
            recent_ipc_latency_samples: input.u64()?,
            recent_ipc_latency_p50_us: input.u64()?,
            recent_ipc_latency_p95_us: input.u64()?,
            recent_ipc_latency_p99_us: input.u64()?,
            recent_queue_wait_p95_us: input.u64()?,
            wal_flush_lag_ms: input.optional_u64("wal_flush_lag_ms")?,
            last_wal_flush_duration_us: input.optional_u64("last_wal_flush_duration_us")?,
        };
        value.validate()?;
        Ok(value)
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
            Self::BrokerDiagnosticsRequest => 4_u8,
            Self::BrokerDiagnosticsResponse(value) => {
                value.encode_into(&mut payload)?;
                5_u8
            }
        };
        if payload.len() > MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES
            || payload.len() > u16::MAX as usize
        {
            return Err(IpcError::Oversized);
        }
        let mut output = Vec::with_capacity(IPC_FRAME_HEADER_BYTES + payload.len());
        output.extend_from_slice(&IPC_MAGIC);
        output.push(IPC_PROTOCOL_VERSION);
        output.push(frame_type);
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        output.extend_from_slice(&payload);
        Ok(output)
    }

    pub fn decode(input: &[u8]) -> Result<Self, IpcError> {
        if input.len() < IPC_FRAME_HEADER_BYTES {
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
        if u16::from_le_bytes([input[6], input[7]]) != 0 {
            return Err(IpcError::Invalid("flags"));
        }
        let payload_len = u16::from_le_bytes([input[8], input[9]]) as usize;
        if payload_len > MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES
            || input.len() != IPC_FRAME_HEADER_BYTES + payload_len
        {
            return Err(IpcError::Invalid("frame_length"));
        }
        let mut cursor = Cursor::new(&input[IPC_FRAME_HEADER_BYTES..]);
        let frame = match input[5] {
            1 => Self::Start(LifecycleFrame::decode_from(&mut cursor)?),
            2 => Self::Complete {
                lifecycle: LifecycleFrame::decode_from(&mut cursor)?,
                completion: Completion::decode_from(&mut cursor)?,
            },
            3 => Self::Ack(BrokerAcknowledgement::decode(cursor.u8()?)?),
            4 => Self::BrokerDiagnosticsRequest,
            5 => Self::BrokerDiagnosticsResponse(BrokerDiagnostics::decode_from(&mut cursor)?),
            _ => return Err(IpcError::Invalid("frame_type")),
        };
        if !cursor.is_empty() {
            return Err(IpcError::Invalid("trailing_payload"));
        }
        Ok(frame)
    }

    pub fn is_lifecycle(&self) -> bool {
        matches!(self, Self::Start(_) | Self::Complete { .. })
    }
}

pub fn read_frame(mut input: impl Read) -> Result<IpcFrame, IpcError> {
    let mut header = [0_u8; IPC_FRAME_HEADER_BYTES];
    input.read_exact(&mut header).map_err(IpcError::Io)?;
    let length = u16::from_le_bytes([header[8], header[9]]) as usize;
    if length > MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES {
        return Err(IpcError::Oversized);
    }
    let mut encoded = Vec::with_capacity(IPC_FRAME_HEADER_BYTES + length);
    encoded.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    input.read_exact(&mut payload).map_err(IpcError::Io)?;
    encoded.extend_from_slice(&payload);
    IpcFrame::decode(&encoded)
}

pub fn write_frame(mut output: impl Write, frame: &IpcFrame) -> Result<(), IpcError> {
    output.write_all(&frame.encode()?).map_err(IpcError::Io)
}

pub fn read_frame_bounded(input: &mut Stream, timeout: Duration) -> Result<IpcFrame, IpcError> {
    read_frame_until(input, Instant::now() + timeout)
}

fn read_frame_until(input: &mut Stream, deadline: Instant) -> Result<IpcFrame, IpcError> {
    let mut header = [0_u8; IPC_FRAME_HEADER_BYTES];
    read_exact_bounded(input, &mut header, deadline)?;
    let length = u16::from_le_bytes([header[8], header[9]]) as usize;
    if length > MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES {
        return Err(IpcError::Oversized);
    }
    let mut encoded = Vec::with_capacity(IPC_FRAME_HEADER_BYTES + length);
    encoded.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    read_exact_bounded(input, &mut payload, deadline)?;
    encoded.extend_from_slice(&payload);
    IpcFrame::decode(&encoded)
}

pub fn write_frame_bounded(
    stream: &mut Stream,
    frame: &IpcFrame,
    timeout: Duration,
) -> Result<(), IpcError> {
    write_frame_until(stream, frame, Instant::now() + timeout)
}

fn write_frame_until(
    stream: &mut Stream,
    frame: &IpcFrame,
    deadline: Instant,
) -> Result<(), IpcError> {
    let encoded = frame.encode()?;
    write_all_bounded(stream, &encoded, deadline)
}

#[cfg(windows)]
async fn read_frame_bounded_tokio(
    input: &mut TokioStream,
    timeout: Duration,
) -> Result<IpcFrame, IpcError> {
    use tokio::io::AsyncReadExt;

    let deadline = tokio::time::Instant::now() + timeout;
    let mut header = [0_u8; IPC_FRAME_HEADER_BYTES];
    tokio::time::timeout_at(deadline, input.read_exact(&mut header))
        .await
        .map_err(|_| timed_out("bounded IPC read"))?
        .map_err(IpcError::Io)?;
    let length = u16::from_le_bytes([header[8], header[9]]) as usize;
    if length > MAX_IPC_FRAME_BYTES - IPC_FRAME_HEADER_BYTES {
        return Err(IpcError::Oversized);
    }
    let mut encoded = Vec::with_capacity(IPC_FRAME_HEADER_BYTES + length);
    encoded.extend_from_slice(&header);
    let mut payload = vec![0_u8; length];
    tokio::time::timeout_at(deadline, input.read_exact(&mut payload))
        .await
        .map_err(|_| timed_out("bounded IPC read"))?
        .map_err(IpcError::Io)?;
    encoded.extend_from_slice(&payload);
    IpcFrame::decode(&encoded)
}

#[cfg(windows)]
async fn write_frame_bounded_tokio(
    output: &mut TokioStream,
    frame: &IpcFrame,
    timeout: Duration,
) -> Result<(), IpcError> {
    let encoded = frame.encode()?;
    write_encoded_bounded_tokio(output, &encoded, timeout).await
}

#[cfg(windows)]
async fn write_encoded_bounded_tokio(
    output: &mut TokioStream,
    encoded: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    use tokio::io::AsyncWriteExt;

    tokio::time::timeout(timeout, output.write_all(encoded))
        .await
        .map_err(|_| timed_out("bounded IPC write"))?
        .map_err(IpcError::Io)
}

fn read_exact_bounded(
    input: &mut Stream,
    mut buffer: &mut [u8],
    deadline: Instant,
) -> Result<(), IpcError> {
    #[cfg(windows)]
    let mut spins = 0_u32;
    while !buffer.is_empty() {
        match input.read(buffer) {
            Ok(0) => {
                #[cfg(unix)]
                {
                    if unix_peer_closed(input)? {
                        return Err(IpcError::Io(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "Unix IPC peer closed",
                        )));
                    }
                    if Instant::now() >= deadline {
                        return Err(timed_out("bounded IPC read"));
                    }
                    thread::sleep(Duration::from_micros(100));
                }
                #[cfg(windows)]
                {
                    if Instant::now() >= deadline {
                        return Err(timed_out("bounded IPC read"));
                    }
                    spins += 1;
                    if spins < 8 {
                        std::hint::spin_loop();
                    } else {
                        spins = 0;
                        thread::yield_now();
                    }
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
                    return Err(timed_out("bounded IPC read"));
                }
                thread::sleep(Duration::from_micros(100));
            }
            Err(error) => return Err(IpcError::Io(error)),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn unix_peer_closed(input: &Stream) -> Result<bool, IpcError> {
    use nix::{
        errno::Errno,
        sys::socket::{MsgFlags, recv},
    };
    use std::os::fd::AsRawFd;

    let mut probe = [0_u8; 1];
    match input {
        Stream::UdSocket(stream) => match recv(
            stream.inner().as_raw_fd(),
            &mut probe,
            MsgFlags::MSG_PEEK | MsgFlags::MSG_DONTWAIT,
        ) {
            Ok(0) => Ok(true),
            Ok(_) => Ok(false),
            Err(Errno::EAGAIN) => Ok(false),
            Err(error) => Err(IpcError::Io(io::Error::from_raw_os_error(error as i32))),
        },
    }
}

fn write_all_bounded(
    stream: &mut Stream,
    mut buffer: &[u8],
    deadline: Instant,
) -> Result<(), IpcError> {
    let mut spins = 0_u32;
    while !buffer.is_empty() {
        match stream.write(buffer) {
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
                    return Err(timed_out("bounded IPC write"));
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

fn timed_out(message: &'static str) -> IpcError {
    IpcError::Io(io::Error::new(io::ErrorKind::TimedOut, message))
}

/// Secure, per-user local endpoint. Its identifier is opaque; producer frames
/// never select a socket, Named Pipe, state root, or WAL path.
#[derive(Clone, Debug)]
pub struct LocalEndpoint {
    state_root: std::path::PathBuf,
    endpoint_id: String,
}

impl LocalEndpoint {
    pub fn from_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> {
        let state_root = prepare_state_root(root.as_ref())?;
        let endpoint = Self::from_canonical_state_root(state_root);
        endpoint.transport_dir()?;
        Ok(endpoint)
    }

    /// Derives an endpoint only from state that is already present. This is for
    /// read-only observers: unlike [`Self::from_state_root`], it never creates
    /// either the state root or its IPC transport directory.
    pub(crate) fn from_existing_state_root(
        root: impl AsRef<std::path::Path>,
    ) -> Result<Self, IpcError> {
        let state_root = inspect_existing_state_root(root.as_ref())?;
        let endpoint = Self::from_canonical_state_root(state_root);
        endpoint.existing_transport_dir()?;
        Ok(endpoint)
    }

    fn from_canonical_state_root(state_root: std::path::PathBuf) -> Self {
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
        Self {
            state_root,
            endpoint_id,
        }
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

    pub fn transport_dir(&self) -> Result<std::path::PathBuf, IpcError> {
        let dir = self.state_root.join("ipc");
        if dir.exists() {
            return self.existing_transport_dir();
        } else {
            std::fs::create_dir(&dir).map_err(IpcError::Io)?;
        }
        self.existing_transport_dir()
    }

    /// Confirms that the transport directory still exists without creating it.
    /// A read-only observer calls this immediately before connecting so a
    /// concurrent cleanup is reported rather than silently repaired.
    pub(crate) fn validate_existing_transport(&self) -> Result<(), IpcError> {
        self.existing_transport_dir().map(|_| ())
    }

    fn existing_transport_dir(&self) -> Result<std::path::PathBuf, IpcError> {
        let dir = self.state_root.join("ipc");
        let metadata = std::fs::symlink_metadata(&dir).map_err(IpcError::Io)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || state_metadata_is_unsafe(&metadata)
        {
            return Err(IpcError::UnsafeStateObject);
        }
        let canonical = std::fs::canonicalize(&dir).map_err(IpcError::Io)?;
        if canonical.parent() != Some(self.state_root.as_path()) {
            return Err(IpcError::UnsafeStateObject);
        }
        Ok(canonical)
    }

    #[cfg(unix)]
    pub fn connect_stream(&self, timeout: Duration) -> Result<Stream, IpcError> {
        use interprocess::local_socket::{GenericFilePath, ToFsName};

        let name = self
            .unix_socket_path()?
            .to_fs_name::<GenericFilePath>()
            .map_err(IpcError::Io)?;
        let stream = ConnectOptions::new()
            .name(name)
            .wait_mode(ConnectWaitMode::Timeout(timeout))
            .connect_sync()
            .map_err(IpcError::Io)?;
        stream.set_nonblocking(true).map_err(IpcError::Io)?;
        Ok(stream)
    }

    #[cfg(windows)]
    async fn connect_tokio_stream(&self, timeout: Duration) -> Result<TokioStream, IpcError> {
        use interprocess::os::windows::named_pipe::{
            local_socket::tokio::Stream as WindowsTokioStream, pipe_mode::Bytes,
            tokio::DuplexPipeStream,
        };

        let path = format!(r"\\.\pipe\{}", self.named_pipe_name());
        let stream = DuplexPipeStream::<Bytes>::connect_by_path_with_wait_mode(
            path,
            ConnectWaitMode::Timeout(timeout),
        )
        .await
        .map_err(IpcError::Io)?;
        Ok(TokioStream::NamedPipe(WindowsTokioStream::from(stream)))
    }

    pub fn bind(&self) -> Result<Listener, IpcError> {
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
            ListenerOptions::new()
                .name(path.to_fs_name::<GenericFilePath>().map_err(IpcError::Io)?)
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
    pub fn remove_socket_if_owned(&self) {
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

pub fn prepare_state_root(root: &std::path::Path) -> Result<std::path::PathBuf, IpcError> {
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

fn inspect_existing_state_root(root: &std::path::Path) -> Result<std::path::PathBuf, IpcError> {
    let metadata = std::fs::symlink_metadata(root).map_err(IpcError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(IpcError::UnsafeStateObject);
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

/// Direct connected producer retained for broker tests and callers that own
/// a connection lifecycle.
pub struct IpcClient {
    #[cfg(unix)]
    stream: Stream,
    #[cfg(windows)]
    stream: TokioStream,
    #[cfg(windows)]
    runtime: Arc<tokio::runtime::Runtime>,
    timeout: Duration,
}

struct CachedConnection {
    client: IpcClient,
    last_acknowledgement: Instant,
}

/// Sanitized timings used only by the developer-only qualification harness.
#[cfg(feature = "performance-harness")]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct QualificationClientStageSample {
    pub client_frame_encode_ns: u64,
    pub client_write_ns: u64,
    pub client_ack_read_ns: u64,
}

#[cfg(feature = "performance-harness")]
#[derive(Debug)]
pub(crate) enum QualificationSendFailure {
    Write(IpcError),
    Read(IpcError),
    UnexpectedAcknowledgement,
}

#[cfg(feature = "performance-harness")]
fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

impl IpcClient {
    pub fn connect(endpoint: &LocalEndpoint, timeout: Duration) -> Result<Self, IpcError> {
        Self::connect_with_timeouts(endpoint, timeout, timeout)
    }

    /// Connect and acknowledge under independently bounded budgets. The
    /// producer contract reserves a short endpoint probe separately from the
    /// complete write-plus-acknowledgement exchange.
    pub fn connect_with_timeouts(
        endpoint: &LocalEndpoint,
        connect_timeout: Duration,
        acknowledgement_timeout: Duration,
    ) -> Result<Self, IpcError> {
        if connect_timeout.is_zero() || acknowledgement_timeout.is_zero() {
            return Err(IpcError::Invalid("client_timeout"));
        }
        #[cfg(unix)]
        {
            Ok(Self {
                stream: endpoint.connect_stream(connect_timeout)?,
                timeout: acknowledgement_timeout,
            })
        }
        #[cfg(windows)]
        {
            let runtime = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .map_err(IpcError::Io)?,
            );
            Self::connect_with_runtime(endpoint, connect_timeout, acknowledgement_timeout, runtime)
        }
    }

    #[cfg(windows)]
    fn connect_with_runtime(
        endpoint: &LocalEndpoint,
        connect_timeout: Duration,
        acknowledgement_timeout: Duration,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Result<Self, IpcError> {
        let stream = runtime.block_on(endpoint.connect_tokio_stream(connect_timeout))?;
        Ok(Self {
            stream,
            runtime,
            timeout: acknowledgement_timeout,
        })
    }

    pub fn send(&mut self, frame: &IpcFrame) -> Result<BrokerAcknowledgement, IpcError> {
        self.send_with_timeout(frame, self.timeout)
    }
    pub fn send_with_timeout(
        &mut self,
        frame: &IpcFrame,
        timeout: Duration,
    ) -> Result<BrokerAcknowledgement, IpcError> {
        if !frame.is_lifecycle() {
            return Err(IpcError::Invalid("producer_frame_type"));
        }
        if timeout.is_zero() {
            return Err(timed_out("bounded IPC acknowledgement"));
        }
        let deadline = Instant::now() + timeout;
        #[cfg(unix)]
        write_frame_until(&mut self.stream, frame, deadline)?;
        #[cfg(windows)]
        self.runtime.block_on(write_frame_bounded_tokio(
            &mut self.stream,
            frame,
            deadline.saturating_duration_since(Instant::now()),
        ))?;
        #[cfg(unix)]
        match read_frame_until(&mut self.stream, deadline)? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
        #[cfg(windows)]
        match self.runtime.block_on(read_frame_bounded_tokio(
            &mut self.stream,
            deadline.saturating_duration_since(Instant::now()),
        ))? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
    }

    /// Requests one sanitized numeric broker snapshot over the existing local
    /// HSIP control plane. It does not enqueue, append, acknowledge, replay, or
    /// otherwise create evidence.
    pub fn diagnostics(&mut self) -> Result<BrokerDiagnostics, IpcError> {
        let deadline = Instant::now() + self.timeout;
        let request = IpcFrame::BrokerDiagnosticsRequest;
        #[cfg(unix)]
        write_frame_until(&mut self.stream, &request, deadline)?;
        #[cfg(windows)]
        self.runtime.block_on(write_frame_bounded_tokio(
            &mut self.stream,
            &request,
            deadline.saturating_duration_since(Instant::now()),
        ))?;
        #[cfg(unix)]
        let response = read_frame_until(&mut self.stream, deadline)?;
        #[cfg(windows)]
        let response = self.runtime.block_on(read_frame_bounded_tokio(
            &mut self.stream,
            deadline.saturating_duration_since(Instant::now()),
        ))?;
        match response {
            IpcFrame::BrokerDiagnosticsResponse(value) => Ok(value),
            _ => Err(IpcError::Invalid("broker_diagnostics_response")),
        }
    }

    /// Sends a deliberately malformed, already-encoded HSIP test fixture over
    /// the ordinary bounded client connection. This crate-visible seam exists
    /// solely for the reference conformance kit: production producers retain
    /// `send`, which accepts only validated lifecycle frames.
    pub(crate) fn send_encoded_for_conformance(
        &mut self,
        encoded: &[u8],
    ) -> Result<BrokerAcknowledgement, IpcError> {
        if encoded.is_empty() || self.timeout.is_zero() {
            return Err(IpcError::Invalid("conformance_encoded_frame"));
        }
        let deadline = Instant::now() + self.timeout;
        #[cfg(unix)]
        write_all_bounded(&mut self.stream, encoded, deadline)?;
        #[cfg(windows)]
        self.runtime.block_on(write_encoded_bounded_tokio(
            &mut self.stream,
            encoded,
            deadline.saturating_duration_since(Instant::now()),
        ))?;
        #[cfg(unix)]
        match read_frame_until(&mut self.stream, deadline)? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
        #[cfg(windows)]
        match self.runtime.block_on(read_frame_bounded_tokio(
            &mut self.stream,
            deadline.saturating_duration_since(Instant::now()),
        ))? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
    }

    #[cfg(feature = "performance-harness")]
    pub(crate) fn send_for_qualification(
        &mut self,
        frame: &IpcFrame,
    ) -> Result<BrokerAcknowledgement, QualificationSendFailure> {
        if !frame.is_lifecycle() {
            return Err(QualificationSendFailure::Write(IpcError::Invalid(
                "producer_frame_type",
            )));
        }
        let deadline = Instant::now() + self.timeout;
        #[cfg(unix)]
        write_frame_until(&mut self.stream, frame, deadline)
            .map_err(QualificationSendFailure::Write)?;
        #[cfg(windows)]
        self.runtime
            .block_on(write_frame_bounded_tokio(
                &mut self.stream,
                frame,
                deadline.saturating_duration_since(Instant::now()),
            ))
            .map_err(QualificationSendFailure::Write)?;
        #[cfg(unix)]
        let acknowledgement =
            read_frame_until(&mut self.stream, deadline).map_err(QualificationSendFailure::Read)?;
        #[cfg(windows)]
        let acknowledgement = self
            .runtime
            .block_on(read_frame_bounded_tokio(
                &mut self.stream,
                deadline.saturating_duration_since(Instant::now()),
            ))
            .map_err(QualificationSendFailure::Read)?;
        match acknowledgement {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(QualificationSendFailure::UnexpectedAcknowledgement),
        }
    }

    #[cfg(feature = "performance-harness")]
    pub(crate) fn send_for_qualification_timed(
        &mut self,
        frame: &IpcFrame,
    ) -> Result<(BrokerAcknowledgement, QualificationClientStageSample), QualificationSendFailure>
    {
        if !frame.is_lifecycle() {
            return Err(QualificationSendFailure::Write(IpcError::Invalid(
                "producer_frame_type",
            )));
        }
        let deadline = Instant::now() + self.timeout;
        let encode_started = Instant::now();
        let encoded = frame.encode().map_err(QualificationSendFailure::Write)?;
        let client_frame_encode_ns = elapsed_nanos(encode_started);
        let write_started = Instant::now();
        #[cfg(unix)]
        write_all_bounded(&mut self.stream, &encoded, deadline)
            .map_err(QualificationSendFailure::Write)?;
        #[cfg(windows)]
        self.runtime
            .block_on(write_encoded_bounded_tokio(
                &mut self.stream,
                &encoded,
                deadline.saturating_duration_since(Instant::now()),
            ))
            .map_err(QualificationSendFailure::Write)?;
        let client_write_ns = elapsed_nanos(write_started);
        let read_started = Instant::now();
        #[cfg(unix)]
        let acknowledgement =
            read_frame_until(&mut self.stream, deadline).map_err(QualificationSendFailure::Read)?;
        #[cfg(windows)]
        let acknowledgement = self
            .runtime
            .block_on(read_frame_bounded_tokio(
                &mut self.stream,
                deadline.saturating_duration_since(Instant::now()),
            ))
            .map_err(QualificationSendFailure::Read)?;
        let acknowledgement = match acknowledgement {
            IpcFrame::Ack(value) => value,
            _ => return Err(QualificationSendFailure::UnexpectedAcknowledgement),
        };
        Ok((
            acknowledgement,
            QualificationClientStageSample {
                client_frame_encode_ns,
                client_write_ns,
                client_ack_read_ns: elapsed_nanos(read_started),
            },
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerPolicy {
    pub connect_timeout: Duration,
    pub acknowledgement_timeout: Duration,
}
impl Default for ProducerPolicy {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_millis(2),
            acknowledgement_timeout: Duration::from_millis(5),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationDisposition {
    Accepted,
    DroppedOverloaded,
    Busy,
    Rejected,
    Unavailable,
    BudgetExhausted,
}

/// Sanitized feature-only timing for one successful current-producer emit.
/// Durations contain no endpoint, frame, or environment content.
#[cfg(feature = "performance-harness")]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct QualificationProducerStageSample {
    pub endpoint_producer_prework_ns: u64,
    pub connection_acquisition_ns: u64,
    pub frame_encode_ns: u64,
    pub frame_write_ns: u64,
    pub acknowledgement_read_ns: u64,
    pub total_ns: u64,
    pub other_bounded_remainder_ns: u64,
}

/// Tiny, runtime-neutral START/COMPLETE producer. Every failure mode is an
/// observation result: callers must not convert it into a Hook failure.
#[derive(Clone)]
pub struct CooperativeProducer {
    endpoint: LocalEndpoint,
    policy: ProducerPolicy,
    // One local connection is shared by producer clones. `try_lock` makes
    // contention an explicit fail-open observation gap rather than allowing a
    // Hook to block behind another emitter. The broker releases its server-side
    // slot after its bounded read window; a retained stale client is discarded
    // on its next failed operation and is never replayed automatically.
    connection: Arc<Mutex<Option<CachedConnection>>>,
    // The overlapped-I/O runtime is built once per producer, rather than once
    // per lifecycle frame.
    #[cfg(windows)]
    runtime: Arc<tokio::runtime::Runtime>,
}

impl CooperativeProducer {
    pub fn new(endpoint: LocalEndpoint, policy: ProducerPolicy) -> Result<Self, IpcError> {
        if policy.connect_timeout.is_zero() || policy.acknowledgement_timeout.is_zero() {
            return Err(IpcError::Invalid("producer_policy"));
        }
        #[cfg(windows)]
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .map_err(IpcError::Io)?,
        );
        Ok(Self {
            endpoint,
            policy,
            connection: Arc::new(Mutex::new(None)),
            #[cfg(windows)]
            runtime,
        })
    }
    pub fn for_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> {
        Self::new(
            LocalEndpoint::from_state_root(root)?,
            ProducerPolicy::default(),
        )
    }
    pub fn endpoint(&self) -> &LocalEndpoint {
        &self.endpoint
    }
    pub fn emit_start(&self, lifecycle: LifecycleFrame) -> ObservationDisposition {
        self.emit(IpcFrame::Start(lifecycle))
    }
    pub fn emit_complete(
        &self,
        lifecycle: LifecycleFrame,
        completion: Completion,
    ) -> ObservationDisposition {
        self.emit(IpcFrame::Complete {
            lifecycle,
            completion,
        })
    }
    pub fn emit(&self, frame: IpcFrame) -> ObservationDisposition {
        self.emit_with_budget(frame, Duration::MAX)
    }

    /// Measures the current reusable producer without changing its production
    /// behavior. It is unavailable outside the developer performance feature.
    #[cfg(feature = "performance-harness")]
    pub(crate) fn emit_for_qualification_timed(
        &self,
        frame: IpcFrame,
    ) -> Result<QualificationProducerStageSample, ObservationDisposition> {
        let total_started = Instant::now();
        let prework_started = Instant::now();
        let mut connection = self
            .connection
            .try_lock()
            .map_err(|_| ObservationDisposition::Busy)?;
        let endpoint_producer_prework_ns = elapsed_nanos(prework_started);
        let mut connection_acquisition_ns = 0;
        if connection.as_ref().is_some_and(|cached| {
            cached.last_acknowledgement.elapsed() >= PRODUCER_CONNECTION_REUSE_WINDOW
        }) {
            let _ = connection.take();
        }
        if connection.is_none() {
            let connected_started = Instant::now();
            #[cfg(unix)]
            let connected = IpcClient::connect_with_timeouts(
                &self.endpoint,
                self.policy.connect_timeout,
                self.policy.acknowledgement_timeout,
            );
            #[cfg(windows)]
            let connected = IpcClient::connect_with_runtime(
                &self.endpoint,
                self.policy.connect_timeout,
                self.policy.acknowledgement_timeout,
                Arc::clone(&self.runtime),
            );
            let client = connected.map_err(|_| ObservationDisposition::Unavailable)?;
            connection_acquisition_ns = elapsed_nanos(connected_started);
            *connection = Some(CachedConnection {
                client,
                last_acknowledgement: Instant::now(),
            });
        }
        let result = connection
            .as_mut()
            .expect("cooperative connection is established")
            .client
            .send_for_qualification_timed(&frame);
        let (_, client) = match result {
            Ok(value) => {
                connection
                    .as_mut()
                    .expect("cooperative connection remains established")
                    .last_acknowledgement = Instant::now();
                value
            }
            Err(error) => {
                let _ = connection.take();
                return Err(match error {
                    QualificationSendFailure::Write(IpcError::Io(_))
                    | QualificationSendFailure::Read(IpcError::Io(_)) => {
                        ObservationDisposition::Unavailable
                    }
                    _ => ObservationDisposition::Rejected,
                });
            }
        };
        let total_ns = elapsed_nanos(total_started);
        let accounted_ns = endpoint_producer_prework_ns
            .saturating_add(connection_acquisition_ns)
            .saturating_add(client.client_frame_encode_ns)
            .saturating_add(client.client_write_ns)
            .saturating_add(client.client_ack_read_ns);
        Ok(QualificationProducerStageSample {
            endpoint_producer_prework_ns,
            connection_acquisition_ns,
            frame_encode_ns: client.client_frame_encode_ns,
            frame_write_ns: client.client_write_ns,
            acknowledgement_read_ns: client.client_ack_read_ns,
            total_ns,
            other_bounded_remainder_ns: total_ns.saturating_sub(accounted_ns),
        })
    }

    /// Emit one observation without spending more than `budget` on the
    /// endpoint probe plus the complete frame/acknowledgement exchange. This
    /// is deliberately an observation result: a depleted budget never changes
    /// an observed Hook's terminal result.
    pub fn emit_with_budget(&self, frame: IpcFrame, budget: Duration) -> ObservationDisposition {
        if budget.is_zero() {
            return ObservationDisposition::BudgetExhausted;
        }
        let deadline = Instant::now().checked_add(budget);
        self.emit_with_deadline(frame, deadline)
    }

    fn emit_unbounded(&self, frame: IpcFrame) -> ObservationDisposition {
        self.emit_with_deadline(frame, None)
    }

    fn emit_with_deadline(
        &self,
        frame: IpcFrame,
        deadline: Option<Instant>,
    ) -> ObservationDisposition {
        let mut connection = match self.connection.try_lock() {
            Ok(connection) => connection,
            Err(_) => return ObservationDisposition::Busy,
        };
        let remaining = |limit: Duration| {
            deadline.map_or(limit, |deadline| {
                limit.min(deadline.saturating_duration_since(Instant::now()))
            })
        };
        if connection.as_ref().is_some_and(|cached| {
            cached.last_acknowledgement.elapsed() >= PRODUCER_CONNECTION_REUSE_WINDOW
        }) {
            // Dropping before a new frame is safe: no lifecycle data is in
            // flight. A long-running Hook therefore reconnects transparently
            // rather than attempting COMPLETE on the broker's expired read
            // window.
            let _ = connection.take();
        }
        if connection.is_none() {
            let connect_timeout = remaining(self.policy.connect_timeout);
            if connect_timeout.is_zero() {
                return ObservationDisposition::BudgetExhausted;
            }
            #[cfg(unix)]
            let connected = IpcClient::connect_with_timeouts(
                &self.endpoint,
                connect_timeout,
                self.policy.acknowledgement_timeout,
            );
            #[cfg(windows)]
            let connected = IpcClient::connect_with_runtime(
                &self.endpoint,
                connect_timeout,
                self.policy.acknowledgement_timeout,
                Arc::clone(&self.runtime),
            );
            let Ok(client) = connected else {
                return ObservationDisposition::Unavailable;
            };
            *connection = Some(CachedConnection {
                client,
                last_acknowledgement: Instant::now(),
            });
        }
        let acknowledgement_timeout = remaining(self.policy.acknowledgement_timeout);
        if acknowledgement_timeout.is_zero() {
            return ObservationDisposition::BudgetExhausted;
        }
        let result = connection
            .as_mut()
            .expect("cooperative connection is established")
            .client
            .send_with_timeout(&frame, acknowledgement_timeout);
        if result.is_err() {
            // An ACK read failure can occur after the broker has appended the
            // frame. Discard the connection for a future bounded reconnect but
            // never replay this frame and risk fabricating duplicate evidence.
            let _ = connection.take();
        }
        match result {
            Ok(value) => {
                connection
                    .as_mut()
                    .expect("cooperative connection remains established")
                    .last_acknowledgement = Instant::now();
                observation_from_ack(value)
            }
            Err(IpcError::Io(error)) if error.kind() == io::ErrorKind::TimedOut => {
                ObservationDisposition::BudgetExhausted
            }
            Err(IpcError::Io(_)) => ObservationDisposition::Unavailable,
            Err(_) => ObservationDisposition::Rejected,
        }
    }
}

/// Maps every broker response to an explicit observation disposition. None of
/// these values is an execution failure for an observed Hook.
#[must_use]
pub const fn observation_from_ack(value: BrokerAcknowledgement) -> ObservationDisposition {
    match value {
        BrokerAcknowledgement::Accepted => ObservationDisposition::Accepted,
        BrokerAcknowledgement::DroppedOverloaded => ObservationDisposition::DroppedOverloaded,
        BrokerAcknowledgement::Busy => ObservationDisposition::Busy,
        BrokerAcknowledgement::Rejected => ObservationDisposition::Rejected,
    }
}

/// Bounded race-safe startup election. The actual broker launch remains an
/// injected, product-owned responsibility; this client never starts a network
/// service or a global daemon by itself.
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
                // A follower can miss a healthy Windows pipe while every
                // current instance is busy, then acquire the lease just after
                // the original starter releases it. Recheck while elected so
                // a transiently busy endpoint cannot trigger a second broker.
                // A genuinely absent pipe returns immediately; the wait is
                // bounded only for an existing busy endpoint.
                let recheck_timeout = self.timeout.min(Duration::from_millis(25));
                if let Ok(client) = IpcClient::connect(&self.endpoint, recheck_timeout) {
                    return Ok(client);
                }
                start()?;
                loop {
                    if let Ok(client) = IpcClient::connect(&self.endpoint, Duration::from_millis(2))
                    {
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
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                ) =>
            {
                let metadata = match std::fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(metadata_error)
                        if matches!(
                            metadata_error.kind(),
                            io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                        ) =>
                    {
                        // The current lease can disappear between create-new
                        // and inspection. Windows can also report a live lease
                        // as access denied. Neither state grants ownership;
                        // the caller retries under its startup deadline.
                        return Ok(None);
                    }
                    Err(metadata_error) => return Err(IpcError::Io(metadata_error)),
                };
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(IpcError::UnsafeStateObject);
                }
                if metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.elapsed().ok())
                    .is_some_and(|elapsed| elapsed >= stale_after)
                {
                    match std::fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(remove_error)
                            if matches!(
                                remove_error.kind(),
                                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                            ) =>
                        {
                            // Cleanup lost a race or the lease is still in
                            // transition; ownership remains ungranted.
                        }
                        Err(remove_error) => return Err(IpcError::Io(remove_error)),
                    }
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

#[derive(Debug)]
pub enum IpcError {
    Io(io::Error),
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
        formatter.write_str(match self {
            Self::Io(_) => "local IPC I/O failed",
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
        })
    }
}
impl std::error::Error for IpcError {}

pub fn checksum(value: &[u8]) -> u32 {
    let digest = Sha256::digest(value);
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}
fn validate_reference(field: &'static str, value: &str) -> Result<(), IpcError> {
    if value.is_empty()
        || value.len() > MAX_IPC_REFERENCE_BYTES
        || value.chars().any(
            |value| !matches!(value, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':'),
        )
    {
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
fn encode_optional_u64(output: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_le_bytes());
        }
        None => output.push(0),
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
    fn optional_u64(&mut self, field: &'static str) -> Result<Option<u64>, IpcError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            _ => Err(IpcError::Invalid(field)),
        }
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
                terminal_status: TerminalOutcome::Failed,
                exit_classification: ExitClassification::ExitCode,
                exit_value: Some(7),
                duration_ms: 12,
            },
        }
    }
    #[test]
    fn frame_round_trip_is_bounded_binary_and_versioned() {
        let frame = complete();
        let encoded = frame.encode().unwrap();
        assert!(encoded.starts_with(&IPC_MAGIC));
        assert_eq!(encoded[4], IPC_PROTOCOL_VERSION);
        assert_eq!(IpcFrame::decode(&encoded).unwrap(), frame);
    }

    #[test]
    fn broker_diagnostics_control_frames_are_bounded_numeric_and_not_evidence() {
        let diagnostics = BrokerDiagnostics {
            schema_version: BROKER_DIAGNOSTICS_SCHEMA_VERSION,
            accepted: 10,
            rejected: 1,
            dropped: 2,
            malformed: 3,
            replayed: 4,
            duplicates: 5,
            ack_timeouts: 6,
            queue_depth: 7,
            active_connections: 8,
            queue_high_water: 9,
            durability_requests: 10,
            durability_requests_coalesced: 11,
            group_flushes: 12,
            durability_failures: 0,
            recent_ipc_latency_samples: 13,
            recent_ipc_latency_p50_us: 14,
            recent_ipc_latency_p95_us: 15,
            recent_ipc_latency_p99_us: 16,
            recent_queue_wait_p95_us: 17,
            wal_flush_lag_ms: Some(18),
            last_wal_flush_duration_us: Some(19),
        };
        for frame in [
            IpcFrame::BrokerDiagnosticsRequest,
            IpcFrame::BrokerDiagnosticsResponse(diagnostics),
        ] {
            let encoded = frame.encode().unwrap();
            assert!(encoded.len() <= MAX_IPC_FRAME_BYTES);
            assert_eq!(IpcFrame::decode(&encoded).unwrap(), frame);
            assert!(!frame.is_lifecycle());
        }
        let serialized = serde_json::to_string(&diagnostics).unwrap();
        for forbidden in [
            "runtime",
            "handler",
            "command",
            "path",
            "prompt",
            "payload",
            "stdout",
            "stderr",
            "credential",
        ] {
            assert!(!serialized.contains(forbidden));
        }
    }
    #[test]
    fn malformed_and_private_values_are_rejected_or_absent() {
        let encoded = complete().encode().unwrap();
        assert!(
            !encoded
                .windows(b"raw-command-value".len())
                .any(|value| value == b"raw-command-value")
        );
        let mut invalid = lifecycle();
        invalid.runtime = "x".repeat(MAX_IPC_REFERENCE_BYTES + 1);
        assert!(invalid.validate().is_err());
        let mut bad = encoded;
        bad[4] += 1;
        assert!(matches!(
            IpcFrame::decode(&bad),
            Err(IpcError::UnsupportedVersion)
        ));
    }
    #[test]
    fn overload_and_busy_are_explicit_fail_open_observation_outcomes() {
        assert_eq!(
            observation_from_ack(BrokerAcknowledgement::DroppedOverloaded),
            ObservationDisposition::DroppedOverloaded
        );
        assert_eq!(
            observation_from_ack(BrokerAcknowledgement::Busy),
            ObservationDisposition::Busy
        );
        assert_eq!(
            observation_from_ack(BrokerAcknowledgement::Rejected),
            ObservationDisposition::Rejected
        );
    }

    #[test]
    fn producer_keeps_the_acknowledgement_budget_after_a_fast_connection() {
        let temp = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(temp.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (hold_ack_tx, hold_ack_rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            ready_tx.send(()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut stream = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "producer did not connect");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("listener failed: {error}"),
                }
            };
            assert!(matches!(
                read_frame_bounded(&mut stream, Duration::from_millis(100)),
                Ok(IpcFrame::Start(_))
            ));
            // This deliberately exceeds the short connect budget but remains
            // inside the independent acknowledgement budget.
            thread::sleep(Duration::from_millis(30));
            write_frame_bounded(
                &mut stream,
                &IpcFrame::Ack(BrokerAcknowledgement::Accepted),
                Duration::from_millis(100),
            )
            .unwrap();
            hold_ack_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        });
        ready_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let producer = CooperativeProducer::new(
            endpoint,
            ProducerPolicy {
                connect_timeout: Duration::from_millis(10),
                acknowledgement_timeout: Duration::from_millis(100),
            },
        )
        .unwrap();
        assert_eq!(
            producer.emit_start(lifecycle()),
            ObservationDisposition::Accepted
        );
        hold_ack_tx.send(()).unwrap();
        server.join().unwrap();
    }

    #[test]
    fn producer_reuses_one_connection_for_adjacent_lifecycle_frames() {
        let temp = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(temp.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut stream = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "producer did not connect");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("listener failed: {error}"),
                }
            };
            for expected in [true, false] {
                let frame = read_frame_bounded(&mut stream, Duration::from_millis(100));
                assert!(matches!(
                    (expected, frame),
                    (true, Ok(IpcFrame::Start(_))) | (false, Ok(IpcFrame::Complete { .. }))
                ));
                write_frame_bounded(
                    &mut stream,
                    &IpcFrame::Ack(BrokerAcknowledgement::Accepted),
                    Duration::from_millis(100),
                )
                .unwrap();
            }
        });
        let producer = CooperativeProducer::new(
            endpoint,
            ProducerPolicy {
                connect_timeout: Duration::from_millis(100),
                acknowledgement_timeout: Duration::from_millis(100),
            },
        )
        .unwrap();
        let frame = lifecycle();
        assert_eq!(
            producer.emit_start(frame.clone()),
            ObservationDisposition::Accepted
        );
        assert_eq!(
            producer.emit_complete(
                frame,
                Completion {
                    terminal_status: TerminalOutcome::Completed,
                    exit_classification: ExitClassification::ExitCode,
                    exit_value: Some(0),
                    duration_ms: 1,
                }
            ),
            ObservationDisposition::Accepted
        );
        server.join().unwrap();
    }

    #[test]
    fn producer_reconnects_before_a_long_running_hook_completion() {
        let temp = tempfile::tempdir().unwrap();
        let endpoint = LocalEndpoint::from_state_root(temp.path()).unwrap();
        let listener = endpoint.bind().unwrap();
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut first = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "producer did not connect");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("listener failed: {error}"),
                }
            };
            assert!(matches!(
                read_frame_bounded(&mut first, Duration::from_millis(100)),
                Ok(IpcFrame::Start(_))
            ));
            write_frame_bounded(
                &mut first,
                &IpcFrame::Ack(BrokerAcknowledgement::Accepted),
                Duration::from_millis(100),
            )
            .unwrap();
            drop(first);
            let mut second = loop {
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "producer did not reconnect");
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("listener failed: {error}"),
                }
            };
            assert!(matches!(
                read_frame_bounded(&mut second, Duration::from_millis(100)),
                Ok(IpcFrame::Complete { .. })
            ));
            write_frame_bounded(
                &mut second,
                &IpcFrame::Ack(BrokerAcknowledgement::Accepted),
                Duration::from_millis(100),
            )
            .unwrap();
        });
        let producer = CooperativeProducer::new(
            endpoint,
            ProducerPolicy {
                connect_timeout: Duration::from_millis(100),
                acknowledgement_timeout: Duration::from_millis(100),
            },
        )
        .unwrap();
        let frame = lifecycle();
        assert_eq!(
            producer.emit_start(frame.clone()),
            ObservationDisposition::Accepted
        );
        // This exceeds the producer reuse window but does not consume an
        // unbounded broker slot. COMPLETE uses a fresh connection before any
        // lifecycle frame is attempted on the stale pipe.
        thread::sleep(PRODUCER_CONNECTION_REUSE_WINDOW + Duration::from_millis(3));
        assert_eq!(
            producer.emit_complete(
                frame,
                Completion {
                    terminal_status: TerminalOutcome::Completed,
                    exit_classification: ExitClassification::ExitCode,
                    exit_value: Some(0),
                    duration_ms: 1,
                }
            ),
            ObservationDisposition::Accepted
        );
        server.join().unwrap();
    }
}
