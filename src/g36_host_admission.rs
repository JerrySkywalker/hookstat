//! Deterministic G36 warm host-substrate admission policy.
//!
//! This module is compiled only for the developer performance harness. The
//! limits are predefined constants: candidate observations never derive or alter
//! the control threshold, and control values are never subtracted from the
//! product metric.

use serde::Serialize;

pub const HOST_CONTROL_METHODOLOGY: &str =
    "g28_cache_warmed_minimal_shim_process_start_pre_and_post_v1";
pub const G28_REFERENCE_WARM_P95_MS: f64 = 20.0;
pub const G28_REFERENCE_WARM_P99_MS: f64 = 25.0;
pub const HOST_CONTROL_P95_LIMIT_MS: f64 = 20.0;
pub const HOST_CONTROL_P99_LIMIT_MS: f64 = 25.0;
pub const PRODUCT_WARM_P95_LIMIT_MS: f64 = 25.0;
pub const PRODUCT_WARM_P99_LIMIT_MS: f64 = 30.0;
pub const MAX_COMPARABLE_STARTUP_BIAS_MS: f64 = 2.0;

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
    FailRecalibratedBudget,
    RejectedHostSubstrate,
    InvalidatedByMethod,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StartupComparabilityDisposition {
    Accepted,
    RejectedHostSubstrate,
    InvalidatedBuildProfile,
}

impl WarmWindowDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdmittedPass => "ADMITTED_PASS",
            Self::FailRecalibratedBudget => "FAIL_RECALIBRATED_BUDGET",
            Self::RejectedHostSubstrate => "REJECTED_HOST_SUBSTRATE",
            Self::InvalidatedByMethod => "INVALIDATED_BY_METHOD",
        }
    }
}

pub fn control_passes(control: TailLatency) -> bool {
    control.p95_ms <= HOST_CONTROL_P95_LIMIT_MS && control.p99_ms <= HOST_CONTROL_P99_LIMIT_MS
}

pub fn product_passes(product: TailLatency) -> bool {
    product.p95_ms <= PRODUCT_WARM_P95_LIMIT_MS && product.p99_ms <= PRODUCT_WARM_P99_LIMIT_MS
}

pub fn classify_startup_comparability(
    pre_control: TailLatency,
    startup_tail_bias_correction_ms: f64,
    post_control: TailLatency,
) -> StartupComparabilityDisposition {
    if !control_passes(pre_control) || !control_passes(post_control) {
        StartupComparabilityDisposition::RejectedHostSubstrate
    } else if startup_tail_bias_correction_ms >= MAX_COMPARABLE_STARTUP_BIAS_MS {
        StartupComparabilityDisposition::InvalidatedBuildProfile
    } else {
        StartupComparabilityDisposition::Accepted
    }
}

pub fn classify_warm_window(
    pre_control: TailLatency,
    product: TailLatency,
    post_control: TailLatency,
) -> WarmWindowDisposition {
    classify_warm_window_with_health_and_oracle(pre_control, product, post_control, 0, 0, 0)
}

pub fn classify_warm_window_with_health(
    pre_control: TailLatency,
    product: TailLatency,
    post_control: TailLatency,
    product_healthy_timeouts: usize,
    product_unexpected_terminal_results: usize,
) -> WarmWindowDisposition {
    classify_warm_window_with_health_and_oracle(
        pre_control,
        product,
        post_control,
        product_healthy_timeouts,
        product_unexpected_terminal_results,
        0,
    )
}

pub fn classify_warm_window_with_health_and_oracle(
    pre_control: TailLatency,
    product: TailLatency,
    post_control: TailLatency,
    product_healthy_timeouts: usize,
    product_unexpected_terminal_results: usize,
    oracle_observation_gaps: usize,
) -> WarmWindowDisposition {
    if !control_passes(pre_control) || !control_passes(post_control) {
        WarmWindowDisposition::RejectedHostSubstrate
    } else if product_healthy_timeouts > 0 || product_unexpected_terminal_results > 0 {
        WarmWindowDisposition::FailRecalibratedBudget
    } else if oracle_observation_gaps > 0 {
        WarmWindowDisposition::InvalidatedByMethod
    } else if product_passes(product) {
        WarmWindowDisposition::AdmittedPass
    } else {
        WarmWindowDisposition::FailRecalibratedBudget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROL_PASS: TailLatency = TailLatency::new(19.0, 24.0);
    const CONTROL_FAIL_P95: TailLatency = TailLatency::new(21.0, 24.0);
    const CONTROL_FAIL_P99: TailLatency = TailLatency::new(19.0, 26.0);
    const PRODUCT_PASS: TailLatency = TailLatency::new(24.0, 29.0);
    const PRODUCT_FAIL_P95: TailLatency = TailLatency::new(26.0, 29.0);
    const PRODUCT_FAIL_P99: TailLatency = TailLatency::new(24.0, 31.0);

    #[test]
    fn passing_controls_and_product_are_admitted() {
        assert_eq!(
            classify_warm_window(CONTROL_PASS, PRODUCT_PASS, CONTROL_PASS),
            WarmWindowDisposition::AdmittedPass
        );
    }

    #[test]
    fn passing_controls_do_not_hide_a_product_failure() {
        assert_eq!(
            classify_warm_window(CONTROL_PASS, PRODUCT_FAIL_P95, CONTROL_PASS),
            WarmWindowDisposition::FailRecalibratedBudget
        );
        assert_eq!(
            classify_warm_window(CONTROL_PASS, PRODUCT_FAIL_P99, CONTROL_PASS),
            WarmWindowDisposition::FailRecalibratedBudget
        );
    }

    #[test]
    fn failed_pre_control_rejects_the_complete_window() {
        assert_eq!(
            classify_warm_window(CONTROL_FAIL_P95, PRODUCT_PASS, CONTROL_PASS),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn failed_post_control_rejects_the_complete_window() {
        assert_eq!(
            classify_warm_window(CONTROL_PASS, PRODUCT_PASS, CONTROL_FAIL_P99),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn a_failed_control_takes_precedence_over_product_failure() {
        assert_eq!(
            classify_warm_window(CONTROL_FAIL_P95, PRODUCT_FAIL_P99, CONTROL_PASS),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn passing_controls_make_a_healthy_timeout_a_product_failure() {
        assert_eq!(
            classify_warm_window_with_health(CONTROL_PASS, PRODUCT_PASS, CONTROL_PASS, 1, 0,),
            WarmWindowDisposition::FailRecalibratedBudget
        );
        assert_eq!(
            classify_warm_window_with_health(CONTROL_PASS, PRODUCT_PASS, CONTROL_PASS, 0, 1,),
            WarmWindowDisposition::FailRecalibratedBudget
        );
    }

    #[test]
    fn failed_control_rejects_even_when_candidate_timed_out() {
        assert_eq!(
            classify_warm_window_with_health(CONTROL_FAIL_P95, PRODUCT_PASS, CONTROL_PASS, 1, 0,),
            WarmWindowDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn an_oracle_gap_invalidates_only_an_otherwise_admitted_window() {
        assert_eq!(
            classify_warm_window_with_health_and_oracle(
                CONTROL_PASS,
                PRODUCT_PASS,
                CONTROL_PASS,
                0,
                0,
                1,
            ),
            WarmWindowDisposition::InvalidatedByMethod
        );
        assert_eq!(
            classify_warm_window_with_health_and_oracle(
                CONTROL_FAIL_P95,
                PRODUCT_PASS,
                CONTROL_PASS,
                0,
                0,
                1,
            ),
            WarmWindowDisposition::RejectedHostSubstrate
        );
        assert_eq!(
            classify_warm_window_with_health_and_oracle(
                CONTROL_PASS,
                PRODUCT_PASS,
                CONTROL_PASS,
                1,
                0,
                1,
            ),
            WarmWindowDisposition::FailRecalibratedBudget
        );
    }

    #[test]
    fn startup_comparability_requires_passing_pre_and_post_controls() {
        assert_eq!(
            classify_startup_comparability(CONTROL_FAIL_P95, 0.0, CONTROL_PASS),
            StartupComparabilityDisposition::RejectedHostSubstrate
        );
        assert_eq!(
            classify_startup_comparability(CONTROL_PASS, 0.0, CONTROL_FAIL_P99),
            StartupComparabilityDisposition::RejectedHostSubstrate
        );
    }

    #[test]
    fn admitted_startup_comparability_uses_the_preexisting_fixed_stop() {
        assert_eq!(
            classify_startup_comparability(
                CONTROL_PASS,
                MAX_COMPARABLE_STARTUP_BIAS_MS,
                CONTROL_PASS,
            ),
            StartupComparabilityDisposition::InvalidatedBuildProfile
        );
        assert_eq!(
            classify_startup_comparability(
                CONTROL_PASS,
                MAX_COMPARABLE_STARTUP_BIAS_MS - 0.000_1,
                CONTROL_PASS,
            ),
            StartupComparabilityDisposition::Accepted
        );
    }

    #[test]
    fn reference_target_host_admission_and_release_cap_are_separate_contracts() {
        assert_eq!(
            (G28_REFERENCE_WARM_P95_MS, G28_REFERENCE_WARM_P99_MS),
            (20.0, 25.0)
        );
        assert_eq!(
            (HOST_CONTROL_P95_LIMIT_MS, HOST_CONTROL_P99_LIMIT_MS),
            (20.0, 25.0)
        );
        assert_eq!(
            (PRODUCT_WARM_P95_LIMIT_MS, PRODUCT_WARM_P99_LIMIT_MS),
            (25.0, 30.0)
        );
        assert_eq!(
            classify_warm_window(CONTROL_PASS, PRODUCT_FAIL_P95, CONTROL_PASS).as_str(),
            "FAIL_RECALIBRATED_BUDGET"
        );
    }
}
