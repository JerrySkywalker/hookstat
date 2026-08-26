# HS-G37 M1–M5 — migration, shadow, identity, restore, and local candidate

## Exact source

```text
START_MAIN=d8d00e3da3e24a91cd9405c14d297a12ce33eb23
G37_START_HEAD=fea26f818849dc27d1d134f5b9de160b51e9e069
G37_SOURCE_HEAD=4680ff7af3424a6f6647b205e2f93a8c1092cee4
G37_SOURCE_TREE=11612cd1b9facf39bb8f317bd8f92b6df038ab49
G37_MODE=ACCEPTANCE_CANDIDATE
```

This receipt is documentation-only relative to `G37_SOURCE_HEAD`. Exact-head
CI and independent review must use the later commit containing this receipt.

## M1 — additive legacy migration

```text
LEDGER_SCHEMA=4
LEGACY_V03_GENERATION=legacy_v03_proxy
V031_NATIVE_GENERATION=v031_native
V031_COOPERATIVE_IPC_GENERATION=v031_cooperative_ipc
LEGACY_V1_DATA_PRESERVED=true
HISTORICAL_INCOMPLETE_REWRITTEN=false
HISTORICAL_FAILURE_REWRITTEN=false
ALIASES_PRESERVED=true
REVISION_HISTORY_PRESERVED=true
READ_ONLY_UNMIGRATED_LEDGER=PASS
MIGRATION_IDEMPOTENCE=PASS
MALFORMED_LEGACY_ISOLATION=PASS
```

The deterministic disposable fixtures cover empty v0.3 state, completed,
failed, incomplete/start-only, aliases, adjacent revisions, and mixed legacy +
v0.3.1 evidence. The migration adds a generation column with a legacy default;
it does not rewrite the preserved row taxonomy. A sanitized issue count records
uninterpretable legacy taxonomy without copying the malformed value.

## M2 — shadow mismatch gate

```text
SHADOW_COMPARISON=MATCH|MISMATCH|INSUFFICIENT_EVIDENCE
SHADOW_DOUBLE_COUNT=0
SHADOW_EVIDENCE_IN_DENOMINATOR=false
MATCH_PROMOTION_DECISION=ELIGIBLE
MISMATCH_PROMOTION_DECISION=BLOCKED
INSUFFICIENT_PROMOTION_DECISION=BLOCKED
```

The fixed comparator covers invocation presence/count, handler, revision,
terminal result, duration semantics, and coverage. Duplicate, shadow-only,
production-only, or semantic disagreement blocks promotion. Shadow values have
no ledger-ingress API.

## M3 — ownership and Human identity

```text
OWNERSHIP_PROVENANCE=PASS
TABBEACON_COOPERATIVE_PATH_PROVEN=true
ORIGINAL_HANDLER_OWNER_PRESERVED=true
HOOKSTAT_OBSERVATION_INTEGRATION_ONLY=true
HUMAN_IDENTITY_REGRESSION=false
RAW_COMMAND_OR_PATH_FIELD=false
```

Provenance separates original owner, original definition identity, observation
integration, and effective revision. User alias and safe original metadata win
over transport process labels. `Hookstat Exe` and `hookstat-hook` cannot replace
better Human identity.

## M4 — restore and integration safety

```text
EXACT_RESTORE=PASS
DRIFT_AWARE_RESTORE=PASS
TRUST_BYPASS=false
NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NOT_ADMITTED_IS_EVIDENCE_PATH=false
NO_THIRD_EVIDENCE_PATH=true
```

Restore proof used only disposable configuration and state roots. Apply still
requires explicit trust review and does not grant trust. The G36 transparent
shim remains packaged only as a correctness-qualified, non-admitted internal
executable.

## M5 — final local gates

```text
FMT=PASS
CLIPPY_ALL_TARGETS_ALL_FEATURES=PASS
TESTS_ALL_FEATURES=PASS
TESTS_PASSED=311
TESTS_FAILED=0
TESTS_IGNORED_BY_EXPLICIT_CONTRACT=6
BUILD_LOCKED=PASS

G37_AUTHORITY_ROUTING=3/3 PASS
G37_MIGRATION_SHADOW_IDENTITY_RESTORE=6/6 PASS
CODEX_NATIVE_L1_REGRESSION=8/8 PASS
RUNTIME_NEUTRAL_ROUTER_REGRESSION=16/16 PASS
G36_COOPERATIVE_IPC_REGRESSION=8/8 PASS
TRANSPARENT_SHIM_LOCKOUT=4/4 PASS
TRANSPARENT_SHIM_CORRECTNESS=23/23 PASS
```

Ignored tests remain the predeclared installed-App-Server, explicit scale, and
release-performance qualifications. No ignored test is represented as newly
executed evidence.

## Owner safety

```text
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
FURTHER_TRANSPARENT_BUDGET_RELAXATION=false
PUBLICATION_AUTHORIZED=false
G38_STARTED=false
G38R_STARTED=false
```
