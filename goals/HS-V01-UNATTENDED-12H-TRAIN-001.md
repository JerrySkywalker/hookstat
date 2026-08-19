# HS-V01-UNATTENDED-12H-TRAIN-001 — v0.1 Overnight Development Train

## Objective

Use an unattended development window of up to roughly 12 hours to advance HookStat from the accepted HS-B00 foundation through as much of HS-G01→HS-G07 as can be truthfully proven, targeting a usable **v0.1.0 release-ready candidate** in one train.

Time budget is an execution ceiling, not permission to idle and not a reason to weaken gates. Finish early if v0.1 is genuinely complete. If a hard architectural blocker is reached, preserve evidence and stop the blocked claims rather than manufacturing success.

## Starting authority

- Repository: `JerrySkywalker/hookstat`
- Begin from fresh `origin/main` after HS-B00 is present and green.
- At runtime record exact `START_MAIN` and verify no unrelated working-tree drift.
- Create/reuse one durable remote train branch such as `agent/v01-unattended-train-001`; push checkpoints after each completed goal or major qualification result so recovery after reboot is straightforward.
- Never force-push `main` or rewrite published history.

## Product target

At successful closeout, ordinary user flow is:

```text
codex
# ...normal daily work, no HookStat wrapper...

hookstat
# TUI shows trustworthy per-handler Codex historical reliability
```

The candidate should support:

- admitted read-only retrospective Codex evidence ingestion;
- stable per-handler attribution for all rows it reports;
- incremental/idempotent HookStat-owned SQLite ledger;
- 24h/7d/30d/All counts and failure rates;
- terminal-state breakdown and recent failures;
- latency percentiles only if source evidence proves duration;
- explicit coverage semantics;
- frozen TUI home/detail style from `docs/design/TUI_SPEC.md`;
- deterministic JSON/report path useful for tests/automation;
- Windows-first owner experience plus passing Windows/Linux CI;
- release-ready Cargo package metadata at 0.1.0 if all preceding gates pass.

## Non-negotiable boundaries

DO NOT:

- require `hookstat codex` or any launcher wrapper;
- install a daemon/service as a prerequisite;
- wrap or replace existing user hooks;
- mutate `~/.codex` / `%USERPROFILE%\.codex` config, hooks, trust state, or session history;
- change unrelated TabBeacon/HAPI/ntfy/PUA configuration;
- commit raw private prompts, tool arguments, credentials, tokens, or full personal rollout transcripts;
- send runtime/session evidence over the network;
- claim unsupported OpenCode/DeepSeek Harness/Claude runtime support;
- publish crates.io, create a public GitHub Release, or push a release tag;
- collapse `Blocked`/`Stopped`/policy denial into `Failed` without Codex-semantic proof;
- show incomplete coverage as `0.00% healthy`.

## Execution strategy

### Phase A — Admit the data plane (HS-G01)

1. Read current repository governance and current Codex official/source contracts relevant to hooks.
2. Inspect the owner's actual Codex installation/version and local data surfaces **read-only**.
3. Build a source qualification matrix for active/archived rollout/session files, local Codex databases/logs actually present, and any useful App Server/OTel surfaces for comparison.
4. Use sanitized repository fixtures to exercise completed/failed/blocked/stopped and same-event multi-handler identity as evidence permits.
5. Select the narrowest durable v0.1 evidence contract that proves handler identity + invocation denominator + terminal status + timestamp.
6. Record exact coverage limitations.

**HARD STOP:** if no durable source meets the minimum, set:

```text
HS_G01=BLOCKED_DATA_SOURCE_DECISION_REQUIRED
```

Do not solve this by adding a wrapper, daemon, mandatory App Server attachment, or live mutation. In that case, spend remaining safe time improving the qualification harness, canonical model, synthetic analytics/TUI infrastructure, documentation and tests **without claiming real v0.1 Codex reliability support**, then return a blocker-focused closeout.

### Phase B — Vertical slice (HS-G02)

Once G01 is admitted, implement real Codex evidence parsing/normalization and deterministic report/JSON output. Preserve runtime/evidence-source abstractions but keep one Rust package. Add only dependencies justified by current functionality.

### Phase C — Ledger (HS-G03)

Implement HookStat-owned SQLite under the platform-appropriate user data directory. Prove cursor/dedup/idempotence and malformed-evidence isolation. Never lock or rewrite Codex-owned files.

### Phase D — Analytics (HS-G04)

Implement exact per-handler aggregates for 24h/7d/30d/All. Keep numerator/denominator definitions explicit and tested. Add latency only if the source proves it.

### Phase E — Frozen TUI (HS-G05)

Implement the normative TUI baseline. Preserve the visual hierarchy and compact style instead of redesigning it. Add deterministic buffer tests and keyboard flow. Omit unsupported latency/revision sections rather than inventing values.

### Phase F — Real owner dogfood (HS-G06)

Run read-only real Codex smoke against the owner's existing history. Cross-check a sanitized known-count fixture exactly. Test concurrent read while Codex may be running, repeated refresh, partial coverage, Windows paths, and archived data if applicable.

### Phase G — v0.1 RC closure (HS-G07)

Only if G01-G06 are accepted: version to 0.1.0, remove bootstrap `publish=false` as appropriate for a release-ready package, polish README/help, run the full settled-candidate gate, `cargo package`, and `cargo publish --dry-run`. Do not actually publish/tag/release.

## Agile / Fast Lane rules

- Work in coherent vertical increments; avoid speculative frameworks.
- Use focused tests during iteration.
- Do not run hosted CI after every small commit; push remote checkpoints, settle the candidate, then run the relevant final CI. A second hosted run is justified only if the prior CI found a real defect or the risk-relevant code changed.
- Reuse accepted evidence when the relevant diff is empty.
- Prefer fixing product/test defects over adding governance commentary.
- No generic auditor loop. Perform a dedicated final audit only for privacy/data-mutation ambiguity, unresolved evidence semantics, or release closure.
- Transient GitHub/network/registry errors may receive bounded retries with backoff; do not spin indefinitely.

## Recommended implementation shape

Remain a modular monolith for v0.1. A likely module layout is:

```text
src/
  cli.rs
  domain/
  runtime/
    codex/
  evidence/
  ingest/
  store/
  analytics/
  tui/
```

Choose current stable crates deliberately (likely CLI/serde/SQLite/TUI/platform-dir/error crates), commit `Cargo.lock`, and keep MSRV/toolchain policy consistent unless a dependency forces an explicit reviewed change.

## Goal checkpoints

After each completed goal, write/update a compact receipt under `runs/` or an equivalent ignored/generated-safe path if evidence is private; commit only sanitized durable summaries that help future recovery. Push the train branch after milestones.

Suggested statuses:

```text
HS_G01=<PASS|BLOCKED>
HS_G02=<PASS|NOT_STARTED|PARTIAL>
HS_G03=<PASS|NOT_STARTED|PARTIAL>
HS_G04=<PASS|NOT_STARTED|PARTIAL>
HS_G05=<PASS|NOT_STARTED|PARTIAL>
HS_G06=<PASS|NOT_STARTED|PARTIAL>
HS_G07=<PASS_RC|NOT_STARTED|BLOCKED>
```

## Final acceptance if v0.1 succeeds

- `hookstat` opens the frozen-style Codex reliability TUI from ordinary local historical evidence.
- At least one real Codex handler row has a trustworthy denominator/status provenance or the available real history is truthfully empty while the admitted mechanism is proven by equivalent real/sanitized evidence.
- Same-event multiple handlers cannot be conflated.
- Failure rate accompanies sample count everywhere.
- Coverage limitations are visible.
- Re-ingest is idempotent.
- Codex-owned content/config/trust is unchanged.
- No raw private session data is committed.
- full local gate PASS;
- final hosted Windows/Linux CI PASS on exact candidate;
- `cargo package` PASS;
- `cargo publish --dry-run` PASS;
- no publication/tag/release performed.

## Required final return

Return one concise durable receipt, not a long narrative:

```text
DISPOSITION=<V01_RC_READY|PARTIAL|BLOCKED_DATA_SOURCE_DECISION_REQUIRED|BLOCKED_OTHER>
RUN_ID=HS-V01-UNATTENDED-12H-TRAIN-001
START_MAIN=<sha>
FINAL_HEAD=<sha>
TRAIN_BRANCH=<branch>

HS_G01=<...>
HS_G02=<...>
HS_G03=<...>
HS_G04=<...>
HS_G05=<...>
HS_G06=<...>
HS_G07=<...>

CODEX_VERSION=<...>
PRIMARY_EVIDENCE_SOURCE=<...>
COVERAGE=<...>
HANDLER_IDENTITY_PROVEN=<true|false>
INVOCATION_DENOMINATOR_PROVEN=<true|false>
TERMINAL_STATUS_PROVEN=<true|false>

REAL_CODEX_READONLY_SMOKE=<PASS|FAIL|BLOCKED|N/A>
SQLITE_IDEMPOTENCE=<PASS|FAIL|N/A>
TUI_BASELINE=<PASS|FAIL|N/A>
LOCAL_GATE=<PASS|FAIL>
HOSTED_CI=<PASS|FAIL|BLOCKED>
CARGO_PACKAGE=<PASS|FAIL|N/A>
CARGO_PUBLISH_DRY_RUN=<PASS|FAIL|N/A>

CODEX_MUTATED=false
RAW_PRIVATE_SESSION_CONTENT_COMMITTED=false
CRATES_IO_PUBLISHED=false
RELEASE_TAG_CREATED=false
UNRELATED_DRIFT_TOUCHED=false

NEXT_GOAL=<one concrete next step or NONE_V01_RC_READY>
```
