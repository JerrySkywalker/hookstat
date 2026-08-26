# HS-G37 Evidence Migration and Shadow Gate

## Authority remains unchanged

G37 keeps exactly two evidence transports: `Native` and `Ipc`.
`NotAdmitted` is a coverage/authority result, not a third transport. For every
coverage domain the production rule is fixed before observing a candidate:

```text
Native admitted -> Native
else admitted IPC integration -> IPC
else -> NOT_ADMITTED
```

The v0.3.1 admitted IPC integration is cooperative HSIP v1. The retained
transparent shim remains `QUALIFIED_NOT_ADMITTED_PERFORMANCE` and has no
production activation route.

## Additive legacy migration

Ledger schema v4 adds one non-null `evidence_generation` column. Its default is
`legacy_v03_proxy`, so every row written before G37 remains unambiguously
historical without changing its source key, source record ID, terminal state,
coverage, handler key, revision, label, timestamp, duration, or error taxonomy.

New runtime integrations assign one of:

```text
v031_native
v031_cooperative_ipc
```

The migration is forward-only and idempotent. A second open neither adds a
second column nor changes a row generation. Aliases remain in the additive
annotations table and revision history continues to be derived from the exact
preserved row timeline. Legacy receipt files remain valid read-only input and
continue to produce `legacy_v03_proxy` rows.

Rows with an uninterpretable legacy taxonomy are preserved unchanged and
counted by a sanitized `invalid_legacy_taxonomy` migration issue. The issue
contains only a count, never the malformed value or private runtime content.

## Shadow comparison and promotion

Shadow observations are bounded in-memory values with no ledger-ingress API.
The fixed comparator evaluates the same coverage domain and invocation key for:

- presence and count;
- handler attribution;
- revision attribution;
- terminal outcome;
- duration semantics;
- source and invocation coverage.

It returns exactly `MATCH`, `MISMATCH`, or `INSUFFICIENT_EVIDENCE`. Only
`MATCH` is eligible for a later explicit promotion decision. Duplicate,
production-only, candidate-only, handler, revision, terminal, duration, or
coverage disagreement blocks promotion. The comparator performs no fuzzy
deduplication and cannot alter runs, failures, failure rate, or denominator.

## Ownership and Human identity

Cooperative observation does not replace the tool that owns the original
handler. The privacy-bounded provenance model records separately:

```text
original_handler_owner
original_definition_identity
hookstat_observation_integration
effective_revision
```

There is no command or path field. The TabBeacon cooperative model therefore
keeps TabBeacon ownership/currentness directly provable while HookStat remains
only the HSIP v1 observation integration.

Human display priority remains user alias, safe original metadata, safe
runtime label, then a truthful fallback. Transport process labels such as
`Hookstat Exe` and `hookstat-hook` cannot overwrite better original metadata.
Stable handler keys and legacy revision labels remain authoritative and
unchanged.

## Restore and privacy

G37 adds no live configuration mutation. Disposable migration fixtures prove
byte-exact restore, idempotence, and drift refusal for the legacy compatibility
path. Apply still requires a separate trust review; it never grants trust.

Canonical evidence, generation metadata, shadow values, provenance, migration
issues, and tests contain no prompt, tool payload, stdin, stdout, stderr, raw
command, credential, or Owner path.
