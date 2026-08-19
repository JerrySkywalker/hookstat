# HS-G04 — Reliability Analytics

Add per-handler 24h/7d/30d/All runs, failed runs, failure rate, terminal-state breakdown, previous-window delta, and recent failures. Add p50/p95/p99 duration only when the admitted source proves duration.

Every percentage is rendered/serialized with sample counts. Policy/control outcomes such as blocked/stopped are not automatically classified as execution failure. Aggregation tests use deterministic fixtures with known expected totals.
