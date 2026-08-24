# HS-G35 runtime-neutral IPC broker and WAL

## Scope and boundary

G35 adds the second and only fallback evidence transport: local IPC. It owns
receive, validation, bounded enqueue, append-WAL, grouped durability, recovery,
acknowledgement, idle expiry, and bounded health counters. It stops at
`CanonicalEvidence` and the G29 correlator ingress.

It does not infer runtime identity, derive handler semantics, choose Native
authority, calculate analytics, render a UI, execute a Hook, manage trust, or
mutate a runtime configuration. In particular, this Goal does not add the G36
cooperative client, transparent shim, handler capsule, or a `hookstat codex`
launcher.

`Native` remains `Qualified` but not `Admitted` for the G34 Codex L1 result.
Nothing in this broker changes that fact. An authority table continues to select
`Ipc` for a non-admitted coverage domain before replay enters G29.

## Protocol v1

The high-frequency wire is binary only. Each frame has this fixed envelope:

| Bytes | Field | Rule |
| ---: | --- | --- |
| 0–3 | magic | `HSIP` exactly |
| 4 | protocol version | `1` exactly |
| 5 | frame type | START, COMPLETE, or broker ACK |
| 6–7 | flags | must be zero |
| 8–9 | payload length | must match exactly and keep the full frame at or below 1024 bytes |

START and COMPLETE share bounded opaque values for runtime, runtime instance,
invocation, handler, event, source scope, optional revision, and timestamp.
Every variable field has a one-byte length and a 128-byte maximum, is ASCII
identifier grammar only, and rejects path separators. COMPLETE additionally
contains a closed terminal-status enum, closed exit-classification enum,
optional bounded numeric exit value, and duration. ACK is a closed control enum
and never enters the WAL.

Unknown version/type/enum, invalid flags, malformed length, incomplete frame,
invalid UTF-8/reference, and trailing payload are rejected deterministically.
No JSON is parsed on this wire.

The schema intentionally contains no prompt, assistant text, tool input/output,
stdin, stdout, stderr, Hook command, credential, token, secret, or raw source
path field. The same binary frame is the only durable WAL payload.

## Local transport and startup

`LocalEndpoint` hashes the user-local HookStat state root and current account
binding to form an opaque collision-resistant endpoint identifier. Clients have
no path-bearing protocol field and cannot choose a WAL or endpoint target.

| Platform | Transport | Local safety rule |
| --- | --- | --- |
| Windows | `interprocess` generic namespaced local socket, implemented as a Named Pipe | owner-rights protected DACL (`GR`/`GW` only); no TCP address or network listener |
| Unix | filesystem Unix Domain Socket under `<state-root>/ipc` | root and `ipc` directory reject symlink/non-directory/unsafe permission objects; socket mode is `0600` |

The Unix endpoint is not reclaimed through the transport library's unsafe
automatic overwrite mode. G35 validates the secure state directory, probes a
candidate stale socket, and removes only a dead socket object in that directory.
Windows reparse points exposed by standard metadata are rejected. State paths,
WAL files, and startup leases reject symlinks and unexpected object types.

`BrokerStartup` uses a state-root-local `create_new` lease. The elected starter
retains the lease until the endpoint is connectable; competing producers probe
at a bounded interval and never create a second broker. A stale lease is
eligible for bounded safe takeover only inside the verified IPC directory.

## Backpressure and lifecycle

The WAL queue and connection queues are fixed-capacity `sync_channel`s. A
full WAL queue replies `DroppedOverloaded`; an unavailable/failed append replies
`Rejected`; an acknowledgement that misses the producer's deadline replies
`Busy`. None means successful evidence. All are counted in `BrokerHealth`,
including queue high-water mark, malformed input, group flushes, and explicit
drops.

The broker has a bounded idle timeout. On expiry it stops accepting, drains the
bounded append queue, requests final durability, joins the one durability
worker, and exits. It is per-user/on-demand and is neither a machine-global
service nor a listener on TCP, HTTP, or any network interface.

## WAL and recovery

`ipc-evidence-v1.wal` is append-only. Each record is:

```text
HSWL | WAL-version=1 | reserved=0 | binary-frame-length | SHA-256-prefix checksum | HSIP frame
```

The WAL has a 64 MiB hard cap. Group durability requests are cut at a maximum
of 64 records, 64 KiB, or 50 ms, plus an explicit clean-shutdown request. A
producer ACK follows a successful complete WAL append to the operating system
file buffer. One logical append worker is the only record writer and preserves
strict append order. One separately owned durability worker is the only caller
of `sync_data()`; the append worker never waits for a group sync before ACKing
the current record or dequeuing a subsequent record. The ACK does not claim
power-loss durability, and there is deliberately no per-record fsync.

Durability scheduling uses constant-size shared state rather than one message
per record. A request advances a monotonically increasing append generation.
If sync is already active, later due generations coalesce to the newest target;
the single durability worker serially completes the active target and then the
newest requested target. Record/byte-triggered requests have a fixed 2 ms
coalescing window capped by the existing 50 ms interval deadline; interval and
shutdown requests are immediate. There is no overlapping `sync_data()`, unbounded
message queue, or per-record thread creation. The append worker's 2 ms bounded
receive poll continues to evaluate the 50 ms trigger, so one low-traffic record
cannot wait indefinitely for a later frame.

The durability worker reopens the already validated WAL with append permission
(required by Windows `FlushFileBuffers`) and verifies that both handles identify
the same file using the cross-platform `same-file` handle identity API. On
Windows that identity contains volume serial plus file index; on Unix it uses
device plus inode. This provides a distinct open-file handle while the ordered WAL owner
remains the only writer. The durability handle never writes or seeks, so it
cannot reorder or interleave records. Exact Windows and Linux tests exercise
independent-handle sync plus WAL recovery. See the primary platform contracts
for [`same-file`](https://github.com/BurntSushi/same-file),
[`FlushFileBuffers`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers),
[`fsync`](https://pubs.opengroup.org/onlinepubs/009695399/functions/fsync.html).

A sync failure is published through a small append/failure ordering gate. An
append that owns the gate first may complete and return its truthful OS-buffer
`Accepted`; once failure publication owns the gate, later queued frames are
rejected and the broker stops accepting. Already returned ACKs are never
rewritten. Clean shutdown stops acceptance, drains queued appends, schedules
the final pending generation, waits for its completion, joins the durability
worker, and then exits.

This creates a bounded final power-loss window of **possible observational
evidence loss**. It never means a Hook succeeded, and a missing COMPLETE stays
missing/incomplete.

Recovery scans in append order. Valid records replay deterministically. A
truncated final record is discarded by truncating only that unvalidated tail.
Bad magic/version/checksum/frame in a non-tail record fails closed; it is never
converted to evidence. Replay normalizes frames to runtime-neutral
`CanonicalEvidence`, then calls the accepted G29 core. G29 supplies lifecycle
idempotence; an external runtime identity resolver remains responsible for the
later `HookInvocation`/ledger boundary.

## Review checklist

- Protocol parser is allocation-bounded before payload allocation and rejects
  unowned/trailing bytes.
- WAL never serializes a runtime command or content field, has no producer-side
  direct SQLite operation, and does not call per-record `sync_data`.
- One ordered append owner and one non-overlapping durability owner preserve
  framing/order while keeping current and subsequent ACK paths free of sync.
- Durability requests are bounded/coalescing; low traffic, clean shutdown, and
  post-ACK failure visibility have deterministic tests.
- Queue overload is observable and does not manufacture an ACK.
- Windows uses an owner-only Named Pipe DACL; Unix uses a `0600` socket under a
  verified private state directory. Neither path exposes a network listener.
- Runtime names remain opaque data; the broker contains no Codex wire/config,
  trust, App Server, matcher, or source-path semantics.
- Existing v0.3 receipt history is neither read, rewritten, nor deleted by G35.
