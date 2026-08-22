# HS-G28 frozen v0.3.1 performance budget

## Status

FROZEN from the sanitized Windows receipt
[`../../runs/HS-V031-TRAIN-A-G28-G29-001/g28-windows-baseline.json`](../../runs/HS-V031-TRAIN-A-G28-G29-001/g28-windows-baseline.json).

```text
receipt_sha256=93ab1788be945099a119577505781b9568e22d6a3885595ebb73319d605ecda9
platform=windows/x86_64
windows_version_build=10.0.26300
logical_processors=16
benchmark_machine_fingerprint=095d00be722c7349cc1102e4ab02797cced3c05c0449f2016f7c3485ee4e3985
rustc=1.97.1
cargo=1.97.1
build_profile=release
```

The receipt has 16 bounded operation records: 100 samples for repeated-fresh
process and Job Object paths, 1,000 for each Named Pipe path, and 10,000 for
each receipt or journal path. It records neither hostname, path, command text,
nor raw processor model, and contains no private-content field.

## Measured v0.3 cost decomposition

| Operation | p50 ms | p95 ms | p99 ms |
| --- | ---: | ---: | ---: |
| Direct repeated-fresh shell handler fixture | 33.44 | 43.74 | 56.83 |
| Current repeated-fresh v0.3 proxy | 65.55 | 81.18 | 86.21 |
| Repeated-fresh HookStat executable startup | 14.59 | 16.71 | 22.23 |
| Repeated-fresh `cmd.exe /C` fixture | 21.92 | 27.09 | 29.88 |
| Repeated-fresh direct Rust process spawn | 10.83 | 14.29 | 16.82 |
| Job Object create/assign/release | 0.04 | 0.08 | 0.12 |
| Receipt start write | 2.13 | 2.92 | 3.96 |
| Receipt completion write | 2.22 | 3.20 | 4.38 |
| Journal append before durability | 0.29 | 0.53 | 0.74 |
| Dirty-file `sync_data()` | 0.89 | 1.23 | 1.43 |
| Minimal shim repeated-fresh process start | 11.87 | 14.32 | 15.80 |
| Minimal shim cache-warmed fresh process start | 11.58 | 14.82 | 18.47 |
| Named Pipe cold connection | 0.04 | 0.08 | 0.13 |
| Named Pipe warm connection | 0.06 | 0.15 | 0.22 |
| Named Pipe 64-byte one-way write | 0.04 | 0.09 | 0.13 |
| Named Pipe 64-byte acknowledgement round trip | 0.10 | 0.18 | 0.24 |

The current proxy is dominated by repeated whole-process and shell work: its
81.18 ms p95 exceeds the 43.74 ms direct shell-handler path, while its two
atomic receipt writes and dirty-file durability cost about 3.20 ms, 2.92 ms,
and 1.23 ms respectively when isolated. The Job Object lifecycle is not
dominant. A local Named Pipe has ample headroom for a cooperative producer;
this establishes the G35 transport stop gate as credible, not as a broker
implementation claim.

The disposable one-second fixture emitted start evidence, was terminated at the
one-second boundary, had no completion evidence, and reconciled to
`Incomplete`. This records the v0.3 failure class truthfully; it does not claim
that the existing proxy meets the v0.3.1 timeout requirement.

## Release-governing budget

```text
NATIVE_ADDED_SYNCHRONOUS_LATENCY_MS=0

COOPERATIVE_IPC_P95_MS<=1
COOPERATIVE_IPC_P99_MS<=2

TRANSPARENT_SHIM_WARM_P95_MS<=20
TRANSPARENT_SHIM_WARM_P99_MS<=25
TRANSPARENT_SHIM_COLD_P95_MS<=50

HOOKSTAT_INDUCED_TIMEOUTS_FOR_HEALTHY_HOOK=0
```

The cooperative IPC limits retain the roadmap provisional values because the
measured 64-byte local acknowledgement round trip is 0.18 ms p95 and 0.24 ms
p99. The transparent-shim warm p95 is calibrated once from 15 ms to 20 ms:
the explicitly cache-warmed fresh minimal-shim start is already 14.82 ms p95,
before bounded IPC/finalization work. Its 18.47 ms p99 supports retaining the
25 ms p99 cap. The 50 ms cold p95 cap is retained as a conservative release
limit, not a claim of an unmeasured OS-cache-cold percentile; destructive
Windows cache control is intentionally outside this laboratory.

Later v0.3.1 Goals may improve these values but must not silently weaken them.
The zero-induced-timeout requirement is unchanged and blocks release if a
previously healthy Hook can be reproducibly disturbed by HookStat.

## G28 acceptance record

```text
OWNER_WINDOWS_BASELINE=PASS
CURRENT_PROXY_P50_P95_P99=RECORDED
PROCESS_SPAWN_COST=RECORDED
SHELL_COST=RECORDED
JOB_OBJECT_COST=RECORDED
RECEIPT_IO_COST=RECORDED
SYNC_DATA_COST=RECORDED
NAMED_PIPE_COST=RECORDED
MINIMAL_SHIM_FIXTURE_COST=RECORDED
MACHINE_RUNTIME_TOOLCHAIN_IDENTIFIED=PASS
WARM_COLD_CLASSIFICATION_EXPLICIT=PASS
ONE_SECOND_FAILURE_CLASS_REPRODUCED=PASS
LIVE_OWNER_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
DOMINANT_COSTS_IDENTIFIED=true
PERFORMANCE_BUDGET_FROZEN=true
NO_UNMEASURED_PERFORMANCE_CLAIMS=true
```
