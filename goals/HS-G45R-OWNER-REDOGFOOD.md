# HS-G45R — Owner Re-Dogfood After Visual-CI Correction

## Objective

Repeat the owned Windows Terminal / Codex `/hooks` A/B acceptance only after G45V-A/B/C have been accepted, and prove the first G45 visual failure is closed on exact accepted main.

## Preconditions

```text
G45V_A=PASS
G45V_B=PASS
G45V_C=PASS
G45_OWNER_FIRST_PASS=FAIL_HISTORICAL
G46R=HOLD
```

Do not overwrite or erase the first failure receipt. It remains evidence that the visual CI correction train was necessary.

## Exact tested baseline

Before dogfood record:

```text
HOOKSTAT_MAIN=<exact accepted SHA>
CODEX_VERSION=<actual version>
CODEX_SOURCE_PIN_STATUS=MATCHES_V0151|REQUALIFIED|DRIFT_REQUIRES_REVIEW
```

If Codex has advanced materially beyond the G40/G45V protocol pin, perform a bounded read-only parity requalification before calling the A/B result valid.

## Build isolation

Use a clean exact-main worktree or equivalent clean checkout.

Do not build from a stale historical development branch.

Record:

```text
WORKTREE_CLEAN=true
BUILT_SHA=<exact>
HOOKSTAT_BINARY_VERSION=<version>
```

Do not destructively clean unrelated Owner worktrees.

## Owner visual scenarios

At minimum check:

### Events

- known events appear once;
- installed/active/review counts are truthful;
- `Interrupt` appears once;
- known event names/descriptions are localized in zh-CN;
- unknown future event remains visible when fixture-backed/live-visible;
- runtime warnings/errors are not shown as reliability failures.

### Handlers

- installed-but-unobserved handlers remain visible;
- current handler state precedes reliability information;
- source/type/mode/trust/review/managed state is understandable;
- historical-only handlers do not masquerade as current.

### Detail

Verify section order:

```text
Runtime Configuration
Reliability Summary
Observation History
Advanced Intelligence / Technical Metadata
```

Confirm long command/matcher/source are usable at wide and narrow widths.

### Human reliability

Confirm:

- no raw Unix milliseconds;
- zero terminal samples are not presented as healthy 0.00%;
- metric scope is explicit;
- coverage explanation is understandable;
- risk category/reason is understandable;
- current versus historical revision scope is clear.

### Interaction

Verify:

- Up/Down and j/k;
- Enter descends one local level;
- Esc returns one local level;
- Help/footer actions match actual capability;
- explicit runtime refresh works;
- period switching does not rediscover runtime catalog;
- selection survives non-destructive refresh where identity remains.

## G44 write boundary

Do not mutate Owner hooks/configuration.

Expected v0.4 behavior remains:

```text
READ_PARITY=PASS
WRITE_PARITY=UPSTREAM_UNAVAILABLE
```

The TUI should explain read-only management truthfully and expose no misleading write control.

## Primary acceptance questions

Owner records short answers:

1. Do I still need Codex `/hooks` for basic current-hook information?
2. Is source, command, and matcher understandable?
3. Is current versus history obvious?
4. Are metrics understandable without decoding machine-oriented values?
5. Is narrow layout usable?

Required answers:

```text
DO_I_STILL_NEED_CODEX_HOOKS_FOR_BASIC_INFO=NO
SOURCE_COMMAND_MATCHER_UNDERSTANDABLE=YES
CURRENT_VS_HISTORY_OBVIOUS=YES
METRICS_UNDERSTANDABLE=YES
NARROW_LAYOUT_USABLE=YES
```

## Owner receipt

Use a compact receipt such as:

```text
G45_OWNER_REDOGFOOD=PASS|FAIL
TESTED_MAIN=
CODEX_VERSION=
WIDE_LAYOUT=
NARROW_LAYOUT=
ZH_CN=
EN_US=
EVENTS_PAGE=
HANDLERS_PAGE=
HOOK_DETAIL_PAGE=
EVENT_DUPLICATES=
KNOWN_EVENT_LOCALIZATION=
CODEX_HOOKS_INFORMATION_PARITY=
RUNTIME_TRUTH_FIRST=
INSTALLED_UNOBSERVED=
HISTORICAL_CURRENT_DISTINCTION=
LONG_COMMAND=
LONG_MATCHER=
LONG_SOURCE=
HUMAN_TIME=
ZERO_SAMPLE_PRESENTATION=
METRIC_SCOPE_CLARITY=
COVERAGE_EXPLANATION=
RISK_EXPLANATION=
NAVIGATION=
REFRESH_BEHAVIOR=
SAFE_WRITE_UX=
FINDINGS=
```

Do not commit Owner-private screenshots or raw hook values.

## Acceptance

```text
G45_OWNER_REDOGFOOD=PASS
EVENT_DISPLAY_IDENTITY_DUPLICATES=0
KNOWN_EVENT_LOCALIZATION=PASS
CODEX_HOOKS_INFORMATION_PARITY=PASS
HUMAN_RUNTIME_TRUTH=PASS
HUMAN_RELIABILITY_INTELLIGENCE=PASS
PRIVACY=PASS
OWNER_CONFIGURATION_MUTATIONS=0
```

## Next

Only after this goal passes may HS-G46R begin.
