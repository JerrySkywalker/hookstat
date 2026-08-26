# HS-G36 ultra-light IPC clients and transparent shim

## Boundary

The package-internal `src/ipc_client.rs` module is the single source of the
G35 binary IPC v1 wire definition. `src/ipc.rs` owns only the broker/WAL and
imports that internal module; both the cooperative producer and
`hookstat-hook` compile the same internal source directly. There is no second
frame parser, JSON receipt path, SQLite access, analytics, report, workbench,
localization, Ratatui, or Crossterm dependency on either producer path.

```text
cooperative Hook / hookstat-hook
             |
             +-- src/ipc_client.rs (bounded binary START / COMPLETE)
                                      |
                                      v
                         src/ipc.rs G35 local broker and append WAL
```

The only evidence transport remains `EvidenceTransport::Ipc`.

## v0.3.1 admission boundary

The Owner changed release scope after the transparent shim failed both its
original `20/25`-ms and one-time recalibrated `25/30`-ms warm contracts under
admitted Windows substrate. The implementation and correctness evidence remain;
the production authority does not.

```text
COOPERATIVE_IPC=ADMITTED
TRANSPARENT_SHIM=QUALIFIED_NOT_ADMITTED_PERFORMANCE
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
NOT_ADMITTED_IS_EVIDENCE_PATH=false
EVIDENCE_PATHS=2
```

`src/admission.rs` records the two integration states. G29's existing
domain-authority router now selects Native when Native is admitted, otherwise
IPC only when a concrete IPC integration is admitted, otherwise
`NOT_ADMITTED`. The last state cannot enter the correlator or denominator. It
does not add an `EvidenceTransport` variant.

The installed `hookstat-hook` can report
`qualified_not_admitted_performance` through `--admission-status`; its help
labels it internal/experimental. Ordinary CLI, Codex, proxy, Native, and runtime
adapter activation sources contain no path that selects the binary.

## Cooperative producer

`CooperativeProducer` accepts bounded opaque lifecycle metadata and emits
`START` or `COMPLETE` over the accepted local endpoint. Its result is one of
`Accepted`, `DroppedOverloaded`, `Busy`, `Rejected`, or `Unavailable`.
Every non-`Accepted` value is an explicit observation disposition, never an
observed-Hook failure. The local connection and acknowledgement limits default
to 2 ms and 5 ms respectively; the acknowledgement limit covers the complete
post-connect frame write and reply, rather than being accidentally reduced to
the connection limit. A producer owns at most one local connection. It uses a
nonblocking local synchronization attempt, returns `Busy` rather than making a
Hook wait behind another emitter, and reuses an acknowledged connection only
for 25 milliseconds. This is intentionally shorter than the broker's bounded
50-millisecond idle read release. A later `COMPLETE` after a long-running
original Hook therefore drops the idle client before it writes any lifecycle
frame and reconnects under the ordinary bounded policy. An uncertain write or
ACK failure discards the client but never replays that frame. A capsule-scoped
instrumentation allowance can stop an observation at `BudgetExhausted`; that is
a truthful coverage gap and never a handler outcome.

The broker is optional and idle-expiring. `hookstat-hook` requests its narrow
`hookstat-ipc-broker` sibling only after an unavailable result, does not wait
for that process to become ready, and preserves the current Hook outcome. A
subsequent lifecycle event or invocation can use the newly available broker.
This prevents startup races from consuming unbounded business time.

## Private handler capsule

`hookstat-hook` accepts only a bounded binary `HSHC` v1 capsule under a
verified private capsule root. It verifies schema, complete length, HMAC-SHA256
integrity, regular-file containment, Unix private permissions, and Windows
reparse-point safety before launching anything. Private execution text remains
inside this control plane and is not an IPC, WAL, ledger, diagnostic, fixture,
or run-artifact field.

The HMAC covers the schema version, handler key, revision, definition
fingerprint, runtime metadata, both timeout values, execution-plan kind, and
every command/argument byte. Verification uses the dependency's constant-time
MAC comparison and rejects truncation, trailing bytes, wrong keys, altered
versions, and malformed integer bounds. The 32-byte key is owner-private
control-plane material supplied by the future activation writer; G36 neither
generates, prints, persists, nor rotates one. Its test-only writer is not an
activation API. The owner-gated G37 activation must bind the selected sealed
capsule to the exact current configuration definition and provision its key in
the existing private-state boundary before it can claim production readiness.

A trusted capsule preclassifies execution as exactly one of:

- `Direct { executable, argv }`: spawned with `std::process::Command`;
- `Shell { command }`: dispatched through the platform's existing shell.

The shim never parses a Windows shell command in an attempt to synthesize a
direct plan.

## Timeout and containment

`OriginalHandlerBudget` is enforced by the shim's child wait loop. The
`InstrumentationEnvelope` is separately stored capsule metadata and is capped
at the frozen G28 50 ms cold-shim budget; it cannot grant the handler more
runtime. The shim accounts the bounded pre/post IPC attempts against one
cumulative allowance and moves optional broker process startup off the observed
Hook's deadline. A timed-out handler is terminated and produces `TimedOut`
evidence if finalization is possible; forced external shim termination can
retain only truthful start/incomplete coverage.

The supported legacy Codex configuration represents a handler timeout as an
optional numeric `timeout` field. Its existing adapter preserves that field
opaque while changing legacy proxy commands; it neither parses nor changes it.
G36 does not alter Codex configuration or install an outer declaration, so the
only currently enforced business deadline is the capsule's exact
`OriginalHandlerBudget`. A future owner-gated activation must prove its actual
runtime timeout granularity and reserve any rounded outer allowance solely for
the bounded instrumentation envelope. Until that G37 proof exists, G36 makes
no end-to-end Codex-timeout-preservation claim: an outer runtime deadline can
still end the shim before completion, which remains truthful `Incomplete`, not
a HookStat execution failure.

On Windows, `hookstat-hook` enters a kill-on-close Job Object before it spawns
the original handler. Normal root-child exit clears the kill limit, preserving
the established v0.3 descendant behavior; timeout or external shim death keeps
the containment limit armed.

## Controlled TabBeacon proof

The controlled local proof used pinned TabBeacon main
`b3f5685c37f1386f3edceb6d1d3a27403c59dddf`. It ran the existing
`CodexHookRuntime` between accepted HookStat IPC `START` and `COMPLETE` events
in temporary state through the real G35 broker. The source assertion verified
that TabBeacon's declaration construction has no `hookstat-hook` layer.

```text
Codex/runtime -> TabBeacon CodexHookRuntime
                    | START (cooperative IPC)
                    | normal TabBeacon dispatch
                    | COMPLETE (cooperative IPC)
```

The local TabBeacon candidate and its path dependency were not committed,
pushed, released, or applied to Owner configuration.

## Distribution boundary

v0.3.1 retains one public Cargo package: `hookstat`. Its package owns the
ordinary CLI, `hookstat-ipc-broker`, and the explicitly non-admitted
`hookstat-hook`, so an ordinary `cargo install hookstat` supplies the broker and
retained experimental executable without a second hidden install step. The
shared protocol/client and shim are internal source modules in that package;
the shim binary compiles those modules directly and does not initialize the
product library.

Cooperative-producer policy and observation types remain package-internal:
HookStat does not publish a new IPC-client API or crate commitment. A real
consumer owns a narrow client pinned to the versioned HSIP v1 wire contract,
without depending on HookStat's CLI/TUI/analytics/SQLite/workbench product
surface. The controlled TabBeacon proof uses this source boundary and remains
valid because the exact `src/ipc_client.rs` SHA-256 is unchanged. See
[`HSIP-V1-COOPERATIVE-INTEGRATION.md`](HSIP-V1-COOPERATIVE-INTEGRATION.md).

## Performance status

The M1 methodology audit proved that adjacent alternating
`transparent_duration - direct_duration` samples still subtract two distinct
process lifetimes and cannot identify HookStat's Windows tail overhead.  G36
therefore uses one actual transparent invocation as the candidate acceptance
quantity:

```text
parent-observed shipping-shim lifetime
- shim-observed original child spawn/wait interval
= HookStat transparent overhead
```

The developer-only oracle is compiled only by `performance-harness`.  It sends
two fixed 16-byte records over one owner-only local pipe after normal COMPLETE
instrumentation: the same-invocation child interval and the connect plus first
record write duration.  It serializes no command, capsule content, path,
prompt, tool data, stream, or credential.  This is an ephemeral measurement
control channel, not a reliability evidence transport; G35 IPC remains the
single evidence transport and the normal shipping build has no oracle path.

The first 100-sample diagnostic measured raw warm overhead at `16.3280` ms p95
and `16.6528` ms p99 while charging the complete oracle channel.  Its primary
record observation cost was `0.0347/0.0435` ms p95/p99.  The instrumented
binary was 7,168 bytes larger but slightly faster in that diagnostic; the
result established one-process viability without claiming acceptance.

One complete qualification retained five 100-sample populations per path and
compared the complete shipping and instrumented startup envelopes. Its
corrected result was cooperative `0.1733/0.3467` ms p95/p99, warm transparent
`18.3269/20.5058` ms p95/p99, cold transparent `18.5793` ms p95, zero
observation gaps, and zero HookStat-induced timeouts. That historical exact
candidate pass is preserved.

Independent review then required correctness and provenance fixes. The clean
corrected source head's historical complete qualification passes cooperative at
`0.1357/0.2187` ms, cold at `20.5353` ms, and the zero-induced-timeout gate,
but fails warm at `32.6951/245.2032` ms. Four warm populations remain close to
the budget while one retains a large fresh-process scheduling tail. That
receipt predates the Owner-approved G28 minimal-shim pre/post control and has no
contemporaneous control result, so its raw failure is preserved without a
retroactive host-substrate classification.

The subsequent package-excluded helper floor measured a 179-KB fresh frontend
plus one fixed local exchange at `325.5800/451.8487` ms p95/p99 in the current
environment. It contains no handler or private material and is diagnostic
only, but required helper semantics cannot be faster than that floor. The
architecture decision and option comparison are recorded in
`docs/adr/HS-G36-TRANSPARENT-SHIM-WARM-ARCHITECTURE.md`.

The Owner selected the optimized one-process shim as the retained
semantics-complete implementation. Each prospective
warm candidate series is now bracketed by the exact G28 cache-warmed
minimal-shim process-start control. Both controls must pass `20/25` ms before
the candidate window is admitted. A rejected window is retained and is neither
a product pass nor a product fail. In an admitted window, the same-invocation
product metric must itself satisfy the one-time v0.3.1 release hard cap of
`25/30` ms; control values are never subtracted from it. The G28 `20/25`-ms
values remain the performance reference target. The recalibration authority,
shipping-evidence basis, and immutable historical outcome are recorded in
`docs/performance/HS-G36-WARM-BUDGET-RECALIBRATION.md`. Five independently
admitted passing windows are required. Option C remains deferred and no helper
architecture is shipped. The later exact `660142c...` session admitted two
product windows under this policy: `14.6629/15.2830` ms passed, then
`23.0369/38.1377` ms failed the one-time `25/30` cap with both controls passing.
Qualification stopped without score-fishing. The Owner declined any further
relaxation and removed transparent production admission from v0.3.1.

```text
WARM_ACCEPTANCE_METRIC=OTHER_PROVEN_METRIC
OTHER_PROVEN_METRIC=SAME_INVOCATION_PARENT_LIFETIME_MINUS_CHILD_SPAWN_WAIT
PAIRED_METHOD_IDENTIFIABLE=false
ONE_PROCESS_ARCHITECTURE=MARGINAL
HELPER_SEMANTIC_PROTOTYPE=NOT_IMPLEMENTED_FLOOR_FAILED
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
WARM_HOST_ADMISSION_POLICY=G28_CACHE_WARMED_MINIMAL_SHIM_PRE_AND_POST
WARM_ADMITTED_RUNS_REQUIRED=5
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
HELPER_ARCHITECTURE_SHIPPED=false
TRANSPARENT_SHIM_20_25_RESULT=FAIL
TRANSPARENT_SHIM_25_30_RESULT=FAIL
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_IPC_SHIM=QUALIFIED_NOT_ADMITTED_PERFORMANCE
COOPERATIVE_IPC_PRODUCTION_ADMISSION=true
G36_TRANSPARENT_PERFORMANCE=FAIL_PRESERVED_NOT_RELEASE_GATE
OWNER_ARCHITECTURE_DECISION_REQUIRED=false
FROZEN_G28_HISTORICAL_EVIDENCE_CHANGED=false
```
