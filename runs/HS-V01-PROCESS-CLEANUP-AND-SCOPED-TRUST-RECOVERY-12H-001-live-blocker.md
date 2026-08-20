# HS-V01 live dogfood blocker record

Run: `HS-V01-PROCESS-CLEANUP-AND-SCOPED-TRUST-RECOVERY-12H-001`  
Recovery candidate: `5b76fc05f68c9283ea3262d40a0de6e7dd95e433`  
Codex: `codex-cli 0.147.0`

## Completed recovery evidence

- The scoped-trust implementation passed its focused fixtures and the live
  transaction selected, wrote, reloaded, and re-verified exactly 12 current
  HookStat-managed user handlers through Codex App Server `hooks/list` and
  `config/batchWrite` (`hooks.state` upsert of `trusted_hash`).
- Effective live discovery before apply was 16 handlers: 12 supported user
  command handlers and 4 explicit unsupported plugin handlers. No plugin or
  unrelated trust state was selected or changed.
- The Windows Job Object containment gate passed its repeated proxy-only
  cancellation regression test and normal-descendant preservation test.
- The live backup, apply, scoped trust, and exact restore drill all passed.
  Restore returned the supported handlers to the independently captured
  `hooks.json` SHA-256 fingerprint; final Owner configuration state is
  restored off.

## Unresolved real-dogfood admission

The release contract requires real receipts from normal interactive `codex`
sessions. This unattended environment could not safely drive an independently
identified interactive terminal:

- Native noninteractive `codex exec` completed a harmless run but produced no
  HookStat receipts. It is not accepted as a substitute for ordinary
  interactive Codex hook lifecycle evidence.
- The available desktop automation runtime was unavailable. Windows Terminal
  controls could create isolated windows but did not provide a reliable
  input channel. Exact-window foreground authority was rejected on bounded
  retries; no prompt was injected without exact target confirmation.
- No qualifying private prompt, tool payload, or raw hook command was recorded
  in this repository or HookStat evidence by this recovery attempt.

Consequently, real session, invocation, handler, receipt, report, and TUI
counts remain zero for this run. G06 and G07 cannot be closed, so no PR merge,
crates.io publication, tag, or GitHub Release is authorized from this state.

## Required successor admission

Run the already-built candidate in an Owner-accessible interactive Codex TUI
with deterministic, exact-window input control; collect at least the required
multi-session HookStat metadata receipts, validate report/TUI, and then repeat
the final release gates. Re-apply and trust only through HookStat's explicit
apply and scoped-trust commands after fresh preflight.
