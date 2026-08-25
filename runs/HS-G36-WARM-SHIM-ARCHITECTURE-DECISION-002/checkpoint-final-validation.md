# Final safe validation after architecture stop

The final product/test candidate is
`6d8a3a401a529fd7b0f53b041665ee955a1f6b59`. Later receipt-only commits do not
change product or test sources.

## Local gates

```text
FMT=PASS
CLIPPY_LOCKED_ALL_TARGETS_ALL_FEATURES_DENY_WARNINGS=PASS
TEST_LOCKED_ALL_FEATURES_NO_FAIL_FAST=PASS
BUILD_LOCKED=PASS
HELPER_FLOOR_CRATE_FMT=PASS
HELPER_FLOOR_CRATE_CLIPPY_LOCKED_RELEASE_DENY_WARNINGS=PASS
```

The first exact-head all-feature run exposed that the controlled delayed-ACK
fixture's one-second healthy-handler allowance could expire during unusually
slow Windows process creation. The fixture now uses five seconds solely to
isolate its 30-ms observation delay; the independent exact timeout tests are
unchanged. The focused case and the subsequent complete suite pass.

## Package and dry-run proof

The release verifier packaged the clean exact source head, unpacked the
generated archive, rejected path dependencies and developer-only proof trees,
built the archive, installed it into a fresh root, and proved the three
production binaries. No test fixture was installed.

```text
PACKAGE=PASS
FRESH_INSTALL=PASS
PACKAGE_SOURCE_GIT_HEAD=6d8a3a401a529fd7b0f53b041665ee955a1f6b59
PACKAGE_ARCHIVE_SHA256=d8cdfd699d378d410f28bcc02dce7be3c9b32477a31f485676df5c781f4d299b
FRESH_INSTALL_BINARY_SHA256_HOOKSTAT=dea560a2306ae0e4a40d14cf942b035867266a58c03e09ecf483b02b82276afc
FRESH_INSTALL_BINARY_SHA256_HOOKSTAT_HOOK=b8138bb0e2929001955b434dcd01d8cd95975157b8819b512dcbf11a153c6cc3
FRESH_INSTALL_BINARY_SHA256_HOOKSTAT_IPC_BROKER=73a5502a604d860d01aeaf4c1d710937414b1cce75f0709b439fb1bb253182a9
PUBLISH_DRY_RUN=PASS
PUBLICATION_AUTHORIZED=false
```

`cargo publish --dry-run --locked` completed package verification and aborted
the upload as required. Cargo warned that version `0.3.0` already exists; no
publication was attempted.

## Current-source cooperative boundary

`g36-tabbeacon-current-boundary-6d8a3a4.json` proves the exact current
`src/ipc_client.rs` through a package-excluded adapter and real G35 broker
around pinned TabBeacon's real `CodexHookRuntime`. The dependency audit excludes
the HookStat product, SQLite, Ratatui, and Crossterm from the adapter boundary.

```text
TABBEACON_CURRENT_BOUNDARY=PASS
TABBEACON_DECLARATION_HAS_HOOKSTAT_WRAPPER=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
```

## Stop disposition

These gates do not override the retained exact-candidate warm performance
failure. PR #33 remains draft and unmerged. A fresh independent acceptance
review is not launched because its admission requires every G36 gate to pass.

```text
G36_PERFORMANCE=FAIL
INDEPENDENT_REVIEW=NOT_LAUNCHED_PERFORMANCE_GATE_FAIL
G36_MERGED=false
OWNER_ARCHITECTURE_DECISION_REQUIRED=true
G37_STARTED=false
G38_STARTED=false
```
