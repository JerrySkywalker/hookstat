# HS-V02-G02 — Human-readable Hook Identity

## Status

PLANNED after accepted G01.

## Objective

Separate stable internal handler authority from Human display identity so HookStat shows useful names without changing database attribution, deduplication, trust, proxy routing, or revision semantics.

## Scope

- Implement the identity split in `HUMAN_READABLE_HOOK_IDENTITY.md`.
- Resolve names in strict priority: user annotation, admitted explicit metadata, script filename, command basename, event fallback.
- Introduce a privacy-safe display-identity projection for TUI, Human reports, and diagnostics.
- Add an additive/versioned HookStat-owned schema or preference migration for sanitized catalog candidates and user annotations.
- Preserve existing invocation rows and `handler_key`/`handler_revision` authority.
- Keep v0.1 ledgers readable and migration idempotent/interruption safe.
- Version any machine-report schema change and keep stable internal keys explicit.
- Test duplicate display names, revision changes, retired keys, Windows/Unix command parsing, and privacy reduction.

## Non-goals

- Do not rename, merge, or deduplicate handlers based on display text.
- Do not persist full commands, arguments, paths, matchers, prompts, payloads, stdout/stderr, tokens, or private backups.
- Do not edit Codex `hooks.json`, trust, instrumentation manifests, receipts, or proxy routing.
- Do not infer explicit metadata that the runtime adapter has not qualified.
- Do not implement reliability intelligence or a second runtime.

## Acceptance criteria

```text
INTERNAL_IDENTITY_UNCHANGED=true
DISPLAY_IDENTITY_SEPARATE=true
NAME_RESOLUTION_PRIORITY=annotation|metadata|script|command|event
USER_ANNOTATION_LOCAL_ONLY=true
DISPLAY_NAME_USED_FOR_DEDUP=false
DISPLAY_NAME_USED_FOR_TRUST=false
REVISION_COMPARISON_KEY=handler_key+handler_revision
FULL_COMMAND_PERSISTED=false
FULL_PATH_PERSISTED=false
V01_LEDGER_MIGRATION=PASS
MIGRATION_IDEMPOTENT=true
REPORT_SCHEMA_VERSIONED=true
DUPLICATE_DISPLAY_NAMES=PASS
PRIVACY_GATE=PASS
```

## Dependencies

- Accepted `HS-V02-G01-RELIABILITY-CENTER-TUI`
- `docs/design/HUMAN_READABLE_HOOK_IDENTITY.md`
- Existing `HandlerIdentity`, ledger, discovery, receipt, manifest, and trust tests

## Next

`HS-V02-G03 — Internationalization`.
