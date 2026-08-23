//! Developer-only, sanitized G35 IPC performance qualification.
//!
//! This module is feature-gated and is never linked by the cooperative client
//! or transparent shim. It measures only bounded synthetic IPC metadata in
//! disposable state; it never inspects processes, Hook configuration, commands,
//! prompts, payloads, power settings, process priority, or affinity.

use crate::ipc::{
    BrokerAcknowledgement, BrokerConfig, BrokerHost, IpcClient, IpcError, IpcFrame, LifecycleFrame,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RECEIPT_SCHEMA_VERSION: u8 = 1;
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchmarkObservation {
    pub series: String,
    pub attempt: u16,
    pub latency: Option<LatencyStatistics>,
    pub frozen_budget_passed: bool,
    pub disposition: String,
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
    pub plan: QualificationPlan,
    pub controls: Vec<ControlObservation>,
    pub admitted_runs: Vec<BenchmarkObservation>,
    pub rejected_runs: Vec<BenchmarkObservation>,
    pub series: Vec<SeriesSummary>,
    pub outcome: String,
    pub owner_live_codex_config_mutated: bool,
    pub raw_private_content_captured: bool,
}

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
            Err(_) => {
                receipt.rejected_runs.push(BenchmarkObservation {
                    series: series.into(),
                    attempt,
                    latency: None,
                    frozen_budget_passed: false,
                    disposition: "rejected_measurement_error".into(),
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
            }
        }
        Err(_) => ControlObservation {
            series: series.into(),
            attempt,
            position: position.into(),
            latency: None,
            admitted: false,
            disposition: "rejected_measurement_error".into(),
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
) -> Result<LatencyStatistics, QualificationError> {
    let root = DisposableStateRoot::create()?;
    let host = BrokerHost::start(BrokerConfig::for_state_root(root.path()))?;
    let samples = Arc::new(Mutex::new(Vec::with_capacity(clients * samples_per_client)));
    let result = thread::scope(|scope| {
        let mut workers = Vec::with_capacity(clients);
        for client in 0..clients {
            let samples = Arc::clone(&samples);
            let endpoint = host.endpoint().clone();
            workers.push(scope.spawn(move || -> Result<(), QualificationError> {
                let mut connection = IpcClient::connect(&endpoint, Duration::from_millis(5))?;
                for sequence in 0..samples_per_client {
                    let before = Instant::now();
                    if connection.send(&frame(client, sequence))? != BrokerAcknowledgement::Accepted
                    {
                        return Err(QualificationError::Invalid("broker_acknowledgement"));
                    }
                    samples
                        .lock()
                        .map_err(|_| QualificationError::Invalid("samples_lock"))?
                        .push(u64::try_from(before.elapsed().as_nanos()).unwrap_or(u64::MAX));
                }
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|_| QualificationError::Invalid("measurement_worker"))??;
        }
        Ok::<(), QualificationError>(())
    });
    host.stop();
    result?;
    let samples = Arc::try_unwrap(samples)
        .map_err(|_| QualificationError::Invalid("samples_ownership"))?
        .into_inner()
        .map_err(|_| QualificationError::Invalid("samples_lock"))?;
    latency_statistics(samples)
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
}
