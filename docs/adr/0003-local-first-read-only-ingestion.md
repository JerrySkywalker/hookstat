# ADR 0003 — Local-first, read-only runtime ingestion

Status: Accepted

HookStat's default product mode is retrospective analytics. Users should be able to use their coding agent normally and open HookStat later. Durable local evidence is therefore preferred over a mandatory live attachment.

v0.1 must not require a launcher wrapper, daemon, hook wrapper, or live Codex configuration/trust mutation. HookStat may maintain its own local SQLite ledger. Runtime-native sources are read-only.

If durable Codex evidence cannot support a trustworthy per-handler denominator and terminal result, HS-G01 stops for an owner data-source decision rather than silently widening the architecture.
