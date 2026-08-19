# HS-G03 — Persistent Reliability Ledger

Add HookStat-owned local SQLite persistence with schema/versioning, ingest-source cursors, deduplication and incremental refresh.

Required proof: repeated scan does not duplicate rows; new evidence increments exactly once; malformed evidence cannot corrupt accepted history; live Codex files remain read-only; default durable record contains only reliability-relevant metadata.
