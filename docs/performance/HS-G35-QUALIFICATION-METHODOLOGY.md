# HS-G35 IPC performance qualification methodology

## Status

Developer-only acceptance methodology. It does not alter the frozen G28
product budget and is not linked to the cooperative producer or transparent
shim hot paths.

## Why the former CPU gate is not an acceptance requirement

The prior aggregate-CPU mean/max probe is retained only as
`LEGACY_EXPERIMENTAL_HOST_HEURISTIC`. G28 freezes latency and zero-induced-
timeout limits; it does not define Task Manager CPU percentages as a HookStat
product SLO. The qualification runner therefore does not inspect process names,
command lines, priorities, affinity, Defender, power policy, or Owner state.

## Paired control admission

Before and after every candidate release run, the runner measures a fresh
single-client persistent local broker ACK path using bounded synthetic IPC
metadata. It includes the Named Pipe, v1 frame validation, bounded enqueue,
WAL append to the operating-system file buffer, and ACK—the same substrate G35
is qualifying. No Hook, command, prompt, output, or private path enters the
receipt.

The control limits are conservative methodology limits, derived from the G28
Named Pipe ACK baseline and frozen G35 cooperative budget:

```text
control p95 <= min(0.5 * 1.00 ms, 4 * 0.18 ms) = 0.50 ms
control p99 <= min(0.5 * 2.00 ms, 4 * 0.24 ms) = 0.96 ms
```

They reserve at least half the frozen budget for the benchmarked path and
reject ambiguous scheduler/IPC noise. They do not relax, replace, or become
product SLOs. A post-run control failure aborts that candidate series: its
measured latency is retained as rejected/degraded evidence and cannot be used
for acceptance.

## Runner

Build the feature-gated binary in an isolated target directory and run it only
against disposable state:

```powershell
$env:CARGO_TARGET_DIR = 'C:\temp\hookstat-g35-qualification-target'
$env:RUSTFLAGS = '-C debuginfo=0'
cargo +1.97.1 build --locked --release --features performance-harness --bin hookstat-ipc-qualify
& "$env:CARGO_TARGET_DIR\release\hookstat-ipc-qualify.exe" `
  --output C:\temp\g35-qualification.json `
  --max-attempts 720 --wait-ms 60000
```

The runner waits only after a rejected preflight, never longer than 60 seconds
per interval. It preserves all controls, all admitted measurements, and all
post-control-rejected measurements in one sanitized JSON receipt. It requires
five independent qualifying single-client runs and five independent qualifying
16-client persistent runs; it reports the worst qualifying p95/p99. Any
admitted run above the immutable 1 ms p95 or 2 ms p99 limit produces
`FAIL_FROZEN_G28_BUDGET`; insufficient admitted runs produce
`BLOCKED_NO_QUALIFYING_WINDOW`.

The existing loaded-host p95=2.141 ms / p99=8.314 ms observation remains
historical non-acceptance evidence and is neither deleted nor reclassified.
