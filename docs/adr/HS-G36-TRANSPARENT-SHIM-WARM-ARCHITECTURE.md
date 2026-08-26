# ADR — G36 transparent-shim warm architecture

Status: Accepted by Owner for v0.3.1

## Context

G36 preserves the G28 `20 ms` p95 and `25 ms` p99 warm values as its reference
performance target and the `50 ms` p95 cold limit as its release limit. After
the architecture decision, the Owner approved a one-time v0.3.1 warm release
hard-cap recalibration to `25/30` ms. The accepted warm quantity remains
one-invocation parent-observed shim lifetime minus the shim-observed
original-child spawn/wait interval. Adjacent paired subtraction is not
identifiable at Windows tails.

The optimized one-process design produced one complete pass at
`18.3269/20.5058` ms p95/p99, but had no preferred `18/23`-ms engineering
margin. After independent-review correctness fixes, the clean exact-head
qualification retained a `32.6951/245.2032`-ms warm failure. Four of its five
warm populations remained near the gate; the fifth retained large
fresh-process scheduling delay. The current design is therefore capable but
not reliably acceptable.

A bounded helper-floor experiment then measured a 179-KB fresh frontend doing
only one fixed eight-byte local exchange with an idle-expiring helper. Its
500-sample cache-warmed fresh-process p95/p99 was `325.5800/451.8487` ms in the
same admitted environment. The persistent-client control exchange alone was
`3.2235/5.8826` ms. This diagnostic carries no private material and is not
acceptance evidence, but it is a strict floor: adding capsule verification,
START/COMPLETE evidence, containment, and handler semantics cannot improve
that observed population below the frozen gate.

## Options

### Option A — optimized repeated-fresh one-process shim

The existing `hookstat-hook` verifies its private capsule, emits START, owns
the Job Object, executes and waits for the original handler with inherited
streams, emits COMPLETE, and exits with the original terminal result.

### Option B — tiny frontend plus helper-owned handler execution

A fresh `hookstat-hook` delegates the full private execution request to a
local per-user on-demand idle-expiring helper. The helper verifies the capsule,
owns G35 producer state and the Job Object, executes the child, and returns a
bounded result to the waiting frontend.

### Option C — tiny frontend executes; helper prepares and observes

A local per-user idle-expiring helper verifies the capsule, owns reusable G35
producer state, emits START, and returns a bounded private execution plan. The
fresh frontend retains inherited standard handles, Job Object ownership,
handler spawn/wait, timeout enforcement, and exit propagation, then returns a
bounded completion to the helper for COMPLETE emission.

The Option C control channel is private control plane, not a reliability
evidence transport. G35 IPC remains the only instrumented evidence transport.
No private command, capsule content, path, prompt, stream, or credential may
enter G35 IPC, WAL, ledger, diagnostics, or public receipts.

## Comparison

| Concern | Option A | Option B | Option C |
| --- | --- | --- | --- |
| Warm startup floor | Stable-host shipping p95 about `12.0281` ms; current exact qualification lacks tail margin. | Tiny-frontend floor failed at `325.5800/451.8487` ms p95/p99 before semantics. | Same measured frontend/control floor as B. |
| Cold path | Exact-head cold p95 `20.5353` ms passes. | Helper startup plus frontend and execution are unmeasured; safe one-shot fallback would add another fresh process. | Helper startup plus frontend is unmeasured; a one-shot preparation fallback adds another fresh process. |
| Fresh/persistent processes | Fresh shim plus child; optional G35 broker. | Fresh frontend plus child; one per-user idle helper and optional G35 broker. | Fresh frontend plus child; one per-user idle helper and optional G35 broker. |
| Local IPC operations | G35 START and COMPLETE only. | One private frontend/helper session plus helper G35 START/COMPLETE. | Private plan and completion exchange plus helper G35 START/COMPLETE. |
| Capsule/HMAC ownership | Fresh shim validates on every invocation. | Helper can retain verified context but must prove exact-manifest invalidation. | Helper can retain verified context but must prove exact-manifest invalidation. |
| Private-command boundary | Capsule stays in the fresh shim process. | Command crosses the private control plane; never evidence IPC or storage. | Plan crosses the private control plane; never evidence IPC or storage. |
| Job Object containment | Already implemented and tested; frontend death closes the owned kill-on-close Job. | Helper must own and monitor the Job while independently detecting frontend loss. | Frontend retains current kill-on-close ownership and naturally contains its child tree. |
| Timeout semantics | Corrected deadline begins before spawn and preserves the original budget. | Helper must receive a deadline without granting startup/control time to the handler. | Frontend can preserve the corrected current deadline directly. |
| Frontend/stage-0 death | Existing Job handle closes; child tree is contained. | Requires concurrent control-disconnect detection in the helper and exact Job termination. | Existing frontend Job ownership provides the current containment behavior. |
| Helper death | Not applicable. | Frontend can lose both child ownership and authoritative terminal status; it must not fabricate success. | Frontend still owns and observes the child; helper death produces a truthful evidence gap without changing the handler result. |
| Broker independence | Broker failure is fail-open observation loss. | Helper must keep broker failure separate from private execution control. | Helper must keep broker failure separate from private preparation control. |
| Startup race | Optional broker startup is bounded and never awaited. | Helper absence requires bounded startup and a safe fallback; neither is qualified. | Same; frontend must not wait unboundedly for a plan. |
| Restore behavior | Existing future G37 manifest/capsule restore boundary remains unchanged. | Restore must also retire helper-held verified state and endpoint ownership. | Restore must also retire helper-held verified state and endpoint ownership. |
| Packaging | One standard Cargo package and existing binaries. | Standard Cargo install can add a sibling helper, but helper lifecycle/install cleanup needs proof. | Same packaging requirement as B. |
| Future G37 migration | Lowest complexity; current owner-gated activation design applies. | Highest complexity due stream/handle semantics and helper-owned child lifecycle. | Moderate complexity; private plan protocol and helper lifecycle are new, while execution remains local to the shim. |
| Security/privacy | Smallest process and channel surface. | Largest private boundary and cross-process standard-handle problem. | Private plan crosses one local channel, but standard streams remain out of it. |
| Failure truthfulness | Implemented: observation loss does not relabel handler outcome. | Hardest: helper loss can make handler outcome unknowable to the frontend. | Stronger: frontend observes handler truth; helper loss remains an incomplete observation. |

## Decision

The Owner selects Option A, the optimized repeated-fresh one-process shim, for
v0.3.1. It is the only semantics-complete implementation and preserves the
smallest process, control-channel, packaging, privacy, timeout, and containment
surface.

G36 warm qualification now has a prospective process-substrate admission gate
using the exact G28 cache-warmed minimal-shim fixture. Every candidate warm
series is bracketed by predefined pre and post controls. A failed control
retains the complete window as `REJECTED_HOST_SUBSTRATE`; when both controls
pass, the unadjusted product metric must independently pass the v0.3.1
`25/30`-ms release hard cap. The G28 `20/25`-ms values remain the reference
target and the host-control limits remain `20/25` ms. The controls are never
subtracted from the product result.

The historical `08f83bc...` failure has no contemporaneous execution of that
predefined control. Its raw `FAIL` is preserved and it is not retroactively
relabelled as a host rejection.

The later exact `93a5a1e...` receipt is the first strict host-admitted
semantics-complete shipping observation: its controls pass and its product
tail is `24.2950/26.5946` ms. It remains a raw
`FAIL_FROZEN_BUDGET` under the historical `20/25`-ms contract. It is the
shipping-evidence basis for the prospective Owner-approved recalibration, not
a receipt rewritten as a pass. New exact-head evidence must qualify under the
versioned contract in
`docs/performance/HS-G36-WARM-BUDGET-RECALIBRATION.md`.

Option B is rejected for v0.3.1. Option C remains documented only as a
deferred future architecture; it is not a G36 landing dependency. No helper
frontend, private helper protocol, or helper runtime is shipped by this
decision.

```text
FINAL_G36_SHIM_ARCHITECTURE=OPTIMIZED_ONE_PROCESS
ONE_PROCESS_ARCHITECTURE=MARGINAL
HELPER_SEMANTIC_PROTOTYPE=NOT_IMPLEMENTED_FLOOR_FAILED
OWNER_ARCHITECTURE_DECISION_REQUIRED=false
WARM_HOST_ADMISSION_POLICY=G28_CACHE_WARMED_MINIMAL_SHIM_PRE_AND_POST
HELPER_ARCHITECTURE_SHIPPED=false
G28_REFERENCE_WARM_P95_P99_MS=20/25
V031_RELEASE_WARM_P95_P99_MS=25/30
HOST_ADMISSION_P95_P99_MS=20/25
FURTHER_AUTOMATIC_BUDGET_RELAXATION=false
FROZEN_G28_HISTORICAL_EVIDENCE_CHANGED=false
NATIVE_ADMISSION_CHANGED=false
G37_STARTED=false
```
