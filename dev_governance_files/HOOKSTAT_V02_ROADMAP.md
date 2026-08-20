# HookStat v0.2 Development Roadmap

## Release Theme

# HookStat v0.2 — Reliability Center

From Hook Tracker to Agent Reliability Control Center.

## 1. Release Vision

HookStat v0.1 established the reliability observation loop:

```
Codex runtime
    ↓
Hook instrumentation
    ↓
Reliability receipt
    ↓
SQLite ledger
    ↓
Analytics
    ↓
TUI report
```

v0.2 focuses on operational usability rather than expanding runtime support.
The primary goal is to make HookStat a mature terminal control center aligned with the TabBeacon terminal UI system.

Core objectives:

- adopt shared Jerry terminal UI design conventions;
- improve human readability;
- introduce bilingual support;
- improve diagnostics;
- improve reliability interpretation;
- prepare for future multi-runtime support.

---

# 2. Non Goals

The following are explicitly out of scope for v0.2:

- OpenCode runtime adapter;
- DeepSeek Harness adapter;
- Claude Code adapter;
- Web dashboard;
- Cloud synchronization;
- AI-generated root cause analysis;
- Remote telemetry.

---

# 3. Design Principles

## Shared Terminal UI System

HookStat must align with TabBeacon.

Future Jerry terminal tools should share:

- layout system;
- theme system;
- colors;
- typography;
- navigation;
- dialogs;
- localization;
- refresh behavior;
- async rendering model.

## Identity Separation

Internal handler identity and human display identity must be separated.

Example:

```
Internal:
hk_d46e169e621930c2

Display:
TabBeacon Stop Notification
```

Internal identity remains stable for storage and verification.

---

# 4. Development Goals

## HS-V02-G00 — Terminal UI Foundation

Create shared UI infrastructure compatible with TabBeacon.

Required shared components:

- theme;
- layout;
- widgets;
- navigation;
- dialogs;
- i18n.

Acceptance:

```
TUI_FRAMEWORK_SHARED=true
THEME_SYSTEM_SHARED=true
LAYOUT_SYSTEM_SHARED=true
NAVIGATION_MODEL_SHARED=true
```

---

## HS-V02-G01 — HookStat Reliability Center TUI

Replace the current data viewer with a control center.

Target layout:

```
Application Title
-----------------
Navigation | Main Content
-----------------
Shortcut Bar
```

Pages:

### Overview

Show:

- runtime;
- coverage;
- total runs;
- failure rate;
- health state;
- highest risk hooks.

### Hooks

Human-readable hook list:

- name;
- event;
- failure rate;
- trend;
- risk.

### Hook Detail

Show:

- display name;
- internal identity;
- runtime;
- event;
- statistics;
- trends;
- recent failures.

### Diagnostics

Show:

- installation status;
- Codex detection;
- trust status;
- instrumentation status;
- receipt storage;
- coverage.

---

## HS-V02-G02 — Human-readable Reliability Model

Add display metadata:

- display_name;
- description;
- category;
- source label.

Name resolution priority:

1. user annotation;
2. HookStat metadata;
3. script filename;
4. command basename;
5. event fallback.

Internal identifiers must not be the primary user-facing display.

---

## HS-V02-G03 — Internationalization

Implement bilingual UI.

Supported locales:

- zh-CN;
- en-US.

All UI strings must use localization keys.

Support runtime language switching and persistent preference.

---

## HS-V02-G04 — Diagnostics and Operational UX

Add:

```
hookstat doctor
```

Checks:

- binary;
- Codex installation;
- hooks configuration;
- trust;
- instrumentation;
- receipt storage;
- SQLite health;
- coverage.

Add:

```
hookstat diagnostics export
```

Export must exclude:

- prompts;
- tool payloads;
- credentials;
- raw hook output.

---

## HS-V02-G05 — Reliability Intelligence

Improve interpretation of reliability data.

Add:

### Trend analysis

- 7 day trend;
- 30 day trend;
- regression detection.

### Risk score

Consider:

- failure rate;
- sample count;
- trend;
- impact.

Avoid ranking only by percentage.

### Failure fingerprint

Group failures by:

- exit code;
- platform;
- shell;
- error category.

### Revision comparison

Support:

```
Current revision
Previous revision
```

Requires:

- handler revision hash;
- configuration timeline.

---

## HS-V02-G06 — Release Candidate

Required:

```
TUI_SYSTEM_PASS
I18N_PASS
DISPLAY_IDENTITY_PASS
DIAGNOSTICS_PASS
REGRESSION_PASS
WINDOWS_PASS
LINUX_PASS
```

---

# 5. Final v0.2 Acceptance

HookStat v0.2 should provide:

- TabBeacon-compatible terminal experience;
- Chinese/English switching;
- human-readable hook names;
- reliability dashboard;
- diagnostics center;
- improved hook detail analysis;
- trend analysis;
- failure understanding.

---

# Future Direction

After v0.2:

v0.3 should focus on multi-runtime support:

- Codex;
- OpenCode;
- DeepSeek Harness;
- Claude Code.

The runtime-neutral reliability model from v0.1 should be preserved.
