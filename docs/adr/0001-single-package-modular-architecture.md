# ADR 0001 — Single-package modular architecture

Status: Accepted

HookStat starts as one publishable Rust package instead of a multi-crate workspace. v0.1 needs one runtime and one product binary; splitting crates before a second runtime would add release/build/governance cost without user value.

Internal module boundaries still separate domain, runtime adapters, evidence ingestion, storage, analytics, CLI and TUI. Revisit workspace extraction when DeepSeek Harness is implemented and real reuse pressure is measurable.
