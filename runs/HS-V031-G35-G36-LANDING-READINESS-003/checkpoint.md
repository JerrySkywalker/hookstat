# HS-V031 G35/G36 landing-readiness diagnostic checkpoint

```text
RUN_ID=HS-V031-G35-G36-LANDING-READINESS-003
G35_CODE_HEAD=40b5f05c7e3825da8b7cac88a5ce3894cd84d88d
G35_DIAGNOSTIC_RECEIPT=g35-diagnostic-001.json
G35_DIAGNOSTIC_SCHEMA_VERSION=2
G35_DIAGNOSTIC_CONFIGURATION=max_attempts:5;wait_ms:1000;control_samples:100;single_samples:100;client16_samples:100
G35_DIAGNOSTIC_OUTCOME=BLOCKED_NO_QUALIFYING_WINDOW
G35_DIAGNOSTIC_CONTROLS_TOTAL=12
G35_DIAGNOSTIC_CONTROLS_ADMITTED=10
G35_DIAGNOSTIC_CONTROL_MEASUREMENT_ERRORS=1
G35_DIAGNOSTIC_CONTROL_NOISE_REJECTIONS=1
G35_DIAGNOSTIC_REJECTED_RUN_MEASUREMENT_ERRORS=1
G35_DIAGNOSTIC_ADMITTED_RUNS=4
G35_DIAGNOSTIC_ERROR_CLASS=read_ack_timeout
G35_DIAGNOSTIC_CLASSIFICATION=A_HOST_SCHEDULER_REJECTION
G35_DETERMINISTIC_RUNNER_DEFECT_REPRODUCED=false
G35_BROKER_CLIENT_DEFECT_REPRODUCED=false
G35_PERFORMANCE_ACCEPTED=false
G35_LOW_FREQUENCY_RECEIPT=g35-qualification-low-frequency-001.json
G35_LOW_FREQUENCY_OUTCOME=FAIL_FROZEN_G28_BUDGET
G35_LOW_FREQUENCY_CONTROLS_ADMITTED=12_OF_12
G35_LOW_FREQUENCY_SINGLE_ADMITTED_RUNS=5_OF_5
G35_LOW_FREQUENCY_SINGLE_WORST_P95_MS=0.1353
G35_LOW_FREQUENCY_SINGLE_WORST_P99_MS=0.4068
G35_LOW_FREQUENCY_CLIENT16_ADMITTED_RUNS=1_OF_5
G35_LOW_FREQUENCY_CLIENT16_FAILING_P95_MS=1.1558
G35_LOW_FREQUENCY_CLIENT16_FAILING_P99_MS=1.4763
G35_PERFORMANCE=FAIL_FROZEN_BUDGET
G35_MERGED=false
FROZEN_G28_BUDGET_CHANGED=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```

The developer-only runner now records only the bounded schema-v2 error class
for failed measurements. This receipt contains successful measurements plus an
intermittent `read_ack_timeout` in one control and one candidate, rather than a
deterministic failure in every attempt. That is a host/scheduler rejection and
not a performance pass, product latency failure, or reproduced harness defect.
The receipt does not contain raw OS error text, paths, usernames, hostnames,
process data, commands, Hook content, prompts, or payloads.

The subsequent normal low-frequency pass admitted every paired control and all
five single-client runs, but its first admitted 16-client run exceeded the
immutable p95 budget. That makes G35 a frozen-budget performance failure, not
a host-noise rejection. Do not merge G35 from this head. No performance change
was made in response; the next safe action is focused performance diagnosis or
Owner direction.
