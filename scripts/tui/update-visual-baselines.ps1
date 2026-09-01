[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repository = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$gitTopLevelRaw = (& git -C $repository rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "RepositoryRoot is not a Git worktree: $repository"
}
$gitTopLevel = (Resolve-Path -LiteralPath $gitTopLevelRaw).Path
if ($gitTopLevel -ne $repository) {
    throw "RepositoryRoot must be the exact HookStat worktree root: $repository"
}

$baselinePath = 'tests/fixtures/tui_visual'
$previousUpdate = [Environment]::GetEnvironmentVariable('HOOKSTAT_UPDATE_VISUAL_BASELINES', 'Process')

Push-Location $repository
try {
    [Environment]::SetEnvironmentVariable('HOOKSTAT_UPDATE_VISUAL_BASELINES', '1', 'Process')
    & cargo test --locked --lib tui::tui_visual::update_tui_visual_baselines -- --ignored --exact --nocapture
    if ($LASTEXITCODE -ne 0) {
        throw 'structural checks rejected the candidate baselines'
    }
}
finally {
    [Environment]::SetEnvironmentVariable('HOOKSTAT_UPDATE_VISUAL_BASELINES', $previousUpdate, 'Process')
    Pop-Location
}

& git -C $repository status --short -- $baselinePath
if ($LASTEXITCODE -ne 0) {
    throw 'could not enumerate changed TUI visual baselines'
}

'BASELINE_UPDATE_COMMIT=false'
'BASELINE_UPDATE_PUSH=false'
'CI_AUTO_ACCEPT_BASELINE=false'
