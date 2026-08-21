//! Local-only startup and refresh work observability.
//!
//! This module records only phase names, durations, ranges, and work counts.
//! It deliberately has no network transport and never retains hook payloads,
//! commands, or receipt contents.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StartupPhase {
    ProcessStart,
    TerminalGuardEntered,
    FirstFrameDrawn,
    ReceiptIngestReady,
    LedgerQueryReady,
    ReliabilitySnapshotReady,
    DiagnosticsReady,
}

impl StartupPhase {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ProcessStart => "process_start",
            Self::TerminalGuardEntered => "terminal_guard_entered",
            Self::FirstFrameDrawn => "first_frame_drawn",
            Self::ReceiptIngestReady => "receipt_ingest_ready",
            Self::LedgerQueryReady => "ledger_query_ready",
            Self::ReliabilitySnapshotReady => "reliability_snapshot_ready",
            Self::DiagnosticsReady => "diagnostics_ready",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkCounters {
    pub receipt_files_inspected: u64,
    pub receipt_files_parsed: u64,
    pub ledger_rows_materialized: u64,
    pub selected_query_range: Option<String>,
    pub requested_generation: Option<u64>,
    pub accepted_generation: Option<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimingSnapshot {
    pub phase_elapsed_ms: BTreeMap<&'static str, u128>,
    pub latency_ms: BTreeMap<&'static str, u128>,
    pub counters: WorkCounters,
}

#[derive(Clone, Debug)]
pub struct StartupObservatory {
    started: Instant,
    inner: Arc<Mutex<TimingSnapshot>>,
}

impl StartupObservatory {
    pub fn start() -> Self {
        let result = Self {
            started: Instant::now(),
            inner: Arc::new(Mutex::new(TimingSnapshot::default())),
        };
        result.mark(StartupPhase::ProcessStart);
        result
    }

    pub fn mark(&self, phase: StartupPhase) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot
                .phase_elapsed_ms
                .entry(phase.name())
                .or_insert_with(|| self.started.elapsed().as_millis());
        }
    }

    pub fn record_work(&self, work: WorkCounters) {
        if let Ok(mut snapshot) = self.inner.lock() {
            let requested_generation = work
                .requested_generation
                .or(snapshot.counters.requested_generation);
            let accepted_generation = work
                .accepted_generation
                .or(snapshot.counters.accepted_generation);
            snapshot.counters = work;
            snapshot.counters.requested_generation = requested_generation;
            snapshot.counters.accepted_generation = accepted_generation;
        }
    }

    pub fn record_latency(&self, name: &'static str, elapsed_ms: u128) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.latency_ms.insert(name, elapsed_ms);
        }
    }

    pub fn record_requested_generation(&self, generation: u64) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.counters.requested_generation = Some(generation);
        }
    }

    pub fn record_accepted_generation(&self, generation: u64) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.counters.accepted_generation = Some(generation);
        }
    }

    pub fn snapshot(&self) -> TimingSnapshot {
        self.inner
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    /// Deterministic, local-only text suitable for a development harness. It
    /// contains no paths, commands, receipt bodies, or runtime payloads.
    pub fn sanitized_output(&self) -> String {
        let snapshot = self.snapshot();
        let mut lines = Vec::new();
        for (phase, elapsed) in snapshot.phase_elapsed_ms {
            lines.push(format!("phase.{phase}_ms={elapsed}"));
        }
        for (name, elapsed) in snapshot.latency_ms {
            lines.push(format!("latency.{name}_ms={elapsed}"));
        }
        lines.push(format!(
            "work.receipt_files_inspected={}",
            snapshot.counters.receipt_files_inspected
        ));
        lines.push(format!(
            "work.receipt_files_parsed={}",
            snapshot.counters.receipt_files_parsed
        ));
        lines.push(format!(
            "work.ledger_rows_materialized={}",
            snapshot.counters.ledger_rows_materialized
        ));
        if let Some(range) = snapshot.counters.selected_query_range {
            lines.push(format!("work.selected_query_range={range}"));
        }
        if let Some(generation) = snapshot.counters.requested_generation {
            lines.push(format!("generation.requested={generation}"));
        }
        if let Some(generation) = snapshot.counters.accepted_generation {
            lines.push(format!("generation.accepted={generation}"));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_and_sanitized_work_are_deterministically_available() {
        let observatory = StartupObservatory::start();
        observatory.mark(StartupPhase::FirstFrameDrawn);
        observatory.record_work(WorkCounters {
            receipt_files_inspected: 2,
            receipt_files_parsed: 1,
            ledger_rows_materialized: 3,
            selected_query_range: Some("[100, 200]".into()),
            requested_generation: Some(7),
            accepted_generation: Some(7),
        });
        let snapshot = observatory.snapshot();
        assert!(snapshot.phase_elapsed_ms.contains_key("process_start"));
        assert!(snapshot.phase_elapsed_ms.contains_key("first_frame_drawn"));
        assert_eq!(snapshot.counters.ledger_rows_materialized, 3);
        assert!(
            observatory
                .sanitized_output()
                .contains("work.ledger_rows_materialized=3")
        );
    }
}
