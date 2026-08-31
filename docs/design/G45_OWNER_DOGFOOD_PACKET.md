# G45 Owner Dogfood Packet — Hooks Control Center

## Historical status

The first Owner visual pass using this packet failed and is now historical evidence.

```text
G45_OWNER_FIRST_PASS=FAIL
TESTED_MAIN=c24139842e35f83368db00dbf56d9025817d4a9e
FINDING=G45-OV-001
EVENT_CATALOG_SEMANTIC_DUPLICATES=true
KNOWN_EVENT_LOCALIZATION_DEFECT=true
G46R=HOLD
```

See:

- `docs/qualification/G45_OWNER_VISUAL_FINDING_001.md`;
- `dev_governance_files/ROADMAP_V040.md`;
- `goals/HS-G45R-OWNER-REDOGFOOD.md`.

This first-pass packet is retained so the failed Human gate is not rewritten as if it succeeded. After G45V-A/B/C are accepted, use the dedicated **G45R Owner Re-Dogfood** contract rather than declaring this original pass successful retroactively.

The original packet contained no Owner hook values, screenshots, commands, matchers, source paths, prompts, or tool payloads.

## Original first-pass baseline

```text
CODEX_BASELINE=rust-v0.151.0 / 78c290807ce710180111df227df3b7a4fe845452
PRODUCT_BASELINE_BEFORE_G45_PREP=cf5be21c134fbfa83b9c6a0adf5cc68800d8e4dd
G45_PREP_MERGE_MAIN=c24139842e35f83368db00dbf56d9025817d4a9e
BUILD_COMMAND=cargo build --locked
LAUNCH_EN_US=.\target\debug\hookstat.exe tui --lang en-US
LAUNCH_ZH_CN=.\target\debug\hookstat.exe tui --lang zh-CN
SAFE_WRITE_UX=UPSTREAM_UNAVAILABLE
OWNER_CONFIGURATION_MUTATION=NOT_AUTHORIZED
```

## Original setup

1. Build exact accepted main in a clean worktree.
2. Open Windows Terminal at a wide viewport and launch HookStat in en-US.
3. Repeat at narrow geometry and in zh-CN.
4. Press `r` on Hooks and wait for the current-runtime catalog to settle.

The first attempt initially exposed a stale historical local checkout; that was corrected by building a clean detached worktree at current main. The actual release-blocking finding described below was then reproduced on the correct current-main build.

## Original Codex `/hooks` A/B sequence

1. Record `codex --version` and compare against the pinned baseline.
2. Launch normal Codex, open `/hooks`, and inspect representative current hooks without mutating them.
3. Launch HookStat from exact accepted main, refresh Hooks, and compare current event/handler/detail information hierarchy.
4. Do not retain private screenshots or committed raw runtime values.

## First-pass failure

The Event catalog displayed duplicate Human rows for the same semantic Codex events. Synthetic zero-handler rows and real `hooks/list` rows were both visible because they were keyed by distinct raw event strings that later localized to the same Human label.

The zh-CN pass also exposed known event descriptions leaking English text.

Therefore the original acceptance sequence stopped early. Continuing the remaining perception checks would not have changed the release disposition.

```text
G45_OWNER_VISUAL_CHECK=FAIL
CORRECTION_TRAIN=G45V
```

## Correction and re-dogfood path

```text
G45V-A Runtime Event Identity & Localization Repair
  ↓
G45V-B TUI Visual Regression CI Foundation
  ↓
G45V-C Real-Wire End-to-End Visual Matrix
  ↓
G45R Owner Re-Dogfood
```

The future re-dogfood must use exact accepted main after G45V-C. It must confirm that duplicate Event rows and known-event localization leakage are gone before re-running the broader A/B questions.

## Original Human acceptance questions retained for G45R

1. Do I still need Codex `/hooks` for basic current-hook information?
2. Is source, command, and matcher understandable?
3. Is current versus history obvious?
4. Are metrics understandable without decoding?
5. Is narrow layout usable?

Required future answers remain:

```text
DO_I_STILL_NEED_CODEX_HOOKS_FOR_BASIC_INFO=NO
SOURCE_COMMAND_MATCHER_UNDERSTANDABLE=YES
CURRENT_VS_HISTORY_OBVIOUS=YES
METRICS_UNDERSTANDABLE=YES
NARROW_LAYOUT_USABLE=YES
```

Do not self-certify these answers before the post-G45V Owner re-dogfood.
