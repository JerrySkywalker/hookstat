# HS-V031 G35/G36 landing-readiness diagnostic checkpoint

```text
RUN_ID=HS-V031-G35-G36-LANDING-READINESS-003
G35_CODE_HEAD=4f2eba9d82692c59696b61d0101afb5fad3d604d
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

The next safe action is low-frequency qualification using the same paired
controls after exact-head CI; no long run was started by this diagnostic.
