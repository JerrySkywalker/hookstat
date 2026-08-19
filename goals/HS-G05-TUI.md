# HS-G05 — TUI

Implement `docs/design/TUI_SPEC.md` using Ratatui or a documented equivalent. Preserve the frozen information hierarchy/style. v0.1 renders only supported Codex data; no fake DeepSeek Harness/OpenCode placeholder rows.

Required tests: deterministic render buffers for normal, empty, partial-coverage, ingest-error and constrained-width states; sample counts remain visible with rates; detail view respects unavailable latency/revision fields.
