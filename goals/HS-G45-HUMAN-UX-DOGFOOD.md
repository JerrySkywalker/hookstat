# HS-G45 — Human UX & Codex `/hooks` A/B Dogfood

## Objective

Prove on an owned Windows Terminal environment that HookStat v0.4 is at least as understandable as the pinned Codex `/hooks` surface for current hook information, while adding useful reliability intelligence without creating contradictory or machine-oriented presentation.

## Preconditions

```text
G42=PASS
G43=PASS
G44=PASS_OR_TRUTHFULLY_UPSTREAM_UNAVAILABLE
```

## A/B reference

Use the exact Codex `/hooks` source/version baseline pinned by G40. Record the current runtime version used during dogfood.

Do not compare HookStat against screenshots from a different unrecorded Codex version and call that parity proof.

## Required owner scenarios

At minimum exercise:

- wide ZenBook Duo / Windows Terminal layout;
- narrower terminal layout;
- zh-CN;
- en-US;
- event list scrolling;
- handler list scrolling;
- long command;
- long matcher;
- long source;
- Command handler;
- MCP Tool handler;
- Prompt handler when available/fixture-backed;
- Agent handler when available/fixture-backed;
- managed handler;
- needs-review handler;
- disabled handler;
- installed but unobserved handler;
- partial coverage;
- zero terminal samples;
- historical-only hook;
- runtime warnings/errors;
- current/new event with unavailable reliability semantics.

Synthetic fixtures may cover states not safely available in the owner environment, but the final information hierarchy and current real catalog must receive an owned visual/interaction check.

## Primary acceptance question

> If the user only opens HookStat, do they still need to open Codex `/hooks` to learn what a current hook is, where it came from, how it is configured, and how Codex currently sees its state?

Required answer:

```text
NO
```

If the answer is yes because HookStat omits a runtime field, G45 fails.

## Reliability acceptance questions

The owner must also be able to answer without decoding internal values:

- When was this hook last observed?
- Does it have terminal samples in the selected scope?
- What scope does the shown failure rate use?
- Why is coverage limited?
- Why is the risk category what it is?
- Is this current revision data or all historical revisions?
- Is this hook installed now or historical only?

Raw Unix milliseconds or unexplained sample-count contradictions fail the Human contract.

## Interaction acceptance

Verify:

- direct top-level navigation remains intact;
- Events -> Handlers -> Detail navigation is predictable;
- Esc returns one local level;
- Help/footer hints match available actions;
- selection remains stable across non-destructive refresh where possible;
- refresh clearly indicates loading/error without erasing accepted data misleadingly;
- write actions, if admitted, clearly reflect managed/review constraints;
- no crash/layout corruption during resize.

## Privacy check

Owner-visible command/source/matcher text is allowed in the local interactive TUI under the v0.4 ephemeral presentation contract.

Verify those raw values do not appear in:

- HookStat SQLite;
- normal reliability receipts;
- diagnostics export;
- committed dogfood receipts/screenshots containing owner-private values unless explicitly sanitized.

Dogfood evidence committed to the repository must be sanitized.

## Receipt

Record machine-readable outcome such as:

```text
CODEX_BASELINE=
HOOKSTAT_HEAD=
WINDOWS_TERMINAL=true
WIDE_LAYOUT=PASS
NARROW_LAYOUT=PASS
ZH_CN=PASS
EN_US=PASS
CODEX_HOOKS_INFORMATION_PARITY=PASS
HUMAN_TIME=PASS
METRIC_SCOPE_CLARITY=PASS
COVERAGE_EXPLANATION=PASS
RISK_EXPLANATION=PASS
INSTALLED_UNOBSERVED=PASS
HISTORICAL_DISTINCTION=PASS
SAFE_WRITE_UX=PASS|UPSTREAM_UNAVAILABLE
RAW_PRESENTATION_PERSISTENCE=0
OWNER_DISPOSITION=PASS
```

## Acceptance

```text
OWNER_CODEX_HOOKS_AB_DOGFOOD=PASS
CODEX_HOOKS_INFORMATION_PARITY=PASS
HUMAN_RUNTIME_TRUTH=PASS
HUMAN_RELIABILITY_INTELLIGENCE=PASS
RAW_UNIX_MS_VISIBLE=false
UNEXPLAINED_METRIC_SCOPE_MISMATCH=false
PRIVACY=PASS
```

## Next

G46R v0.4 release closeout.
