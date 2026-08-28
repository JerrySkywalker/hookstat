# HS-V031 G38D acceptance closeout 012

This is the single-repository convergence receipt for the accepted G38B HSIP
conformance kit and G38C Windows hardening work. It records only sanitized
counts, durations, immutable Git identities, and hosted-check identities. It
does not contain prompts, tool payloads, commands, paths from user-owned
configuration, credential material, or external-runtime evidence.

```text
RUN_ID=HS-V031-G38BCD-G38R-FINAL-CLOSEOUT-012
GOAL_ID=HS-G38D-G38-ACCEPTANCE-CLOSEOUT
START_MAIN=03ebe07810e158e3009bffa294183a909e7e45b9
G38D_HEAD=PENDING_FINAL_PR_ACCEPTANCE_COMMENT
FINAL_HEAD=PENDING_FINAL_PR_ACCEPTANCE_COMMENT
RISK_CLASSES=CONVERGENCE_RECEIPT|HOSTED_CI|NON_INTERFERENCE|PRIVACY|RECOVERY
LOCAL_VALIDATION=N/A
HOSTED_CI=PENDING_FINAL_EXACT_HEAD
REAL_RUNTIME_EVIDENCE=N/A
CODEX_MUTATED=false
RAW_PRIVATE_SESSION_CONTENT_COMMITTED=false
UNRELATED_DRIFT_TOUCHED=false
DISPOSITION=PENDING_FINAL_EXACT_HEAD_REVIEW

G38A=PASS
G38B=PASS
G38B_ACCEPTED_SHA=d498faecbfa9c240573e1b221cb45721ffaed4c4
G38C=PASS
G38C_ACCEPTED_SHA=03ebe07810e158e3009bffa294183a909e7e45b9
G38B_AND_G38C_REPRESENTED_ON_START_MAIN=true

HSIP_V1_CONFORMANCE=PASS
HSIP_CONFORMANCE=PASS
HSIP_PROTOCOL_QUALIFIED=true
HOOKSTAT_IPC_INFRASTRUCTURE_READY=true
REFERENCE_PRODUCER_IMPLEMENTED=true
REFERENCE_PRODUCER_PRODUCTION_AUTHORITY=false
REFERENCE_HSIP_P50_MS=0.1048
REFERENCE_HSIP_P95_MS=0.3254
REFERENCE_HSIP_P99_MS=0.7124
REFERENCE_HSIP_MAX_MS=2.8944
REFERENCE_HSIP_OBSERVATION_GAPS=0
REFERENCE_HSIP_PERFORMANCE=PASS

BROKER_RESTART=PASS
BROKER_RECONNECT=PASS
BROKER_STARTUP_RACE=PASS
BROKER_RECOVERY=PASS
WAL_VALID_PREFIX=PASS
WAL_PARTIAL_TAIL=PASS
WAL_RECOVERY=PASS
DUPLICATE_GUARD=PASS
PROCESS_LEAK=0
IPC_DROPPED_FRAMES=0_CONTROLLED

DIAGNOSTICS_BOUNDED=PASS
DIAGNOSTICS_TRUTHFUL=PASS
DIAGNOSTICS_PRIVACY=PASS
DIAGNOSTICS=PASS
REPORT_DOCTOR_TUI_CONSISTENCY=PASS

NATIVE_L2_STATE=UPSTREAM_UNAVAILABLE
DEFAULT_UNADMITTED_DOMAIN_STATE=NOT_ADMITTED
NATIVE_FIRST=true
COVERAGE_TRUTHFUL=PASS
NO_THIRD_EVIDENCE_PATH=true
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false

NORMAL_CODEX_LAUNCH=PASS
NORMAL_CODEX_NON_INTERFERENCE=PASS
HOOKSTAT_AS_CODEX_LAUNCHER=false
TRUST_BYPASS=false
UNWANTED_PERSISTENT_CONFIG_MUTATION=false
NORMAL_CODEX_SMOKE=PASS_EPHEMERAL_READ_ONLY

HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0
PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS

G38B_CI=PASS_RUN_33191658608
G38C_CI=PASS_RUN_33199960233
WINDOWS_CI=PENDING_FINAL_EXACT_HEAD
UBUNTU_CI=PENDING_FINAL_EXACT_HEAD
INDEPENDENT_REVIEW=PENDING_FRESH_EXACT_HEAD
G38D_CI=PENDING_FINAL_EXACT_HEAD
G38D_INDEPENDENT_REVIEW=PENDING_FRESH_EXACT_HEAD

EXTERNAL_REPOSITORY_WRITES=0
TABBEACON_WRITES=0
EXTERNAL_INTEGRATION_REQUIRED=false
PUBLICATION_AUTHORIZED=false
```

## Evidence basis

The 1/5/10-client reference HSIP qualification accepted 12,000 observations
with zero gaps. The figures above retain the worst observed accepted series:
P50 0.1048 ms, P95 0.3254 ms, P99 0.7124 ms, and diagnostic maximum 2.8944
ms. The frozen release gate is P95 <= 1 ms and P99 <= 2 ms; maximum latency is
retained as a diagnostic, not an invented gate.

G38B and G38C are both ancestors of `START_MAIN`. Historical G38 foundation
and safe-activation receipts remain intact as historical evidence. The older
PR #35 is already merged historical predecessor work; no obsolete PR was
merged during this closeout.

The normal Codex smoke invoked ordinary `codex` non-interactively with an
ephemeral, read-only sandbox and user configuration disabled. It performed no
commands or mutations. It is non-interference proof only, not runtime-hook
coverage or a Native/IPC admission claim.

No external cooperative producer is bundled or required. An unnamed or
unadmitted coverage domain remains `NOT_ADMITTED`; HookStat's reference
producer remains a conformance instrument and cannot become production
authority.

The acceptance document is deliberately non-final while its own commit is
still being reviewed. A Git-tracked receipt cannot name the SHA of the commit
that introduces or revises that name without changing the SHA again. Once this
settled head has completed exact-head CI and a fresh independent review, the
immutable final acceptance receipt is recorded as a sanitized PR #40 comment
bound to that reviewed SHA, its two CI job identities, and the final
disposition. Until then, the pending fields above are not acceptance claims.
