//! Private handler capsule validation and transparent shim execution.
//!
//! Capsule command material is a private control-plane payload. This crate
//! never writes it to IPC, diagnostics, standard output/error, a WAL, or a
//! HookStat ledger.

use crate::ipc_client::{
    Completion, CooperativeProducer, ExitClassification, IpcError, IpcFrame, LifecycleFrame,
    ObservationDisposition, TerminalOutcome,
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

type HmacSha256 = Hmac<Sha256>;

#[cfg(feature = "performance-harness")]
fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

const CAPSULE_MAGIC: [u8; 4] = *b"HSHC";
const CAPSULE_SCHEMA_VERSION: u8 = 1;
const CAPSULE_MAC_BYTES: usize = 32;
const MAX_CAPSULE_BYTES: usize = 16 * 1024;
const MAX_PRIVATE_COMMAND_BYTES: usize = 8 * 1024;
const MAX_ARGUMENTS: usize = 32;

/// Original handler time remains independent of any outer declaration. The
/// shim enforces this exact budget itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginalHandlerBudget(pub Duration);

/// An outer runtime may grant this much time only for bounded instrumentation.
/// The value is capsule metadata; it never increases the handler budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrumentationEnvelope(pub Duration);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionPlan {
    /// The plan was preclassified by trusted instrumentation as executable and
    /// argv. The shim never reparses a shell command to construct this plan.
    Direct {
        executable: String,
        arguments: Vec<String>,
    },
    /// Exact platform shell fallback for command semantics that cannot be
    /// safely represented as a direct executable/argv plan.
    Shell { command: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerCapsule {
    pub handler_key: String,
    pub revision: String,
    pub definition_fingerprint: String,
    pub runtime: String,
    pub runtime_instance: String,
    pub event: String,
    pub source_scope: String,
    pub original_budget: OriginalHandlerBudget,
    pub instrumentation_envelope: InstrumentationEnvelope,
    pub execution: ExecutionPlan,
}

impl HandlerCapsule {
    pub fn validate(&self) -> Result<(), CapsuleError> {
        for value in [
            &self.handler_key,
            &self.revision,
            &self.definition_fingerprint,
            &self.runtime,
            &self.runtime_instance,
            &self.event,
            &self.source_scope,
        ] {
            validate_reference(value)?;
        }
        // G28's frozen cold shim p95 is 50 ms. This ceiling is not a business
        // timeout increase; it is the maximum metadata envelope that a later
        // runtime declaration may reserve for instrumentation alone.
        if self.original_budget.0.is_zero()
            || self.original_budget.0.as_millis() > u128::from(u64::MAX)
            || self.instrumentation_envelope.0 > Duration::from_millis(50)
        {
            return Err(CapsuleError::Invalid("budget"));
        }
        match &self.execution {
            ExecutionPlan::Direct {
                executable,
                arguments,
            } => {
                validate_private_text(executable)?;
                if arguments.len() > MAX_ARGUMENTS {
                    return Err(CapsuleError::Invalid("argument_count"));
                }
                for value in arguments {
                    validate_private_text(value)?;
                }
            }
            ExecutionPlan::Shell { command } => validate_private_text(command)?,
        }
        Ok(())
    }

    pub fn lifecycle(
        &self,
        invocation: String,
        occurred_at_unix_ms: i64,
    ) -> Result<LifecycleFrame, CapsuleError> {
        self.validate()?;
        Ok(LifecycleFrame {
            runtime: self.runtime.clone(),
            runtime_instance: self.runtime_instance.clone(),
            invocation,
            handler: self.handler_key.clone(),
            event: self.event.clone(),
            source_scope: self.source_scope.clone(),
            revision: Some(self.revision.clone()),
            occurred_at_unix_ms,
        })
    }

    /// Only an instrumentation/control-plane caller that holds the private
    /// key may emit this sealed payload. No capsule body is sent to IPC.
    pub fn seal(&self, key: &[u8; CAPSULE_MAC_BYTES]) -> Result<Vec<u8>, CapsuleError> {
        self.validate()?;
        let mut body = Vec::with_capacity(512);
        body.extend_from_slice(&CAPSULE_MAGIC);
        body.push(CAPSULE_SCHEMA_VERSION);
        for value in [
            &self.handler_key,
            &self.revision,
            &self.definition_fingerprint,
            &self.runtime,
            &self.runtime_instance,
            &self.event,
            &self.source_scope,
        ] {
            put_text(&mut body, value)?;
        }
        body.extend_from_slice(&(self.original_budget.0.as_millis() as u64).to_le_bytes());
        body.extend_from_slice(&(self.instrumentation_envelope.0.as_millis() as u64).to_le_bytes());
        match &self.execution {
            ExecutionPlan::Direct {
                executable,
                arguments,
            } => {
                body.push(1);
                put_text(&mut body, executable)?;
                body.push(
                    u8::try_from(arguments.len())
                        .map_err(|_| CapsuleError::Invalid("argument_count"))?,
                );
                for argument in arguments {
                    put_text(&mut body, argument)?;
                }
            }
            ExecutionPlan::Shell { command } => {
                body.push(2);
                put_text(&mut body, command)?;
            }
        }
        if body.len() + CAPSULE_MAC_BYTES > MAX_CAPSULE_BYTES {
            return Err(CapsuleError::Invalid("capsule_size"));
        }
        let mut mac =
            HmacSha256::new_from_slice(key).map_err(|_| CapsuleError::Invalid("capsule_key"))?;
        mac.update(&body);
        body.extend_from_slice(&mac.finalize().into_bytes());
        Ok(body)
    }

    fn unseal(input: &[u8], key: &[u8; CAPSULE_MAC_BYTES]) -> Result<Self, CapsuleError> {
        if input.len() <= CAPSULE_MAC_BYTES || input.len() > MAX_CAPSULE_BYTES {
            return Err(CapsuleError::Invalid("capsule_size"));
        }
        let (body, tag) = input.split_at(input.len() - CAPSULE_MAC_BYTES);
        let mut mac =
            HmacSha256::new_from_slice(key).map_err(|_| CapsuleError::Invalid("capsule_key"))?;
        mac.update(body);
        mac.verify_slice(tag).map_err(|_| CapsuleError::Tampered)?;
        let mut cursor = CapsuleCursor::new(body);
        if cursor.bytes(4)? != CAPSULE_MAGIC || cursor.u8()? != CAPSULE_SCHEMA_VERSION {
            return Err(CapsuleError::Invalid("schema"));
        }
        let handler_key = cursor.text()?;
        let revision = cursor.text()?;
        let definition_fingerprint = cursor.text()?;
        let runtime = cursor.text()?;
        let runtime_instance = cursor.text()?;
        let event = cursor.text()?;
        let source_scope = cursor.text()?;
        let original_budget = OriginalHandlerBudget(Duration::from_millis(cursor.u64()?));
        let instrumentation_envelope =
            InstrumentationEnvelope(Duration::from_millis(cursor.u64()?));
        let execution = match cursor.u8()? {
            1 => {
                let executable = cursor.text()?;
                let count = cursor.u8()? as usize;
                if count > MAX_ARGUMENTS {
                    return Err(CapsuleError::Invalid("argument_count"));
                }
                let mut arguments = Vec::with_capacity(count);
                for _ in 0..count {
                    arguments.push(cursor.text()?);
                }
                ExecutionPlan::Direct {
                    executable,
                    arguments,
                }
            }
            2 => ExecutionPlan::Shell {
                command: cursor.text()?,
            },
            _ => return Err(CapsuleError::Invalid("execution_plan")),
        };
        if !cursor.is_empty() {
            return Err(CapsuleError::Invalid("trailing_bytes"));
        }
        let capsule = Self {
            handler_key,
            revision,
            definition_fingerprint,
            runtime,
            runtime_instance,
            event,
            source_scope,
            original_budget,
            instrumentation_envelope,
            execution,
        };
        capsule.validate()?;
        Ok(capsule)
    }
}

/// The exact private file name derived from the HMAC-protected identity
/// fields. The activation writer uses this to prevent one valid capsule from
/// being substituted at another handler's selected path.
pub fn capsule_file_name(capsule: &HandlerCapsule) -> Result<String, CapsuleError> {
    capsule.validate()?;
    let mut hasher = Sha256::new();
    hasher.update(b"hookstat-handler-capsule-path-v1\0");
    for value in [
        &capsule.handler_key,
        &capsule.revision,
        &capsule.definition_fingerprint,
    ] {
        hasher.update(value.as_bytes());
        hasher.update(b"\0");
    }
    let digest = hasher.finalize();
    Ok(format!(
        "hshc-{}.bin",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

/// Private capsule store: root, file, and key are all checked before bytes
/// are read. The HMAC stops a capsule edit from becoming executable dispatch.
pub struct CapsuleStore {
    root: PathBuf,
}
impl CapsuleStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, CapsuleError> {
        let root = secure_directory(root.as_ref())?;
        Ok(Self { root })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn load(&self, path: impl AsRef<Path>) -> Result<HandlerCapsule, CapsuleError> {
        let file = contained_regular_file(&self.root, path.as_ref())?;
        let key = read_key(&contained_regular_file(
            &self.root,
            Path::new("capsule.key"),
        )?)?;
        let capsule =
            HandlerCapsule::unseal(&fs::read(&file).map_err(|_| CapsuleError::Io)?, &key)?;
        let expected = capsule_file_name(&capsule)?;
        if file.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
            return Err(CapsuleError::Path);
        }
        Ok(capsule)
    }

    /// Feature-only, sanitized capsule-load decomposition. It returns no
    /// private bytes or paths; callers may serialize only the durations.
    #[cfg(feature = "performance-harness")]
    pub(crate) fn load_for_qualification_timed(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(HandlerCapsule, CapsuleLoadStageTiming), CapsuleError> {
        let validation_started = Instant::now();
        let file = contained_regular_file(&self.root, path.as_ref())?;
        let key_file = contained_regular_file(&self.root, Path::new("capsule.key"))?;
        let capsule_directory_file_validation_ns = elapsed_ns(validation_started);
        let key_read_started = Instant::now();
        let key = read_key(&key_file)?;
        let key_read_ns = elapsed_ns(key_read_started);
        let capsule_read_started = Instant::now();
        let bytes = fs::read(&file).map_err(|_| CapsuleError::Io)?;
        let capsule_read_ns = elapsed_ns(capsule_read_started);
        let hmac_validation_started = Instant::now();
        let capsule = HandlerCapsule::unseal(&bytes, &key)?;
        let expected = capsule_file_name(&capsule)?;
        if file.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
            return Err(CapsuleError::Path);
        }
        Ok((
            capsule,
            CapsuleLoadStageTiming {
                capsule_directory_file_validation_ns,
                key_read_ns,
                capsule_read_ns,
                hmac_and_capsule_validation_ns: elapsed_ns(hmac_validation_started),
            },
        ))
    }
    pub fn write_for_test(
        &self,
        relative: &Path,
        capsule: &HandlerCapsule,
        key: &[u8; CAPSULE_MAC_BYTES],
    ) -> Result<(), CapsuleError> {
        if relative != Path::new(&capsule_file_name(capsule)?) {
            return Err(CapsuleError::Path);
        }
        let path = self.root.join(relative);
        if path.parent() != Some(self.root.as_path()) {
            return Err(CapsuleError::Path);
        }
        fs::write(&path, capsule.seal(key)?).map_err(|_| CapsuleError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .map_err(|_| CapsuleError::Io)?;
        }
        Ok(())
    }
}

/// Sanitized feature-only capsule-load timings. None of these fields contains
/// a path, key, capsule body, command, argument, or handler identity.
#[cfg(feature = "performance-harness")]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CapsuleLoadStageTiming {
    pub capsule_directory_file_validation_ns: u64,
    pub key_read_ns: u64,
    pub capsule_read_ns: u64,
    pub hmac_and_capsule_validation_ns: u64,
}

pub fn write_key_for_test(root: &Path, key: &[u8; CAPSULE_MAC_BYTES]) -> Result<(), CapsuleError> {
    let path = root.join("capsule.key");
    fs::write(&path, key).map_err(|_| CapsuleError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|_| CapsuleError::Io)?;
    }
    Ok(())
}

/// Result returned to the CLI without revealing private command material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShimOutcome {
    pub exit_code: i32,
    pub started: ObservationDisposition,
    pub completed: ObservationDisposition,
    pub timed_out: bool,
    pub direct_process: bool,
    /// Developer-only duration from immediately before the original child
    /// spawn through completion of its wait in this same invocation.
    #[cfg(feature = "performance-harness")]
    pub original_child_interval_ns: u64,
}

/// Sanitized feature-only execution stages for one real shim transaction.
#[cfg(feature = "performance-harness")]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ShimExecutionStageTiming {
    pub capsule_validate_ns: u64,
    pub producer_construction_ns: u64,
    pub lifecycle_construction_ns: u64,
    pub start_ipc_ns: u64,
    pub job_object_establish_ns: u64,
    pub command_construction_ns: u64,
    pub original_child_spawn_ns: u64,
    pub child_wait_poll_ns: u64,
    pub job_object_release_ns: u64,
    pub complete_ipc_ns: u64,
    pub total_execution_ns: u64,
}

pub fn run_capsule(capsule: &HandlerCapsule, state_root: &Path) -> Result<ShimOutcome, ShimError> {
    capsule.validate().map_err(ShimError::Capsule)?;
    // IPC state is observational. A bad, unavailable, or concurrently removed
    // state root must not prevent the original handler from executing.
    let producer = CooperativeProducer::for_state_root(state_root).ok();
    let mut instrumentation = InstrumentationAllowance::new(capsule.instrumentation_envelope.0);
    let started_at = now_unix_ms();
    let invocation = invocation_key(started_at);
    let lifecycle = capsule
        .lifecycle(invocation, started_at)
        .map_err(ShimError::Capsule)?;
    let started = emit_or_request_on_demand_broker(
        producer.as_ref(),
        IpcFrameKind::Start(lifecycle.clone()),
        state_root,
        &mut instrumentation,
    );
    let mut containment = ProcessContainment::establish()?;
    let mut command = command_for(&capsule.execution);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "performance-harness")]
    let original_child_interval_started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            // No original child exists. Clear the inherited kill limit before
            // returning an ordinary setup error from this shim process.
            containment.release_after_normal_root_exit()?;
            return Err(ShimError::Spawn);
        }
    };
    let (status, timed_out) = match wait_with_original_budget(&mut child, capsule.original_budget.0)
    {
        Ok(value) => value,
        Err(error) => {
            // Preserve process-tree containment at process exit while still
            // allowing the CLI to choose its documented nonzero exit code.
            containment.retain_until_process_exit();
            return Err(error);
        }
    };
    #[cfg(feature = "performance-harness")]
    let original_child_interval_ns = elapsed_ns(original_child_interval_started);
    if !timed_out {
        containment.release_after_normal_root_exit()?;
    }
    let completed_at = now_unix_ms();
    let (exit_code, terminal_status, exit_classification, exit_value) =
        completion_from_status(status.as_ref(), timed_out);
    let completed = emit_or_request_on_demand_broker(
        producer.as_ref(),
        IpcFrameKind::Complete(
            lifecycle,
            Completion {
                terminal_status,
                exit_classification,
                exit_value,
                duration_ms: completed_at.saturating_sub(started_at) as u64,
            },
        ),
        state_root,
        &mut instrumentation,
    );
    if timed_out {
        // Closing a kill-on-close Job while this shim itself is a member can
        // terminate the shim before it publishes exit 124. Retain the sole
        // handle until normal process exit; Windows then closes it and kills
        // the child tree while `main` preserves the timeout exit class.
        containment.retain_until_process_exit();
    }
    Ok(ShimOutcome {
        exit_code,
        started,
        completed,
        timed_out,
        direct_process: matches!(capsule.execution, ExecutionPlan::Direct { .. }),
        #[cfg(feature = "performance-harness")]
        original_child_interval_ns,
    })
}

/// Runs the exact shim execution path while measuring only sanitized stage
/// durations. This function exists only in the developer performance build.
#[cfg(feature = "performance-harness")]
pub(crate) fn run_capsule_for_qualification_timed(
    capsule: &HandlerCapsule,
    state_root: &Path,
) -> Result<(ShimOutcome, ShimExecutionStageTiming), ShimError> {
    let total_started = Instant::now();
    let validate_started = Instant::now();
    capsule.validate().map_err(ShimError::Capsule)?;
    let capsule_validate_ns = elapsed_ns(validate_started);
    let producer_started = Instant::now();
    let producer = CooperativeProducer::for_state_root(state_root).ok();
    let producer_construction_ns = elapsed_ns(producer_started);
    let mut instrumentation = InstrumentationAllowance::new(capsule.instrumentation_envelope.0);
    let lifecycle_started = Instant::now();
    let started_at = now_unix_ms();
    let invocation = invocation_key(started_at);
    let lifecycle = capsule
        .lifecycle(invocation, started_at)
        .map_err(ShimError::Capsule)?;
    let lifecycle_construction_ns = elapsed_ns(lifecycle_started);
    let start_ipc_started = Instant::now();
    let started = emit_or_request_on_demand_broker(
        producer.as_ref(),
        IpcFrameKind::Start(lifecycle.clone()),
        state_root,
        &mut instrumentation,
    );
    let start_ipc_ns = elapsed_ns(start_ipc_started);
    let containment_started = Instant::now();
    let mut containment = ProcessContainment::establish()?;
    let job_object_establish_ns = elapsed_ns(containment_started);
    let command_started = Instant::now();
    let mut command = command_for(&capsule.execution);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let command_construction_ns = elapsed_ns(command_started);
    let original_child_interval_started = Instant::now();
    let spawn_started = Instant::now();
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            containment.release_after_normal_root_exit()?;
            return Err(ShimError::Spawn);
        }
    };
    let original_child_spawn_ns = elapsed_ns(spawn_started);
    let wait_started = Instant::now();
    let (status, timed_out) = match wait_with_original_budget(&mut child, capsule.original_budget.0)
    {
        Ok(value) => value,
        Err(error) => {
            containment.retain_until_process_exit();
            return Err(error);
        }
    };
    let child_wait_poll_ns = elapsed_ns(wait_started);
    let original_child_interval_ns = elapsed_ns(original_child_interval_started);
    let release_started = Instant::now();
    if !timed_out {
        containment.release_after_normal_root_exit()?;
    }
    let job_object_release_ns = elapsed_ns(release_started);
    let completed_at = now_unix_ms();
    let (exit_code, terminal_status, exit_classification, exit_value) =
        completion_from_status(status.as_ref(), timed_out);
    let complete_ipc_started = Instant::now();
    let completed = emit_or_request_on_demand_broker(
        producer.as_ref(),
        IpcFrameKind::Complete(
            lifecycle,
            Completion {
                terminal_status,
                exit_classification,
                exit_value,
                duration_ms: completed_at.saturating_sub(started_at) as u64,
            },
        ),
        state_root,
        &mut instrumentation,
    );
    let complete_ipc_ns = elapsed_ns(complete_ipc_started);
    if timed_out {
        containment.retain_until_process_exit();
    }
    Ok((
        ShimOutcome {
            exit_code,
            started,
            completed,
            timed_out,
            direct_process: matches!(capsule.execution, ExecutionPlan::Direct { .. }),
            original_child_interval_ns,
        },
        ShimExecutionStageTiming {
            capsule_validate_ns,
            producer_construction_ns,
            lifecycle_construction_ns,
            start_ipc_ns,
            job_object_establish_ns,
            command_construction_ns,
            original_child_spawn_ns,
            child_wait_poll_ns,
            job_object_release_ns,
            complete_ipc_ns,
            total_execution_ns: elapsed_ns(total_started),
        },
    ))
}

enum IpcFrameKind {
    Start(LifecycleFrame),
    Complete(LifecycleFrame, Completion),
}

/// Tracks the finite post-handler allowance separately from business time.
/// The child always receives its full `OriginalHandlerBudget`; exhausted
/// instrumentation time merely creates an explicit observation gap.
struct InstrumentationAllowance {
    remaining: Duration,
}

impl InstrumentationAllowance {
    const fn new(remaining: Duration) -> Self {
        Self { remaining }
    }

    fn observe(
        &mut self,
        producer: Option<&CooperativeProducer>,
        frame: IpcFrameKind,
        state_root: &Path,
    ) -> ObservationDisposition {
        let started = Instant::now();
        let result = match (producer, frame) {
            (Some(producer), IpcFrameKind::Start(lifecycle)) => {
                producer.emit_with_budget(IpcFrame::Start(lifecycle), self.remaining)
            }
            (Some(producer), IpcFrameKind::Complete(lifecycle, completion)) => producer
                .emit_with_budget(
                    IpcFrame::Complete {
                        lifecycle,
                        completion,
                    },
                    self.remaining,
                ),
            (None, _) => ObservationDisposition::Unavailable,
        };
        if result == ObservationDisposition::Unavailable {
            request_broker_start_async(state_root);
        }
        self.remaining = self.remaining.saturating_sub(started.elapsed());
        result
    }
}

/// The shim never waits for a broker process to become ready. An absent broker
/// is an observation gap, not a Hook failure or a reason to consume the
/// original business timeout. It requests the private idle-expiring broker in
/// the background for a subsequent lifecycle event or invocation.
fn emit_or_request_on_demand_broker(
    producer: Option<&CooperativeProducer>,
    frame: IpcFrameKind,
    state_root: &Path,
    instrumentation: &mut InstrumentationAllowance,
) -> ObservationDisposition {
    instrumentation.observe(producer, frame, state_root)
}

fn request_broker_start_async(state_root: &Path) {
    let state_root = state_root.to_path_buf();
    // Broker startup can involve process creation, so it cannot remain on the
    // observed Hook's deadline. The request is best-effort for a following
    // lifecycle event or invocation and never changes this handler's result.
    let _ = thread::Builder::new()
        .name("hookstat-ipc-broker-request".into())
        .spawn(move || {
            let _ = request_broker_start(&state_root);
        });
}

fn request_broker_start(state_root: &Path) -> Result<(), ShimError> {
    // Controlled tests can exercise the broker-unavailable fail-open path
    // without retaining an idle broker process beyond the fixture lifetime.
    if std::env::var_os("HOOKSTAT_IPC_NO_BROKER_START").is_some() {
        return Ok(());
    }
    let executable = std::env::current_exe().map_err(|_| ShimError::Spawn)?;
    let Some(parent) = executable.parent() else {
        return Err(ShimError::Spawn);
    };
    #[cfg(windows)]
    let broker = parent.join("hookstat-ipc-broker.exe");
    #[cfg(not(windows))]
    let broker = parent.join("hookstat-ipc-broker");
    if !broker.is_file() {
        return Err(ShimError::Spawn);
    }
    Command::new(broker)
        .arg("--state-root")
        .arg(state_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|_| ShimError::Spawn)
}

fn command_for(plan: &ExecutionPlan) -> Command {
    match plan {
        ExecutionPlan::Direct {
            executable,
            arguments,
        } => {
            let mut value = Command::new(executable);
            value.args(arguments);
            value
        }
        ExecutionPlan::Shell { command } => platform_shell(command),
    }
}

#[cfg(windows)]
fn platform_shell(command_line: &str) -> Command {
    let mut value = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()));
    value.arg("/C").arg(command_line);
    value
}
#[cfg(not(windows))]
fn platform_shell(command_line: &str) -> Command {
    let mut value = Command::new(std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()));
    value.args(["-lc", command_line]);
    value
}

fn wait_with_original_budget(
    child: &mut std::process::Child,
    budget: Duration,
) -> Result<(Option<ExitStatus>, bool), ShimError> {
    let deadline = Instant::now() + budget;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        let _ = child.kill();
        let _ = child.wait();
        return Ok((None, true));
    }
    match child.wait_timeout(remaining).map_err(|_| ShimError::Wait)? {
        Some(status) if !deadline_expired(Instant::now(), deadline) => Ok((Some(status), false)),
        Some(_) | None => {
            // Equality is intentionally a timeout. This retains the previous
            // strict original-budget contract without the Windows scheduler
            // quantum added by a user-space 1 ms polling sleep.
            let _ = child.kill();
            let _ = child.wait();
            Ok((None, true))
        }
    }
}

/// The exact deadline is owned by the original-handler budget: equality is a
/// timeout, so a child observed only after its allotted budget cannot be
/// reported as an on-time success.
fn deadline_expired(now: Instant, deadline: Instant) -> bool {
    now >= deadline
}

fn completion_from_status(
    status: Option<&ExitStatus>,
    timed_out: bool,
) -> (i32, TerminalOutcome, ExitClassification, Option<i32>) {
    if timed_out {
        return (
            124,
            TerminalOutcome::TimedOut,
            ExitClassification::RuntimeControlled,
            Some(124),
        );
    }
    match status.and_then(ExitStatus::code) {
        Some(0) => (
            0,
            TerminalOutcome::Completed,
            ExitClassification::ExitCode,
            Some(0),
        ),
        Some(code) => (
            code,
            TerminalOutcome::Failed,
            ExitClassification::ExitCode,
            Some(code),
        ),
        None => (
            1,
            TerminalOutcome::ProtocolFailure,
            ExitClassification::RuntimeControlled,
            Some(1),
        ),
    }
}

#[cfg(windows)]
struct ProcessContainment {
    job: win32job::Job,
}
#[cfg(windows)]
impl ProcessContainment {
    fn establish() -> Result<Self, ShimError> {
        let mut limits = win32job::ExtendedLimitInfo::new();
        limits.limit_kill_on_job_close();
        let job =
            win32job::Job::create_with_limit_info(&limits).map_err(|_| ShimError::Containment)?;
        job.assign_current_process()
            .map_err(|_| ShimError::Containment)?;
        Ok(Self { job })
    }
    fn release_after_normal_root_exit(&mut self) -> Result<(), ShimError> {
        let mut limits = self
            .job
            .query_extended_limit_info()
            .map_err(|_| ShimError::Containment)?;
        limits.clear_limits();
        self.job
            .set_extended_limit_info(&limits)
            .map_err(|_| ShimError::Containment)
    }
    fn retain_until_process_exit(self) {
        std::mem::forget(self);
    }
}
#[cfg(not(windows))]
struct ProcessContainment;
#[cfg(not(windows))]
impl ProcessContainment {
    fn establish() -> Result<Self, ShimError> {
        Ok(Self)
    }
    fn release_after_normal_root_exit(&mut self) -> Result<(), ShimError> {
        Ok(())
    }
    fn retain_until_process_exit(self) {}
}

#[derive(Debug)]
pub enum CapsuleError {
    Io,
    Path,
    Tampered,
    Invalid(&'static str),
}
impl fmt::Display for CapsuleError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Io => "private capsule I/O failed",
            Self::Path => "private capsule path was rejected",
            Self::Tampered => "private capsule integrity validation failed",
            Self::Invalid(_) => "private capsule was invalid",
        })
    }
}
impl std::error::Error for CapsuleError {}

#[derive(Debug)]
pub enum ShimError {
    Capsule(CapsuleError),
    Ipc(IpcError),
    Spawn,
    Wait,
    Containment,
}
impl fmt::Display for ShimError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Capsule(_) => "hookstat-hook could not load its private capsule",
            Self::Ipc(_) => "hookstat-hook could not initialize bounded local IPC",
            Self::Spawn => "hookstat-hook could not start the original handler",
            Self::Wait => "hookstat-hook could not wait for the original handler",
            Self::Containment => "hookstat-hook could not establish required process containment",
        })
    }
}
impl std::error::Error for ShimError {}

fn secure_directory(root: &Path) -> Result<PathBuf, CapsuleError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| CapsuleError::Path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || metadata_is_unsafe(&metadata) {
        return Err(CapsuleError::Path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(CapsuleError::Path);
        }
    }
    fs::canonicalize(root).map_err(|_| CapsuleError::Path)
}
fn contained_regular_file(root: &Path, candidate: &Path) -> Result<PathBuf, CapsuleError> {
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        let mut components = candidate.components();
        if !matches!(components.next(), Some(std::path::Component::Normal(_)))
            || components.next().is_some()
        {
            return Err(CapsuleError::Path);
        }
        root.join(candidate)
    };
    // Capsules are deliberately a flat private control-plane directory.  By
    // accepting only an immediate child, no intermediate symlink/junction or
    // `..` component can redirect a capsule lookup before its final metadata
    // is inspected.
    let parent = candidate.parent().ok_or(CapsuleError::Path)?;
    let parent_metadata = fs::symlink_metadata(parent).map_err(|_| CapsuleError::Path)?;
    if parent_metadata.file_type().is_symlink()
        || !parent_metadata.is_dir()
        || metadata_is_unsafe(&parent_metadata)
        || fs::canonicalize(parent).map_err(|_| CapsuleError::Path)? != root
    {
        return Err(CapsuleError::Path);
    }
    let metadata = fs::symlink_metadata(&candidate).map_err(|_| CapsuleError::Path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata_is_unsafe(&metadata) {
        return Err(CapsuleError::Path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(CapsuleError::Path);
        }
    }
    // The parent has just been canonicalized to the store's already-secure
    // root, and this immediate child has been proved a non-reparse regular
    // file. A second canonicalization of that same child cannot strengthen
    // containment, but it does add a synchronous filesystem traversal to the
    // shipping shim hot path. Preserve the checked path for the subsequent
    // read; a TOCTOU replacement still goes through the existing HMAC check.
    Ok(candidate)
}
fn metadata_is_unsafe(metadata: &fs::Metadata) -> bool {
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
fn read_key(path: &Path) -> Result<[u8; CAPSULE_MAC_BYTES], CapsuleError> {
    fs::read(path)
        .map_err(|_| CapsuleError::Io)?
        .try_into()
        .map_err(|_| CapsuleError::Invalid("capsule_key"))
}
fn validate_reference(value: &str) -> Result<(), CapsuleError> {
    if value.is_empty()
        || value.len() > 128
        || value.chars().any(
            |value| !matches!(value, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | ':'),
        )
    {
        return Err(CapsuleError::Invalid("reference"));
    }
    Ok(())
}
fn validate_private_text(value: &str) -> Result<(), CapsuleError> {
    if value.is_empty() || value.len() > MAX_PRIVATE_COMMAND_BYTES || value.contains('\0') {
        return Err(CapsuleError::Invalid("private_text"));
    }
    Ok(())
}
fn put_text(output: &mut Vec<u8>, value: &str) -> Result<(), CapsuleError> {
    validate_private_text(value)?;
    let length = u16::try_from(value.len()).map_err(|_| CapsuleError::Invalid("private_text"))?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}
struct CapsuleCursor<'a> {
    input: &'a [u8],
    offset: usize,
}
impl<'a> CapsuleCursor<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }
    fn is_empty(&self) -> bool {
        self.offset == self.input.len()
    }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], CapsuleError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(CapsuleError::Invalid("truncated"))?;
        let value = self
            .input
            .get(self.offset..end)
            .ok_or(CapsuleError::Invalid("truncated"))?;
        self.offset = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, CapsuleError> {
        Ok(self.bytes(1)?[0])
    }
    fn u64(&mut self) -> Result<u64, CapsuleError> {
        Ok(u64::from_le_bytes(
            self.bytes(8)?
                .try_into()
                .map_err(|_| CapsuleError::Invalid("truncated"))?,
        ))
    }
    fn text(&mut self) -> Result<String, CapsuleError> {
        let len = u16::from_le_bytes(
            self.bytes(2)?
                .try_into()
                .map_err(|_| CapsuleError::Invalid("truncated"))?,
        ) as usize;
        if len == 0 || len > MAX_PRIVATE_COMMAND_BYTES {
            return Err(CapsuleError::Invalid("private_text"));
        }
        String::from_utf8(self.bytes(len)?.to_vec())
            .map_err(|_| CapsuleError::Invalid("private_text"))
    }
}
fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}
fn invocation_key(now: i64) -> String {
    format!("i{:016x}{:08x}", now as u64, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capsule() -> HandlerCapsule {
        HandlerCapsule {
            handler_key: "hk_synthetic".into(),
            revision: "rev_1".into(),
            definition_fingerprint: "sha256:synthetic".into(),
            runtime: "synthetic_runtime".into(),
            runtime_instance: "instance_1".into(),
            event: "PreToolUse".into(),
            source_scope: "controlled".into(),
            original_budget: OriginalHandlerBudget(Duration::from_millis(25)),
            instrumentation_envelope: InstrumentationEnvelope(Duration::from_millis(20)),
            execution: ExecutionPlan::Direct {
                executable: "synthetic.exe".into(),
                arguments: vec!["arg".into()],
            },
        }
    }
    #[test]
    fn sealed_capsule_rejects_tamper_and_keeps_command_out_of_protocol_types() {
        let key = [7_u8; CAPSULE_MAC_BYTES];
        let mut bytes = capsule().seal(&key).unwrap();
        bytes[8] ^= 1;
        assert!(matches!(
            HandlerCapsule::unseal(&bytes, &key),
            Err(CapsuleError::Tampered)
        ));
        assert!(capsule().lifecycle("invocation".into(), 1).is_ok());
    }

    #[test]
    fn sealed_capsule_rejects_wrong_key_truncation_unknown_version_and_signed_trailing_bytes() {
        let key = [7_u8; CAPSULE_MAC_BYTES];
        let sealed = capsule().seal(&key).unwrap();
        assert!(matches!(
            HandlerCapsule::unseal(&sealed, &[8_u8; CAPSULE_MAC_BYTES]),
            Err(CapsuleError::Tampered)
        ));
        assert!(HandlerCapsule::unseal(&sealed[..sealed.len() - 1], &key).is_err());

        let (body, _) = sealed.split_at(sealed.len() - CAPSULE_MAC_BYTES);
        let mut unknown_version = body.to_vec();
        unknown_version[CAPSULE_MAGIC.len()] = CAPSULE_SCHEMA_VERSION + 1;
        append_test_mac(&mut unknown_version, &key);
        assert!(matches!(
            HandlerCapsule::unseal(&unknown_version, &key),
            Err(CapsuleError::Invalid("schema"))
        ));

        let mut trailing = body.to_vec();
        trailing.push(0);
        append_test_mac(&mut trailing, &key);
        assert!(matches!(
            HandlerCapsule::unseal(&trailing, &key),
            Err(CapsuleError::Invalid("trailing_bytes"))
        ));
    }

    fn append_test_mac(body: &mut Vec<u8>, key: &[u8; CAPSULE_MAC_BYTES]) {
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(body);
        body.extend_from_slice(&mac.finalize().into_bytes());
    }
    #[test]
    fn capsule_root_rejects_redirection_and_outside_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("capsules");
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = CapsuleStore::open(&root).unwrap();
        let key = [9_u8; CAPSULE_MAC_BYTES];
        write_key_for_test(&root, &key).unwrap();
        let name = capsule_file_name(&capsule()).unwrap();
        store
            .write_for_test(Path::new(&name), &capsule(), &key)
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root.join(&name), fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(store.load(&name).unwrap().handler_key, "hk_synthetic");
        assert!(store.load(temp.path().join("elsewhere.bin")).is_err());
    }

    #[test]
    fn sealed_capsule_file_rejects_tamper() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("capsules");
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = CapsuleStore::open(&root).unwrap();
        let key = [5_u8; CAPSULE_MAC_BYTES];
        write_key_for_test(&root, &key).unwrap();
        let name = capsule_file_name(&capsule()).unwrap();
        store
            .write_for_test(Path::new(&name), &capsule(), &key)
            .unwrap();
        let path = root.join(&name);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        }
        let mut bytes = fs::read(&path).unwrap();
        let final_byte = bytes.len() - 1;
        bytes[final_byte] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(matches!(store.load(&name), Err(CapsuleError::Tampered)));
    }

    #[cfg(windows)]
    #[test]
    fn capsule_root_rejects_windows_reparse_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("capsules");
        fs::create_dir(&root).unwrap();
        let store = CapsuleStore::open(&root).unwrap();
        let target = temp.path().join("outside.bin");
        fs::write(&target, b"synthetic").unwrap();
        let link = root.join("redirected.bin");
        // Developer Mode or equivalent rights are needed to create a test
        // symlink on some Windows machines. The production check is always
        // active; skip only the fixture creation when Windows denies it.
        if std::os::windows::fs::symlink_file(&target, &link).is_ok() {
            assert!(matches!(
                store.load("redirected.bin"),
                Err(CapsuleError::Path)
            ));
        }
    }
    #[test]
    fn outer_envelope_never_changes_original_budget() {
        let data = capsule();
        assert_eq!(data.original_budget.0, Duration::from_millis(25));
        assert_eq!(data.instrumentation_envelope.0, Duration::from_millis(20));
    }

    #[test]
    fn original_budget_deadline_is_fail_closed_at_the_exact_boundary() {
        let deadline = Instant::now() + Duration::from_millis(10);
        let well_inside = deadline
            .checked_sub(Duration::from_millis(5))
            .expect("test deadline has room");
        let close_below = deadline
            .checked_sub(Duration::from_nanos(1))
            .expect("test deadline has room");
        assert!(!deadline_expired(well_inside, deadline));
        assert!(!deadline_expired(close_below, deadline));
        assert!(deadline_expired(deadline, deadline));
        assert!(deadline_expired(
            deadline + Duration::from_nanos(1),
            deadline
        ));
    }

    #[test]
    fn capsule_file_name_binds_handler_revision_and_definition() {
        let key = [3_u8; CAPSULE_MAC_BYTES];
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("capsules");
        fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let store = CapsuleStore::open(&root).unwrap();
        write_key_for_test(&root, &key).unwrap();
        let first = capsule();
        let first_name = capsule_file_name(&first).unwrap();
        store
            .write_for_test(Path::new(&first_name), &first, &key)
            .unwrap();
        let mut replacement = first.clone();
        replacement.revision = "rev_2".into();
        let replacement_bytes = replacement.seal(&key).unwrap();
        fs::write(root.join(&first_name), replacement_bytes).unwrap();
        assert!(matches!(store.load(&first_name), Err(CapsuleError::Path)));
    }

    #[test]
    fn original_budget_rejects_millisecond_serialization_overflow() {
        let mut data = capsule();
        data.original_budget = OriginalHandlerBudget(
            Duration::from_millis(u64::MAX).saturating_add(Duration::from_millis(1)),
        );
        assert!(matches!(
            data.validate(),
            Err(CapsuleError::Invalid("budget"))
        ));
    }
}
