# HookStat v0.4 Implementation Checklist

This checklist is the compact audit surface for the v0.4 Hooks Control Center train. Detailed requirements live in `dev_governance_files/ROADMAP_V040.md`, `docs/design/HOOKS_CONTROL_CENTER_SPEC.md`, and G40–G46R goal contracts.

## P0 — Runtime truth parity

- [ ] Pin exact Codex `/hooks` source/version baseline.
- [ ] Record event parity matrix.
- [ ] Record handler-field parity matrix.
- [ ] Show every runtime event returned by current catalog.
- [ ] Audit/add `Interrupt` canonical reliability semantics or mark presentation-only.
- [ ] Show Installed count.
- [ ] Show Active count.
- [ ] Show Review/needs-review count.
- [ ] Show event description.
- [ ] Show runtime hook discovery warnings/errors.
- [ ] Show enabled/disabled state.
- [ ] Show managed state.
- [ ] Show trust/review state.
- [ ] Show matcher when present.
- [ ] Show source when present.
- [ ] Show handler type.
- [ ] Show command handler text locally when runtime exposes it.
- [ ] Show MCP server/tool.
- [ ] Show Prompt/Agent handler type.
- [ ] Show Sync/Async mode.
- [ ] Show timeout.
- [ ] Show additional-context limit.

## P0 — Runtime presentation architecture

- [ ] Add ephemeral runtime presentation snapshot.
- [ ] Keep raw runtime presentation data in memory only.
- [ ] Prove no raw command/matcher/source path enters SQLite.
- [ ] Prove no raw presentation data enters receipts.
- [ ] Prove no raw presentation data enters diagnostics export.
- [ ] Prove no raw presentation data enters performance receipts.
- [ ] Keep runtime catalog loading separate from reliability period loading.
- [ ] Explicit refresh reloads runtime catalog.
- [ ] Period switch does not rediscover runtime.
- [ ] Catalog failure does not fabricate current state from history.

## P0 — Catalog/history join

- [ ] Current installed catalog is primary current truth.
- [ ] Installed + observed state.
- [ ] Installed + no history state.
- [ ] Ambiguous reliability join state.
- [ ] Unsupported join state.
- [ ] Never guess ambiguous handler history.
- [ ] Historical + not installed is excluded from current handler list.
- [ ] Historical-only state remains available in Changes/history.

## P0 — Hooks Control Center UX

- [ ] Level 1 Events page.
- [ ] Level 2 Handlers page.
- [ ] Level 3 Hook Detail page.
- [ ] Runtime configuration appears before reliability.
- [ ] Reliability summary is additive.
- [ ] Observation history follows reliability summary.
- [ ] Advanced intelligence is below primary Human information.
- [ ] Long command wraps/scrolls safely.
- [ ] Long matcher/source wraps/scrolls safely.
- [ ] Wide layout passes.
- [ ] Narrow layout passes.
- [ ] Chinese passes.
- [ ] English passes.
- [ ] Large hook list scrolling passes.

## P0 — Human reliability presentation

- [ ] No Unix milliseconds in normal TUI.
- [ ] First seen uses local Human time.
- [ ] Last observed uses local Human time + relative age.
- [ ] Latest evidence uses local Human time + relative age.
- [ ] Recent failures use Human time.
- [ ] Revision timeline uses Human time ranges.
- [ ] Failure fingerprints use Human occurrence times.
- [ ] No `0.00% healthy` with zero terminal samples.
- [ ] Terminal-sample denominator is explicit.
- [ ] Selected-window scope is explicit.
- [ ] Current-revision scope is explicit.
- [ ] All-time/history scope is explicit.
- [ ] Audit current `runs=5 / sample=0 / trend sample=227` behavior.
- [ ] Same metric + same scope produces consistent values.
- [ ] Coverage states have Human explanations.
- [ ] Risk score has Human category + explanation.
- [ ] Revision hashes are shortened in primary Human presentation.
- [ ] Full internal IDs are advanced metadata, not primary labels.

## P1 — Safe Hook management

- [ ] Audit official external enable/disable route.
- [ ] Audit official external trust route.
- [ ] Do not treat internal TUI events as public API without proof.
- [ ] Exact runtime identity precondition.
- [ ] Current-hash/stale-state protection where applicable.
- [ ] Managed hooks always read-only.
- [ ] Enable does not implicitly trust.
- [ ] Trust affects only exact eligible hooks.
- [ ] Refresh official runtime catalog after mutation.
- [ ] Verify runtime-confirmed resulting state.
- [ ] If unsupported, expose truthful `UPSTREAM_UNAVAILABLE` rather than guessing config writes.

## P1 — Diagnostics / explanations

- [ ] Runtime catalog issues remain distinct from historical execution failures.
- [ ] Health explains why degraded/limited.
- [ ] Coverage explains what evidence is missing.
- [ ] Installed but unobserved state is understandable.
- [ ] Ambiguous historical join is understandable.
- [ ] Historical-only state is understandable.

## P1 — Compatibility

- [ ] Current Codex event set qualified.
- [ ] Unknown future event remains visible.
- [ ] Unsupported handler type remains visible rather than dropped.
- [ ] Runtime capability/version drift is explicit.
- [ ] Normal `codex` launch unchanged.
- [ ] No mandatory daemon.
- [ ] No third evidence path.

## Dogfood

- [ ] Pin real Codex version used for owner A/B.
- [ ] Compare Codex `/hooks` event page with HookStat.
- [ ] Compare selected handler detail with HookStat.
- [ ] Verify user no longer needs `/hooks` for missing basic current-hook information.
- [ ] ZenBook Duo wide view approved.
- [ ] Narrow view approved.
- [ ] Chinese view approved.
- [ ] English view approved.
- [ ] Zero-sample presentation approved.
- [ ] Partial-coverage presentation approved.
- [ ] Current-vs-historical distinction approved.

## Exploration governance

- [ ] `main` remains production truth.
- [ ] Product work uses `agent/*`.
- [ ] Narrow released defects use `fix/*`.
- [ ] Exploration uses `exp/*` with no direct merge intent.
- [ ] Productization uses `promote/*` from latest main.
- [ ] `exp/* -> main` direct merge prohibited.
- [ ] DeepSeek experiment does not block v0.4.
- [ ] OpenCode experiment does not block v0.4.
- [ ] Future version numbers are not preassigned to experiments.

## Release

- [ ] G40 accepted.
- [ ] G41 accepted.
- [ ] G42 accepted.
- [ ] G43 accepted.
- [ ] G44 accepted or write parity truthfully upstream-unavailable.
- [ ] G45 owner A/B PASS.
- [ ] All code/docs settled before candidate freeze.
- [ ] Exact candidate SHA frozen.
- [ ] No post-freeze commits.
- [ ] Windows CI PASS.
- [ ] Ubuntu CI PASS.
- [ ] Independent review PASS.
- [ ] Package PASS.
- [ ] Publish dry-run PASS.
- [ ] Fresh install PASS.
- [ ] v0.3.1 -> v0.4 upgrade PASS.
- [ ] Legacy evidence/preferences preserved.
- [ ] Public publication separately Owner-authorized.
