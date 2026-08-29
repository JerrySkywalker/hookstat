# HookStat v0.3.1 Release Candidate

## Status

This document describes the HookStat-only v0.3.1 release candidate. It is not
a crates.io publication, public Git tag, or GitHub Release.

```text
VERSION=0.3.1
V031_ARCHITECTURE_FROZEN=true
NEW_ARCHITECTURE_WORK=false
NEW_PRODUCT_FEATURE=false
NEW_RUNTIME=false
EXTERNAL_INTEGRATION_IMPLEMENTATION=false
EXTERNAL_INTEGRATION_REQUIRED=false
PUBLICATION_AUTHORIZED=false
```

HookStat v0.3.1 ships HSIP infrastructure and conformance. External cooperative
producers have independent admission lifecycles.

## Architecture and availability

HookStat remains one Rust package with a runtime-neutral canonical model,
ledger, analytics, JSON report, and TUI. HSIP v1 adds a local bounded broker,
WAL/recovery, reference producer, conformance harness, and read-only
diagnostics. It does not add a daemon requirement, network transport, a third
evidence path, or a HookStat-as-Codex launcher.

Production authority is selected independently for each coverage domain:

```text
Native admitted                       -> Native
else named IPC integration admitted   -> IPC
else                                  -> NOT_ADMITTED
```

The release-qualified Windows Native L2 result for `codex-cli 0.149.0` is
`UPSTREAM_UNAVAILABLE`. That pin must not be generalized to a newer Codex
version. The transparent shim is non-admitted and cannot become a fallback.

## HSIP conformance and performance

The broker retains an explicitly bounded 50 ms idle-read window after the
25 ms producer connection-reuse cutoff. This preserves the short adjacent
START/COMPLETE reuse path while promptly reclaiming a stale Windows pipe slot
before a later COMPLETE reconnects at the bounded connection cap. It does not
replay an ambiguous lifecycle frame.

The included reference producer exercises the same bounded local broker path as
a future integration. It is a conformance instrument, never a runtime adapter
or production authority.

```text
REFERENCE_PRODUCER_PRODUCTION_AUTHORITY=false
P95_MS<=1
P99_MS<=2
OBSERVATION_GAPS=0
```

The conformance surface covers START/COMPLETE correlation, duplicates,
out-of-order frames, malformed/oversized ingress, broker absence/restart,
uncertain ACK/no replay, WAL valid-prefix and partial-tail recovery, privacy,
and domain authority truthfulness. A third party qualifies its own exact
artifact, platform, protocol version, performance, privacy/security, and
independent review before any domain receives IPC authority.

## Fresh install and normal operation

Use an isolated Cargo root to avoid replacing a user-wide installation while
testing a candidate:

```powershell
$candidateRoot = Join-Path $env:TEMP 'hookstat-v031-candidate'
cargo install --path . --locked --root $candidateRoot
& "$candidateRoot\\bin\\hookstat.exe" report --data-root "$candidateRoot\\empty-data"
& "$candidateRoot\\bin\\hookstat.exe" doctor --json --data-root "$candidateRoot\\empty-data"
```

The ordinary binary provides report and doctor without a pre-existing external
producer. The TUI starts through `hookstat` or `hookstat tui`; its normal
interactive smoke is performed in a terminal. The optional reference
qualification binary is built with `--features performance-harness` and writes
only a sanitized measurement receipt. Normal coding-agent launch remains
`codex`; HookStat never requires `hookstat codex`.

## Upgrade and migration

v0.3.1 migration is additive and idempotent. Its release proof installs the
public v0.3.0 binary in an isolated root, reconciles a v0.3.0-format receipt
spool and journal containing completed and incomplete evidence, then runs the
candidate against that same state. It verifies retained report semantics and
unchanged canonical receipt/journal hashes alongside legacy ledger evidence,
handler aliases, revision epochs, and saved interface preferences. New broker,
diagnostic, and conformance metadata does not rewrite historical evidence
semantics.

If no Native or named IPC source is admitted, migration does not create one and
does not mutate another repository or Codex configuration. The affected domain
is reported as `NOT_ADMITTED`.

## Privacy and diagnostics

HSIP lifecycle frames, WAL, ledger, and diagnostics exclude raw prompts, tool
payloads, commands, paths, stdout/stderr, credentials, tokens, and session
content. Diagnostics are fixed, bounded, local control-plane observations;
they do not become lifecycle evidence, WAL data, ledger rows, or denominators.
No network telemetry is sent.

## Publication gate

The release candidate may run `cargo package --locked` and
`cargo publish --dry-run --locked`. It must not run `cargo publish`, create or
push a public `v0.3.1` tag, or create a GitHub Release without explicit Owner
authorization.
