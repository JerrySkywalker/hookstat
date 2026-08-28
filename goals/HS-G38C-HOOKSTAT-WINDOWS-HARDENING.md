# HS-G38C — HookStat Windows Hardening & Diagnostics Acceptance

## Status

PLANNED after accepted G38A. Independent of G38B after that point.

Substantial implementation already exists on the historical G38 draft branch/PR. G38C should preserve and reconcile that work rather than rebuild it.

## Objective

Qualify HookStat's own broker/WAL/diagnostics/resource behavior on Windows without requiring a named external runtime producer.

## Scope boundary

```text
PRIMARY_REPOSITORY=hookstat
EXTERNAL_REPOSITORY_WRITE=false
EXTERNAL_INTEGRATION_REQUIRED=false
REFERENCE_PRODUCER_ALLOWED=true
NORMAL_CODEX_SMOKE_ALLOWED=true
```

Normal Codex smoke is a non-interference check, not fabricated Hook event coverage.

## Existing implementation reuse

Recover the current G38 draft PR and inspect it against accepted post-G38A `main`.

Preserve validated work for:

- diagnostics schema v2;
- Native/IPC/NOT_ADMITTED authority diagnostics;
- broker numeric health/control query;
- diagnostics exclusion from WAL/ledger/denominator;
- persisted bounded receipt aggregates;
- controlled 1/5/10-client and 10,000-event tests;
- broker stop/restart/reconnect;
- WAL valid-prefix and partial-tail recovery;
- duplicate/replay guards;
- structural performance/privacy guards;
- transparent-shim production lockout.

Do not discard working code merely because old documentation named an external producer as a blocker.

## Reconciliation

After G38A merge:

1. bring the historical G38 implementation branch onto accepted `main` using safe normal Git operations;
2. resolve documentation conflicts in favor of the new single-repo contract;
3. retain historical blocked receipts as history;
4. remove stale normative statements that require an external product for G38 acceptance;
5. rerun only evidence invalidated by code/rebase changes, plus required exact-head gates.

## Owner Windows qualification

Use an Owner-controlled Windows environment to prove HookStat itself:

```text
Windows 11
PowerShell 7
HookStat candidate
reference producer / controlled HSIP clients
normal Codex CLI for non-interference smoke
```

Windows Terminal may be used where TUI/process behavior is relevant but is not a transport dependency.

## Controlled workload matrix

At minimum:

```text
1 client
5 concurrent clients
10 concurrent clients
10,000 controlled evidence frames
```

Verify:

```text
IPC_DROPPED_FRAMES=0_OR_EXPLICIT_OVERLOAD_TEST_ONLY
IPC_CORRUPTION=0
BROKER_RESTART=PASS
BROKER_RECONNECT=PASS
CONCURRENT_CLIENTS=PASS
PROCESS_LEAK=0
```

## Broker/WAL recovery

Test:

- normal idle expiry/restart;
- broker killed with clients present;
- bounded behavior during unavailability;
- reconnect/startup races;
- valid-prefix replay;
- partial-tail recovery;
- duplicate/replay resistance;
- dropped/withheld evidence becomes visible coverage degradation;
- malformed data isolation.

## Diagnostics

Diagnostics must truthfully expose:

```text
runtime
Native admission/capability
named IPC integration admission where known
authoritative source per domain
NOT_ADMITTED domains
broker state
accepted/rejected/dropped/malformed counters
queue state/lag
WAL flush lag
recent/reference IPC p50/p95/p99
transparent shim status
```

Diagnostics must be bounded and read-only.

Control frames:

```text
ENTER_WAL=false
ENTER_LEDGER=false
ENTER_DENOMINATOR=false
CREATE_THIRD_TRANSPORT=false
```

Large legacy history must not make routine diagnostics unbounded. Persisted aggregate/cursor semantics must remain truthful and crash-safe.

## Report / doctor / TUI consistency

Using controlled canonical evidence and empty/NOT_ADMITTED domains, verify that report, doctor, and TUI agree on:

- authority;
- coverage;
- handler/invocation counts;
- failure counts/rates;
- missing/incomplete state;
- broker/diagnostic health.

No incomplete or unobserved domain may appear as healthy `0.00%`.

## Normal Codex non-interference smoke

Run normal `codex` without turning HookStat into a launcher.

Verify:

```text
NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false
TRUST_BYPASS=false
UNWANTED_CONFIG_MUTATION=false
```

If ordinary Native L2 is unavailable and no named external IPC integration is admitted, expected affected-domain state is `NOT_ADMITTED`.

Do not modify another repository to change that state.

## Privacy/security review

Re-audit:

- endpoint scope/permissions;
- state-root containment;
- WAL privacy;
- diagnostics privacy;
- malformed/oversized client handling;
- process cleanup/containment;
- no network listener;
- no raw prompt/tool/stdout/stderr/raw command;
- transparent-shim activation lockout.

## Code gates

At settled exact head:

```text
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 clippy --locked --all-targets --all-features -- -D warnings
cargo +1.97.1 test --locked --all-features --no-fail-fast
cargo +1.97.1 build --locked --all-features
```

Require hosted Windows and Ubuntu exact-head CI.

## Independent review

Fresh read-only review must focus on:

- bounded diagnostics and aggregate semantics;
- recovery/corruption behavior;
- denominator/coverage truthfulness;
- process/resource leaks;
- privacy/security;
- no external integration release dependency;
- no third path or transparent-shim activation.

## Acceptance

```text
DIAGNOSTICS_BOUNDED=PASS
DIAGNOSTICS_TRUTHFUL=PASS
BROKER_RESTART=PASS
BROKER_RECONNECT=PASS
WAL_RECOVERY=PASS
DUPLICATE_GUARD=PASS
PROCESS_LEAK=0
REPORT_DOCTOR_TUI_CONSISTENCY=PASS
NORMAL_CODEX_NON_INTERFERENCE=PASS
PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NO_THIRD_EVIDENCE_PATH=true
EXTERNAL_INTEGRATION_REQUIRED=false
WINDOWS_CI=PASS
UBUNTU_CI=PASS
INDEPENDENT_REVIEW=PASS
```

## Next

G38D after G38B and G38C acceptance.
