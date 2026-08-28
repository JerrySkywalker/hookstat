# HS-G38 diagnostics and hardening foundation

## Status

Draft foundation. This document does not claim Owner Windows dogfood, G38
acceptance, release readiness, or authority to start G38R.

## Authority remains unchanged

Production authority remains a per-domain decision:

```text
Native admitted                 -> Native
else cooperative IPC admitted   -> IPC
else                             -> NOT_ADMITTED
```

`NOT_ADMITTED` is coverage truth, not a transport. The only evidence
transports remain Native and IPC. The transparent shim remains
`QUALIFIED_NOT_ADMITTED_PERFORMANCE` and inactive.

## Read-only diagnostics contract

Diagnostics schema v2 adds:

- Codex runtime and pinned Native L2 admission status;
- the admission and selected authority for each supplied bounded coverage
  domain, with a truthful `NOT_ADMITTED` default when no integration table is
  supplied;
- the HookStat cooperative-IPC substrate state, plus an explicit
  `NOT_ADMITTED` default IPC authority until a named coverage domain is
  governed, and transparent-shim admission/activation status;
- broker state, accepted/rejected/dropped/malformed counts, bounded queue
  state, grouped-durability counters, WAL flush lag, and recent latency
  percentiles.

The broker snapshot is carried by request/response frame types on the existing
local HSIP control plane. It is not evidence and is not a third reliability
transport:

```text
DIAGNOSTIC_CONTROL_FRAME_IS_LIFECYCLE=false
DIAGNOSTIC_CONTROL_FRAME_ENTERS_WAL=false
DIAGNOSTIC_CONTROL_FRAME_ENTERS_LEDGER=false
DIAGNOSTIC_CONTROL_FRAME_ENTERS_DENOMINATOR=false
```

The response contains fixed numeric fields only. It has no runtime instance,
handler, command, path, prompt, payload, stdout/stderr, or credential field.
The wire remains bounded by the existing 1,024-byte frame limit. Recent broker
service latency uses a fixed 128-sample in-memory window; queue-wait p95 uses
the same bound. Both reset on broker restart. Diagnostics can allocate and sort
only while serving an explicit query, never on the producer record path.

If the state or endpoint is absent, diagnostics reports `absent` or
`unavailable` without creating it. An unsafe IPC state object reports
`unsafe_state`. A broker query has a bounded 20-ms diagnostic-only deadline,
does not change the accepted evidence count, and is never recovered from the
WAL.

## Controlled hardening evidence

All foundation tests use disposable state roots. The controlled concurrency
matrix sends 10,000 unique lifecycle events across 1-, 5-, and 10-client
stages. The recovery fixtures prove:

- a stopped broker cannot acknowledge a frame from an already-connected
  client;
- restart replays the accepted prefix and a replacement client reconnects;
- recovery produces no duplicate canonical invocation;
- a truncated final WAL tail is counted and removed without losing valid
  records;
- retained records for a non-admitted domain stay visible as coverage
  degradation and produce no denominator evidence.

Structural guards retain the 1,024-byte frame, 16,384-record maximum queue,
128-connection maximum, 64-MiB WAL, no JSON/ledger/CLI dependency in the
producer module, no producer `sync_data`, zero shadow denominator
contribution, and transparent-shim production lockout.

## Remaining acceptance boundary

The Draft foundation is synthetic and controlled evidence only, so it cannot
by itself accept G38C. Remaining G38C proof includes recovery/resource
behavior, bounded diagnostics, privacy/security review, exact-head CI, and a
normal-Codex non-interference smoke where no trust or configuration mutation
is needed. G38B owns reference-producer performance; that separate proof
converges with G38C only at G38D. A named external producer is not a G38C
release prerequisite: any domain without an admitted Native or IPC producer
remains `NOT_ADMITTED`. No live Owner Codex configuration was read for private
content or mutated by this checkpoint.
