# HS-G29 runtime-neutral evidence core

## Boundary

The G29 core normalizes lifecycle facts before any `HookInvocation` is
constructed. Its public canonical fields are bounded opaque references for the
runtime, runtime instance, invocation, handler, event family, source scope,
and optional revision. The core stores no runtime wire object, path, command,
prompt, tool payload, or process stream.

`RuntimeHandlerRef` is deliberately unresolved in the core. A runtime
integration resolves it to HookStat's stable `HandlerIdentity` only after the
correlator has produced one canonical result. This keeps runtime definition
semantics out of the core and ledger attribution boundary.

The existing ledger receipt key is two-part, while a correlation key has a
runtime, runtime-instance, and invocation component. Integrations preserve all
three bounded opaque components in a deterministic adapter-side digest before
writing the legacy receipt key, so a reused invocation key cannot undercount
separate runtime instances.

## Lifecycle correlation

`EvidenceCorrelator` owns all pairing by the bounded triple:

```text
runtime + runtime_instance + invocation_key
```

Its results are deterministic:

| Evidence available | Result |
| --- | --- |
| start only | `Incomplete`, legacy coverage `Unknown` |
| completion only | terminal result with `BestEffort` coverage |
| start + completion | terminal result with `Complete` invocation coverage |
| exact duplicate | no new result |
| conflicting duplicate or identity | `Unknown`, never `Completed` |

The adapter-facing `legacy_coverage()` mapping preserves the released ledger
meaning: incomplete or conflicting evidence never becomes a terminal sample;
completion-only evidence remains best effort; a limited source cannot claim
complete coverage merely because one pair correlated.

The existing ledger already upgrades `BestEffort` rows when later lifecycle
evidence becomes complete. G29 makes that upgrade retain the correlator's start
timestamp. A bounded `evidence_conflict` taxonomy marker permits the inverse,
conservative correction from a previously persisted terminal row to `Unknown`
when a later duplicate contradicts it. This is an in-place row correction with
no schema or destructive ledger migration; it removes a disproven terminal
result from the denominator rather than preserving a false success or failure.

## Exactly one authority per domain

`CoverageDomain` is:

```text
runtime + event family + source scope
```

`AuthorityRouter` requires one unique rule per configured domain. It chooses
`Native` only if that domain's native admission state is `Admitted`; otherwise
it chooses `Ipc`. The transports have no runtime-specific variants.

Evidence delivered on the other transport is `Shadow`; an unknown domain is
`Unconfigured`. Neither outcome is passed to the correlator, so it cannot be
converted into a ledger row or a failure-rate denominator sample. The router
does not rely on post-ingest fuzzy deduplication to prevent double counting.

## Synthetic-only proof

The integration suite uses three deliberately generic fixtures:

- Runtime A: live ordered and out-of-order lifecycle delivery.
- Runtime B: adapter-side durable invoked/result records with a monotonic
  cursor. The fixture normalizes those records, replays them through a fresh
  core into an existing ledger, and proves one durable row and denominator
  sample after replay.
- Runtime C: a partial native-authoritative domain beside an IPC-authoritative
  fallback domain in one synthetic runtime, including reciprocal shadow tests.

They are architecture tests, not production runtime adapters. G29 creates no
native reader, IPC broker, runtime integration, ledger migration, network
listener, or user configuration mutation.
