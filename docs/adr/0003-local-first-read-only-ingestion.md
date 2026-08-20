# ADR 0003 — Local-first, read-only runtime ingestion

Status: Superseded by ADR 0004 for the explicitly authorized instrumented fallback

HookStat's default product mode is retrospective analytics. Users should be able to use their coding agent normally and open HookStat later. Durable local evidence is therefore preferred over a mandatory live attachment.

v0.1 must not require a launcher wrapper, daemon, or trust mutation. HookStat
may maintain its own local SQLite ledger. Runtime-native passive sources are
read-only. ADR 0004 defines the only exception: explicit opt-in per-handler
instrumentation with safe apply/restore and, when explicitly authorized, a
separate exact-target official trust action; neither is an implicit unattended
live-owner change.

If durable Codex evidence cannot support a trustworthy per-handler denominator and terminal result, HS-G01 stops for an owner data-source decision rather than silently widening the architecture.
