# HS-G28 Windows hot-path methodology

## Scope and safety

`hookstat-perf` is a feature-gated developer laboratory. It creates one
unique disposable fixture root containing a synthetic metadata-only manifest.
It never discovers, reads, enables, trusts, changes, or restores the Owner's
Codex configuration. The fixture handler neither reads stdin nor writes output.
Its temporary command is never copied into the committed receipt.

The output receipt is schema-checked JSON. It contains only platform/toolchain
labels, a Windows version/build, an opaque benchmark-machine fingerprint,
sample-plan counts, operation labels, latency statistics, the bounded one-second
reproduction outcome, and two privacy booleans. The fingerprint is SHA-256 over
the platform labels and raw processor model, which is used only in memory and is
never serialized. The receipt contains no hostname, path, command, prompt,
assistant content, tool data, stdout, stderr, or credentials.

## Build and execution

On Windows, build all G28 fixture binaries in release mode and run the harness
with the same target directory:

```powershell
$env:CARGO_TARGET_DIR = 'V:\build\hookstat-g28-target'
$env:RUSTFLAGS = '-C debuginfo=0'
cargo build --release --features performance-harness --bins
& "$env:CARGO_TARGET_DIR\release\hookstat-perf.exe" --output runs\HS-V031-TRAIN-A-G28-G29-001\g28-windows-baseline.json
```

The ordinary HookStat binary never enables `performance-harness`; it is not a
production Hook path dependency.

## Timed regions

All durations use `Instant`. Fixture setup, warm-up, harness-result parsing,
benchmark-result serialization, and cleanup remain outside each measured
region. Production operations deliberately include their own serialization
where it is part of the shipped operation (for example atomic receipt writes
and journal append). Percentiles use nearest rank over sorted samples:
`ceil(n * p) - 1`.

| Operation | Timed region | Default samples |
| --- | --- | ---: |
| `direct_original_fixture` | repeated fresh Windows shell invocation of the no-output synthetic handler | 100 |
| `current_v030_proxy` | repeated fresh current full executable, manifest load, receipt writes, durable journal, shell, Job Object, and handler | 100 |
| `hookstat_executable_startup` | repeated fresh ordinary executable help-path startup | 100 |
| `cmd_shell_overhead` | repeated fresh `cmd.exe /D /C exit 0` | 100 |
| `rust_createprocess_direct_spawn` | repeated fresh direct Rust process creation of the synthetic handler | 100 |
| `windows_job_object_cycle` | create, assign a fresh already-spawned child, clear limits, and release; spawn/wait excluded | 100 |
| receipt start/completion | existing atomic JSON record write separately | 10,000 each |
| journal append | existing NDJSON write before durability | 10,000 |
| `sync_data` | existing journal durability primitive immediately after one unmeasured bounded append | 10,000 |
| Named Pipe connect/write/ack | local .NET Named Pipe probe with a 64-byte frame | 1,000 each |
| minimal shim fixture | isolated no-product-dependency repeated fresh startup and an explicitly cache-warmed fresh-start variant | 100 each |

The current proxy and direct-handler fixtures use a verified bare executable
path without whitespace or shell metacharacters. This measures the shipping
`cmd /C` route, not Rust argument escaping behavior. The dedicated Job Object probe uses a fresh child for each
sample because Windows does not allow repeated self-assignment after the first
Job membership; assigning that already-spawned child precisely covers
create/assign/release while excluding its setup and wait.

`sync_data` samples first append one bounded metadata-only journal record outside
the timed interval, then time exactly one immediately following `sync_data()`.
They therefore measure dirty-file durability rather than repeated clean-file
flushes.

Named Pipe `cold` means first connection to a newly-created local endpoint.
`warm` means fresh clients reconnect to the same long-lived server instance and
endpoint name after explicit unmeasured warm-up exchanges. One-way and
acknowledgement samples use a persistent warmed connection. This is a local
transport microbenchmark, not a broker claim.

Repeated fresh process starts are intentionally **not** labeled OS-cache-cold:
Windows file-cache state is not controllable in this non-destructive laboratory.
The cache-warmed minimal-shim variant performs 25 unmeasured fixture launches
before every timed fresh launch. That operationally defined warm measurement is
the only fixture value used to calibrate the warm shim budget; no unmeasured
OS-cache-cold percentile is claimed.

## One-second incomplete reproduction

The harness emits a start receipt through the current proxy for a disposable
synthetic handler that exceeds one second. The harness kills the proxy at the
one-second declaration boundary, scans only the temporary receipt spool, and
requires exactly one start-only `Incomplete` invocation. It does not alter a
Codex Hook declaration or increase any timeout.

## Interpretation

The current proxy is intentionally measured as a full repeated-fresh executable
path, not a controlled OS-cache-cold path, and must not be compared directly to
future warm IPC producer timings. Receipt and journal numbers expose the
current persistence components, while Named Pipe timings establish whether G35
has a credible local low-latency transport basis. The frozen v0.3.1 budget is
recorded separately after the full Windows receipt is reviewed.

## G36 warm host-substrate admission

The Owner-approved G36 landing methodology uses the exact existing
`hookstat-hook-fixture` cache-warmed process-start operation as a prospective
admission control. This changes whether a Windows qualification window is
admitted; it does not change or subtract from any frozen product limit.

Each candidate warm series is measured in one release-mode Rust 1.97.1
qualification session as:

```text
PRE G28 cache-warmed minimal-shim process-start control
G36 warm same-invocation product series
POST G28 cache-warmed minimal-shim process-start control
```

Every control performs 25 unmeasured fresh launches of the unchanged G28
minimal-shim fixture before each of at least 100 measured fresh launches.
Percentiles use the same nearest-rank calculation as G28. No destructive cache
manipulation, host scheduling intervention, dynamic threshold, or threshold
derived from the candidate result is permitted.

The predefined control limits are `20 ms` p95 and `25 ms` p99. Both controls
must pass. If either control fails, the complete pre/product/post observation
is retained as `REJECTED_HOST_SUBSTRATE`; it is neither a product pass nor a
product failure. When both controls pass, the reported G36 product metric must
independently satisfy `20 ms` p95 and `25 ms` p99. No control value is
subtracted from, used to correct, or used to relax that product metric.

The G36 product metric remains the already-audited same-invocation quantity:
parent-observed instrumented shim lifetime minus the original-child spawn/wait
interval observed inside that same invocation. This removes only the original
business-process interval. The host controls are a separate admission gate.

Historical receipts retain their original outcomes. They are not
retroactively classified as host-substrate rejections unless a contemporaneous
execution of this predefined G28 control proves that classification.
