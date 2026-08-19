# HookStat Evidence Contract

Every goal closeout records enough information to reproduce what was proven without committing private runtime content.

Minimum receipt:

```text
GOAL_ID=<HS-...>
START_MAIN=<sha>
FINAL_HEAD=<sha>
RISK_CLASSES=<...>
LOCAL_VALIDATION=<PASS|FAIL|BLOCKED|N/A>
HOSTED_CI=<PASS|FAIL|BLOCKED|N/A>
REAL_RUNTIME_EVIDENCE=<PASS|FAIL|BLOCKED|N/A>
CODEX_MUTATED=false
RAW_PRIVATE_SESSION_CONTENT_COMMITTED=false
UNRELATED_DRIFT_TOUCHED=false
DISPOSITION=<...>
```

Runtime evidence receipts must additionally state:

```text
RUNTIME=codex
RUNTIME_VERSION=<...>
EVIDENCE_SOURCE=<...>
COVERAGE=<COMPLETE|PARTIAL|SYNC_ONLY|BEST_EFFORT|UNKNOWN>
HANDLER_IDENTITY_PROVEN=<true|false>
INVOCATION_DENOMINATOR_PROVEN=<true|false>
TERMINAL_STATUS_PROVEN=<true|false>
```

Evidence architecture must additionally state one of:

```text
EVIDENCE_SOURCE_CLASS=PassiveEvidenceSource|InstrumentedEvidenceSource
PASSIVE_EVIDENCE_PREFERRED=true
INSTRUMENTATION_OPT_IN=<true|false>
OWNER_LIVE_CODEX_MUTATED=<true|false>
PROXY_STREAM_CONTENT_PERSISTED=false
```

Instrumented receipts may contain only bounded invocation metadata: opaque
invocation id, handler key/revision, event, source kind, execution mode,
timestamps/duration, exit/result taxonomy, and coverage. Exact configuration
backup/restore material is private local control-plane data, never ledger,
report, fixture, or committed evidence.

Never commit unsanitized prompts, tool arguments, tokens, credentials, or full personal session transcripts as evidence.
