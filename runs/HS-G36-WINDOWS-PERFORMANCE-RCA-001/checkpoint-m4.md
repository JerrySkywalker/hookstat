# M4 — transparent shim stage RCA

`g36-shim-stage-diagnostic-001.json` is a 100-sample, feature-gated,
breakaway-worker diagnostic. It is not acceptance evidence. The worker is a
disposable local process needed because the Codex parent is already in a Windows
Job and cannot assign itself to the shim's required containment Job.

The primary warm-path cost is the actual shipping `hookstat-hook` process
startup baseline (14.5110 ms p95). The direct child wait is 17.7629 ms p95 but
belongs to the original handler rather than transparent overhead. The highest
incremental stages are START IPC (1.9914 ms p95), COMPLETE IPC (2.1073 ms p95),
capsule directory/file validation (1.1162 ms p95), and producer/Tokio runtime
construction (0.8653 ms p95). HMAC verification and Job Object establish/release
are not primary costs.

```text
SHIM_WARM_PRIMARY_HOTSPOT=SHIPPING_PROCESS_STARTUP
SHIM_WARM_SECONDARY_HOTSPOT=START_AND_COMPLETE_IPC_WITH_CAPSULE_FILE_VALIDATION
SHIM_DIRECT_HANDLER_WAIT_IS_NOT_INCREMENTAL_OVERHEAD=true
WINDOWS_JOB_CONTAINMENT_MEASURED_IN_BREAKAWAY_WORKER=true
```
