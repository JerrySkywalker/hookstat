# HookStat v0.4.0 Release-Ready Candidate

## Status

HookStat v0.4.0 is release-ready only after its exact candidate SHA completes
the repository gates. This document is not a crates.io publication, public
Git tag, or GitHub Release.

```text
CURRENT_RELEASE=0.4.0
UPGRADE_BASELINE=0.3.1
PUBLICATION_AUTHORIZED=false
```

## Hooks Control Center

The Codex production surface is the Hooks Control Center: **Runtime Truth
First, Reliability Second**. It follows **Events → Handlers → Detail** using
Codex `/hooks` current information. Current installed handlers remain visible
when they have no observations; retained historical evidence remains distinct
when its handler is no longer installed. Known event names are localized,
unknown events remain visible, and Interrupt is presented as a runtime control
event rather than invented reliability health.

Reliability joins only admitted history to a current handler identity. Human
time, metric scope, sample denominator, coverage explanation, and risk
explanation make unavailable or partial evidence explicit. NoHistory,
NOT_ADMITTED, unknown, and zero-sample states do not become healthy values.

The release retains the deterministic 30-frame TUI visual-regression matrix,
including the offline official-shaped Codex v0.151 real-wire fixture through
parser, App, and final frame. Its pinned source evidence is:

```text
CODEX_SOURCE_REF=rust-v0.151.0
CODEX_SOURCE_SHA=78c290807ce710180111df227df3b7a4fe845452
```

Runtime-owned command, source, matcher, and context values are ephemeral
in-memory presentation data. They are not persisted to the HookStat ledger,
diagnostics export, or telemetry.

## Availability and safe management

Codex is the only production runtime in v0.4. Read/information parity is the
supported `/hooks` surface. Safe configuration management remains:

```text
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

That truthful limitation does not authorize configuration guessing. DeepSeek,
OpenCode, and every other experimental runtime remain non-production unless
separately promoted and admitted.

## G45R and publication boundary

The accepted Owner G45R disposition is `NO_HISTORY_PRESENTATION=PASS` and
`LIVE_RELIABILITY_SMOKE=BOUNDED_UNAVAILABLE_ACCEPTED`. It records no populated
live-reliability observations and no product defect; it must not be read as a
claim of populated reliability history.

Release qualification may run `cargo package --locked` and
`cargo publish --dry-run --locked`. It must not run `cargo publish`, create or
push a `v0.4.0` tag, or create a GitHub Release without separate explicit Owner
authorization.
