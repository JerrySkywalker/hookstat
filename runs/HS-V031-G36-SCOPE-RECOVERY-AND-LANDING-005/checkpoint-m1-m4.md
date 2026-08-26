# G36 scope recovery checkpoint M1-M4

## Admission

```text
RUN_ID=HS-V031-G36-SCOPE-RECOVERY-AND-LANDING-005
START_MAIN=e67972d027582b18a0c48705084e00127fc693ce
G36_START_HEAD=b9a701d89e33d5a291f2735ad0dec35984d75022
WORKTREE_CLEAN_AT_ADMISSION=true
INDEX_LOCK=false
ACTIVE_CARGO_RUST_HOOKSTAT_PROCESSES=0
```

## Scope and evidence truth

```text
COOPERATIVE_IPC=PRODUCTION_ADMITTED_REQUIRED
TRANSPARENT_IPC_SHIM=QUALIFIED_NOT_ADMITTED_PERFORMANCE
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_SHIM_20_25_RESULT=FAIL
TRANSPARENT_SHIM_25_30_RESULT=FAIL
FURTHER_BUDGET_RELAXATION=false
```

No historical receipt was edited or relabelled. The exact
`660142c5cba8ff3e3716d6dc04e43d1460d3ed6b` admitted
`FAIL_RECALIBRATED_BUDGET` remains `FULL_ACCEPTANCE_FAIL` in
`G36_PERF_EVIDENCE_INDEX.md`. G36T is a deferred v0.3.2-or-later track and is
not on the v0.3.1 dependency path.

## Authority and lockout

The existing G29 domain router now requires explicit admission on both
transports:

```text
Native admitted -> Native
else IPC integration admitted -> IPC
else -> NOT_ADMITTED
```

`EvidenceTransport` remains exhaustively Native/IPC. `NOT_ADMITTED` is a
coverage/ingress result, never a transport, shadow, correlated invocation, or
denominator sample. Legacy serialized authority documents without the new IPC
field default fail-closed to `Unavailable`.

The v0.3.1 release constants record cooperative IPC as `Admitted` and the
transparent shim as `QualifiedNotAdmittedPerformance`. The retained packaged
shim reports that state through `--admission-status` and labels itself
internal/experimental in help. Deterministic source-boundary tests prove the
ordinary CLI, Codex, proxy, Native, and runtime adapter cannot select it.

## Cooperative distribution boundary

```text
COOPERATIVE_DISTRIBUTION_MODEL=RUNTIME_INTEGRATION_OWNED_HSIP_V1_CLIENT
IPC_PROTOCOL_VERSION=1
FULL_HOOKSTAT_PRODUCT_DEPENDENCIES_REQUIRED=false
TABBEACON_COOPERATIVE_PATH_REMAINS_POSSIBLE=true
```

The retained TabBeacon proof at
`b3f5685c37f1386f3edceb6d1d3a27403c59dddf` is source-bound to
`src/ipc_client.rs` SHA-256
`a6431eea9e2b373781d46b7f3a0694b5658ea8ca2b8076df6cd3a496ca6acdd6`.
The current candidate retains that exact client hash. No TabBeacon release or
Owner configuration mutation occurred.

## Focused validation

Rust 1.97.1 focused tests passed:

```text
G36_SCOPE_RECOVERY_TESTS=4_PASS
G29_ROUTER_AND_CORRELATOR_TESTS=16_PASS
G35_BROKER_WAL_TESTS=9_PASS
PACKAGE_LAYOUT_TESTS=3_PASS
G36_PERFORMANCE_HARNESS_CONTRACT_TESTS=19_PASS_2_EXPLICITLY_IGNORED
FMT=PASS
DIFF_CHECK=PASS
```

The ignored G36 harness now includes a dedicated release-profile cooperative
acceptance path that writes one sanitized five-run receipt and does not execute
transparent warm qualification.

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
NO_THIRD_EVIDENCE_PATH=true
G37_STARTED=false
G38_STARTED=false
PUBLICATION_AUTHORIZED=false
```
