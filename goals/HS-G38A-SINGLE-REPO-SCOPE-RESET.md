# HS-G38A — Single-Repo Scope & Admission Contract Reset

## Status

This Goal defines the v0.3.1 single-repository release contract. It becomes accepted when its governance/documentation change is merged to authoritative `main` with no unresolved material review finding.

## Objective

Remove the accidental release-critical dependency on a named external cooperative producer without weakening HookStat's evidence, privacy, performance, or coverage semantics.

G38A is a governance/architecture-boundary Goal. It does not implement a runtime producer and must not modify another repository.

## Required starting point

```text
G37_ACCEPTED=true
EXPECTED_ACCEPTED_MAIN=ac9683a151741a28341b357dc11ae6fd3b701dfd
```

If `main` has legitimately advanced, revalidate that G37 remains represented before proceeding.

## Required changes

Align at minimum:

```text
dev_governance_files/ROADMAP.md
dev_governance_files/ROADMAP_V031.md
goals/HS-V031-NATIVE-IPC-HIGH-PERFORMANCE-EVIDENCE-RUNTIME.md
goals/HS-G38-PERFORMANCE-DOGFOOD-HARDENING.md
goals/HS-G38R-V031-HARDENING-AND-RELEASE.md
docs/architecture/HSIP-V1-CONFORMANCE-AND-ADMISSION.md
```

Add the concrete G38A/B/C/D Goal contracts.

## Required semantic reset

The settled documentation must state:

```text
HOOKSTAT_RELEASE_CAN_COMPLETE_WITH_HOOKSTAT_REPO_ONLY=true
EXTERNAL_REPOSITORY_WRITE_REQUIRED=false
EXTERNAL_INTEGRATION_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_MERGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_PACKAGE_REQUIRED_FOR_RELEASE=false
EXTERNAL_INTEGRATION_PUBLICATION_REQUIRED_FOR_RELEASE=false

HSIP_PROTOCOL_RELEASE_OWNED_BY_HOOKSTAT=true
HOOKSTAT_IPC_INFRASTRUCTURE_RELEASE_OWNED_BY_HOOKSTAT=true
INTEGRATION_CONFORMANCE=PER_INTEGRATION
INTEGRATION_ADMISSION=PER_INTEGRATION
DOMAIN_WITHOUT_ADMITTED_SOURCE=NOT_ADMITTED
```

## Frozen invariants that MUST NOT change

```text
EVIDENCE_PATHS=2
NATIVE_FIRST=true
NO_THIRD_EVIDENCE_PATH=true
NOT_ADMITTED_IS_EVIDENCE_PATH=false
ONE_AUTHORITY_PER_COVERAGE_DOMAIN=true
MISSING_EVIDENCE_NEVER_BECOMES_SUCCESS=true
SHADOW_EVIDENCE_IN_DENOMINATOR=false
NORMAL_CODEX_LAUNCH=codex
HOOKSTAT_AS_CODEX_LAUNCHER=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
LEGACY_V03_EVIDENCE_PRESERVED=true
FURTHER_BUDGET_RELAXATION=false
```

The frozen cooperative 1/2 ms target is retained and reassigned correctly:

- HookStat reference producer + broker: HookStat G38B/G38D release substrate gate;
- each external producer: that producer's independent admission gate.

## Historical evidence policy

Do not delete, rewrite, hide, or relabel historical external-producer receipts or performance failures.

The scope change alters only whether such evidence is a mandatory predecessor for HookStat release.

## Revised DAG

```text
G37
 ↓
G38A
 ├──────→ G38B
 └──────→ G38C
           ↑
G38B ──────┘
     both converge
           ↓
          G38D
           ↓
          G38R
```

## Validation

Because this Goal is documentation/governance only:

- verify Markdown links/paths and internal Goal references;
- search for stale normative statements such as `TabBeacon ... required`, `cooperative IPC admitted for v0.3.1` when referring to a specific producer, or `real external producer dogfood required`;
- distinguish historical receipts from active normative requirements;
- run repository formatting/docs checks if defined by governance;
- require CI if the PR triggers CI;
- require a fresh read-only review of the settled diff before merge when practical.

## Acceptance

```text
SINGLE_REPO_RELEASE_BOUNDARY=true
EXTERNAL_REPOSITORY_WRITE_REQUIRED=false
EXTERNAL_INTEGRATION_REQUIRED_FOR_RELEASE=false
HSIP_CONFORMANCE_CONTRACT_DEFINED=true
G38A_B_C_D_GOALS_DEFINED=true
FROZEN_PERFORMANCE_BUDGET_PRESERVED=true
COVERAGE_SEMANTICS_PRESERVED=true
HISTORICAL_RECEIPTS_PRESERVED=true
NO_THIRD_EVIDENCE_PATH=true
REVIEW=PASS
CI=PASS_OR_NOT_APPLICABLE
```

## Next

G38B and G38C are unblocked after G38A merge. A single unattended writer should normally execute G38B first, then G38C. Independent isolated writers may run them in parallel.
