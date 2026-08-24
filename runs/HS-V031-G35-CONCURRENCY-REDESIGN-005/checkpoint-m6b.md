# M6B — target-specific Clippy correction

Status: CI_CORRECTION_PENDING_FRESH_EXACT_HEAD

Parent head: `cdc8f9d2621884afb57d4d667fd783b1b680adf8`

Final-head CI run `32754982009` reached the exact parent SHA. Ubuntu failed in
Clippy because the Unix-only `IpcClient::connect` block used an explicit
`return` that becomes needless when the Windows block is removed by `cfg`.
GitHub's fail-fast matrix then cancelled the in-progress Windows job.

The correction removes only that explicit `return`. It does not change the
Unix result value and is not compiled into the Windows target. The qualified
Windows code tree and its persistent-send behavior are therefore unchanged;
the accepted paired-control receipt is not rerun merely because this
target-specific syntax correction advances HEAD.

After the correction, local Windows formatting, diff integrity, Clippy
warnings-as-errors, the full all-feature test suite, and the locked all-feature
release build pass. Fresh Windows and Ubuntu CI are required on the new exact
head.

OWNER_LIVE_CODEX_CONFIG_MUTATED=false
RAW_PRIVATE_CONTENT_CAPTURED=false
FROZEN_G28_BUDGET_CHANGED=false
G37_STARTED=false
