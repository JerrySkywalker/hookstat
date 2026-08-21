# Human-readable Hook Identity

Status: v0.2 design. No migration or runtime behavior is changed by this document.

## Problem

HookStat v0.1 has a trustworthy internal handler identity but not a true Human identity. `HandlerIdentity` contains `key`, `revision`, source/event/structural fields, and a `label`. Codex discovery currently generates labels such as `Codex / Stop / <opaque suffix>`, and `handler_label` is copied into every ledger invocation. The TUI sometimes bypasses even that label and renders the event plus the internal key suffix.

The stable key/revision machinery is correct for attribution and safety. It must not become the primary Human label, and a friendly label must never replace it as an authority.

## Separation contract

### Internal identity

Internal identity is machine authority.

Conceptually:

```text
InternalHandlerIdentity {
  runtime
  handler_key
  handler_revision
  event
  source_kind
  matcher_identity
  structural_identity
  execution_mode
}
```

It is used for:

- database keys and queries;
- invocation attribution and deduplication;
- manifest lookup and transparent proxy routing;
- exact trust/current-hash review;
- revision comparison and configuration timelines;
- stable selection across refresh;
- machine JSON identifiers and diagnostic correlation.

Internal identity is not localized and is not derived from a display name. Renaming a display identity must not change invocation attribution, revision, trust status, or deduplication.

### Display identity

Display identity is Human presentation.

Conceptually:

```text
HookDisplayIdentity {
  display_name
  description?
  category?
  source_label
  resolved_by
}
```

It is used for:

- TUI headings and hook lists;
- Human CLI reports;
- Human diagnostics and exports;
- search and filter labels;
- accessibility/help text.

The internal key remains available as secondary metadata in Hook Detail and machine output. Display names are not required to be unique; the UI disambiguates duplicates with event/source metadata and retains the internal reference invisibly for selection.

## Name resolution priority

Resolution is deterministic and uses the first non-empty, admitted candidate:

### 1. User annotation

A user-local HookStat annotation explicitly bound to `(runtime, handler_key)`.

- Highest priority.
- Stored only in HookStat-owned user state.
- Does not edit Codex `hooks.json`, Hook trust, proxy manifests, receipts, or the repository.
- Survives a handler revision change because it describes the stable handler location/identity.
- Must be length-bounded, control-character sanitized, and treated as literal user text, not a locale key.

### 2. Explicit metadata

An admitted Human name supplied explicitly by HookStat metadata or a runtime field whose semantics have been qualified.

- Do not assume Codex currently supplies such a field; v0.1 evidence does not prove one.
- Runtime adapters must declare which field is safe and what identity it binds to.
- Full commands, matchers, paths, hashes, or arbitrary payload text are not explicit display metadata.

### 3. Script filename

The filename of a safely parsed script target, without its directory and with a known script extension removed.

Examples:

```text
C:\tools\tabbeacon-stop.ps1 -> Tabbeacon Stop
/opt/hooks/notify.sh         -> Notify
```

Rules:

- Use platform-correct command parsing; do not split naively on spaces.
- Choose the effective platform command (`commandWindows` on Windows when present) under the same qualified semantics used by instrumentation.
- Persist or expose only the sanitized basename, never the parent path.
- A basename containing a probable secret/token shape is rejected and resolution continues.
- Shell wrappers such as `pwsh -File`, `powershell -File`, `bash`, `sh`, `python`, and `node` may identify a following script argument only through a tested parser.

### 4. Command basename

The sanitized basename of the executable when no script target is safely resolved.

Examples:

```text
notify-agent.exe -> Notify Agent
hookstat         -> Hookstat
```

Shell names alone (`cmd`, `powershell`, `pwsh`, `bash`, `sh`) are weak candidates and should normally fall through unless no safer event fallback exists.

### 5. Event fallback

A localized event-derived label such as `Stop hook` or `Permission request hook`.

- This is always available for supported events.
- The stored resolution material is the stable `HookEvent`, not an English rendered string.
- Duplicate event fallbacks are disambiguated in the UI with safe source metadata and, only when needed, a short internal suffix in `Metadata` style.

## Resolution output

The resolver should return both presentation and provenance:

```text
ResolvedDisplayIdentity {
  name: DisplayName::Literal | DisplayName::EventFallback(HookEvent)
  source: UserAnnotation | ExplicitMetadata | ScriptFilename |
          CommandBasename | EventFallback
  description?
  category?
  source_label
  resolver_version
}
```

Literal names are rendered unchanged after display sanitization. Event fallback is localized at render time. `source_label` is a safe Human classification such as `User hooks`, `Project hooks`, `Plugin`, or `Managed`; it is not a private filesystem path.

## Schema design for v0.2

No schema change is made in this foundation train. G02 should introduce an additive, reversible HookStat-owned migration after exact tests are written.

### Preserve invocation authority

`hook_invocations` retains:

- `handler_key` as the stable handler reference;
- `handler_revision` as the revision reference;
- event/source/structural identity needed to interpret historical records;
- its existing primary key `(source_key, source_record_id)`.

The current `handler_label` column remains readable for v0.1 database compatibility. After migration it is legacy presentation evidence, not the source of truth for a resolved display identity. Do not rewrite historical invocation rows merely to rename a handler.

### Add handler catalog

Recommended additive table:

```sql
CREATE TABLE handler_catalog (
    runtime TEXT NOT NULL,
    handler_key TEXT NOT NULL,
    latest_revision TEXT NOT NULL,
    explicit_name TEXT,
    script_filename TEXT,
    command_basename TEXT,
    source_label_key TEXT NOT NULL,
    resolver_version INTEGER NOT NULL,
    observed_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (runtime, handler_key)
);
```

Constraints:

- `explicit_name`, `script_filename`, and `command_basename` contain only bounded sanitized display candidates.
- No raw command, matcher text, full path, prompt, tool payload, stdout/stderr, token, credential, or private backup material is stored.
- Updating `latest_revision` never changes `handler_key`.
- `source_label_key` is a stable locale-neutral key/classification.

### Add user annotations

Recommended separate table or ownership-safe user preference store:

```sql
CREATE TABLE handler_annotations (
    runtime TEXT NOT NULL,
    handler_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT,
    category TEXT,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (runtime, handler_key)
);
```

The implementation goal must decide whether annotations belong in the main SQLite database or a separate HookStat-owned preference document. The acceptance criteria are the same: explicit writes only, no Codex mutation, no display identity in dedup/trust, atomicity, drift safety where applicable, and private-data review.

### Report schema evolution

A future machine report version should expose both fields explicitly:

```text
handler {
  key
  revision
  ...internal stable fields...
  display_identity {
    display_name
    description?
    category?
    source_label
    resolved_by
  }
}
```

Machine consumers must continue to key by `handler.key`, not `display_name`. Report schema versioning and backward compatibility are acceptance requirements of G02; this document does not silently change schema version 1.

## Revision and lifecycle rules

- User annotations bind to stable key and normally survive revision changes.
- Explicit metadata/script/command candidates are re-observed per latest revision and resolver version.
- Historical invocation rows retain the revision recorded at invocation time.
- Revision comparison groups by stable handler key and compares revision values; it never groups by display name.
- If an internal key is retired and a new key appears, no automatic identity merge occurs from matching names alone.
- Two handlers with the same display name remain separate rows unless the internal identity model proves equivalence.

## Privacy and safety

The resolver is an information-reduction boundary.

Allowed:

- bounded Human annotation;
- qualified explicit Human metadata;
- sanitized filename/basename without a directory;
- stable source classification;
- event fallback;
- opaque internal key in detail/machine output.

Prohibited:

- full command or arguments;
- absolute/relative parent paths;
- environment assignments;
- matcher contents;
- prompts or tool payloads;
- stdout/stderr or raw hook output;
- credentials/tokens/private keys;
- exact backup/restore configuration material.

Search operates on display name, safe source label, event, and optionally the internal key. It never searches raw commands or private runtime content.

## Acceptance requirements for G02

```text
INTERNAL_IDENTITY_UNCHANGED=true
DISPLAY_IDENTITY_SEPARATE=true
NAME_PRIORITY=annotation|metadata|script|command|event
DISPLAY_NAME_UNIQUE_REQUIRED=false
ANNOTATION_CHANGES_DEDUP=false
ANNOTATION_CHANGES_TRUST=false
FULL_COMMAND_PERSISTED=false
FULL_PATH_PERSISTED=false
HISTORICAL_ROWS_REWRITTEN=false
V01_LEDGER_MIGRATION=PASS
REPORT_SCHEMA_VERSIONED=true
PRIVACY_GATE=PASS
```
