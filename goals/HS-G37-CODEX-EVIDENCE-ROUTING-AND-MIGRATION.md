# HS-G37 — Codex Evidence Routing / Native L2 / Migration

## Status

IN PROGRESS as a Draft train after G36 merged to `main` at
`d8d00e3da3e24a91cd9405c14d297a12ce33eb23`.

The first checkpoint retains the G36 source router, records Windows Codex
0.149.0 Native L2 as `UPSTREAM_UNAVAILABLE`, and routes each domain to an
admitted cooperative IPC integration or truthfully to `NOT_ADMITTED`. No
transparent-shim production activation is in scope.

## Objective

Make the new two-path evidence architecture authoritative for real Codex production, qualify ordinary-CLI Native attachment if upstream permits it, and migrate v0.3 instrumentation safely without rewriting historical truth.

## Source router

Authority selection operates by coverage domain, not by a runtime-wide boolean.

Minimum conceptual domain:

```text
runtime + event family + source class
```

Routing rule:

```text
if Native is Admitted for the domain:
    authority = Native
else if an IPC integration is Admitted for the domain:
    authority = IPC
else:
    authority = NOT_ADMITTED
```

The user should not have to maintain two independent evidence-source configurations.

`NOT_ADMITTED` is a coverage state and is never serialized as an evidence path.
The only `EvidenceTransport` values remain Native and IPC. Missing or
non-admitted evidence cannot become success, and shadow evidence never enters a
production denominator.

## One-authority rule

For any production domain exactly one source contributes to runs/failure denominators.

During qualification a second source may run in shadow mode, but:

```text
SHADOW_EVIDENCE_IN_DENOMINATOR=false
```

Do not rely on fuzzy after-the-fact deduplication to fix dual-authority ingestion.

## Codex Native L2 qualification

Attempt to prove a supported external-observer path for an ordinary user-launched:

```text
codex
```

Required success condition:

```text
user launches normal codex
HookStat independently receives authoritative lifecycle evidence
same real Hook executions correlate correctly
no HookStat launcher/wrapper/PTY host is introduced
```

If current Codex upstream does not expose a supported attach path, record truthfully:

```text
CODEX_NATIVE_L2=UPSTREAM_UNAVAILABLE
```

and use IPC as production authority only for a domain with an admitted IPC
integration. Otherwise route the domain to `NOT_ADMITTED`.

This is not a v0.3.1 release blocker.

## IPC production integration

For domains without admitted Native evidence, migrate from the v0.3 full proxy
only when G36 supplies an admitted IPC integration for that domain. In v0.3.1
that means cooperative IPC. The transparent shim is
`QUALIFIED_NOT_ADMITTED_PERFORMANCE` and must not be selected, installed, or
described as production fallback merely because Native is unavailable.

Requirements:

- normal daily command remains `codex`;
- cooperative integrations avoid wrapping where proven and use the versioned
  integration-owned HSIP v1 boundary;
- a domain without admitted Native or cooperative IPC remains truthfully
  `NOT_ADMITTED`;
- trust is never inferred or bypassed merely because HookStat owns instrumentation;
- restore remains exact and drift-aware;
- unsupported/managed/plugin sources remain explicit coverage limitations rather than optimistic mutation targets.

## Shadow qualification

Use shadow collection to compare authoritative candidates before cutover where technically possible.

Compare at minimum:

```text
invocation count
handler attribution
revision attribution
terminal outcome
duration semantics
coverage
```

A mismatch blocks authority promotion until explained.

## Legacy v0.3 evidence

Preserve existing v0.3 state:

```text
legacy receipt files
legacy ledger rows
historical incomplete rows
historical failure rows
aliases
revision history
```

Do not delete or recategorize old incomplete evidence merely because the new architecture prevents future occurrences.

Legacy v1 receipt files may become read-only compatibility input. New evidence should carry a new source/protocol generation sufficient to distinguish Native/IPC v0.3.1 records from legacy v1 proxy records.

## Ownership/currentness coordination

Resolve or explicitly model the current cross-tool ownership problem in which wrapping a TabBeacon command causes TabBeacon to see the effective declaration as modified/third-party.

Preferred result for cooperative tools:

```text
no wrapper declaration
cooperative IPC evidence
original tool ownership remains directly provable
```

For transparent third-party instrumentation, preserve provenance sufficient to distinguish:

```text
original handler owner/definition
HookStat instrumentation owner
effective revision
```

without persisting private raw commands in the ledger or public diagnostics.

## Human identity

Ensure instrumentation does not degrade Human display identity to `Hookstat Exe` when safer original metadata/alias information is available. Stable internal key remains authoritative; Human labels remain presentation metadata.

## Required tests

- domain-level Native authority;
- domain-level admitted IPC fallback;
- domain-level `NOT_ADMITTED` when neither integration is admitted;
- mixed Native/IPC domains in one Codex runtime;
- shadow source never changes production denominator;
- `NOT_ADMITTED` never enters a denominator and never becomes success;
- Native/IPC mismatch blocks promotion;
- ordinary Codex Native success path if upstream available;
- upstream-unavailable Native L2 uses admitted IPC or degrades truthfully to
  `NOT_ADMITTED`;
- v0.3 upgrade preserves legacy evidence counts/history;
- old incomplete evidence is not silently rewritten;
- new IPC evidence uses new generation/source identity;
- exact restore after v0.3.1 instrumentation;
- trust boundary remains explicit;
- cooperative TabBeacon path preserves ownership/currentness;
- Human identity regression is prevented.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=true
PERSISTENCE_CHANGED=true
CODEX_INTEGRATION_CHANGED=true
USER_PERSISTENT_CONFIG_CHANGED=true
EVIDENCE_AUTHORITY_CHANGED=true
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

Independent migration/authority review is required.

## Acceptance

```text
CODEX_SOURCE_ROUTER=PASS
ONE_AUTHORITY_PER_DOMAIN=PASS
SHADOW_DOUBLE_COUNT=0
SHADOW_MISMATCH_GATE=PASS

ORDINARY_CODEX_NATIVE_ATTACH=PASS|UPSTREAM_UNAVAILABLE
HOOKSTAT_AS_CODEX_LAUNCHER=false
NATIVE_UNAVAILABLE_ADMITTED_IPC_OR_NOT_ADMITTED=PASS
NOT_ADMITTED_ROUTER_STATE=PASS
NOT_ADMITTED_IS_EVIDENCE_PATH=false
SHADOW_EVIDENCE_IN_DENOMINATOR=false
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true

TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
COOPERATIVE_IPC_ADMISSION=ADMITTED

LEGACY_V1_DATA_PRESERVED=true
HISTORICAL_INCOMPLETE_REWRITTEN=false
V031_EVIDENCE_GENERATION_DISTINCT=true

NORMAL_CODEX_LAUNCH=codex
TRUST_BYPASS=false
EXACT_RESTORE=PASS

TABBEACON_COOPERATIVE_PATH_PROVEN=true
OWNERSHIP_PROVENANCE=PASS
HUMAN_IDENTITY_REGRESSION=false

MIGRATION_REVIEW=PASS
CODE_CI=PASS
```

## Estimated effort

**8–12 effective engineering hours.**

## Next

`HS-G38 — Performance & Windows Dogfood Hardening`.
