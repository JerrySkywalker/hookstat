# HS-G37 M0 — accepted-main admission and Native L2 audit

## Source admission

```text
START_MAIN=d8d00e3da3e24a91cd9405c14d297a12ce33eb23
G36_MERGED=true
G37_BRANCH=agent/hs-v031-g37-codex-evidence-routing-migration-001
G37_MODE=DRAFT
```

The worktree was created cleanly from accepted remote `main`. The already
merged G36 router is retained as the G37 baseline:

```text
Native admitted -> Native
else admitted IPC integration -> IPC
else -> NOT_ADMITTED
```

## Native L2 audit

```text
CODEX_CLI=0.149.0
CODEX_SOURCE=758ef40f50c1a458425c7cfbf1eb12cbc07af0b0
TARGET_PLATFORM=Windows
ORDINARY_CODEX_NATIVE_L2=UPSTREAM_UNAVAILABLE
HOOKSTAT_AS_CODEX_LAUNCHER=false
OWNER_LIVE_CODEX_CONFIG_MUTATED=false
```

The exact upstream daemon and TUI sources establish that the shared daemon is
Unix-only, the non-Unix default-daemon probe returns no endpoint, and ordinary
Windows CLI launch therefore selects an embedded App Server. The published App
Server surface provides no passive subscription method for an external client
to join the already-owned ordinary thread.

## Routing consequence

```text
COOPERATIVE_IPC_ADMISSION=ADMITTED
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NATIVE_UNAVAILABLE_ADMITTED_IPC_OR_NOT_ADMITTED=PASS
NOT_ADMITTED_IS_EVIDENCE_PATH=false
G37_MERGED=false
```

This checkpoint does not start a real Owner Codex session, does not activate
the transparent shim, and does not modify live configuration.

## Validation

```text
FMT=PASS
CLIPPY_ALL_TARGETS_ALL_FEATURES=PASS
TESTS_ALL_FEATURES=PASS
G37_AUTHORITY_ROUTING_TESTS=3/3 PASS
CODEX_NATIVE_L1_REGRESSION=8/8 PASS
RUNTIME_NEUTRAL_ROUTER_REGRESSION=16/16 PASS
```

The full test train also retained the G35 durability, cooperative IPC,
transparent-shim correctness, timeout, Job containment, package-boundary, and
privacy tests. Explicit live App Server and performance qualifications remained
ignored according to their existing contracts; no ignored test was represented
as executed evidence.
