[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$TabBeaconRemote = 'https://github.com/JerrySkywalker/tabbeacon.git',
    [string]$TabBeaconCommit = 'b3f5685c37f1386f3edceb6d1d3a27403c59dddf',
    [string]$RustToolchain = '1.97.1',
    [Parameter(Mandatory = $true)][string]$Output,
    [switch]$KeepLab
)

$ErrorActionPreference = 'Stop'
$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$sourceHead = (& git -C $resolvedRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $sourceHead -notmatch '^[0-9a-f]{40}$') {
    throw 'cooperative proof could not resolve the HookStat source HEAD'
}
$trackedStatus = @(& git -C $resolvedRoot status --porcelain=v1 --untracked-files=no)
if ($LASTEXITCODE -ne 0 -or $trackedStatus.Count -ne 0) {
    throw 'cooperative proof requires a tracked-clean HookStat source HEAD'
}

$proofManifest = Join-Path $resolvedRoot 'dev_proof/hookstat-ipc-client/Cargo.toml'
$proofTest = Join-Path $resolvedRoot 'dev_proof/tabbeacon_g36_cooperative.rs'
$clientSource = Join-Path $resolvedRoot 'src/ipc_client.rs'
$clientSourceSha256 = (Get-FileHash -LiteralPath $clientSource -Algorithm SHA256).Hash.ToLowerInvariant()
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-tabbeacon-proof-' + [guid]::NewGuid().ToString('N'))
$tabbeacon = Join-Path $lab 'tabbeacon'
$target = Join-Path $lab 'target'
$resolvedOutput = [System.IO.Path]::GetFullPath($Output)

try {
    New-Item -ItemType Directory -Path $lab, $target | Out-Null
    & git clone --quiet --no-checkout $TabBeaconRemote $tabbeacon
    if ($LASTEXITCODE -ne 0) { throw 'TabBeacon disposable clone failed' }
    & git -C $tabbeacon checkout --quiet --detach $TabBeaconCommit
    if ($LASTEXITCODE -ne 0) { throw 'TabBeacon pinned checkout failed' }
    $actualTabBeaconCommit = (& git -C $tabbeacon rev-parse HEAD).Trim()
    if ($actualTabBeaconCommit -ne $TabBeaconCommit) {
        throw 'TabBeacon pinned checkout identity mismatch'
    }

    $hookstatTomlPath = $resolvedRoot.Replace('\', '/')
    $proofTomlPath = (Split-Path -Parent $proofManifest).Replace('\', '/')
    Add-Content -LiteralPath (Join-Path $tabbeacon 'Cargo.toml') -Value @"

[dev-dependencies.hookstat]
path = '$hookstatTomlPath'

[dev-dependencies.hookstat-ipc-client-proof]
path = '$proofTomlPath'
"@
    Copy-Item -LiteralPath $proofTest -Destination (Join-Path $tabbeacon 'tests/g36_hookstat_cooperative.rs')

    $env:CARGO_TARGET_DIR = $target
    & rustup run $RustToolchain cargo test --manifest-path (Join-Path $tabbeacon 'Cargo.toml') --test g36_hookstat_cooperative
    if ($LASTEXITCODE -ne 0) { throw 'current-source TabBeacon cooperative proof failed' }

    $tree = @(& rustup run $RustToolchain cargo tree --manifest-path $proofManifest --locked --target all --edges normal --prefix none)
    if ($LASTEXITCODE -ne 0) { throw 'proof-adapter dependency audit failed' }
    $forbidden = @($tree | Where-Object { $_ -match '^(ratatui|crossterm|rusqlite|hookstat v)' })
    if ($forbidden.Count -ne 0) {
        throw 'proof adapter acquired a forbidden HookStat product dependency'
    }

    $receipt = [ordered]@{
        schema_version = 1
        run_kind = 'g36_current_boundary_tabbeacon_cooperative_proof'
        classification = 'FULL_ACCEPTANCE_PASS'
        hookstat_source_git_head = $sourceHead
        hookstat_ipc_client_source_sha256 = $clientSourceSha256
        tabbeacon_git_head = $actualTabBeaconCommit
        current_source_adapter = $true
        adapter_publishable = $false
        adapter_packaged = $false
        real_g35_broker_start_complete = $true
        real_tabbeacon_codex_hook_runtime = $true
        tabbeacon_declaration_has_hookstat_wrapper = $false
        product_dependency_boundary = 'PASS'
        owner_live_codex_config_mutated = $false
        raw_private_content_captured = $false
    }
    $outputParent = Split-Path -Parent $resolvedOutput
    if (-not [string]::IsNullOrWhiteSpace($outputParent)) {
        New-Item -ItemType Directory -Path $outputParent -Force | Out-Null
    }
    $receipt | ConvertTo-Json | Set-Content -LiteralPath $resolvedOutput -Encoding utf8NoBOM
    'TABBEACON_COOPERATIVE_PROOF=PASS'
    "HOOKSTAT_SOURCE_GIT_HEAD=$sourceHead"
    "TABBEACON_GIT_HEAD=$actualTabBeaconCommit"
    "HOOKSTAT_IPC_CLIENT_SOURCE_SHA256=$clientSourceSha256"
}
finally {
    if (-not $KeepLab -and (Test-Path -LiteralPath $lab)) {
        $resolvedLab = (Resolve-Path -LiteralPath $lab).Path
        if (-not $resolvedLab.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'refusing to remove a cooperative-proof path outside the temporary root'
        }
        Remove-Item -LiteralPath $resolvedLab -Recurse -Force
    }
}
