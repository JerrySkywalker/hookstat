# HookStat Roadmap

## Active execution overlay — v0.3.1

Current v0.3.1 development is governed by [`ROADMAP_V031.md`](ROADMAP_V031.md) and the corresponding `goals/HS-G38A*` through `goals/HS-G38R*` contracts.

The v0.3.1 release train is deliberately **single-repository**: HookStat must be able to implement, qualify, harden, package, and prepare the release candidate without writing to another product repository. Runtime-specific cooperative producers are independent integration tracks. An external producer may become admitted after satisfying HookStat's HSIP conformance/admission contract, but no external repository, producer package, merge, or release is a prerequisite for HookStat v0.3.1.

Where an ordinary runtime domain has neither admitted Native evidence nor an admitted external IPC integration, HookStat reports that domain truthfully as `NOT_ADMITTED`; it does not manufacture coverage and it does not make another repository a release dependency.

The historical version sections below remain product-history context. When they conflict with the active v0.3.1 execution overlay, `ROADMAP_V031.md` is authoritative for the current train.

## v0.1 — Codex historical reliability

Critical path:

```text
HS-B00 Repository Foundation
  ↓
HS-G01 Codex Evidence Qualification (passive preferred; opt-in instrumented only by owner decision)
  ↓
HS-G02 Codex Vertical Slice
  ↓
HS-G03 Persistent Reliability Ledger
  ↓
HS-G04 Reliability Analytics
  ↓
HS-G05 TUI
  ↓
HS-G06 Real Codex Dogfood & Hardening
  ↓
HS-G07 Usable v0.1 Release Candidate
```

### HS-B00 — Repository Foundation

Rust skeleton, pinned toolchain, CI, governance, architecture/ADRs, normative TUI spec. No runtime integration claim.

### HS-G01 — Codex Evidence Qualification

Empirically determine which local Codex evidence surfaces can durably recover per-handler invocations and terminal outcomes. Candidate sources include rollout/session files, archived sessions, local diagnostic/state stores, App Server live events, and OTel only as evidence warrants. App Server is not presumed to be the primary source.

Hard stop: if no passive durable source can identify individual handlers plus
invocation/result, return `BLOCKED_DATA_SOURCE_DECISION_REQUIRED` unless the
Owner explicitly admits the separately governed opt-in instrumented receipt
architecture. Do not widen architecture automatically.

### HS-G02 — Codex Vertical Slice

Implement the smallest real path from admitted Codex evidence to canonical HookInvocation records and deterministic machine-readable aggregate output. No TUI dependency yet.

### HS-G03 — Persistent Reliability Ledger

Add incremental/idempotent local SQLite persistence, ingestion cursors, deduplication, malformed-record isolation, and data-minimization/privacy rules.

### HS-G04 — Reliability Analytics

Add 24h/7d/30d/All windows, runs, failures, failure rate, terminal-state breakdown, previous-window delta, recent failures, and latency percentiles only when source support is proven.

### HS-G05 — TUI

Implement the frozen `docs/design/TUI_SPEC.md` with Ratatui or a justified equivalent. Do not show unsupported runtimes as fake rows.

### HS-G06 — Real Codex Dogfood & Hardening

Validate sanitized fixtures plus real ordinary Codex use. When passive durable
evidence is unavailable and the Owner explicitly authorizes activation, this
means: live opt-in instrumentation, a scoped official Codex trust review if
required, normal `codex` sessions, real metadata receipts, report/TUI
validation, repeated refresh/idempotence, visible unsupported coverage, and a
proven restore. It must preserve the normal `codex` launch and never bypass
trust.

### HS-G07 — Usable v0.1 Release Candidate

Polish README/help, version to 0.1.0, validate package metadata and `cargo package`/`cargo publish --dry-run`, create a release-ready candidate. Actual crates.io publication and public GitHub Release require separate explicit Owner authorization for the named release train.

## v0.2 — Reliability depth

Handler revisions/config hashes, before/after comparison, error fingerprinting/clustering, runtime-version correlation. Implement the frozen revision-comparison section of the TUI.

## v0.3 — DeepSeek Harness

Add the second RuntimeAdapter and durable hook/invoked + hook/result ingestion. This is the first mandatory proof that the canonical model is not Codex-specific.

## v0.4 — OpenCode

Add OpenCode using the best available persistent evidence plus a minimal runtime-native bridge only if needed. Do not distort the Rust core around OpenCode's plugin model.

## Later

Optional Claude Code, daemon/watch mode, notifications, doctor/probe/repair, Web UI, OTel export, and additional runtimes.
