# G36 M5 cooperative performance acceptance

The dedicated cooperative-only release qualification ran against the clean,
committed scope-recovery source. It did not invoke `hookstat-hook`, run the
transparent warm harness, or change any transparent evidence classification.

```text
QUALIFICATION_SOURCE_HEAD=b37cea32c9500d198a0aa1e1b8defa8e73caaf12
SOURCE_TRACKED_WORKTREE_CLEAN=true
RUST_TOOLCHAIN=1.97.1
BUILD_PROFILE=release
PERCENTILE_METHOD=nearest_rank
COOPERATIVE_RUNS=5/5
SAMPLES_PER_RUN=100
COOPERATIVE_WORST_P95_MS=0.1790
COOPERATIVE_WORST_P99_MS=0.3036
COOPERATIVE_OBSERVATION_GAPS=0
COOPERATIVE_P95_LIMIT_MS=1
COOPERATIVE_P99_LIMIT_MS=2
COOPERATIVE_PERFORMANCE=PASS
```

Per-run retained results:

| Run | Samples | p95 ms | p99 ms | Observation gaps |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 100 | 0.1688 | 0.2530 | 0 |
| 2 | 100 | 0.1091 | 0.1483 | 0 |
| 3 | 100 | 0.1790 | 0.3036 | 0 |
| 4 | 100 | 0.1064 | 0.1281 | 0 |
| 5 | 100 | 0.0881 | 0.1196 | 0 |

Receipt SHA-256:

```text
CBC69178B29FF97E050866D4B80F162D090F958CCBB19086F21DFFD852DF0525
```

The controlled TabBeacon proof remains valid for the unchanged
`src/ipc_client.rs` SHA-256
`a6431eea9e2b373781d46b7f3a0694b5658ea8ca2b8076df6cd3a496ca6acdd6`.

```text
TRANSPARENT_SHIM_PRODUCTION_ADMISSION=false
TRANSPARENT_SHIM_20_25_RESULT=FAIL
TRANSPARENT_SHIM_25_30_RESULT=FAIL
FURTHER_BUDGET_RELAXATION=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
G37_STARTED=false
PUBLICATION_AUTHORIZED=false
```
