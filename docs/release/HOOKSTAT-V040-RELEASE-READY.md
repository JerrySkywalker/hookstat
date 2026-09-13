# Historical HookStat v0.4.0 Release-Ready Candidate

## Status

This is the historical pre-publication candidate record. HookStat v0.4.0 was
later publicly released from accepted main
`58e605f5cd61f5952cba8440bbbd3ed5d2609b44` with tag `v0.4.0`. The separate
v0.4.1 mixed-state instrumentation safety patch remains a candidate pending
fresh review and separate publication authorization.

```text
CURRENT_RELEASE=0.4.0
UPGRADE_BASELINE=0.3.1
HISTORICAL_CANDIDATE_PUBLICATION_AUTHORIZED=false
V040_PUBLIC_RELEASE=true
V041_PATCH_CANDIDATE=true
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
`LIVE_RELIABILITY_SMOKE=BOUNDED_UNAVAILABLE_ACCEPTED`. At that time it recorded
no populated live-reliability observations. The later public post-release Owner
dogfood found the separate mixed-state Apply defect; neither result is a claim
of populated reliability history.

Release qualification may run `cargo package --locked` and
`cargo publish --dry-run --locked`. It must not run `cargo publish`, create or
push a `v0.4.0` tag, or create a GitHub Release without separate explicit Owner
authorization.

`verify-package.ps1` and `release-gate.ps1` always bind Cargo to a newly
created disposable lab home. They never fall back to the caller's normal
`CARGO_HOME`, registry configuration, or credential store. The release gate
also requires durable external metadata before it can run qualification:

```text
OWNER_G45R=PASS
OWNER_G45R_RECEIPT_ID=<durable receipt identifier>
OWNER_G45R_TESTED_MAIN=6125734fdbc3edbe33712929abcd4cd1e0e07e1b
OWNER_G45R_NO_HISTORY_PRESENTATION=PASS
OWNER_G45R_LIVE_RELIABILITY_SMOKE=BOUNDED_UNAVAILABLE_ACCEPTED

INDEPENDENT_REVIEW=PASS
INDEPENDENT_REVIEW_RECEIPT_ID=<durable exact-candidate review receipt>
INDEPENDENT_REVIEW_SHA=<exact candidate SHA>
```

The Owner receipt must remain truthful about bounded-unavailable live
reliability; it is never evidence of populated live observations. The tested
main must be an ancestor of the candidate, and any candidate change to the
Owner-dogfood Human surface requires renewed Owner dogfood. The independent
review receipt must bind exactly to the candidate supplied to the gate.

For the final, real qualification only, supply those values to
`release-gate.ps1` using its `-OwnerG45R*` and `-IndependentReview*` parameters.
`-PreflightOnly` exists solely for the deterministic local regression harness:
it exercises fail-closed metadata and Cargo-home binding but reports
`OVERALL=NOT_RUN_PREFLIGHT_ONLY`, never a release qualification PASS.
