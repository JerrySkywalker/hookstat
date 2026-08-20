# HS-V02-G04 — Diagnostics

## Status

PLANNED after accepted G03.

## Objective

Provide a truthful, localized, read-only operational diagnosis surface for HookStat installation, Codex detection, evidence, storage, and coverage, plus a sanitized support export.

## Scope

- Add `hookstat doctor` with Human and stable machine-readable output.
- Implement the Reliability Center Diagnostics view from `HOOKSTAT_V02_VIEW_MODEL.md`.
- Define typed, stable check IDs and statuses separate from localized wording.
- Check binary/version, Codex detection/support, Hook configuration visibility, trust status, instrumentation status, receipt storage, SQLite health, and evidence coverage using already admitted safe interfaces.
- Explain warnings/failures and safe next steps without applying them.
- Add `hookstat diagnostics export` with preview-first path handling and an explicit sanitized schema.
- Exclude prompts, tool payloads, credentials, tokens, raw hook output, raw commands, arbitrary environment values, private backup material, and unnecessary full paths.
- Refresh diagnostics off the UI thread and preserve the last accepted snapshot on error.

## Non-goals

- Do not automatically repair, instrument, trust, restore, edit, delete, or disable hooks.
- Do not bypass Codex review/trust mechanisms.
- Do not expose raw App Server messages, `hooks.json`, proxy manifests, receipt payloads, or ledger rows in the export.
- Do not add network upload/telemetry or a daemon.
- Do not turn unsupported/owner-required state into a failure or success claim.

## Acceptance criteria

```text
HOOKSTAT_DOCTOR=PASS
DIAGNOSTICS_TUI=PASS
DIAGNOSTIC_IDS_LOCALE_NEUTRAL=true
DIAGNOSTICS_REFRESH_READ_ONLY=true
AUTO_REPAIR=false
AUTO_INSTRUMENT=false
AUTO_TRUST=false
DIAGNOSTICS_EXPORT=PASS
EXPORT_PREVIEW_FIRST=true
PROMPTS_EXPORTED=false
TOOL_PAYLOADS_EXPORTED=false
CREDENTIALS_EXPORTED=false
RAW_HOOK_OUTPUT_EXPORTED=false
RAW_COMMANDS_EXPORTED=false
PRIVATE_BACKUPS_EXPORTED=false
ZH_CN_DIAGNOSTICS=PASS
EN_US_DIAGNOSTICS=PASS
PRIVACY_GATE=PASS
```

## Dependencies

- Accepted `HS-V02-G03-I18N`
- Existing read-only Codex discovery and ledger/receipt status boundaries
- `docs/design/HOOKSTAT_V02_VIEW_MODEL.md`
- `dev_governance_files/EVIDENCE_CONTRACT.md`

## Next

`HS-V02-G05 — Reliability Intelligence`.
