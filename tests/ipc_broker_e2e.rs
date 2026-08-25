use hookstat::domain::{
    EvidenceCoverage, EvidenceKind, ExecutionMode, HandlerIdentity, HookEvent, HookInvocation,
    Runtime, TerminalStatus,
};
use hookstat::evidence::{
    AuthorityRouter, CoverageDomain, DomainAuthority, NativeAdmissionState,
    RuntimeNeutralEvidenceCore,
};
use hookstat::ipc::{
    BrokerAcknowledgement, BrokerConfig, BrokerHost, BrokerStartup, Completion, ExitClassification,
    GroupDurabilityPolicy, IpcClient, IpcFrame, LifecycleFrame, LocalEndpoint, Wal,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

fn frame(client: u32, sequence: u32) -> IpcFrame {
    IpcFrame::Start(LifecycleFrame {
        runtime: "synthetic_runtime".into(),
        runtime_instance: format!("instance_{client}"),
        invocation: format!("invocation_{client}_{sequence}"),
        handler: "opaque_handler".into(),
        event: "synthetic_event".into(),
        source_scope: "synthetic_scope".into(),
        revision: Some("synthetic_revision".into()),
        occurred_at_unix_ms: 1_000 + i64::from(sequence),
    })
}

fn completion(client: u32, sequence: u32) -> IpcFrame {
    let IpcFrame::Start(lifecycle) = frame(client, sequence) else {
        unreachable!()
    };
    IpcFrame::Complete {
        lifecycle,
        completion: Completion {
            terminal_status: TerminalStatus::Completed,
            exit_classification: ExitClassification::ExitCode,
            exit_value: Some(0),
            duration_ms: 7,
        },
    }
}

fn config(root: &std::path::Path) -> BrokerConfig {
    BrokerConfig {
        state_root: root.to_path_buf(),
        queue_capacity: 256,
        max_connections: 128,
        ack_timeout: Duration::from_millis(100),
        idle_timeout: Duration::from_secs(3),
        group_durability: GroupDurabilityPolicy {
            max_records: 64,
            max_bytes: 64 * 1024,
            max_interval: Duration::from_millis(50),
        },
    }
}

fn started_client(host: &BrokerHost) -> IpcClient {
    IpcClient::connect(host.endpoint(), Duration::from_millis(100)).unwrap()
}

fn production_performance_config(root: &std::path::Path) -> BrokerConfig {
    let mut config = BrokerConfig::for_state_root(root);
    config.idle_timeout = Duration::from_secs(3);
    // The frozen G28 budget is release-governing. Debug harnesses retain a
    // bounded staging allowance under debugger and test-runner overhead; the
    // optimized release path below exercises the production 5 ms deadline.
    if cfg!(debug_assertions) {
        config.ack_timeout = Duration::from_millis(100);
    }
    config
}

fn started_production_client(host: &BrokerHost) -> IpcClient {
    let timeout = if cfg!(debug_assertions) {
        Duration::from_millis(100)
    } else {
        Duration::from_millis(5)
    };
    IpcClient::connect(host.endpoint(), timeout).unwrap()
}

fn e2e_serial_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn local_transport_acknowledges_start_and_complete_without_network_listener() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let host = BrokerHost::start(config(temp.path())).unwrap();
    #[cfg(windows)]
    assert!(
        host.endpoint()
            .named_pipe_name()
            .starts_with("hookstat-g35-")
    );
    #[cfg(unix)]
    assert_eq!(
        host.endpoint().unix_socket_path().unwrap().extension(),
        Some("sock".as_ref())
    );

    let mut client = started_client(&host);
    assert_eq!(
        client.send(&frame(1, 1)).unwrap(),
        BrokerAcknowledgement::Accepted
    );
    assert_eq!(
        client.send(&completion(1, 1)).unwrap(),
        BrokerAcknowledgement::Accepted
    );
    drop(client);

    let health = host.health();
    assert_eq!(health.accepted, 2);
    assert_eq!(health.dropped, 0);
    host.stop();

    let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
    let recovery = wal.recover_and_replay().unwrap();
    assert_eq!(recovery.frames.len(), 2);
}

#[test]
fn startup_race_elects_one_broker_and_healthy_endpoint_is_reused() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let configuration = config(temp.path());
    let endpoint = LocalEndpoint::from_state_root(temp.path()).unwrap();
    let startup = BrokerStartup::new(endpoint, Duration::from_millis(500)).unwrap();
    let starts = Arc::new(AtomicU64::new(0));
    let host = Arc::new(Mutex::new(None));
    let mut workers = Vec::new();
    for _ in 0..16 {
        let startup = startup.clone();
        let starts = Arc::clone(&starts);
        let host = Arc::clone(&host);
        let configuration = configuration.clone();
        workers.push(thread::spawn(move || {
            let client = startup
                .connect_or_start(|| {
                    starts.fetch_add(1, Ordering::SeqCst);
                    *host.lock().unwrap() = Some(BrokerHost::start(configuration.clone())?);
                    Ok(())
                })
                .unwrap();
            drop(client);
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(starts.load(Ordering::SeqCst), 1);

    let client = startup
        .connect_or_start(|| panic!("healthy broker must be reused"))
        .unwrap();
    drop(client);
    host.lock().unwrap().take().unwrap().stop();
}

#[cfg(unix)]
#[test]
fn unix_stale_socket_is_recovered_only_inside_the_secure_state_root() {
    let _guard = e2e_serial_guard();
    use std::os::unix::net::UnixListener;

    let temp = tempfile::tempdir().unwrap();
    let endpoint = LocalEndpoint::from_state_root(temp.path()).unwrap();
    let stale = endpoint.unix_socket_path().unwrap();
    let listener = UnixListener::bind(&stale).unwrap();
    drop(listener); // leaves a dead socket inode, not an active endpoint.
    let host = BrokerHost::start(config(temp.path())).unwrap();
    let mut client = started_client(&host);
    assert_eq!(
        client.send(&frame(8, 1)).unwrap(),
        BrokerAcknowledgement::Accepted
    );
    host.stop();
}

#[test]
fn idle_broker_flushes_and_expires_without_a_global_service() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let mut configuration = config(temp.path());
    configuration.idle_timeout = Duration::from_millis(25);
    let mut host = BrokerHost::start(configuration).unwrap();
    let mut client = started_client(&host);
    assert_eq!(
        client.send(&frame(7, 1)).unwrap(),
        BrokerAcknowledgement::Accepted
    );
    drop(client);
    assert!(host.wait_for_idle(Duration::from_secs(1)));
    assert!(IpcClient::connect(host.endpoint(), Duration::from_millis(2)).is_err());
}

#[test]
fn replay_enters_g29_core_idempotently_then_resolves_outside_the_broker() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let mut wal = Wal::open(temp.path(), GroupDurabilityPolicy::default()).unwrap();
    wal.append(&frame(1, 1)).unwrap();
    wal.append(&completion(1, 1)).unwrap();
    wal.append(&completion(1, 1)).unwrap();
    wal.flush_group().unwrap();
    let recovery = wal.recover_and_replay().unwrap();
    let broker_recovery = hookstat::ipc::BrokerRecovery {
        frames: recovery.frames,
        truncated_tail_bytes: recovery.truncated_tail_bytes,
    };
    let domain = CoverageDomain {
        runtime: hookstat::evidence::RuntimeId::new("synthetic_runtime").unwrap(),
        event: hookstat::evidence::EventFamily::new("synthetic_event").unwrap(),
        source_scope: hookstat::evidence::SourceScope::new("synthetic_scope").unwrap(),
    };
    let mut core = RuntimeNeutralEvidenceCore::new(
        AuthorityRouter::new([DomainAuthority {
            domain,
            native_admission: NativeAdmissionState::Qualified,
        }])
        .unwrap(),
    );
    let result = broker_recovery.ingest_into(&mut core).unwrap();
    assert_eq!(result.produced, 2);
    assert_eq!(result.duplicates, 1);

    let complete = broker_recovery.canonical_evidence().unwrap()[1].clone();
    let correlated = match core.ingest(complete).unwrap() {
        hookstat::evidence::CoreIngestOutcome::Duplicate => {
            // A fresh core demonstrates the produced terminal contract used by
            // a runtime-specific identity resolver.
            let domain = CoverageDomain {
                runtime: hookstat::evidence::RuntimeId::new("synthetic_runtime").unwrap(),
                event: hookstat::evidence::EventFamily::new("synthetic_event").unwrap(),
                source_scope: hookstat::evidence::SourceScope::new("synthetic_scope").unwrap(),
            };
            let mut fresh = RuntimeNeutralEvidenceCore::new(
                AuthorityRouter::new([DomainAuthority {
                    domain,
                    native_admission: NativeAdmissionState::Qualified,
                }])
                .unwrap(),
            );
            fresh
                .ingest(broker_recovery.canonical_evidence().unwrap()[0].clone())
                .unwrap();
            match fresh
                .ingest(broker_recovery.canonical_evidence().unwrap()[1].clone())
                .unwrap()
            {
                hookstat::evidence::CoreIngestOutcome::Produced(value) => value,
                unexpected => panic!("expected correlated IPC completion, got {unexpected:?}"),
            }
        }
        unexpected => panic!("replay must be idempotent, got {unexpected:?}"),
    };
    let invocation = HookInvocation {
        source_key: "synthetic_ipc_v1".into(),
        source_record_id: correlated.correlation_key.invocation_key.as_str().into(),
        runtime: Runtime::OpenCode,
        evidence_kind: EvidenceKind::SyntheticFixture,
        coverage: correlated.legacy_coverage(),
        handler: HandlerIdentity {
            key: "resolved-synthetic-handler".into(),
            revision: "resolved-synthetic-revision".into(),
            label: "Synthetic Handler".into(),
            source_kind: "synthetic_runtime_identity_resolver".into(),
            event: HookEvent::PreToolUse,
            matcher_identity: "opaque".into(),
            structural_identity: "synthetic:0".into(),
            execution_mode: ExecutionMode::Sync,
        },
        occurred_at_unix_ms: correlated.occurred_at_unix_ms,
        terminal_status: correlated.terminal_status,
        duration_ms: correlated.duration_ms,
        error_fingerprint: None,
    };
    assert_eq!(invocation.coverage, EvidenceCoverage::Partial);
    assert_eq!(invocation.terminal_status, TerminalStatus::Completed);
    invocation.validate().unwrap();
}

#[test]
fn concurrency_matrix_accepts_16_clients_10k_frames_and_100_clients_100k_frames() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let mut scale_config = config(temp.path());
    // This correctness matrix intentionally permits a larger but still
    // finite producer deadline while 100 synthetic threads stage local-pipe
    // connections on a finite CI VM. Production retains its 5 ms policy;
    // the separate optimized warm/concurrent smoke tests own the frozen G28
    // latency limits.
    scale_config.ack_timeout = Duration::from_secs(5);
    let host = BrokerHost::start(scale_config).unwrap();
    // This is a correctness/staging deadline for a deliberately contended
    // 100-client test, not the production cooperative IPC acknowledgment
    // budget. Keep the synthetic client deadline aligned with the broker's
    // bounded test deadline so scheduler contention is reported as a bounded
    // failure rather than a mismatched 100 ms client timeout.
    run_clients(&host, 16, 625, Duration::from_secs(5));
    run_clients(&host, 100, 1_000, Duration::from_secs(5));
    let health = host.health();
    assert_eq!(health.accepted, 110_000);
    assert_eq!(health.rejected, 0);
    assert_eq!(health.dropped, 0);
    assert!(health.queue_high_water <= 256);
    host.stop();
}

#[test]
fn broker_ack_latency_smoke_reports_sanitized_percentiles() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let host = BrokerHost::start(production_performance_config(temp.path())).unwrap();
    let mut client = started_production_client(&host);
    let mut samples = Vec::with_capacity(1_000);
    for sequence in 0..1_000 {
        let before = Instant::now();
        assert_eq!(
            client.send(&frame(77, sequence)).unwrap(),
            BrokerAcknowledgement::Accepted
        );
        samples.push(before.elapsed().as_nanos() as u64);
    }
    samples.sort_unstable();
    let percentile =
        |percent: usize| samples[(samples.len() - 1) * percent / 100] as f64 / 1_000_000.0;
    println!(
        "g35_broker_ack_ms p50={:.3} p95={:.3} p99={:.3}",
        percentile(50),
        percentile(95),
        percentile(99)
    );
    // G28 is release-governing. Debug CI preserves the measurement but can
    // include scheduler/debug instrumentation outliers that do not represent
    // the optimized producer path; the release harness enforces this budget.
    if !cfg!(debug_assertions) {
        assert!(
            percentile(95) <= 1.0,
            "G28 cooperative IPC p95 budget exceeded"
        );
        assert!(
            percentile(99) <= 2.0,
            "G28 cooperative IPC p99 budget exceeded"
        );
    }
    host.stop();
}

#[test]
fn concurrent_producer_latency_smoke_is_sanitized_and_bounded() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let host = BrokerHost::start(production_performance_config(temp.path())).unwrap();
    let samples = Arc::new(Mutex::new(Vec::with_capacity(1_600)));
    thread::scope(|scope| {
        let host_ref = &host;
        for client_id in 0..16 {
            let samples = Arc::clone(&samples);
            scope.spawn(move || {
                let mut client = started_production_client(host_ref);
                for sequence in 0..100 {
                    let before = Instant::now();
                    assert_eq!(
                        client.send(&frame(client_id, sequence)).unwrap(),
                        BrokerAcknowledgement::Accepted
                    );
                    samples
                        .lock()
                        .unwrap()
                        .push(before.elapsed().as_nanos() as u64);
                }
            });
        }
    });
    let mut samples = Arc::try_unwrap(samples).unwrap().into_inner().unwrap();
    samples.sort_unstable();
    let percentile =
        |percent: usize| samples[(samples.len() - 1) * percent / 100] as f64 / 1_000_000.0;
    println!(
        "g35_broker_concurrent_ack_ms p50={:.3} p95={:.3} p99={:.3}",
        percentile(50),
        percentile(95),
        percentile(99)
    );
    // G28 is calibrated in an optimized release process. Debug test builds
    // retain this measurement for structural regressions but may include
    // Windows scheduler variance not present in release evidence.
    if !cfg!(debug_assertions) {
        assert!(
            percentile(95) <= 1.0,
            "G28 cooperative IPC p95 budget exceeded under 16 clients"
        );
        assert!(
            percentile(99) <= 2.0,
            "G28 cooperative IPC p99 budget exceeded under 16 clients"
        );
    }
    host.stop();
}

#[test]
fn wal_append_and_group_flush_smoke_reports_sanitized_percentiles() {
    let _guard = e2e_serial_guard();
    let temp = tempfile::tempdir().unwrap();
    let mut wal = Wal::open(
        temp.path(),
        GroupDurabilityPolicy {
            max_records: 10_000,
            max_bytes: 8 * 1024 * 1024,
            max_interval: Duration::from_secs(60),
        },
    )
    .unwrap();
    let mut appends = Vec::with_capacity(1_000);
    for sequence in 0..1_000 {
        let before = Instant::now();
        wal.append(&frame(88, sequence)).unwrap();
        appends.push(before.elapsed().as_nanos() as u64);
    }
    let mut flushes = Vec::with_capacity(100);
    for sequence in 0..100 {
        wal.append(&frame(89, sequence)).unwrap();
        let before = Instant::now();
        wal.flush_group().unwrap();
        flushes.push(before.elapsed().as_nanos() as u64);
    }
    appends.sort_unstable();
    flushes.sort_unstable();
    let percentile = |values: &[u64], percent: usize| {
        values[(values.len() - 1) * percent / 100] as f64 / 1_000_000.0
    };
    println!(
        "g35_wal_append_ms p50={:.3} p95={:.3} p99={:.3}",
        percentile(&appends, 50),
        percentile(&appends, 95),
        percentile(&appends, 99)
    );
    println!(
        "g35_wal_group_flush_ms p50={:.3} p95={:.3} p99={:.3}",
        percentile(&flushes, 50),
        percentile(&flushes, 95),
        percentile(&flushes, 99)
    );
    assert!(appends.iter().all(|value| *value < 50_000_000));
}

fn run_clients(host: &BrokerHost, clients: u32, frames_per_client: u32, client_timeout: Duration) {
    thread::scope(|scope| {
        let mut workers = Vec::new();
        for client_id in 0..clients {
            workers.push(scope.spawn(move || {
                let mut client = IpcClient::connect(host.endpoint(), client_timeout).unwrap();
                for sequence in 0..frames_per_client {
                    assert_eq!(
                        client.send(&frame(client_id, sequence)).unwrap(),
                        BrokerAcknowledgement::Accepted
                    );
                }
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
