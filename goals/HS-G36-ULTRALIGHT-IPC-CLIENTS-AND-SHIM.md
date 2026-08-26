# HS-G36 — Ultra-Light IPC Clients / Transparent Shim

## Status

IN PROGRESS after accepted G35. Cooperative IPC is the v0.3.1 production
admission target. The optimized one-process transparent shim remains
implemented and correctness-qualified, but it is not performance-admitted or
production-activated after exact host-admitted failures under both the original
`20/25` contract and the one-time v0.3.1 `25/30` recalibration. Neither failure
is rewritten. G36T owns any v0.3.2-or-later transparent rearchitecture.

## Objective

Admit the ultra-light cooperative IPC producer for v0.3.1, preserve the
correctness-qualified transparent implementation and its evidence, and prevent
the non-admitted transparent integration from becoming production authority.

IPC has two producer modes but remains one evidence transport:

```text
IPC
├ cooperative producer: ADMITTED for v0.3.1
└ transparent shim: QUALIFIED_NOT_ADMITTED_PERFORMANCE
```

`NOT_ADMITTED` is a coverage state inside this two-path architecture. It is not
a third evidence path.

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

The executable is retained in the v0.3.1 package as internal/experimental. It
must report its non-production admission truth, and no normal activation path
may select or install it while its admission state is
`QUALIFIED_NOT_ADMITTED_PERFORMANCE`.

Its retained execution path is limited to:

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

The existing `hookstat.exe codex proxy` may remain temporarily for legacy
restore/migration compatibility. G37 must not replace it with the non-admitted
shim. Domains without admitted Native or cooperative IPC remain
`NOT_ADMITTED` rather than gaining an implicit transparent authority.

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

## Preserved performance history

The shim retains the optimized one-process implementation and the complete
historical evidence taxonomy. No further redesign or budget increase is part of
v0.3.1 scope recovery.

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

## Real integration proof and distribution

Produce a pinned TabBeacon cooperative integration proof in a controlled/local candidate environment:

```text
Codex -> TabBeacon Hook
              ├ IPC START
              ├ normal TabBeacon work
              └ IPC COMPLETE

HookStat receives evidence without wrapping the TabBeacon declaration.
```

This proof must not require publishing a new TabBeacon release during G36.

The v0.3.1 distribution boundary is an integration-owned HSIP v1 client pinned
to the versioned wire contract. It does not create a new public HookStat crate
or require HookStat's TUI, analytics, workbench, SQLite, or CLI dependencies in
the consumer. See
`docs/architecture/HSIP-V1-COOPERATIVE-INTEGRATION.md`.

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
COOPERATIVE_P95_MS<=1
COOPERATIVE_P99_MS<=2
COOPERATIVE_OBSERVATION_GAPS=0

TRANSPARENT_SHIM_IMPLEMENTATION_RETAINED=true
TRANSPARENT_SHIM_CORRECTNESS_TESTS=PASS
TRANSPARENT_IPC_SHIM=QUALIFIED_NOT_ADMITTED_PERFORMANCE
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_SHIM_20_25_RESULT=FAIL
TRANSPARENT_SHIM_25_30_RESULT=FAIL
TRANSPARENT_SHIM_FAILURE_EVIDENCE_PRESERVED=true

TABBEACON_COOPERATIVE_PROOF=PASS
TABBEACON_COOPERATIVE_PATH_REMAINS_POSSIBLE=true
FULL_HOOKSTAT_PRODUCT_DEPENDENCIES_REQUIRED=false
IPC_PROTOCOL_VERSION=1

G35_ASYNC_DURABILITY_PRESERVED=true
ACK_AFTER_WAL_APPEND=true
WINDOWS_OVERLAPPED_CLIENT_PRESERVED=true

COOPERATIVE_PERFORMANCE=PASS
SECURITY_REVIEW=PASS
CODE_CI=PASS

FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
WARM_HOST_ADMISSION_POLICY=PASS_REVIEWED
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
NO_THIRD_EVIDENCE_PATH=true
NOT_ADMITTED_IS_EVIDENCE_PATH=false
```

## Estimated effort

**10–16 effective engineering hours.**

## Next

`HS-G37 — Codex Evidence Routing / Native L2 / Migration`.
