# HS-V01-LIVE-ACTIVATION-DOGFOOD-RELEASE-12H-001 checkpoint

## Bounded live activation record

The accepted start commit was
`76294634ac3ace6f86879d901961b66a33a46785`; the release-train branch is
`agent/v01-live-activation-release-001`. The installed runtime reported
`codex-cli 0.147.0`.

Current discovery reconciled twelve supported, unmanaged user `hooks.json`
command handlers with the supported static set. Effective runtime discovery
also exposed four plugin handlers. Those plugin handlers are unsupported for
mutation and remained unchanged throughout this train.

HookStat created and verified its normal backup and journal before applying
instrumentation. The release candidate applied exactly the twelve supported
handlers, without double wrapping. The post-apply inspection found only the
expected HookStat-managed command fields changed, while the four plugin
handlers and unrelated configuration remained unchanged.

Codex marked the changed supported handlers as requiring trust review. The
only supported CLI review surface identified for that state was the interactive
`/hooks` flow. No safe interactive execution surface was available to complete
that official review, and this train did not use a bypass or alter trust data.
HookStat therefore restored the live configuration using its tested restore
path. The resulting supported configuration exactly matched the captured
prestate. The Owner Codex configuration is left restored off, not partially
instrumented.

## Release blockers

Real ordinary-Codex dogfood was not run because the required scoped trust
review could not be completed safely. The release is also blocked by a
separate process-cleanup defect: a controlled Windows fixture that cancelled
only the HookStat proxy allowed the valid original handler to complete. The
existing full-tree cancellation fixture passes only when it explicitly kills
the entire tree, so it does not close that gap. Several bounded lifecycle
implementations were rejected and removed because they could not prove
correct cleanup without risking healthy handler semantics.

Consequently G06 and G07 are not closed, and no PR was created or merged. No
crate was published, tag created, or GitHub Release made. No raw hook command,
backup, prompt, tool payload, stream content, credential, or Owner data is
present in this receipt or in the committed train changes.

## Completed non-publication gates

The settled candidate passed formatting, Clippy with warnings denied, all
locked tests, locked build, package verification, and `cargo publish --dry-run`.
Package creation verified the source package successfully; no public registry
upload was attempted.

## Next recovery goal

Implement and prove safe platform-specific proxy child-tree cancellation for
the direct proxy-cancellation path, then execute the official scoped Codex
`/hooks` trust review through an available supported interactive surface before
repeating live dogfood and release gates.
