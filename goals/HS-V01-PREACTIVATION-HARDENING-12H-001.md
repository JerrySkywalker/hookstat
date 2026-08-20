# HS-V01-PREACTIVATION-HARDENING-12H-001

## Authority and boundary

Start from accepted `main` `89046e2ded6a795cbba166eff022e41cabf68ab4` on
`agent/v01-preactivation-hardening-001`. The accepted architecture remains:
passive evidence preferred, Codex passive per-handler history unavailable on
qualified 0.147.0, opt-in instrumented receipts admitted, normal launch
`codex`, no daemon, no launcher wrapper.

This train may fetch, branch, commit, push, create one PR, run CI, and merge
the accepted hardening candidate. It must not mutate Owner-live Codex config or
trust, publish crates, create a tag/release, commit raw commands/configuration,
prompts, tool payloads, credentials, sessions, or stream content.

## Required outcome

Advance `V01_OWNER_ACTIVATION_READY` to
`V01_OWNER_ACTIVATION_READY_HARDENED`. Owner activation remains an attended
short sequence: review dry-run, explicit apply, Codex trust review if asked,
ordinary `codex` smoke, report inspection, final release authorization.

## Work contract

1. Prove exact baseline, merged predecessor PRs, governance, prior receipts,
   and the local gate.
2. Reconcile static discovery with read-only App Server effective discovery.
   Classify handler source/scope/event/type/mode/enablement/trust/managed state
   where exposed; use explicit unsupported coverage rather than guessed support.
3. Produce only aggregate Owner census data. Shadow-copy supported Owner config
   privately; prove dry-run, apply, idempotence, drift refusal, restore, and
   byte-exact recovery; remove copies afterward.
4. Harden transparent proxy and transformation semantics: streams, exit code,
   cwd/environment, timeout/cancellation coverage, handler revision, unknown
   config fields, Windows command form, same-event handlers, and no double wrap.
5. Stress receipt spool/ledger/analytics/TUI with concurrent, malformed,
   duplicate, incomplete, exact-window, refresh, small-terminal, and actual
   production-ingest fixture paths.
6. Measure release-binary operational overhead, verify packaged temporary
   install paths, and document attended activation/rollback.
7. At settled head run format, Clippy, tests, locked build, package, publish
   dry-run, exact-head Windows/Linux CI; then merge if all non-Owner gates pass.

## Stop gate

Only a new destructive/irreversible requirement, unsafe secret handling,
history-corruption risk, a transparent-instrumentation invalidation, or an
unsolved critical privacy/correctness defect may block. Owner live activation
is explicitly not a blocker.

## Closeout

Return the compact receipt specified by the Owner request, including exact
heads, discovery/reconciliation/census, shadow and proxy evidence, durability,
analytics/TUI/E2E/performance/package gates, no-mutation/no-publication facts,
and the one attended next step.
