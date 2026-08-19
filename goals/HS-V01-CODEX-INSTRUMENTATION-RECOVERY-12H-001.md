# HS-V01-CODEX-INSTRUMENTATION-RECOVERY-12H-001

## Owner decision

`HS_G01=BLOCKED_DATA_SOURCE_DECISION_REQUIRED` is accepted. The Owner admits an
opt-in transparent per-handler Codex instrumented evidence source because
Codex 0.147.0 lacks a sufficient passive retrospective per-handler ledger.

```text
ALLOW_OPTIONAL_INSTRUMENTED_CODEX_EVIDENCE=true
PASSIVE_EVIDENCE_REMAINS_PREFERRED=true
LIVE_OWNER_CODEX_MUTATION_DURING_THIS_RUN=false
WRAPPER_AROUND_CODEX=false
DAEMON_REQUIRED=false
```

Normal launch remains `codex`; no product flow may require `hookstat codex` as
a launcher. This train may inspect Owner Codex state read-only but may not
instrument, alter trust, or otherwise modify it.

## Scope

1. Recover the accepted prior train, prove its exact head CI, merge it, and
   create a fresh durable branch.
2. Freeze evidence-source-neutral architecture distinguishing
   `PassiveEvidenceSource` and `InstrumentedEvidenceSource`.
3. Read-only discover documented active Codex command-hook configuration layers.
   Each handler needs a privacy-preserving key and revision plus source kind,
   event, matcher fingerprint, structural index, and execution mode.
4. Provide `hookstat codex instrument --dry-run`, reporting discovered,
   instrumentable, already-instrumented, unsupported coverage, and trust
   consequences without raw hook commands.
5. Provide a transparent proxy that forwards stdin/stdout/stderr/exit code
   unchanged and retains only bounded reliability metadata. Telemetry writes
   fail open. Preserve Codex command/commandWindows/async/timeout/statusMessage/
   additionalContextLimit semantics; never inspect stream contents.
6. Use atomic HookStat-owned per-invocation receipt spool files, then ingest
   into SQLite with idempotence, concurrent safety, malformed isolation, and
   explicit incomplete receipt representation.
7. Provide explicit-path fixture-proven apply/restore with exact backup, atomic
   replacement, idempotence, no double wrapping, drift refusal, rollback
   journal, and no trust automation. Unsupported inline/plugin/managed sources
   remain unsupported coverage.
8. Complete source-neutral 24h/7d/30d/All analytics, deterministic JSON, and
   the normative Ratatui/crossterm TUI.
9. Qualify sanitized fixture E2E: successful/failing/control results, timeout
   or cancellation as start-only incomplete evidence, same-event handlers,
   sync/async/concurrency, already-installed/drift/restore, telemetry failure,
   malformed/duplicate/start-only receipts, and stream/exit preservation.
10. If local and hosted release gates pass, set version 0.1.0 and run package
    plus publish dry-run. Do not publish, tag, release, or mutate Owner config.

## Stop gate

Only a new Owner-required destructive/irreversible mutation, credentials or
secrets, publication, unsafe Owner-live modification, or discovery invalidating
both passive and instrumented models may block this train. A missing isolated
Owner smoke is recorded as `OWNER_ACTIVATION_REQUIRED`, not a blocker.

## Required closure

Return the compact durable recovery receipt specified by the Owner request,
including exact branch/head, each evidence/proxy/storage/TUI qualification,
Owner mutation booleans, release gates, and concrete next goal.
