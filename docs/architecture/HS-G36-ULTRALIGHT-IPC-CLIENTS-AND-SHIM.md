# HS-G36 ultra-light IPC clients and transparent shim

## Boundary

`crates/hookstat-ipc-client` is the single source of the G35 binary IPC v1
wire definition. The application broker imports and re-exports its types;
the cooperative producer and `hookstat-hook` use the same crate directly.
There is no second frame parser, JSON receipt path, SQLite access, analytics,
report, workbench, localization, Ratatui, or Crossterm dependency on either
producer path.

```text
cooperative Hook / hookstat-hook
             |
             +-- hookstat-ipc-client (bounded binary START / COMPLETE)
                                      |
                                      v
                         G35 local broker and append WAL
```

The only evidence transport remains `EvidenceTransport::Ipc`.

## Cooperative producer

`CooperativeProducer` accepts bounded opaque lifecycle metadata and emits
`START` or `COMPLETE` over the accepted local endpoint. Its result is one of
`Accepted`, `DroppedOverloaded`, `Busy`, `Rejected`, or `Unavailable`.
Every non-`Accepted` value is an explicit observation disposition, never an
observed-Hook failure. The local connection and acknowledgement limits default
to 2 ms and 5 ms respectively; the acknowledgement limit covers the complete
post-connect frame write and reply, rather than being accidentally reduced to
the connection limit. A capsule-scoped instrumentation allowance can stop an
observation at `BudgetExhausted`; that is a truthful coverage gap and never a
handler outcome.

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
ordinary CLI, `hookstat-ipc-broker`, and `hookstat-hook`, so an ordinary
`cargo install hookstat` supplies every HookStat runtime executable without a
second hidden install step. The shared protocol/client and shim are internal
source modules in that package; the shim binary compiles those modules directly
and does not initialize the product library. This preserves one binary protocol
source while avoiding a new public IPC-client crate or a public API commitment
to external cooperative consumers. The controlled TabBeacon proof remains a
local, unmerged consumer proof and is not a v0.3.1 publication requirement.

## Performance status

This implementation removes structural costs from the intended shim path but
does not claim qualifying G36 tail-latency evidence. Any loaded-host timings
remain `NON_QUALIFYING_DEVELOPMENT_MEASUREMENT`; the frozen G28 budget is
unchanged.
