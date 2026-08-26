# HS-G38 — Performance & Windows Dogfood Hardening

## Status

PLANNED after accepted G37.

## Objective

Prove the v0.3.1 evidence architecture under real Windows Terminal + Codex usage, concurrency, broker recovery, and tail-latency stress. Performance correctness is a release gate.

Production dogfood follows the G36/G37 authority decision exactly:

```text
Native where admitted
else cooperative IPC where an integration is admitted
else NOT_ADMITTED
```

The transparent shim remains implemented and correctness/security-tested, but
is `QUALIFIED_NOT_ADMITTED_PERFORMANCE`. It is not production-activated, is not
a v0.3.1 dogfood path, and is not a v0.3.1 release-performance PASS gate.

## Required real environment

At minimum qualify on an Owner-controlled Windows 11 environment with:

```text
PowerShell 7
Windows Terminal
accepted/current Codex CLI
HookStat v0.3.1 candidate
accepted TabBeacon baseline/candidate where cooperative IPC is exercised
```

Record exact versions/SHAs in sanitized receipts.

## Hook-event coverage

Exercise every practically triggerable admitted Codex event, including where available:

```text
SessionStart
UserPromptSubmit
PreToolUse
PostToolUse
PermissionRequest
PreCompact
PostCompact
SubagentStart
SubagentStop
Stop
SessionEnd
```

Unavailable/untested events must remain explicit rather than inferred healthy.

## Workload families

At minimum:

```text
1 normal Codex session
5 concurrent Codex sessions
10 concurrent Codex sessions
100 representative tool events
1,000 representative tool events
10,000 synthetic IPC evidence events
```

Use higher synthetic volumes only if they add useful boundedness evidence.

## Performance evidence

Record real candidate values for:

```text
cooperative IPC p50/p95/p99
broker queue lag
WAL flush lag
broker restart recovery time
observed HookStat-induced timeouts
observed HookStat-induced failures
```

Compare cooperative IPC with its frozen G28 budget. Preserve the transparent
shim's historical 20/25 and 25/30 failures without rerunning or relabeling them
as v0.3.1 production acceptance.

Any reproducible HookStat-induced timeout/failure in a previously healthy Hook is a release blocker.

## Broker recovery

Test:

- normal idle expiry and restart;
- broker killed while clients exist;
- broker restart after WAL partial-tail fixture;
- bounded client behavior while broker is unavailable;
- concurrent client reconnect/startup race;
- no duplicate production invocation after replay/reconnect;
- dropped/withheld evidence becomes visible coverage degradation.

## Upgrade and process behavior

Verify:

- no unwanted permanent broker/process leak;
- package binary is not unnecessarily pinned by a long-lived helper design;
- concurrent sessions remain isolated by runtime/invocation identity;
- normal Codex exit leaves bounded HookStat helper state;
- abnormal Codex/Hook termination does not fabricate success.

## Diagnostics / self-observability

Expose sufficient read-only diagnostics to answer, without raw private content:

```text
runtime
authoritative evidence source per domain
Native admission state
IPC mode: cooperative
transparent shim admission status (never active in v0.3.1)
broker state
queue lag
dropped evidence/frame count
WAL flush lag
recent IPC latency percentiles
```

A diagnostic may report `QUALIFIED_NOT_ADMITTED_PERFORMANCE` for the retained
transparent implementation, but must not imply that it is selected or active.

Self-observability instrumentation itself must be bounded and must not reintroduce hot-path synchronous persistence.

## Structural regression guards

CI cannot certify absolute Windows p95/p99 on hosted runners, but it should prevent architectural regressions such as:

```text
per-record fsync returning to Hook producer path
JSON receipt creation returning to hot path
full HookStat CLI returning as shim
unbounded frame/payload size
unbounded queue
extra shell/process layer for simple direct-plan fixtures
shadow evidence entering denominator
transparent shim becoming production authority or activation
```

## Privacy and security review

Re-audit:

- IPC endpoint scope/permissions;
- broker/state path containment;
- capsule privacy;
- WAL record privacy;
- diagnostics privacy;
- malformed/oversized local client behavior;
- process-tree containment;
- retained transparent-shim correctness/security regressions without activation;
- trust/config mutation boundaries.

No prompt/tool/stdout/stderr/raw command content may appear in production evidence/WAL/ledger/diagnostics exports.

## Required artifacts

Commit:

- sanitized Owner Windows dogfood receipt;
- exact cooperative IPC candidate comparison to its G28 budget;
- concurrency/broker-recovery receipt;
- privacy/security review receipt;
- any accepted performance exceptions with evidence and explicit Owner-facing rationale.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=hardening_only
PERSISTENCE_CHANGED=hardening_only
CODEX_INTEGRATION_CHANGED=hardening_only
USER_PERSISTENT_CONFIG_CHANGED=possible
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

## Acceptance

```text
OWNER_WINDOWS_DOGFOOD=PASS
ALL_PRACTICALLY_TRIGGERABLE_EVENTS_COVERED=true
UNTESTED_EVENTS_EXPLICIT=true

CODEX_HOOK_TIMEOUT_REGRESSION=0
HOOKSTAT_INDUCED_FAILURES=0

IPC_DROPPED_FRAMES=0_OR_EXPLICIT_OVERLOAD_TEST_ONLY
IPC_CORRUPTION=0
BROKER_RESTART=PASS
CONCURRENT_CODEX=PASS
PROCESS_LEAK=0

PERFORMANCE_BUDGET=PASS
TAIL_LATENCY_ACCEPTED=true

PRODUCTION_DOGFOOD_AUTHORITY=NATIVE|COOPERATIVE_IPC|NOT_ADMITTED
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
TRANSPARENT_SHIM_RELEASE_PERFORMANCE_GATE=false

DIAGNOSTICS_EVIDENCE_AUTHORITY=true
DIAGNOSTICS_NATIVE_ADMISSION=true
DIAGNOSTICS_IPC_HEALTH=true
DIAGNOSTICS_OVERHEAD_METRICS=true

PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
CODE_CI=PASS
```

## Stop gate

Any reproducible HookStat-induced timeout or execution failure in a previously healthy observed Hook blocks entry to G38R.

## Estimated effort

**7–11 effective engineering hours.**

## Next

`HS-G38R — v0.3.1 Hardening & Release`.
