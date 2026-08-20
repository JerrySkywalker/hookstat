# HS-G07 — Usable v0.1 Release Candidate

Close a usable v0.1.0 candidate after G01-G06 pass. Version package to 0.1.0,
finish help/README, ensure clean first-run data directory behavior, run release
gates, `cargo package`, and `cargo publish --dry-run`. G06 evidence includes
real ordinary-Codex dogfood; when it used instrumented receipts, it must also
prove scoped trust handling (if required), visible unsupported coverage, and
exact restore.

Do not publish to crates.io, create a public release, or push a release tag
without separate explicit Owner authorization for the exact release. If the
Codex evidence contract remains partial, README/TUI must state the exact
coverage limitation.
