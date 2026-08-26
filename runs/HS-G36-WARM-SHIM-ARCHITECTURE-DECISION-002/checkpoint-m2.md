# M2 — same-invocation overhead oracle

The feature-gated oracle measures one real transparent invocation.  The parent
observes the complete instrumented shim lifetime; the shim measures the
interval from immediately before original child spawn through completion of
the same child's wait.  Subtraction therefore removes the actual original
business-process lifetime instead of a second independently scheduled process.

The oracle emits exactly 32 bytes over a disposable owner-only local pipe: one
fixed record carries the child interval and a second fixed record carries the
connect plus first-record write duration.  The benchmark charges the entire
side channel to observed overhead.  No command, capsule content, path, stream,
prompt, tool data, credential, or host identity is serialized.  The channel is
developer measurement control only and is not a third reliability evidence
transport.

The retained ten-sample smoke proved the mechanism and preserved its outliers.
The single planned 100-sample release diagnostic then recorded:

```text
SHIPPING_BINARY_SIZE_BYTES=550400
INSTRUMENTED_BINARY_SIZE_BYTES=557568
SHIPPING_STARTUP_P95_MS=12.0281
SHIPPING_STARTUP_P99_MS=14.8883
INSTRUMENTED_STARTUP_P95_MS=11.6887
INSTRUMENTED_STARTUP_P99_MS=13.7215
INSTRUMENTED_MINUS_SHIPPING_STARTUP_P95_MS=-0.3394
SHIPPING_MINUS_INSTRUMENTED_STARTUP_P99_MS=1.1668

FULL_TRANSPARENT_P95_MS=42.0645
FULL_TRANSPARENT_P99_MS=47.0224
SAME_INVOCATION_CHILD_P95_MS=26.4933
SAME_INVOCATION_CHILD_P99_MS=31.7757
RAW_OBSERVED_OVERHEAD_P95_MS=16.3280
RAW_OBSERVED_OVERHEAD_P99_MS=16.6528
ORACLE_PRIMARY_RECORD_P95_MS=0.0347
ORACLE_PRIMARY_RECORD_P99_MS=0.0435
```

The raw oracle is not promoted to acceptance because the instrumented startup
tail was slightly faster than the ordinary shipping binary.  Applying the
larger p99 startup-tail delta to both raw overhead percentiles produces a
conservative diagnostic of `17.4948/17.8196` ms, still within the frozen
`20/25` limits and engineering-margin targets.  A complete acceptance harness
must encode this correction and reject a materially faster instrumented build.

Focused proof on the oracle candidate: feature-gated Clippy warnings-as-errors
passed; nine capsule/budget unit tests passed; all 22 shim IPC/exit/timeout/
containment e2e tests passed.

```text
SAME_INVOCATION_ORACLE=PASS_DIAGNOSTIC
ORACLE_OVERHEAD=PRIMARY_RECORD_P95_0.0347_MS_P99_0.0435_MS_FULL_CHANNEL_CHARGED
ORACLE_RECEIPT_CLASSIFICATION=DIAGNOSTIC_ONLY
WINDOWS_OVERLAPPED_CLIENT_PRESERVED=true
G35_ASYNC_DURABILITY_PRESERVED=true
ACK_AFTER_WAL_APPEND=true
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
```
