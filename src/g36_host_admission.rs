//! Deterministic G36 warm host-substrate admission policy.
//!
//! This module is compiled only for the developer performance harness. The
//! limits are frozen constants: candidate observations never derive or alter
//! the control threshold, and control values are never subtracted from the
//! product metric.

use serde::Serialize;

pub const HOST_CONTROL_METHODOLOGY: &str =
    "g28_cache_warmed_minimal_shim_process_start_pre_and_post_v1";
pub const HOST_CONTROL_P95_LIMIT_MS: f64 = 20.0;
pub const HOST_CONTROL_P99_LIMIT_MS: f64 = 25.0;
pub const PRODUCT_WARM_P95_LIMIT_MS: f64 = 20.0;
pub const PRODUCT_WARM_P99_LIMIT_MS: f64 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct TailLatency {
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl TailLatency {
    pub const fn new(p95_ms: f64, p99_ms: f64) -> Self {
        Self { p95_ms, p99_ms }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarmWindowDisposition {
    AdmittedPass,
    FailFrozenBudget,
    RejectedHostSubstrate,
}

impl WarmWindowDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdmittedPass => "ADMITTED_PASS",
            Self::FailFrozenBudget => "FAIL_FROZEN_BUDGET",
            Self::RejectedHostSubstrate => "REJECTED_HOST_SUBSTRATE",
        }
    }
}

pub fn control_passes(control: TailLatency) -> bool {
    control.p95_ms <= HOST_CONTROL_P95_LIMIT_MS && control.p99_ms <= HOST_CONTROL_P99_LIMIT_MS
}

pub fn product_passes(product: TailLatency) -> bool {
    product.p95_ms <= PRODUCT_WARM_P95_LIMIT_MS && product.p99_ms <= PRODUCT_WARM_P99_LIMIT_MS
}

pub fn classify_warm_window(
    pre_control: TailLatency,
    product: TailLatency,
    post_control: TailLatency,
) -> WarmWindowDisposition {
    classify_warm_window_with_health(pre_control, product, post_control, 0, 0)
}

pub fn classify_warm_window_with_health(
    pre_control: TailLatency,
    product: TailLatency,
    post_control: TailLatency,
    product_healthy_timeouts: usize,
    product_unexpected_terminal_results: usize,
) -> WarmWindowDisposition {
    if !control_passes(pre_control) || !control_passes(post_control) {
        WarmWindowDisposition::RejectedHostSubstrate
    } else if product_passes(product)
        && product_healthy_timeouts == 0
        && product_unexpected_terminal_results == 0
    {
        WarmWindowDisposition::AdmittedPass
    } else {
        WarmWindowDisposition::FailFrozenBudget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASS: TailLatency = TailLatency::new(20.0, 25.0);
    const FAIL_P95: TailLatency = TailLatency::new(20.000_1, 24.0);
    const FAIL_P99: TailLatency = TailLatency::new(19.0, 25.000_1);

    #[test]
    fn passing_controls_and_product_are_admitted() {
        assert_eq!(
            classify_warm_window(PASS, PASS, PASS),
            WarmWindowDisposition::AdmittedPass
        );
    }

    #[test]
    fn passing_controls_do_not_hide_a_product_failure() {
        assert_eq!(
            classify_warm_window(PASS, FAIL_P95, PASS),
            WarmWindowDisposition::FailFrozenBudget
        );
        assert_eq!(
            classify_warm_window(PASS, FAIL_P99, PASS),
            WarmWindowDisposition::FailFrozenBudget
        );
    }

    #[test]
    fn failed_pre_control_rejects_the_complete_window() {
        assert_eq!(
            classify_warm_window(FAIL_P95, PASS, PASS),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn failed_post_control_rejects_the_complete_window() {
        assert_eq!(
            classify_warm_window(PASS, PASS, FAIL_P99),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn a_failed_control_takes_precedence_over_product_failure() {
        assert_eq!(
            classify_warm_window(FAIL_P95, FAIL_P99, PASS),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn passing_controls_make_a_healthy_timeout_a_product_failure() {
        assert_eq!(
            classify_warm_window_with_health(PASS, PASS, PASS, 1, 0),
            WarmWindowDisposition::FailFrozenBudget
        );
        assert_eq!(
            classify_warm_window_with_health(PASS, PASS, PASS, 0, 1),
            WarmWindowDisposition::FailFrozenBudget
        );
    }

    #[test]
    fn failed_control_rejects_even_when_candidate_timed_out() {
        assert_eq!(
            classify_warm_window_with_health(FAIL_P95, PASS, PASS, 1, 0),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }
}
