# HS-G35 — Runtime-Neutral IPC Broker / WAL

## Status

PLANNED after accepted G34.

## Objective

Implement the single runtime-neutral IPC evidence transport and local broker used whenever a coverage domain lacks admitted Native evidence.

The broker must remove per-invocation synchronous filesystem durability from the observed Hook path while remaining bounded, local-only, crash-recoverable, and privacy-safe.

## Platform transport

Production local transports:

```text
Windows -> Named Pipe
Unix    -> Unix Domain Socket
```

Both implement one runtime-neutral local evidence transport abstraction.

Do not expose a TCP/HTTP listener.

## Binary protocol

Define a small, versioned, bounded binary protocol. JSON is not admitted as the high-frequency IPC wire format.

Frames conceptually include:

```text
magic
protocol version
frame type
flags/runtime identifier
runtime instance reference
invocation reference
handler reference
timestamp
```

Lifecycle frames include at minimum:

```text
START
COMPLETE(status, exit-class, duration)
```

Static handler metadata should be registered/catalogued separately instead of repeated in every high-frequency frame where practical.

Frame sizes and string/reference lengths must have hard upper bounds.

## Broker responsibilities

The broker may only:

```text
receive
validate
enqueue
append WAL
acknowledge
batch/group durability
recover
idle-expire
```

It must not perform:

```text
runtime-specific identity inference
Native admission
evidence authority selection
analytics
TUI rendering
trust decisions
Codex configuration mutation
```

## Lifecycle

The broker is:

```text
per-user
local-only
on-demand
idle-expiring
restartable
```

It is not a mandatory global daemon or network service.

A client encountering no broker may use a bounded race-safe startup/handoff mechanism. Concurrent startup attempts must converge on one usable endpoint without corrupting evidence.

## Queueing and backpressure

- queues are bounded;
- frame payloads are bounded;
- overload must not cause unbounded memory growth;
- overload/drop behavior must be explicit, counted, and surfaced as coverage degradation;
- HookStat must prefer dropping observational evidence over blocking the observed Hook for an unbounded interval.

The exact bounded send/ack policy is governed by the G28 performance budget.

## Compact append WAL

Replace the v0.3 production hot-path pattern of per-invocation JSON files plus synchronous journal durability with a compact append-oriented WAL.

Requirements:

- append-only accepted record framing;
- truncated-tail detection/recovery;
- checks sufficient to reject malformed/partial frames;
- bounded replay;
- idempotent ingestion into canonical evidence/ledger;
- no per-record `fsync`/`sync_data` on the Hook path;
- batch/group durability policy justified by G28 measurements;
- restart must not fabricate terminal success.

A tiny final power-loss window may be accepted only if it is explicitly documented as evidence loss/coverage risk and the performance/correctness tradeoff is justified.

## Concurrency

Exercise:

```text
single client
16 concurrent clients
100 concurrent synthetic clients
10,000 frames
100,000 frames
```

The purpose is bounded correctness and tail-latency confidence, not a marketing throughput number.

## Security / local trust boundary

- endpoint is user-local;
- no network exposure;
- reject oversized/malformed frames;
- reject path redirection/symlink/reparse abuse where applicable to state/WAL paths;
- no executable or raw Hook payload crosses the protocol;
- state-root ownership/permissions follow existing HookStat user-local safety expectations.

## Required tests

- version/magic mismatch rejection;
- maximum frame bounds;
- concurrent producer correctness;
- broker startup race;
- idle expiry and clean restart;
- broker crash during append;
- truncated WAL tail;
- duplicate replay idempotence;
- overload remains bounded and visible;
- no per-record synchronous durability in the producer path;
- Windows Named Pipe integration;
- Unix Domain Socket integration on supported CI;
- privacy fixture proves banned raw fields are absent.

## Risk vector

```text
CODE_CHANGED=true
ARCHITECTURE_CHANGED=true
PERSISTENCE_CHANGED=true
LOCAL_PROCESS_MODEL_CHANGED=true
NETWORK_SURFACE_ADDED=false
SECURITY_OR_PRIVACY_CHANGED=true
RELEASE_BOUNDARY=false
```

Independent persistence and local-IPC safety review is required.

## Acceptance

```text
IPC_PROTOCOL_VERSIONED=true
IPC_PROTOCOL_RUNTIME_NEUTRAL=true
IPC_FRAME_BOUNDED=true

WINDOWS_NAMED_PIPE=PASS
UNIX_DOMAIN_SOCKET=PASS
NETWORK_LISTENER=false

BROKER_RUNTIME_NEUTRAL=true
BROKER_NO_ANALYTICS=true
BROKER_NO_TRUST_AUTHORITY=true

WAL_APPEND_ONLY=true
WAL_CRASH_RECOVERY=PASS
TRUNCATED_TAIL_RECOVERY=PASS
REPLAY_IDEMPOTENT=true

MULTI_CLIENT=PASS
10K_FRAMES=PASS
100K_FRAMES=PASS
UNBOUNDED_QUEUE=false
PER_RECORD_FSYNC=false
DROPPED_EVIDENCE_VISIBLE=true
GLOBAL_MANDATORY_DAEMON=false

PERFORMANCE_BUDGET=PASS
SECURITY_REVIEW=PASS
CODE_CI=PASS
```

## Estimated effort

**9–14 effective engineering hours.**

## Next

`HS-G36 — Ultra-Light IPC Clients / Transparent Shim`.
