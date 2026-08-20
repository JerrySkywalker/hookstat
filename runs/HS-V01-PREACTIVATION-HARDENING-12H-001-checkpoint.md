# HS-V01-PREACTIVATION-HARDENING-12H-001 checkpoint

## Scope and non-mutation boundary

This train starts from `89046e2ded6a795cbba166eff022e41cabf68ab4` on
`agent/v01-preactivation-hardening-001`. The accepted opt-in instrumented
receipt architecture remains in force; passive evidence remains preferred.

No Owner-live Codex configuration or trust state was modified. No raw hook
command, prompt, tool argument, session content, stream payload, credential,
or complete Owner configuration was committed. Temporary shadow copies and
synthetic fixtures were removed after their checks.

## Read-only Codex census and discovery reconciliation

The installed runtime reported `codex-cli 0.147.0`. Static supported-layer
discovery found twelve user `hooks.json` command handlers. A short-lived,
read-only App Server `hooks/list` discovery found sixteen effective handlers:
twelve supported user command handlers and four visible plugin handlers that
are explicitly unsupported for HookStat mutation. All visible handlers were
enabled and trusted. The runtime surface does not expose execution mode for
these effective handlers, so it is represented as unknown rather than guessed.

The twelve static handlers reconciled with the twelve effective supported
handlers using in-memory, privacy-preserving location identities. The four
effective-only plugin handlers remain explicit unsupported coverage, not a
claim of unobserved healthy execution.

## Shadow rehearsal and fixture evidence

- A private byte-for-byte shadow of the supported Owner configuration passed
  dry-run, apply, repeat apply/idempotence, ordinary-drift refusal, restore,
  and exact prestate recovery. The original live configuration was only read.
- Transformer tests preserve root/group/handler unknown fields plus
  `commandWindows`, async, timeout, status message, additional-context limit,
  matcher, ordering, and distinct same-event identities.
- Proxy integration tests prove inherited stdin, stdout, stderr, exit code
  (including Windows exit 259), working directory, and environment behavior;
  large streams and telemetry failure remain fail-open. Controlled process-tree
  cancellation leaves explicit incomplete coverage.
- Receipt tests cover 64 concurrent writers, duplicate and out-of-order
  records, temporary-file interruption, malformed inputs, reingest, and
  start-without-completion. Incomplete observations are never terminal success
  or failure.
- The real CLI preactivation fixture exercises apply, proxy receipts, SQLite
  ingest, JSON report, and exact restore with known counts.

## Operational measurements

On the Windows release binary with a harmless synthetic handler, warm proxy
execution averaged 68.67 ms; the bare handler shell averaged 28.10 ms and
HookStat process startup 20.89 ms. Making telemetry storage unavailable
averaged 58.74 ms, so the receipt write component was about 9.93 ms in that
sample. Initial report ingest processed 49 records in 458.7 ms (106.8 records
per second). The frozen render/data path averaged 121.43 ms over ten release
invocations; interactive terminal initialization remains an attended-console
operation.

## Local qualification at checkpoint

```text
cargo test --test proxy_e2e --locked  PASS (7 tests)
prior full local gate                  PASS
release build                          PASS
```

The final package, install smoke, full local gate, exact-head hosted Windows
and Linux CI, and merge verification are performed after this durable
checkpoint.
