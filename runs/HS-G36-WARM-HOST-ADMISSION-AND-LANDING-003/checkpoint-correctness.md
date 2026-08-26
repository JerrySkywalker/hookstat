# G36 correctness regression before host qualification

Exact implementation head:

```text
G36_SOURCE_HEAD=332070d521955123dc0d86f4e336fad880253980
RUST_TOOLCHAIN=1.97.1
```

`cargo test --locked --all-features --no-fail-fast` passed. The run included
the G36 host-admission reducer, cooperative producer, G35 broker/WAL and async
durability coverage, capsule HMAC/bounds/path/reparse protections, direct and
shell execution, exact timeout boundaries, near-timeout observation behavior,
external shim termination, descendant Job containment, broker absence/loss,
privacy, package layout, and Windows command compatibility.

The fresh current-source TabBeacon proof also passed against pinned
`b3f5685c37f1386f3edceb6d1d3a27403c59dddf`. It exercised the real G35 broker
and TabBeacon `CodexHookRuntime` without a `hookstat-hook` wrapper. The adapter
dependency audit excluded HookStat product, Ratatui, Crossterm, and SQLite
dependencies.

```text
FULL_ALL_FEATURE_TESTS=PASS
COOPERATIVE_CORRECTNESS=PASS
G35_ASYNC_DURABILITY_PRESERVED=true
ACK_AFTER_WAL_APPEND=true
WINDOWS_OVERLAPPED_CLIENT_PRESERVED=true
CAPSULE_HMAC_PATH_REPARSE=PASS
DIRECT_AND_SHELL_EXECUTION=PASS
TIMEOUT_BOUNDARY=PASS
EXTERNAL_SHIM_DEATH_CONTAINMENT=PASS
DESCENDANT_JOB_CONTAINMENT=PASS
BROKER_UNAVAILABLE_AND_LOSS_FAIL_OPEN=PASS
PACKAGE_LAYOUT=PASS
TABBEACON_CURRENT_BOUNDARY=PASS
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```
