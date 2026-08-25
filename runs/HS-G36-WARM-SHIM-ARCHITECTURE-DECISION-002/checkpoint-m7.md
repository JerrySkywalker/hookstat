# M7 — architecture decision

The repeated-fresh one-process shipping frontend remains selected.  Its
measured cache-warmed startup floor is `12.0281` ms p95.  The complete
same-invocation qualification, including the fixed oracle side channel and a
conservative shipping-build correction, is `18.3269/20.5058` ms p95/p99.

The architecture preserves the already accepted reusable START/COMPLETE
producer, Windows overlapped client, ACK-after-WAL-append broker contract,
OS-backed child wait, Job Object containment, private capsule boundary, and
normal `codex` launch.  No helper architecture, third reliability evidence
transport, live activation, or G37 work is justified.

```text
FINAL_G36_SHIM_ARCHITECTURE=ONE_PROCESS_OPTIMIZED
ONE_PROCESS_ARCHITECTURE=VIABLE
HELPER_PROTOTYPE_IMPLEMENTED=false
ARCHITECTURE_ADR=NOT_REQUIRED_ONE_PROCESS_RETAINED
FROZEN_G28_BUDGET_CHANGED=false
NATIVE_ADMISSION_CHANGED=false
G37_STARTED=false
```
