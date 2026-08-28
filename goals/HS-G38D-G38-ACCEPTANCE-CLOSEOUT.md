# HS-G38D — G38 Convergence, Acceptance & PR Closeout

## Status

PLANNED after both G38B and G38C are accepted.

## Objective

Converge the new HSIP conformance proof and the reconciled HookStat Windows-hardening implementation into one accepted G38 state on authoritative `main`, with exact-head CI, fresh independent review, truthful release semantics, and no external-repository dependency.

## Preconditions

```text
G38A=PASS
G38B=PASS
G38C=PASS
```

Both G38B and G38C must be represented in the convergence candidate. Do not infer acceptance from stale branch receipts.

## Branch/PR convergence

The historical G38 draft PR may be reused for G38C if safe. G38B may arrive through a separate accepted PR.

Before G38D acceptance:

1. fetch authoritative `main` and all relevant PR heads;
2. prove both predecessor implementations are represented;
3. reconcile/retarget the surviving G38 closeout branch to current `main` if needed;
4. resolve governance text in favor of accepted G38A;
5. preserve historical blocked/external-integration receipts as non-normative history;
6. ensure no unresolved merge conflict or foreign writer exists.

Do not force-push published `main` or discard foreign changes.

## Final HookStat qualification

Run exact settled-head gates:

```text
cargo +1.97.1 fmt --all -- --check
cargo +1.97.1 clippy --locked --all-targets --all-features -- -D warnings
cargo +1.97.1 test --locked --all-features --no-fail-fast
cargo +1.97.1 build --locked --all-features
```

Require hosted Windows and Ubuntu CI on the exact candidate.

## Required technical proof

```text
HSIP_V1_CONFORMANCE=PASS
REFERENCE_HSIP_P95_MS<=1
REFERENCE_HSIP_P99_MS<=2
REFERENCE_HSIP_OBSERVATION_GAPS=0

BROKER_RESTART=PASS
BROKER_RECONNECT=PASS
WAL_RECOVERY=PASS
DUPLICATE_GUARD=PASS
PROCESS_LEAK=0

DIAGNOSTICS_BOUNDED=PASS
DIAGNOSTICS_TRUTHFUL=PASS
REPORT_DOCTOR_TUI_CONSISTENCY=PASS

HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0

PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
```

## Runtime coverage semantics

G38D does not require a named external cooperative producer.

For each relevant runtime coverage domain, final diagnostics must be able to express one of:

```text
Native
IPC via a separately admitted integration
NOT_ADMITTED
```

`NOT_ADMITTED` is a valid release state when neither source is admitted.

Do not use the reference producer as a fake Codex integration.

Do not describe synthetic/conformance frames as real runtime event coverage.

## Normal Codex boundary

Require a normal `codex` non-interference smoke proving:

```text
NORMAL_CODEX_LAUNCH=PASS
HOOKSTAT_AS_CODEX_LAUNCHER=false
TRUST_BYPASS=false
UNWANTED_PERSISTENT_CONFIG_MUTATION=false
```

No external repository modification is allowed to manufacture live evidence.

## Fresh independent review

The settled exact G38 convergence head requires a fresh mechanically independent read-only review.

Review must verify:

- both G38B and G38C are represented;
- reference producer cannot become production authority;
- 1/2 ms budget was not weakened;
- external-integration failure cannot block HookStat release merely by identity;
- `NOT_ADMITTED` remains truthful and excluded from healthy denominators;
- historical receipts remain intact;
- privacy/security and recovery semantics remain correct;
- no third evidence path or transparent-shim activation was introduced.

If the reviewer finds material defects, fix narrowly and obtain a new exact-head review.

## Required closeout receipt

Record at minimum:

```text
START_MAIN
G38D_HEAD
G38B_ACCEPTED_SHA
G38C_ACCEPTED_SHA
HSIP_CONFORMANCE
REFERENCE_HSIP_P50_MS
REFERENCE_HSIP_P95_MS
REFERENCE_HSIP_P99_MS
BROKER_RECOVERY
WAL_RECOVERY
PROCESS_LEAK
DIAGNOSTICS
COVERAGE_TRUTHFUL
NORMAL_CODEX_SMOKE
PRIVACY_REVIEW
SECURITY_REVIEW
WINDOWS_CI
UBUNTU_CI
INDEPENDENT_REVIEW
EXTERNAL_INTEGRATION_REQUIRED=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NO_THIRD_EVIDENCE_PATH=true
```

## Acceptance

```text
G38A=PASS
G38B=PASS
G38C=PASS
HSIP_CONFORMANCE=PASS
REFERENCE_HSIP_PERFORMANCE=PASS
BROKER_RECOVERY=PASS
DIAGNOSTICS=PASS
COVERAGE_TRUTHFUL=PASS
NORMAL_CODEX_NON_INTERFERENCE=PASS
HOOKSTAT_INDUCED_TIMEOUTS=0
HOOKSTAT_INDUCED_FAILURES=0
PRIVACY_REVIEW=PASS
SECURITY_REVIEW=PASS
WINDOWS_CI=PASS
UBUNTU_CI=PASS
INDEPENDENT_REVIEW=PASS
EXTERNAL_INTEGRATION_REQUIRED=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NO_THIRD_EVIDENCE_PATH=true
```

Only after all fields truthfully pass may the G38 closeout PR be marked ready and merged.

## Next

Immediately begin G38R from exact accepted `main` if running as an authorized unattended train and no release stop gate is present. Public publication remains forbidden until explicit Owner authorization.
