# HookStat v0.1 Owner activation runbook

This is the short attended sequence for the release candidate. It changes only
the Codex configuration root you explicitly pass to `--apply`. Normal use stays
`codex`; do not launch `hookstat codex`.

## 1. Sync and install the candidate

```powershell
git switch main
git pull --ff-only
cargo build --release --locked
cargo install --path . --locked --force
hookstat --version
```

## 2. Review the read-only plan

```powershell
$configRoot = Join-Path $env:USERPROFILE '.codex'
hookstat codex instrument --dry-run
```

Review the reported handler counts, hashes, runtime-effective reconciliation,
and unsupported coverage. The output must not contain raw hook commands. Stop
here if the selected root is not the one you intend to change.

## 3. Apply with an explicit root

```powershell
hookstat codex instrument --apply --config-root $configRoot
```

HookStat makes an exact local prestate backup and an atomic rollback journal.
It never edits Codex trust. If Codex asks to review trust because hook commands
changed, perform that review in Codex; do not bypass or auto-approve it.

## 4. Use Codex normally

```powershell
codex
```

Run a small safe workload that triggers representative configured hooks, then
exit Codex normally. Do not use a HookStat launcher wrapper.

## 5. Inspect real evidence

```powershell
hookstat report
hookstat report --json
hookstat
```

Confirm at least one real per-handler invocation, its sample count, terminal
breakdown, and visible coverage. An empty report or incomplete-only evidence is
not a healthy `0.00%` result.

## Emergency rollback

```powershell
hookstat codex instrument --restore --config-root $configRoot
hookstat codex instrument --dry-run
```

Restore refuses configuration drift rather than overwriting a changed file. If
that occurs, preserve the changed configuration and review it before choosing a
manual recovery plan; do not hand-edit solely to satisfy HookStat.
