# HS-G38 Owner Windows dogfood M1 activation admission

```text
RUN_ID=HS-V031-G38-OWNER-WINDOWS-DOGFOOD-ACCEPTANCE-007
DISPOSITION=BLOCKED_G38_SAFE_ACTIVATION

START_MAIN=ac9683a151741a28341b357dc11ae6fd3b701dfd
G38_START_HEAD=5069b1a48bf28ecbb710111f07ef8c24e03df817
CHECKPOINT_HEAD=COMMIT_CONTAINING_THIS_RECEIPT
PR=35
PR_STATE=OPEN_DRAFT
PR_BASE=main
PR_HEAD=5069b1a48bf28ecbb710111f07ef8c24e03df817
PR_MERGEABLE=MERGEABLE
EXPECTED_HEAD_CONTAINS_MAIN=true
START_WORKTREE_CLEAN=true
EXACT_HEAD_CI_RUN=32996489052
EXACT_HEAD_WINDOWS_CI=PASS
EXACT_HEAD_UBUNTU_CI=PASS

WINDOWS=Windows_11_10.0.26340_build_26340_x64
POWERSHELL=7.6.5_Core
WINDOWS_TERMINAL=1.24.11911.0
CODEX_VERSION=codex-cli_0.149.0
CODEX_ENTRYPOINT_SHA256=0c149db80ed0bf442c810146b0ad0163b74982fe4542d673f56c354d7b8229cb
HOOKSTAT_VERSION=0.3.0
HOOKSTAT_CANDIDATE_SHA=5069b1a48bf28ecbb710111f07ef8c24e03df817
HOOKSTAT_CLEAN_DEBUG_BINARY_SHA256=ae9fb0e29704cc3e0aa96cd8b84510ce6a56ff0da5f1169b046a7daceaf17040
TABBEACON_BASELINE_VERSION=0.5.2
TABBEACON_BASELINE_BINARY_SHA256=4f4819eaf5bb35692cac4547d0304a2a14e269ba42e77aa065673df8dafa60f6
TABBEACON_G36_PROOF_SOURCE=b3f5685c37f1386f3edceb6d1d3a27403c59dddf

TABBEACON_DECLARATION_STATUS=PASS
TABBEACON_HOOK_TRUST=ACTIVE
TABBEACON_OWNED_ENABLED_HOOKS=11
TABBEACON_OWNED_EVENTS=permission_request,post_compact,post_tool_use,pre_compact,pre_tool_use,session_end,session_start,stop,subagent_start,subagent_stop,user_prompt_submit
SANITIZED_HOOK_INVENTORY_SHA256_PRE=3ab1c12d76d5ba8299e59405daa5892a0a43ea247b3e261e4954d7f9ec58a5b9
SANITIZED_HOOK_INVENTORY_SHA256_POST=3ab1c12d76d5ba8299e59405daa5892a0a43ea247b3e261e4954d7f9ec58a5b9
CODEX_CONFIG_BYTES=16948
CODEX_CONFIG_SHA256_PRE=8d7abd3acbeb12770de367e6f6d0744e8bfea2b6af3a67e1d7a304cd9b720b65
CODEX_CONFIG_SHA256_POST=8d7abd3acbeb12770de367e6f6d0744e8bfea2b6af3a67e1d7a304cd9b720b65
OWNER_POSTSTATE_MATCHES_PRESTATE=true

ACTIVE_CODEX_PROCESSES_AT_M1=8
ACTIVE_TABBEACON_PROCESSES_AT_M1=5
ACTIVE_HOOKSTAT_PROCESSES_AFTER_PREFLIGHT=0
UNRELATED_OWNER_PROCESSES_TERMINATED=0

G36_TABBEACON_PROOF_REAL_RUNTIME=true
G36_TABBEACON_PROOF_ADAPTER_PACKAGED=false
G36_TABBEACON_PROOF_ADAPTER_PUBLISHABLE=false
INSTALLED_TABBEACON_HSIP_PRODUCER=false
PACKAGED_ADMITTED_COOPERATIVE_ACTIVATION_AVAILABLE=false

OWNER_WINDOWS_DOGFOOD_STARTED=false
NORMAL_CODEX_LAUNCH_BY_THIS_RUN=false
REAL_HOOKSTAT_EVENTS_COVERED=none
REAL_HOOKSTAT_EVENTS_NOT_ADMITTED=permission_request,post_compact,post_tool_use,pre_compact,pre_tool_use,session_end,session_start,stop,subagent_start,subagent_stop,user_prompt_submit
REAL_LEDGER_EVIDENCE=NOT_RUN
REAL_ANALYTICS=NOT_RUN
REAL_TUI_OR_REPORT=NOT_RUN

LIVE_READ_ONLY_DIAGNOSTICS_PROBE=COMPLETED
DIAGNOSTICS_SCHEMA_V2_EMPTY_ROOT=PASS
DIAGNOSTICS_LIVE_TOTAL_MS_WITH_TIMING_ONLY_INSTRUMENTATION=76997
DIAGNOSTICS_LIVE_RECEIPT_SCAN_STAGE_MS=76871
DIAGNOSTICS_BROKER_QUERY_BOUND_MS=20_UNCHANGED
TEMPORARY_DIAGNOSTIC_INSTRUMENTATION_REMOVED=true
DIAGNOSTICS_REAL_BROKER_STATE=NOT_EXERCISED

LIVE_DOGFOOD_MUTATION_USED=false
EXACT_OWNER_RESTORE=PASS_NO_MUTATION
TRANSPARENT_SHIM_USED=false
TRANSPARENT_SHIM_PRODUCTION_ACTIVATION=false
NO_THIRD_EVIDENCE_PATH=true
RAW_PRIVATE_CONTENT_CAPTURED=false

CONCURRENT_CODEX=NOT_RUN
BROKER_IDLE_RESTART=NOT_RUN
BROKER_UNAVAILABLE_FAIL_OPEN=NOT_RUN
BROKER_RECONNECT=NOT_RUN
COOPERATIVE_PERFORMANCE=NOT_RUN
PRIVACY_REVIEW=NOT_RUN
SECURITY_REVIEW=NOT_RUN
PROCESS_LEAK=0_PREFLIGHT_ONLY

G38_MERGED=false
G38R_STARTED=false
PUBLICATION_AUTHORIZED=false
NEXT_SAFE_GOAL=GOVERNED_PACKAGED_TABBEACON_HSIP_V1_CANDIDATE_THEN_RESUME_G38
```

## Admission decision

The installed TabBeacon baseline is current, trusted, and owns all eleven
Codex 0.149.0 hook declarations, but its shipped source and binary do not
contain an HSIP v1 producer. The retained G36 proof drives the real
`CodexHookRuntime` between cooperative START and COMPLETE frames only inside a
local test; its governed receipt explicitly marks the adapter as neither
packaged nor publishable.

No repository-governed production activation can therefore connect ordinary
Owner-launched `codex` events to the admitted HookStat broker at this prestate.
The following substitutions were rejected because they would violate the G38
authority boundary:

- activating the transparent shim or an old full proxy;
- installing `hookstat-hook` into a production Hook declaration;
- treating the controlled proof or a synthetic emitter as real dogfood;
- inferring events from TabBeacon state, Codex history, or another third path;
- silently building an unreviewed cross-repository TabBeacon integration and
  treating it as an accepted candidate.

The run stopped before live mutation or dogfood. Exact poststate hashes match
the sanitized prestate; no restore write was required. Existing Owner Codex
and TabBeacon processes were left untouched. Two stuck HookStat candidate
processes created only by the initial read-only diagnostic probe were
exact-path verified and removed; the final preflight process audit found zero
HookStat-family processes.

## Diagnostics observation

The initial live `doctor --json` probe appeared hung at the external 30-second
observation boundary. Timing-only local instrumentation localized the complete
read-only run to the legacy receipt scan, not the schema-v2 broker control
query. The temporary instrumentation was removed and a clean candidate was
rebuilt. This is a preflight observation only; it does not claim real broker
diagnostics acceptance and did not loosen the fixed 20-ms broker-query bound.
