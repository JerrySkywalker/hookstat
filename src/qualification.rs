//! Developer-only, sanitized G35 IPC performance qualification.
//!
//! This module is feature-gated and is never linked by the cooperative client
//! or transparent shim. It measures only bounded synthetic IPC metadata in
//! disposable state; it never inspects processes, Hook configuration, commands,
//! prompts, payloads, power settings, process priority, or affinity.

use crate::ipc::{
    BrokerAcknowledgement, BrokerConfig, BrokerHost, IpcClient, IpcError, IpcFrame, LifecycleFrame,
    QualificationBrokerStageSample, QualificationClientStageSample, QualificationSendFailure,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RECEIPT_SCHEMA_VERSION: u8 = 3;
const REQUIRED_QUALIFYING_RUNS: u8 = 5;
const FROZEN_P95_MS: f64 = 1.0;
const FROZEN_P99_MS: f64 = 2.0;
// `min(half frozen budget, four times G28 substrate pXX)`. The values reserve
// at least half the release budget for the actual tested path; they are an
// admission-methodology threshold only, never a product SLO.
const CLIENTS_16: usize = 16;
static DISPOSABLE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct QualificationConfig {
    pub max_attempts: u16,
    pub wait_interval_ms: u64,
    pub control_samples: usize,
    pub single_samples: usize,
    pub client16_samples_per_client: usize,
}

impl Default for QualificationConfig {
    fn default() -> Self {
        Self {
            max_attempts: u16::from(REQUIRED_QUALIFYING_RUNS),
            wait_interval_ms: 60_000,
            control_samples: 1_000,
            single_samples: 1_000,
            client16_samples_per_client: 100,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LatencyStatistics {
    pub samples: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlObservation {
    pub series: String,
    pub attempt: u16,
    pub position: String,
    pub latency: Option<LatencyStatistics>,
    pub admitted: bool,
    pub disposition: String,
    pub measurement_error_class: Option<MeasurementErrorClass>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkObservation {
    pub series: String,
    pub attempt: u16,
    pub latency: Option<LatencyStatistics>,
    pub frozen_budget_passed: bool,
    pub disposition: String,
    pub measurement_error_class: Option<MeasurementErrorClass>,
}

/// Bounded developer-only measurement diagnostics. These values intentionally
/// exclude OS error text, paths, process data, and other private content.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementErrorClass {
    BrokerStartupFailure,
    ConnectTimeout,
    WriteSendTimeout,
    ReadAckTimeout,
    UnexpectedAcknowledgement,
    WorkerFailure,
    StateRootFailure,
    OtherBoundedIoFailure,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SeriesSummary {
    pub series: String,
    pub required_qualifying_runs: u8,
    pub admitted_runs: u8,
    pub worst_admitted_p95_ms: Option<f64>,
    pub worst_admitted_p99_ms: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QualificationReceipt {
    pub schema_version: u8,
    pub run_kind: String,
    pub legacy_cpu_heuristic: String,
    pub control_method: String,
    pub control_derivation: String,
    pub control_limits_ms: ControlLimits,
    pub frozen_g28_budget_ms: FrozenBudget,
    pub measurement_collector_model: MeasurementCollectorModel,
    pub client16_start_barrier: bool,
    pub plan: QualificationPlan,
    pub controls: Vec<ControlObservation>,
    pub admitted_runs: Vec<BenchmarkObservation>,
    pub rejected_runs: Vec<BenchmarkObservation>,
    pub series: Vec<SeriesSummary>,
    pub outcome: String,
    pub owner_live_codex_config_mutated: bool,
    pub raw_private_content_captured: bool,
}

/// The developer-only collector is recorded so an acceptance receipt cannot be
/// confused with the superseded shared-mutex measurement model.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementCollectorModel {
    LegacySharedMutex,
    PerThreadLocalBuffers,
}

#[derive(Clone, Debug)]
pub struct CollectorComparisonConfig {
    pub single_samples: usize,
    pub client16_samples_per_client: usize,
}

impl Default for CollectorComparisonConfig {
    fn default() -> Self {
        Self {
            single_samples: 1_000,
            client16_samples_per_client: 100,
        }
    }
}

/// A non-acceptance diagnostic which changes only benchmark collection.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollectorComparisonReceipt {
    pub schema_version: u8,
    pub run_kind: String,
    pub acceptance_evidence: bool,
    pub legacy_capture_order: String,
    pub corrected_capture_order: String,
    pub client_start_barrier: bool,
    pub observations: Vec<CollectorComparisonObservation>,
    pub owner_live_codex_config_mutated: bool,
    pub raw_private_content_captured: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CollectorComparisonObservation {
    pub collector_model: MeasurementCollectorModel,
    pub series: String,
    pub clients: usize,
    pub samples_per_client: usize,
    pub latency: Option<LatencyStatistics>,
    pub frozen_budget_passed: Option<bool>,
    pub measurement_error_class: Option<MeasurementErrorClass>,
}

#[derive(Clone, Debug)]
pub struct StageTimingConfig {
    pub client16_samples_per_client: usize,
}

impl Default for StageTimingConfig {
    fn default() -> Self {
        Self {
            client16_samples_per_client: 100,
        }
    }
}

/// Sanitized, feature-gated decomposition of a 16-client broker ACK round
/// trip. It is diagnostic evidence only and cannot satisfy or relax G35.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StageTimingReceipt {
    pub schema_version: u8,
    pub run_kind: String,
    pub acceptance_evidence: bool,
    pub collector_model: MeasurementCollectorModel,
    pub client_start_barrier: bool,
    pub clients: usize,
    pub samples_per_client: usize,
    pub round_trip: Option<LatencyStatistics>,
    pub client_write: Option<LatencyStatistics>,
    pub broker_read_decode: Option<LatencyStatistics>,
    pub activity_bookkeeping: Option<LatencyStatistics>,
    pub acknowledgement_channel_allocation: Option<LatencyStatistics>,
    pub queue_submission: Option<LatencyStatistics>,
    pub queue_wait: Option<LatencyStatistics>,
    pub queue_wait_group_sync_overlap: Option<LatencyStatistics>,
    pub queue_wait_residual_after_group_sync: Option<LatencyStatistics>,
    pub worker_dequeue_handoff: Option<LatencyStatistics>,
    pub queue_depth_at_dequeue_max: Option<u64>,
    pub queue_high_water: Option<u64>,
    pub group_sync_attempts: usize,
    pub durability_requests: Option<u64>,
    pub durability_requests_coalesced: Option<u64>,
    pub durability_flushes_completed: Option<u64>,
    pub group_sync_duration: Option<LatencyStatistics>,
    pub queue_wait_sync_correlation: Option<QueueWaitSyncCorrelation>,
    pub wal_append: Option<LatencyStatistics>,
    pub worker_acknowledgement_handoff: Option<LatencyStatistics>,
    pub broker_ack_write: Option<LatencyStatistics>,
    pub client_ack_read: Option<LatencyStatistics>,
    pub measurement_error_class: Option<MeasurementErrorClass>,
    pub owner_live_codex_config_mutated: bool,
    pub raw_private_content_captured: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueueWaitSyncCorrelation {
    pub samples_overlapping_group_sync: usize,
    pub p95_tail_samples: usize,
    pub p95_tail_samples_overlapping_group_sync: usize,
    pub total_queue_wait_ns: u64,
    pub total_group_sync_overlap_ns: u64,
}

type StageTimingMeasurement = (
    Vec<u64>,
    Vec<QualificationClientStageSample>,
    Vec<QualificationBrokerStageSample>,
    Vec<u64>,
    crate::ipc::BrokerHealth,
);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlLimits {
    pub p95_ms_max: f64,
    pub p99_ms_max: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FrozenBudget {
    pub cooperative_p95_ms_max: f64,
    pub cooperative_p99_ms_max: f64,
    pub changed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QualificationPlan {
    pub required_qualifying_runs_per_series: u8,
    pub max_attempts_per_series: u16,
    pub wait_interval_ms: u64,
    pub control_samples: usize,
    pub single_samples: usize,
    pub client16_clients: usize,
    pub client16_samples_per_client: usize,
    pub percentile_method: String,
}

#[derive(Debug)]
pub enum QualificationError {
    Ipc(IpcError),
    Io(std::io::Error),
    Invalid(&'static str),
}

impl fmt::Display for QualificationError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ipc(_) => output.write_str("G35 qualification IPC operation failed"),
            Self::Io(_) => output.write_str("G35 qualification disposable-state operation failed"),
            Self::Invalid(value) => write!(output, "invalid G35 qualification value: {value}"),
        }
    }
}

impl std::error::Error for QualificationError {}

impl From<IpcError> for QualificationError {
    fn from(value: IpcError) -> Self {
        Self::Ipc(value)
    }
}

impl From<std::io::Error> for QualificationError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Runs only after a local control admits the host, retains every observation,
/// and records a blocked or failed outcome rather than choosing a best sample.
pub fn run_g35(config: &QualificationConfig) -> Result<QualificationReceipt, QualificationError> {
    validate_config(config)?;
    let mut receipt = QualificationReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION,
        run_kind: "hs_g35_sanitized_ipc_qualification".into(),
        legacy_cpu_heuristic: "LEGACY_EXPERIMENTAL_HOST_HEURISTIC".into(),
        control_method: "paired_local_broker_ack_control".into(),
        control_derivation: "min(0.5*frozen_budget,4*g28_named_pipe_ack_baseline): p95=min(0.500,0.720)=0.500; p99=min(1.000,0.960)=0.960".into(),
        control_limits_ms: ControlLimits {
            p95_ms_max: control_p95_max_ms(),
            p99_ms_max: control_p99_max_ms(),
        },
        frozen_g28_budget_ms: FrozenBudget {
            cooperative_p95_ms_max: FROZEN_P95_MS,
            cooperative_p99_ms_max: FROZEN_P99_MS,
            changed: false,
        },
        measurement_collector_model: MeasurementCollectorModel::PerThreadLocalBuffers,
        client16_start_barrier: true,
        plan: QualificationPlan {
            required_qualifying_runs_per_series: REQUIRED_QUALIFYING_RUNS,
            max_attempts_per_series: config.max_attempts,
            wait_interval_ms: config.wait_interval_ms,
            control_samples: config.control_samples,
            single_samples: config.single_samples,
            client16_clients: CLIENTS_16,
            client16_samples_per_client: config.client16_samples_per_client,
            percentile_method: "nearest_rank".into(),
        },
        controls: Vec::new(),
        admitted_runs: Vec::new(),
        rejected_runs: Vec::new(),
        series: Vec::new(),
        outcome: "BLOCKED_NO_QUALIFYING_WINDOW".into(),
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    };

    let single = qualify_series(
        "single_client_persistent_release",
        1,
        config.single_samples,
        config,
        &mut receipt,
    )?;
    receipt.series.push(single.summary);
    if single.frozen_budget_failed {
        receipt.outcome = "FAIL_FROZEN_G28_BUDGET".into();
        return Ok(receipt);
    }
    let concurrent = qualify_series(
        "client16_persistent_release",
        CLIENTS_16,
        config.client16_samples_per_client,
        config,
        &mut receipt,
    )?;
    receipt.series.push(concurrent.summary);
    receipt.outcome = if concurrent.frozen_budget_failed {
        "FAIL_FROZEN_G28_BUDGET".into()
    } else if receipt
        .series
        .iter()
        .all(|summary| summary.admitted_runs == REQUIRED_QUALIFYING_RUNS)
    {
        "PASS_REPEATABLE".into()
    } else {
        "BLOCKED_NO_QUALIFYING_WINDOW".into()
    };
    Ok(receipt)
}

/// Measures the superseded and corrected collector models against identical
/// broker traffic. This is diagnostic evidence only: it has no control
/// admission and never contributes to G35 acceptance.
pub fn run_collector_comparison(
    config: &CollectorComparisonConfig,
) -> Result<CollectorComparisonReceipt, QualificationError> {
    validate_collector_comparison_config(config)?;
    let mut observations = Vec::with_capacity(4);
    for (collector_model, series, clients, samples_per_client) in [
        (
            MeasurementCollectorModel::LegacySharedMutex,
            "single_client_persistent_release",
            1,
            config.single_samples,
        ),
        (
            MeasurementCollectorModel::PerThreadLocalBuffers,
            "single_client_persistent_release",
            1,
            config.single_samples,
        ),
        (
            MeasurementCollectorModel::LegacySharedMutex,
            "client16_persistent_release",
            CLIENTS_16,
            config.client16_samples_per_client,
        ),
        (
            MeasurementCollectorModel::PerThreadLocalBuffers,
            "client16_persistent_release",
            CLIENTS_16,
            config.client16_samples_per_client,
        ),
    ] {
        observations.push(collector_comparison_observation(
            collector_model,
            series,
            clients,
            samples_per_client,
        ));
    }
    Ok(CollectorComparisonReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION,
        run_kind: "hs_g35_collector_model_ab_diagnostic".into(),
        acceptance_evidence: false,
        legacy_capture_order: "ack_completion_then_shared_samples_mutex_then_before_elapsed".into(),
        corrected_capture_order: "ack_completion_then_before_elapsed_then_per_thread_collection"
            .into(),
        client_start_barrier: true,
        observations,
        owner_live_codex_config_mutated: false,
        raw_private_content_captured: false,
    })
}

fn collector_comparison_observation(
    collector_model: MeasurementCollectorModel,
    series: &str,
    clients: usize,
    samples_per_client: usize,
) -> CollectorComparisonObservation {
    match measure_broker_ack_with_collector(
        clients,
        samples_per_client,
        collector_model,
        Duration::ZERO,
    ) {
        Ok(latency) => CollectorComparisonObservation {
            collector_model,
            series: series.into(),
            clients,
            samples_per_client,
            frozen_budget_passed: Some(frozen_budget_passes(&latency)),
            latency: Some(latency),
            measurement_error_class: None,
        },
        Err(error) => CollectorComparisonObservation {
            collector_model,
            series: series.into(),
            clients,
            samples_per_client,
            latency: None,
            frozen_budget_passed: None,
            measurement_error_class: Some(error),
        },
    }
}

/// Runs a 16-client stage decomposition with pre-established persistent
/// connections. Timing is enabled only for this feature-gated diagnostic.
pub fn run_stage_timing_diagnostic(
    config: &StageTimingConfig,
) -> Result<StageTimingReceipt, QualificationError> {
    validate_stage_timing_config(config)?;
    match measure_stage_timing(CLIENTS_16, config.client16_samples_per_client) {
        Ok((round_trip, client, broker, group_sync_durations, health)) => Ok(StageTimingReceipt {
            schema_version: RECEIPT_SCHEMA_VERSION,
            run_kind: "hs_g35_stage_timing_diagnostic".into(),
            acceptance_evidence: false,
            collector_model: MeasurementCollectorModel::PerThreadLocalBuffers,
            client_start_barrier: true,
            clients: CLIENTS_16,
            samples_per_client: config.client16_samples_per_client,
            round_trip: stage_statistics(round_trip),
            client_write: stage_statistics(client.iter().map(|value| value.client_write_ns)),
            broker_read_decode: stage_statistics(
                broker.iter().map(|value| value.broker_read_decode_ns),
            ),
            activity_bookkeeping: stage_statistics(
                broker.iter().map(|value| value.activity_bookkeeping_ns),
            ),
            acknowledgement_channel_allocation: stage_statistics(
                broker
                    .iter()
                    .map(|value| value.acknowledgement_channel_allocation_ns),
            ),
            queue_submission: stage_statistics(
                broker.iter().map(|value| value.queue_submission_ns),
            ),
            queue_wait: stage_statistics(broker.iter().map(|value| value.queue_wait_ns)),
            queue_wait_group_sync_overlap: stage_statistics(
                broker
                    .iter()
                    .map(|value| value.queue_wait_group_sync_overlap_ns),
            ),
            queue_wait_residual_after_group_sync: stage_statistics(broker.iter().map(|value| {
                value
                    .queue_wait_ns
                    .saturating_sub(value.queue_wait_group_sync_overlap_ns)
            })),
            worker_dequeue_handoff: stage_statistics(
                broker.iter().map(|value| value.worker_dequeue_handoff_ns),
            ),
            queue_depth_at_dequeue_max: broker
                .iter()
                .map(|value| value.queue_depth_at_dequeue)
                .max(),
            queue_high_water: Some(health.queue_high_water),
            group_sync_attempts: group_sync_durations.len(),
            durability_requests: Some(health.durability_requests),
            durability_requests_coalesced: Some(health.durability_requests_coalesced),
            durability_flushes_completed: Some(health.group_flushes),
            group_sync_duration: stage_statistics(group_sync_durations),
            queue_wait_sync_correlation: queue_wait_sync_correlation(&broker),
            wal_append: stage_statistics(broker.iter().map(|value| value.wal_append_ns)),
            worker_acknowledgement_handoff: stage_statistics(
                broker
                    .iter()
                    .map(|value| value.worker_acknowledgement_handoff_ns),
            ),
            broker_ack_write: stage_statistics(
                broker.iter().map(|value| value.broker_ack_write_ns),
            ),
            client_ack_read: stage_statistics(client.iter().map(|value| value.client_ack_read_ns)),
            measurement_error_class: None,
            owner_live_codex_config_mutated: false,
            raw_private_content_captured: false,
        }),
        Err(error) => Ok(StageTimingReceipt {
            schema_version: RECEIPT_SCHEMA_VERSION,
            run_kind: "hs_g35_stage_timing_diagnostic".into(),
            acceptance_evidence: false,
            collector_model: MeasurementCollectorModel::PerThreadLocalBuffers,
            client_start_barrier: true,
            clients: CLIENTS_16,
            samples_per_client: config.client16_samples_per_client,
            round_trip: None,
            client_write: None,
            broker_read_decode: None,
            activity_bookkeeping: None,
            acknowledgement_channel_allocation: None,
            queue_submission: None,
            queue_wait: None,
            queue_wait_group_sync_overlap: None,
            queue_wait_residual_after_group_sync: None,
            worker_dequeue_handoff: None,
            queue_depth_at_dequeue_max: None,
            queue_high_water: None,
            group_sync_attempts: 0,
            durability_requests: None,
            durability_requests_coalesced: None,
            durability_flushes_completed: None,
            group_sync_duration: None,
            queue_wait_sync_correlation: None,
            wal_append: None,
            worker_acknowledgement_handoff: None,
            broker_ack_write: None,
            client_ack_read: None,
            measurement_error_class: Some(error),
            owner_live_codex_config_mutated: false,
            raw_private_content_captured: false,
        }),
    }
}

fn measure_stage_timing(
    clients: usize,
    samples_per_client: usize,
) -> Result<StageTimingMeasurement, MeasurementErrorClass> {
    let root =
        DisposableStateRoot::create().map_err(|_| MeasurementErrorClass::StateRootFailure)?;
    let host = BrokerHost::start_with_qualification_stage_timing(BrokerConfig::for_state_root(
        root.path(),
    ))
    .map_err(|error| classify_broker_startup_error(&error))?;
    let result = connect_persistent_clients(&host, clients).and_then(|connections| {
        let barrier = Arc::new(Barrier::new(connections.len()));
        let (round_trip, client) = thread::scope(|scope| {
            let mut workers = Vec::with_capacity(connections.len());
            for (client_index, mut connection) in connections.into_iter().enumerate() {
                let barrier = Arc::clone(&barrier);
                workers.push(scope.spawn(move || {
                    let mut local_round_trip = Vec::with_capacity(samples_per_client);
                    let mut local_client = Vec::with_capacity(samples_per_client);
                    barrier.wait();
                    for sequence in 0..samples_per_client {
                        let before = Instant::now();
                        let (acknowledgement, timing) = connection
                            .send_for_qualification_timed(&frame(client_index, sequence))
                            .map_err(classify_send_failure)?;
                        if acknowledgement != BrokerAcknowledgement::Accepted {
                            return Err(MeasurementErrorClass::UnexpectedAcknowledgement);
                        }
                        local_round_trip.push(capture_elapsed_nanos(before));
                        local_client.push(timing);
                    }
                    Ok::<_, MeasurementErrorClass>((local_round_trip, local_client))
                }));
            }
            let mut round_trip = Vec::with_capacity(workers.len() * samples_per_client);
            let mut client = Vec::with_capacity(workers.len() * samples_per_client);
            for worker in workers {
                let (local_round_trip, local_client) = worker
                    .join()
                    .map_err(|_| MeasurementErrorClass::WorkerFailure)??;
                round_trip.extend(local_round_trip);
                client.extend(local_client);
            }
            Ok::<_, MeasurementErrorClass>((round_trip, client))
        })?;
        let broker = host.qualification_stage_samples();
        if broker.len() != clients * samples_per_client {
            return Err(MeasurementErrorClass::WorkerFailure);
        }
        let group_sync_durations = host.qualification_group_sync_durations();
        let health = host.health();
        Ok((round_trip, client, broker, group_sync_durations, health))
    });
    host.stop();
    result
}

fn stage_statistics(values: impl IntoIterator<Item = u64>) -> Option<LatencyStatistics> {
    latency_statistics(values.into_iter().collect()).ok()
}

fn queue_wait_sync_correlation(
    samples: &[QualificationBrokerStageSample],
) -> Option<QueueWaitSyncCorrelation> {
    if samples.is_empty() {
        return None;
    }
    let mut queue_waits = samples
        .iter()
        .map(|sample| sample.queue_wait_ns)
        .collect::<Vec<_>>();
    queue_waits.sort_unstable();
    let p95_threshold = queue_waits[(queue_waits.len() * 95).div_ceil(100) - 1];
    Some(QueueWaitSyncCorrelation {
        samples_overlapping_group_sync: samples
            .iter()
            .filter(|sample| sample.queue_wait_group_sync_overlap_ns > 0)
            .count(),
        p95_tail_samples: samples
            .iter()
            .filter(|sample| sample.queue_wait_ns >= p95_threshold)
            .count(),
        p95_tail_samples_overlapping_group_sync: samples
            .iter()
            .filter(|sample| {
                sample.queue_wait_ns >= p95_threshold && sample.queue_wait_group_sync_overlap_ns > 0
            })
            .count(),
        total_queue_wait_ns: samples.iter().map(|sample| sample.queue_wait_ns).sum(),
        total_group_sync_overlap_ns: samples
            .iter()
            .map(|sample| sample.queue_wait_group_sync_overlap_ns)
            .sum(),
    })
}

struct SeriesResult {
    summary: SeriesSummary,
    frozen_budget_failed: bool,
}

fn qualify_series(
    series: &str,
    clients: usize,
    samples_per_client: usize,
    config: &QualificationConfig,
    receipt: &mut QualificationReceipt,
) -> Result<SeriesResult, QualificationError> {
    let mut qualifying = Vec::new();
    for attempt in 1..=config.max_attempts {
        let before = measure_control(series, attempt, "before", config.control_samples);
        let before_admitted = before.admitted;
        receipt.controls.push(before);
        if !before_admitted {
            wait_before_retry(attempt, config);
            continue;
        }

        let latency = match measure_broker_ack(clients, samples_per_client) {
            Ok(value) => value,
            Err(error) => {
                receipt.rejected_runs.push(BenchmarkObservation {
                    series: series.into(),
                    attempt,
                    latency: None,
                    frozen_budget_passed: false,
                    disposition: "rejected_measurement_error".into(),
                    measurement_error_class: Some(error),
                });
                wait_before_retry(attempt, config);
                continue;
            }
        };
        let after = measure_control(series, attempt, "after", config.control_samples);
        let after_admitted = after.admitted;
        receipt.controls.push(after);
        let budget_passed = frozen_budget_passes(&latency);
        let run = BenchmarkObservation {
            series: series.into(),
            attempt,
            latency: Some(latency),
            frozen_budget_passed: budget_passed,
            disposition: if after_admitted {
                "admitted".into()
            } else {
                "rejected_post_control_degraded".into()
            },
            measurement_error_class: None,
        };
        if !after_admitted {
            // The paired series is aborted rather than restarted in a changed
            // host condition. Its real latency result is retained separately
            // but cannot be used as acceptance evidence; a later invocation
            // begins a fresh qualification series after bounded waiting.
            receipt.rejected_runs.push(run);
            return Ok(SeriesResult {
                summary: summary(series, &qualifying),
                frozen_budget_failed: false,
            });
        }
        receipt.admitted_runs.push(run);
        qualifying.push(
            receipt
                .admitted_runs
                .last()
                .expect("admitted run was just appended")
                .latency
                .as_ref()
                .expect("admitted run has latency")
                .clone(),
        );
        if !budget_passed {
            return Ok(SeriesResult {
                summary: summary(series, &qualifying),
                frozen_budget_failed: true,
            });
        }
        if qualifying.len() == usize::from(REQUIRED_QUALIFYING_RUNS) {
            return Ok(SeriesResult {
                summary: summary(series, &qualifying),
                frozen_budget_failed: false,
            });
        }
    }
    Ok(SeriesResult {
        summary: summary(series, &qualifying),
        frozen_budget_failed: false,
    })
}

fn measure_control(
    series: &str,
    attempt: u16,
    position: &str,
    samples: usize,
) -> ControlObservation {
    match measure_broker_ack(1, samples) {
        Ok(latency) => {
            let admitted = control_is_admissible(&latency);
            ControlObservation {
                series: series.into(),
                attempt,
                position: position.into(),
                latency: Some(latency),
                admitted,
                disposition: if admitted {
                    "admitted".into()
                } else {
                    "rejected_control_noise".into()
                },
                measurement_error_class: None,
            }
        }
        Err(error) => ControlObservation {
            series: series.into(),
            attempt,
            position: position.into(),
            latency: None,
            admitted: false,
            disposition: "rejected_measurement_error".into(),
            measurement_error_class: Some(error),
        },
    }
}

fn wait_before_retry(attempt: u16, config: &QualificationConfig) {
    if attempt < config.max_attempts {
        thread::sleep(Duration::from_millis(config.wait_interval_ms));
    }
}

fn measure_broker_ack(
    clients: usize,
    samples_per_client: usize,
) -> Result<LatencyStatistics, MeasurementErrorClass> {
    measure_broker_ack_with_collector(
        clients,
        samples_per_client,
        MeasurementCollectorModel::PerThreadLocalBuffers,
        Duration::ZERO,
    )
}

fn measure_broker_ack_with_collector(
    clients: usize,
    samples_per_client: usize,
    collector_model: MeasurementCollectorModel,
    post_capture_collector_delay: Duration,
) -> Result<LatencyStatistics, MeasurementErrorClass> {
    let root =
        DisposableStateRoot::create().map_err(|_| MeasurementErrorClass::StateRootFailure)?;
    let host = BrokerHost::start(BrokerConfig::for_state_root(root.path()))
        .map_err(|error| classify_broker_startup_error(&error))?;
    let result = connect_persistent_clients(&host, clients).and_then(|connections| {
        match collector_model {
            MeasurementCollectorModel::LegacySharedMutex => {
                measure_legacy_shared_collector(connections, samples_per_client)
            }
            MeasurementCollectorModel::PerThreadLocalBuffers => measure_per_thread_collector(
                connections,
                samples_per_client,
                post_capture_collector_delay,
            ),
        }
        .and_then(|samples| {
            latency_statistics(samples).map_err(|_| MeasurementErrorClass::WorkerFailure)
        })
    });
    host.stop();
    result
}

fn connect_persistent_clients(
    host: &BrokerHost,
    clients: usize,
) -> Result<Vec<IpcClient>, MeasurementErrorClass> {
    let mut connections = Vec::with_capacity(clients);
    for _ in 0..clients {
        connections.push(
            IpcClient::connect(host.endpoint(), Duration::from_millis(5))
                .map_err(|error| classify_connect_error(&error))?,
        );
    }
    Ok(connections)
}

fn measure_legacy_shared_collector(
    connections: Vec<IpcClient>,
    samples_per_client: usize,
) -> Result<Vec<u64>, MeasurementErrorClass> {
    let samples = Arc::new(Mutex::new(Vec::with_capacity(
        connections.len() * samples_per_client,
    )));
    let barrier = Arc::new(Barrier::new(connections.len()));
    let result = thread::scope(|scope| {
        let mut workers = Vec::with_capacity(connections.len());
        for (client, mut connection) in connections.into_iter().enumerate() {
            let barrier = Arc::clone(&barrier);
            let samples = Arc::clone(&samples);
            workers.push(scope.spawn(move || -> Result<(), MeasurementErrorClass> {
                // Persistent connections have already been established. This
                // barrier is deliberately before the first timed request.
                barrier.wait();
                for sequence in 0..samples_per_client {
                    let before = Instant::now();
                    acknowledge(&mut connection, client, sequence)?;
                    // This deliberately preserves the former collector order
                    // for diagnostic comparison only: wait for the shared
                    // mutex, then observe elapsed time.
                    samples
                        .lock()
                        .map_err(|_| MeasurementErrorClass::WorkerFailure)?
                        .push(capture_elapsed_nanos(before));
                }
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|_| MeasurementErrorClass::WorkerFailure)??;
        }
        Ok::<(), MeasurementErrorClass>(())
    });
    result?;
    Arc::try_unwrap(samples)
        .map_err(|_| MeasurementErrorClass::WorkerFailure)?
        .into_inner()
        .map_err(|_| MeasurementErrorClass::WorkerFailure)
}

fn measure_per_thread_collector(
    connections: Vec<IpcClient>,
    samples_per_client: usize,
    post_capture_collector_delay: Duration,
) -> Result<Vec<u64>, MeasurementErrorClass> {
    let barrier = Arc::new(Barrier::new(connections.len()));
    thread::scope(|scope| {
        let mut workers = Vec::with_capacity(connections.len());
        for (client, mut connection) in connections.into_iter().enumerate() {
            let barrier = Arc::clone(&barrier);
            workers.push(
                scope.spawn(move || -> Result<Vec<u64>, MeasurementErrorClass> {
                    let mut local_samples = Vec::with_capacity(samples_per_client);
                    // Persistent connections have already been established. The
                    // barrier is outside every request's measured interval.
                    barrier.wait();
                    for sequence in 0..samples_per_client {
                        let before = Instant::now();
                        acknowledge(&mut connection, client, sequence)?;
                        let elapsed = capture_elapsed_nanos(before);
                        record_after_elapsed_capture(
                            &mut local_samples,
                            elapsed,
                            post_capture_collector_delay,
                        );
                    }
                    Ok(local_samples)
                }),
            );
        }
        let mut samples = Vec::with_capacity(workers.len() * samples_per_client);
        for worker in workers {
            samples.extend(
                worker
                    .join()
                    .map_err(|_| MeasurementErrorClass::WorkerFailure)??,
            );
        }
        Ok(samples)
    })
}

fn acknowledge(
    connection: &mut IpcClient,
    client: usize,
    sequence: usize,
) -> Result<(), MeasurementErrorClass> {
    match connection.send_for_qualification(&frame(client, sequence)) {
        Ok(BrokerAcknowledgement::Accepted) => Ok(()),
        Ok(_) => Err(MeasurementErrorClass::UnexpectedAcknowledgement),
        Err(error) => Err(classify_send_failure(error)),
    }
}

fn capture_elapsed_nanos(before: Instant) -> u64 {
    u64::try_from(before.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn record_after_elapsed_capture(samples: &mut Vec<u64>, elapsed: u64, collector_delay: Duration) {
    if !collector_delay.is_zero() {
        thread::sleep(collector_delay);
    }
    samples.push(elapsed);
}

fn classify_broker_startup_error(error: &IpcError) -> MeasurementErrorClass {
    match error {
        IpcError::UnsafeStateObject | IpcError::WalTooLarge | IpcError::WalCorrupt(_) => {
            MeasurementErrorClass::StateRootFailure
        }
        IpcError::Io(value)
            if matches!(
                value.kind(),
                std::io::ErrorKind::PermissionDenied
                    | std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::AlreadyExists
                    | std::io::ErrorKind::InvalidInput
            ) =>
        {
            MeasurementErrorClass::StateRootFailure
        }
        _ => MeasurementErrorClass::BrokerStartupFailure,
    }
}

fn classify_connect_error(error: &IpcError) -> MeasurementErrorClass {
    classify_timeout(error, MeasurementErrorClass::ConnectTimeout)
}

fn classify_send_failure(error: QualificationSendFailure) -> MeasurementErrorClass {
    match error {
        QualificationSendFailure::Write(error) => {
            classify_timeout(&error, MeasurementErrorClass::WriteSendTimeout)
        }
        QualificationSendFailure::Read(error) => {
            classify_timeout(&error, MeasurementErrorClass::ReadAckTimeout)
        }
        QualificationSendFailure::UnexpectedAcknowledgement => {
            MeasurementErrorClass::UnexpectedAcknowledgement
        }
    }
}

fn classify_timeout(error: &IpcError, timeout: MeasurementErrorClass) -> MeasurementErrorClass {
    match error {
        IpcError::Io(value) if value.kind() == std::io::ErrorKind::TimedOut => timeout,
        IpcError::StartupTimedOut => timeout,
        _ => MeasurementErrorClass::OtherBoundedIoFailure,
    }
}

fn frame(client: usize, sequence: usize) -> IpcFrame {
    IpcFrame::Start(LifecycleFrame {
        runtime: "qualification_runtime".into(),
        runtime_instance: format!("client_{client}"),
        invocation: format!("attempt_frame_{client}_{sequence}"),
        handler: "qualification_handler".into(),
        event: "qualification_event".into(),
        source_scope: "disposable_qualification".into(),
        revision: Some("qualification_revision".into()),
        occurred_at_unix_ms: 1_700_000_000_000,
    })
}

pub fn control_is_admissible(value: &LatencyStatistics) -> bool {
    value.p95_ms <= control_p95_max_ms() && value.p99_ms <= control_p99_max_ms()
}

pub fn frozen_budget_passes(value: &LatencyStatistics) -> bool {
    value.p95_ms <= FROZEN_P95_MS && value.p99_ms <= FROZEN_P99_MS
}

fn control_p95_max_ms() -> f64 {
    (FROZEN_P95_MS * 0.5).min(0.18 * 4.0)
}

fn control_p99_max_ms() -> f64 {
    (FROZEN_P99_MS * 0.5).min(0.24 * 4.0)
}

fn summary(series: &str, qualifying: &[LatencyStatistics]) -> SeriesSummary {
    SeriesSummary {
        series: series.into(),
        required_qualifying_runs: REQUIRED_QUALIFYING_RUNS,
        admitted_runs: u8::try_from(qualifying.len()).unwrap_or(u8::MAX),
        worst_admitted_p95_ms: qualifying.iter().map(|value| value.p95_ms).reduce(f64::max),
        worst_admitted_p99_ms: qualifying.iter().map(|value| value.p99_ms).reduce(f64::max),
    }
}

fn latency_statistics(mut samples: Vec<u64>) -> Result<LatencyStatistics, QualificationError> {
    if samples.is_empty() {
        return Err(QualificationError::Invalid("latency_samples"));
    }
    samples.sort_unstable();
    let percentile =
        |percent: usize| samples[(samples.len() * percent).div_ceil(100) - 1] as f64 / 1_000_000.0;
    Ok(LatencyStatistics {
        samples: samples.len(),
        p50_ms: percentile(50),
        p95_ms: percentile(95),
        p99_ms: percentile(99),
        max_ms: samples.last().copied().unwrap_or_default() as f64 / 1_000_000.0,
    })
}

fn validate_config(config: &QualificationConfig) -> Result<(), QualificationError> {
    if config.max_attempts < u16::from(REQUIRED_QUALIFYING_RUNS)
        || config.max_attempts > 720
        || !(1_000..=60_000).contains(&config.wait_interval_ms)
        || !(100..=10_000).contains(&config.control_samples)
        || !(100..=10_000).contains(&config.single_samples)
        || !(100..=10_000).contains(&config.client16_samples_per_client)
    {
        return Err(QualificationError::Invalid("qualification_config"));
    }
    Ok(())
}

fn validate_collector_comparison_config(
    config: &CollectorComparisonConfig,
) -> Result<(), QualificationError> {
    if !(100..=10_000).contains(&config.single_samples)
        || !(100..=10_000).contains(&config.client16_samples_per_client)
    {
        return Err(QualificationError::Invalid("collector_comparison_config"));
    }
    Ok(())
}

fn validate_stage_timing_config(config: &StageTimingConfig) -> Result<(), QualificationError> {
    if !(100..=10_000).contains(&config.client16_samples_per_client) {
        return Err(QualificationError::Invalid("stage_timing_config"));
    }
    Ok(())
}

struct DisposableStateRoot(PathBuf);

impl DisposableStateRoot {
    fn create() -> Result<Self, QualificationError> {
        let base = std::env::temp_dir().join("hookstat-g35-qualification");
        fs::create_dir_all(&base)?;
        for _ in 0..100 {
            let sequence = DISPOSABLE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("run-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(QualificationError::Io(error)),
            }
        }
        Err(QualificationError::Invalid("disposable_state_root"))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for DisposableStateRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(p95_ms: f64, p99_ms: f64) -> LatencyStatistics {
        LatencyStatistics {
            samples: 100,
            p50_ms: 0.1,
            p95_ms,
            p99_ms,
            max_ms: p99_ms,
        }
    }

    #[test]
    fn control_thresholds_are_derived_from_g28_baseline_and_release_headroom() {
        assert_eq!(control_p95_max_ms(), (FROZEN_P95_MS * 0.5).min(0.18 * 4.0));
        assert_eq!(control_p99_max_ms(), (FROZEN_P99_MS * 0.5).min(0.24 * 4.0));
        assert!(control_is_admissible(&stats(
            control_p95_max_ms(),
            control_p99_max_ms()
        )));
        assert!(!control_is_admissible(&stats(
            control_p95_max_ms() + 0.001,
            control_p99_max_ms()
        )));
    }

    #[test]
    fn worst_admitted_percentiles_never_select_the_best_run() {
        let values = vec![stats(0.3, 0.4), stats(0.8, 1.2), stats(0.5, 0.9)];
        let result = summary("single_client_persistent_release", &values);
        assert_eq!(result.admitted_runs, 3);
        assert_eq!(result.worst_admitted_p95_ms, Some(0.8));
        assert_eq!(result.worst_admitted_p99_ms, Some(1.2));
    }

    #[test]
    fn frozen_budget_is_not_weakened_by_control_admission() {
        assert!(frozen_budget_passes(&stats(1.0, 2.0)));
        assert!(!frozen_budget_passes(&stats(1.001, 2.0)));
        assert!(!frozen_budget_passes(&stats(1.0, 2.001)));
    }

    #[test]
    fn measurement_error_classes_are_phase_specific_and_bounded() {
        let timeout = IpcError::Io(std::io::Error::from(std::io::ErrorKind::TimedOut));
        assert_eq!(
            classify_connect_error(&timeout),
            MeasurementErrorClass::ConnectTimeout
        );
        assert_eq!(
            classify_send_failure(QualificationSendFailure::Write(IpcError::Io(
                std::io::Error::from(std::io::ErrorKind::TimedOut),
            ))),
            MeasurementErrorClass::WriteSendTimeout
        );
        assert_eq!(
            classify_send_failure(QualificationSendFailure::Read(IpcError::Io(
                std::io::Error::from(std::io::ErrorKind::TimedOut),
            ))),
            MeasurementErrorClass::ReadAckTimeout
        );
        assert_eq!(
            classify_send_failure(QualificationSendFailure::UnexpectedAcknowledgement),
            MeasurementErrorClass::UnexpectedAcknowledgement
        );
        assert_eq!(
            classify_broker_startup_error(&IpcError::UnsafeStateObject),
            MeasurementErrorClass::StateRootFailure
        );
        assert_eq!(
            classify_connect_error(&IpcError::Io(std::io::Error::from(
                std::io::ErrorKind::ConnectionRefused,
            ))),
            MeasurementErrorClass::OtherBoundedIoFailure
        );
    }

    #[test]
    fn measurement_error_receipt_field_is_sanitized_and_optional() {
        let observation = ControlObservation {
            series: "single_client_persistent_release".into(),
            attempt: 1,
            position: "before".into(),
            latency: None,
            admitted: false,
            disposition: "rejected_measurement_error".into(),
            measurement_error_class: Some(MeasurementErrorClass::ReadAckTimeout),
        };
        let serialized = serde_json::to_string(&observation).unwrap();
        assert!(serialized.contains("\"measurement_error_class\":\"read_ack_timeout\""));
        assert!(!serialized.contains("path"));
        assert!(!serialized.contains("username"));
    }

    #[test]
    fn collector_delay_after_elapsed_capture_cannot_change_recorded_latency() {
        let captured_latency_ns = 123_u64;
        let mut samples = Vec::new();
        record_after_elapsed_capture(&mut samples, captured_latency_ns, Duration::from_millis(1));
        assert_eq!(samples, vec![captured_latency_ns]);
    }

    #[test]
    fn collector_comparison_is_explicitly_non_acceptance_evidence() {
        let config = CollectorComparisonConfig::default();
        assert!(validate_collector_comparison_config(&config).is_ok());
        assert!(
            validate_collector_comparison_config(&CollectorComparisonConfig {
                single_samples: 99,
                client16_samples_per_client: 100,
            })
            .is_err()
        );
    }

    #[test]
    fn stage_timing_diagnostic_uses_a_bounded_16_client_configuration() {
        assert!(validate_stage_timing_config(&StageTimingConfig::default()).is_ok());
        assert!(
            validate_stage_timing_config(&StageTimingConfig {
                client16_samples_per_client: 99,
            })
            .is_err()
        );
    }

    #[test]
    fn queue_wait_sync_correlation_counts_only_measured_overlap() {
        let mut samples = vec![QualificationBrokerStageSample::default(); 20];
        for (index, sample) in samples.iter_mut().enumerate() {
            sample.queue_wait_ns = u64::try_from(index + 1).unwrap() * 100;
        }
        samples[18].queue_wait_group_sync_overlap_ns = 1_000;
        samples[19].queue_wait_group_sync_overlap_ns = 1_100;
        let correlation = queue_wait_sync_correlation(&samples).unwrap();
        assert_eq!(correlation.samples_overlapping_group_sync, 2);
        assert_eq!(correlation.p95_tail_samples, 2);
        assert_eq!(correlation.p95_tail_samples_overlapping_group_sync, 2);
        assert_eq!(correlation.total_group_sync_overlap_ns, 2_100);
    }
}
