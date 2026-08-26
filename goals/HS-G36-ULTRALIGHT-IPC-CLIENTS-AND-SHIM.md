# HS-G36 — Ultra-Light IPC Clients / Transparent Shim

## Status

IN PROGRESS after accepted G35. The Owner selected the optimized one-process
transparent shim for v0.3.1. Warm landing evidence requires the prospective
G28 minimal-shim host-substrate admission defined in
`docs/performance/HS-G28-HOT-PATH-METHODOLOGY.md`. The G28 `20/25`-ms warm
values remain the reference target; the Owner-approved, one-time v0.3.1
release hard cap is `25/30` ms as recorded in
`docs/performance/HS-G36-WARM-BUDGET-RECALIBRATION.md`. Host admission remains
`20/25` ms.

## Objective

Replace the v0.3 full HookStat proxy hot path with an ultra-light IPC evidence path that preserves original Hook semantics while adding the smallest practical incremental latency.

IPC has two producer modes but remains one evidence transport:

```text
IPC
├ cooperative producer
└ transparent shim
```

## Cooperative producer

For Hooks whose source we control, provide a tiny runtime-neutral client API that can emit:

```text
START
COMPLETE
```

directly to the HookStat broker.

The cooperative path must not launch a HookStat wrapper process around the observed Hook.

TabBeacon is the first intended real cooperative dogfood consumer because it is high-frequency, latency-sensitive, and already exposes the ownership/currentness problems created by wrapping its command declaration.

A cooperative client must be embeddable without pulling HookStat's product/TUI/analytics dependencies into the consumer.

## Dedicated transparent shim

Add a dedicated minimal executable conceptually named `hookstat-hook` for third-party Hooks that cannot emit HookStat evidence themselves.

Its production hot path is limited to:

```text
process entry
read bounded handler capsule
emit IPC START
spawn original handler
wait / enforce original semantics
emit IPC COMPLETE
propagate terminal result
exit
```

It must not link or initialize:

```text
Ratatui
SQLite
analytics
workbench
report
localization
full HookStat TUI/application state
```

The existing `hookstat.exe codex proxy` may remain temporarily for legacy restore/migration compatibility, but it must no longer be the target production hot path after G37.

## Handler capsule

Instrumentation precompiles a bounded per-handler capsule rather than loading the full private manifest on every invocation.

Conceptual fields:

```text
schema version
stable HookStat handler key
revision
definition fingerprint
bounded execution plan
original timeout semantics
minimal runtime/source metadata required for IPC
```

Capsules are HookStat-owned private control-plane material and may contain the execution information necessary for exact restore/dispatch. They must not be copied into receipts, WAL analytics rows, diagnostics exports, or repository fixtures containing Owner-private commands.

Validate path containment and reject redirection/tampering.

## Process fast path

Where the original handler can be proven to be a simple executable + argv plan, spawn it directly using the platform process API/Rust `Command` without an extra shell.

Commands requiring real shell semantics fall back to the platform shell.

HookStat must not implement an incomplete Windows shell parser merely to avoid `cmd.exe`.

Record fast-path/fallback-path performance separately.

## Timeout correctness

Explicitly distinguish:

```text
OriginalHandlerBudget
InstrumentationEnvelope
```

Requirements:

- the original handler never receives more execution time than its original semantics allow;
- HookStat's startup/finalization overhead does not silently reduce the original handler's intended business budget;
- any outer allowance exists only for bounded instrumentation envelope and is derived from the frozen G28 budget;
- a handler exceeding its original budget becomes truthful `TimedOut`/equivalent evidence;
- forced outer termination that prevents final evidence remains truthful Incomplete/coverage-limited rather than fabricated success/failure.

Blind timeout inflation is prohibited.

## Windows containment

Preserve process-tree containment equivalent to the accepted v0.3 Windows Job Object safety semantics unless a replacement is proven at least as safe and faster.

Benchmark containment overhead separately. Do not remove containment merely to improve a headline number.

## Performance optimization scope

The shim may use dedicated release-profile tuning where justified, including minimizing dependencies/imports, LTO, stripping, codegen settings, and other bounded binary-startup optimizations.

Optimize for:

```text
cold startup
warm startup
working set
process count
shell count
allocation count
IPC send latency
tail latency
```

not just executable file size.

## Required tests

- cooperative START/COMPLETE becomes one correct invocation;
- cooperative producer survives broker unavailable/overload according to bounded fail-open policy;
- transparent shim preserves exit 0;
- transparent shim preserves nonzero exit class;
- original command comfortably inside timeout;
- original command close to timeout boundary;
- original command exceeds original timeout;
- outer instrumentation allowance does not grant extra original-handler runtime;
- broker failure does not hang the observed Hook indefinitely;
- direct fast path and shell fallback preserve command semantics for admitted fixtures;
- capsule tamper/path-redirection rejection;
- forced shim termination produces truthful incomplete evidence where applicable;
- Windows child/descendant containment;
- privacy boundary excludes raw streams/prompt/tool payload.

## Real integration proof

Produce a pinned TabBeacon cooperative integration proof in a controlled/local candidate environment:

```text
Codex -> TabBeacon Hook
              ├ IPC START
              ├ normal TabBeacon work
              └ IPC COMPLETE

HookStat receives evidence without wrapping the TabBeacon declaration.
```

This proof must not require publishing a new TabBeacon release during G36.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=true
CODEX_INTEGRATION_CHANGED=true
PROCESS_MODEL_CHANGED=true
TIMEOUT_SEMANTICS_TOUCHED=true
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

Independent Windows process/timeout safety review is required.

## Acceptance

```text
FULL_HOOKSTAT_CLI_ON_HOT_PATH=false
JSON_ON_HOT_PATH=false
SQLITE_ON_HOT_PATH=false
FSYNC_ON_HOT_PATH=false
FULL_MANIFEST_PARSE_ON_HOT_PATH=false

DEDICATED_MINIMAL_SHIM=true
HANDLER_CAPSULE=true
DIRECT_PROCESS_FAST_PATH=true
SHELL_FALLBACK_PRESERVED=true

ORIGINAL_TIMEOUT_SEMANTICS_PRESERVED=true
INSTRUMENTATION_ENVELOPE_BOUNDED=true
WINDOWS_PROCESS_TREE_CONTAINMENT=PASS

COOPERATIVE_IPC=PASS
TRANSPARENT_IPC_SHIM=PASS
TABBEACON_COOPERATIVE_PROOF=PASS

PERFORMANCE_BUDGET=PASS
SECURITY_REVIEW=PASS
CODE_CI=PASS

FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
WARM_HOST_ADMISSION_POLICY=PASS_REVIEWED
WARM_ADMITTED_RUNS=5/5
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
```

## Estimated effort

**10–16 effective engineering hours.**

## Next

`HS-G37 — Codex Evidence Routing / Native L2 / Migration`.
