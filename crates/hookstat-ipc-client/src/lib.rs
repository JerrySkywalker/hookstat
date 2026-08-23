//! Private, bounded local IPC protocol and fail-open producer surface.
//!
//! This crate is intentionally the only definition of the HookStat IPC v1
//! frame. It is usable by a cooperative Hook or the `hookstat-hook` shim
//! without bringing the HookStat application, SQLite, reporting, analytics,
//! or terminal UI into the hot path.

use interprocess::ConnectWaitMode;
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
use interprocess::local_socket::{
    ConnectOptions, ListenerNonblockingMode, ListenerOptions, prelude::*,
};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Read, Write};
use std::thread;
use std::time::{Duration, Instant};

pub use interprocess::local_socket::{Listener, Stream};

/// The first and only production wire version.
pub const IPC_PROTOCOL_VERSION: u8 = 1;
pub const IPC_MAGIC: [u8; 4] = *b"HSIP";
pub const MAX_IPC_FRAME_BYTES: usize = 1024;
pub const MAX_IPC_REFERENCE_BYTES: usize = 128;

const FRAME_HEADER_BYTES: usize = 10;

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
        output.extend_from_slice(&0_u16.to_le_bytes());
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
        let frame = match input[5] {
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

    pub fn is_lifecycle(&self) -> bool {
        matches!(self, Self::Start(_) | Self::Complete { .. })
    }
}

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
    output.write_all(&frame.encode()?).map_err(IpcError::Io)
}

pub fn read_frame_bounded(input: &mut Stream, timeout: Duration) -> Result<IpcFrame, IpcError> {
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

pub fn write_frame_bounded(
    stream: &mut Stream,
    frame: &IpcFrame,
    timeout: Duration,
) -> Result<(), IpcError> {
    let encoded = frame.encode()?;
    write_all_bounded(stream, &encoded, Instant::now() + timeout)
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
                if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
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
            Ok(0) => return Err(IpcError::Io(io::Error::new(io::ErrorKind::WriteZero, "bounded IPC write"))),
            Ok(written) => buffer = &buffer[written..],
            Err(error)
                if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
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
        let endpoint_id = digest[..16].iter().map(|byte| format!("{byte:02x}")).collect();
        let endpoint = Self { state_root, endpoint_id };
        endpoint.transport_dir()?;
        Ok(endpoint)
    }

    pub fn state_root(&self) -> &std::path::Path { &self.state_root }
    pub fn endpoint_id(&self) -> &str { &self.endpoint_id }

    #[cfg(unix)]
    pub fn unix_socket_path(&self) -> Result<std::path::PathBuf, IpcError> {
        let path = self.transport_dir()?.join(format!("g35-{}.sock", self.endpoint_id));
        if path.as_os_str().as_encoded_bytes().len() > 96 {
            return Err(IpcError::Invalid("unix_socket_path_length"));
        }
        Ok(path)
    }

    #[cfg(windows)]
    pub fn named_pipe_name(&self) -> String { format!("hookstat-g35-{}", self.endpoint_id) }

    pub fn transport_dir(&self) -> Result<std::path::PathBuf, IpcError> {
        let dir = self.state_root.join("ipc");
        if dir.exists() {
            let metadata = std::fs::symlink_metadata(&dir).map_err(IpcError::Io)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() || state_metadata_is_unsafe(&metadata) {
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

    pub fn connect_stream(&self, timeout: Duration) -> Result<Stream, IpcError> {
        #[cfg(unix)]
        let name = {
            use interprocess::local_socket::{GenericFilePath, ToFsName};
            self.unix_socket_path()?.to_fs_name::<GenericFilePath>().map_err(IpcError::Io)?
        };
        #[cfg(windows)]
        let name = self.named_pipe_name().to_ns_name::<GenericNamespaced>().map_err(IpcError::Io)?;
        let stream = ConnectOptions::new().name(name).wait_mode(ConnectWaitMode::Timeout(timeout)).connect_sync().map_err(IpcError::Io)?;
        stream.set_nonblocking(true).map_err(IpcError::Io)?;
        Ok(stream)
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
            ListenerOptions::new().name(path.to_fs_name::<GenericFilePath>().map_err(IpcError::Io)?).nonblocking(ListenerNonblockingMode::Accept).reclaim_name(false).max_spin_time(Duration::ZERO).mode(0o600).create_sync().map_err(IpcError::Io)
        }
        #[cfg(windows)]
        {
            use interprocess::os::windows::local_socket::ListenerOptionsExt;
            let name = self.named_pipe_name().to_ns_name::<GenericNamespaced>().map_err(IpcError::Io)?;
            ListenerOptions::new().name(name).nonblocking(ListenerNonblockingMode::Accept).reclaim_name(false).security_descriptor(owner_only_pipe_security_descriptor()?).create_sync().map_err(|error| if error.kind() == io::ErrorKind::AddrInUse { IpcError::EndpointInUse } else { IpcError::Io(error) })
        }
    }

    #[cfg(unix)]
    pub fn remove_socket_if_owned(&self) {
        if let Ok(path) = self.unix_socket_path() && let Ok(metadata) = std::fs::symlink_metadata(&path) {
            use std::os::unix::fs::FileTypeExt;
            if metadata.file_type().is_socket() { let _ = std::fs::remove_file(path); }
        }
    }
}

#[cfg(windows)]
fn owner_only_pipe_security_descriptor() -> Result<interprocess::os::windows::security_descriptor::SecurityDescriptor, IpcError> {
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;
    let sddl = widestring::U16CString::from_str("D:P(A;;GRGW;;;OW)").map_err(|_| IpcError::Invalid("pipe_security_descriptor"))?;
    SecurityDescriptor::deserialize(sddl.as_ucstr()).map_err(IpcError::Io)
}

pub fn prepare_state_root(root: &std::path::Path) -> Result<std::path::PathBuf, IpcError> {
    if root.exists() {
        let metadata = std::fs::symlink_metadata(root).map_err(IpcError::Io)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() { return Err(IpcError::UnsafeStateObject); }
    } else {
        std::fs::create_dir_all(root).map_err(IpcError::Io)?;
    }
    let root = std::fs::canonicalize(root).map_err(IpcError::Io)?;
    let metadata = std::fs::symlink_metadata(&root).map_err(IpcError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || state_metadata_is_unsafe(&metadata) { return Err(IpcError::UnsafeStateObject); }
    Ok(root)
}

fn state_metadata_is_unsafe(metadata: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o022 != 0 { return true; }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 { return true; }
    }
    false
}

/// Direct connected producer retained for broker tests and callers that own
/// a connection lifecycle.
pub struct IpcClient { stream: Stream, timeout: Duration }

impl IpcClient {
    pub fn connect(endpoint: &LocalEndpoint, timeout: Duration) -> Result<Self, IpcError> {
        Ok(Self { stream: endpoint.connect_stream(timeout)?, timeout })
    }
    pub fn send(&mut self, frame: &IpcFrame) -> Result<BrokerAcknowledgement, IpcError> {
        if !frame.is_lifecycle() { return Err(IpcError::Invalid("producer_frame_type")); }
        write_frame_bounded(&mut self.stream, frame, self.timeout)?;
        match read_frame_bounded(&mut self.stream, self.timeout)? {
            IpcFrame::Ack(value) => Ok(value),
            _ => Err(IpcError::Invalid("acknowledgement")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerPolicy { pub connect_timeout: Duration, pub acknowledgement_timeout: Duration }
impl Default for ProducerPolicy {
    fn default() -> Self { Self { connect_timeout: Duration::from_millis(2), acknowledgement_timeout: Duration::from_millis(5) } }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationDisposition { Accepted, DroppedOverloaded, Busy, Rejected, Unavailable }

/// Tiny, runtime-neutral START/COMPLETE producer. Every failure mode is an
/// observation result: callers must not convert it into a Hook failure.
#[derive(Clone, Debug)]
pub struct CooperativeProducer { endpoint: LocalEndpoint, policy: ProducerPolicy }

impl CooperativeProducer {
    pub fn new(endpoint: LocalEndpoint, policy: ProducerPolicy) -> Result<Self, IpcError> {
        if policy.connect_timeout.is_zero() || policy.acknowledgement_timeout.is_zero() { return Err(IpcError::Invalid("producer_policy")); }
        Ok(Self { endpoint, policy })
    }
    pub fn for_state_root(root: impl AsRef<std::path::Path>) -> Result<Self, IpcError> { Self::new(LocalEndpoint::from_state_root(root)?, ProducerPolicy::default()) }
    pub fn endpoint(&self) -> &LocalEndpoint { &self.endpoint }
    pub fn emit_start(&self, lifecycle: LifecycleFrame) -> ObservationDisposition { self.emit(IpcFrame::Start(lifecycle)) }
    pub fn emit_complete(&self, lifecycle: LifecycleFrame, completion: Completion) -> ObservationDisposition { self.emit(IpcFrame::Complete { lifecycle, completion }) }
    pub fn emit(&self, frame: IpcFrame) -> ObservationDisposition {
        let timeout = self.policy.connect_timeout.min(self.policy.acknowledgement_timeout);
        let Ok(mut client) = IpcClient::connect(&self.endpoint, timeout) else { return ObservationDisposition::Unavailable; };
        match client.send(&frame) {
            Ok(BrokerAcknowledgement::Accepted) => ObservationDisposition::Accepted,
            Ok(BrokerAcknowledgement::DroppedOverloaded) => ObservationDisposition::DroppedOverloaded,
            Ok(BrokerAcknowledgement::Busy) => ObservationDisposition::Busy,
            Ok(BrokerAcknowledgement::Rejected) | Err(_) => ObservationDisposition::Rejected,
        }
    }
}

/// Bounded race-safe startup election. The actual broker launch remains an
/// injected, product-owned responsibility; this client never starts a network
/// service or a global daemon by itself.
#[derive(Clone, Debug)]
pub struct BrokerStartup { endpoint: LocalEndpoint, timeout: Duration, stale_lease_after: Duration }

impl BrokerStartup {
    pub fn new(endpoint: LocalEndpoint, timeout: Duration) -> Result<Self, IpcError> {
        if timeout.is_zero() { return Err(IpcError::Invalid("startup_timeout")); }
        Ok(Self { endpoint, timeout, stale_lease_after: timeout })
    }
    pub fn connect_or_start(&self, mut start: impl FnMut() -> Result<(), IpcError>) -> Result<IpcClient, IpcError> {
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Ok(client) = IpcClient::connect(&self.endpoint, Duration::from_millis(2)) { return Ok(client); }
            if let Some(_lease) = StartupLease::try_acquire(&self.endpoint, self.stale_lease_after)? {
                start()?;
                loop {
                    if let Ok(client) = IpcClient::connect(&self.endpoint, Duration::from_millis(2)) {
                        thread::sleep(Duration::from_millis(25));
                        return Ok(client);
                    }
                    if Instant::now() >= deadline { return Err(IpcError::StartupTimedOut); }
                    thread::sleep(Duration::from_millis(2));
                }
            }
            if Instant::now() >= deadline { return Err(IpcError::StartupTimedOut); }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

struct StartupLease { path: std::path::PathBuf }
impl StartupLease {
    fn try_acquire(endpoint: &LocalEndpoint, stale_after: Duration) -> Result<Option<Self>, IpcError> {
        let path = endpoint.transport_dir()?.join("broker-start-v1.lock");
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Some(Self { path })),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = std::fs::symlink_metadata(&path).map_err(IpcError::Io)?;
                if metadata.file_type().is_symlink() || !metadata.is_file() { return Err(IpcError::UnsafeStateObject); }
                if metadata.modified().ok().and_then(|value| value.elapsed().ok()).is_some_and(|elapsed| elapsed >= stale_after) { std::fs::remove_file(&path).map_err(IpcError::Io)?; }
                Ok(None)
            }
            Err(error) => Err(IpcError::Io(error)),
        }
    }
}
impl Drop for StartupLease { fn drop(&mut self) { let _ = std::fs::remove_file(&self.path); } }

#[derive(Debug)]
pub enum IpcError {
    Io(io::Error), BadMagic, UnsupportedVersion, Oversized, Truncated, Invalid(&'static str), UnsafeStateObject, EndpointInUse, StartupTimedOut, WalTooLarge, WalCorrupt(&'static str),
}
impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Io(_) => "local IPC I/O failed", Self::BadMagic => "IPC frame magic was invalid", Self::UnsupportedVersion => "IPC protocol version is not supported", Self::Oversized => "IPC frame exceeded a bounded limit", Self::Truncated => "IPC frame was truncated", Self::Invalid(_) => "IPC frame structure was invalid", Self::UnsafeStateObject => "IPC state root contained an unsafe object", Self::EndpointInUse => "IPC endpoint is already owned by a healthy broker", Self::StartupTimedOut => "IPC broker startup did not become ready within its bounded timeout", Self::WalTooLarge => "IPC WAL exceeded its bounded size", Self::WalCorrupt(_) => "IPC WAL contained a malformed record",
        })
    }
}
impl std::error::Error for IpcError {}

pub fn checksum(value: &[u8]) -> u32 { let digest = Sha256::digest(value); u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) }
fn validate_reference(field: &'static str, value: &str) -> Result<(), IpcError> {
    if value.is_empty() || value.len() > MAX_IPC_REFERENCE_BYTES || value.chars().any(|value| !matches!(value, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':')) { return Err(IpcError::Invalid(field)); }
    Ok(())
}
fn encode_reference(output: &mut Vec<u8>, value: &str) -> Result<(), IpcError> { validate_reference("reference", value)?; output.push(u8::try_from(value.len()).map_err(|_| IpcError::Oversized)?); output.extend_from_slice(value.as_bytes()); Ok(()) }
struct Cursor<'a> { input: &'a [u8], offset: usize }
impl<'a> Cursor<'a> {
    fn new(input: &'a [u8]) -> Self { Self { input, offset: 0 } }
    fn is_empty(&self) -> bool { self.offset == self.input.len() }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], IpcError> { let end = self.offset.checked_add(length).ok_or(IpcError::Truncated)?; let value = self.input.get(self.offset..end).ok_or(IpcError::Truncated)?; self.offset = end; Ok(value) }
    fn u8(&mut self) -> Result<u8, IpcError> { Ok(self.bytes(1)?[0]) }
    fn i32(&mut self) -> Result<i32, IpcError> { Ok(i32::from_le_bytes(self.bytes(4)?.try_into().map_err(|_| IpcError::Truncated)?)) }
    fn i64(&mut self) -> Result<i64, IpcError> { Ok(i64::from_le_bytes(self.bytes(8)?.try_into().map_err(|_| IpcError::Truncated)?)) }
    fn u64(&mut self) -> Result<u64, IpcError> { Ok(u64::from_le_bytes(self.bytes(8)?.try_into().map_err(|_| IpcError::Truncated)?)) }
    fn reference(&mut self, field: &'static str) -> Result<String, IpcError> { let length = self.u8()? as usize; if length == 0 || length > MAX_IPC_REFERENCE_BYTES { return Err(IpcError::Invalid(field)); } let value = std::str::from_utf8(self.bytes(length)?).map_err(|_| IpcError::Invalid(field))?.to_owned(); validate_reference(field, &value)?; Ok(value) }
    fn optional_reference(&mut self, field: &'static str) -> Result<Option<String>, IpcError> { let length = self.u8()? as usize; if length == 0 { return Ok(None); } if length > MAX_IPC_REFERENCE_BYTES { return Err(IpcError::Invalid(field)); } let value = std::str::from_utf8(self.bytes(length)?).map_err(|_| IpcError::Invalid(field))?.to_owned(); validate_reference(field, &value)?; Ok(Some(value)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lifecycle() -> LifecycleFrame { LifecycleFrame { runtime: "synthetic_runtime".into(), runtime_instance: "instance_a".into(), invocation: "invocation_a".into(), handler: "handler_a".into(), event: "event_a".into(), source_scope: "scope_a".into(), revision: Some("revision_a".into()), occurred_at_unix_ms: 1_000 } }
    fn complete() -> IpcFrame { IpcFrame::Complete { lifecycle: lifecycle(), completion: Completion { terminal_status: TerminalOutcome::Failed, exit_classification: ExitClassification::ExitCode, exit_value: Some(7), duration_ms: 12 } } }
    #[test]
    fn frame_round_trip_is_bounded_binary_and_versioned() { let frame = complete(); let encoded = frame.encode().unwrap(); assert!(encoded.starts_with(&IPC_MAGIC)); assert_eq!(encoded[4], IPC_PROTOCOL_VERSION); assert_eq!(IpcFrame::decode(&encoded).unwrap(), frame); }
    #[test]
    fn malformed_and_private_values_are_rejected_or_absent() { let encoded = complete().encode().unwrap(); assert!(!encoded.windows(b"raw-command-value".len()).any(|value| value == b"raw-command-value")); let mut invalid = lifecycle(); invalid.runtime = "x".repeat(MAX_IPC_REFERENCE_BYTES + 1); assert!(invalid.validate().is_err()); let mut bad = encoded; bad[4] += 1; assert!(matches!(IpcFrame::decode(&bad), Err(IpcError::UnsupportedVersion))); }
}
