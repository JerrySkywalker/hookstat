[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateSha,
    [string]$ExpectedVersion = '0.4.0',
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1',
    [string]$OutputPath,
    [switch]$KeepLab
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Result {
    param([Parameter(Mandatory = $true)][System.Collections.IDictionary]$Result)

    foreach ($entry in $Result.GetEnumerator()) {
        "{0}={1}" -f $entry.Key, [string]$entry.Value
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
        $parent = Split-Path -Parent $resolvedOutput
        if ([string]::IsNullOrWhiteSpace($parent) -or -not (Test-Path -LiteralPath $parent -PathType Container)) {
            throw 'OutputPath parent must already exist'
        }
        [pscustomobject]$Result | ConvertTo-Json -Depth 4 |
            Set-Content -LiteralPath $resolvedOutput -Encoding utf8NoBOM
    }
}

function Invoke-CheckedNative {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Command,
        [Parameter(Mandatory = $true)][string]$FailureMessage
    )

    $output = @(& $Command 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw $FailureMessage
    }
    return $output
}

$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$result = [ordered]@{
    RELEASE_GATE_VERSION        = 2
    CANDIDATE_SHA               = $CandidateSha
    VERSION                     = 'UNVERIFIED'
    PACKAGE                     = 'NOT_RUN'
    PUBLISH_DRY_RUN             = 'NOT_RUN'
    FRESH_INSTALL               = 'NOT_RUN'
    UPGRADE_FROM_PUBLIC_BASELINE = 'NOT_RUN'
    UPGRADE_031_TO_040          = 'NOT_RUN'
    LEDGER_PRESERVED             = 'NOT_RUN'
    LEGACY_EVIDENCE_PRESERVED   = 'NOT_RUN'
    PREFERENCES_PRESERVED        = 'NOT_RUN'
    RUNTIME_PRESENTATION_PERSISTED = 'UNVERIFIED'
    REPORT_SMOKE                = 'NOT_RUN'
    DOCTOR_SMOKE                = 'NOT_RUN'
    TUI_DETERMINISTIC_TESTS      = 'NOT_RUN'
    TUI_GOLDEN_BASELINES         = 'NOT_RUN'
    TUI_STRUCTURAL_INVARIANTS    = 'NOT_RUN'
    REAL_WIRE_TO_FRAME_E2E       = 'NOT_RUN'
    OVERALL                     = 'FAIL'
}
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-release-gate-' + [guid]::NewGuid().ToString('N'))

try {
    if ($CandidateSha -notmatch '^[0-9a-f]{40}$') {
        throw 'CandidateSha must be a full lowercase Git SHA'
    }
    $actualSha = (& git -C $resolvedRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actualSha -ne $CandidateSha) {
        throw 'release gate requires HEAD to equal CandidateSha exactly'
    }
    $worktreeStatus = @(& git -C $resolvedRoot status --porcelain=v1)
    if ($LASTEXITCODE -ne 0 -or $worktreeStatus.Count -ne 0) {
        throw 'release gate requires a completely clean working tree'
    }

    $manifest = Join-Path $resolvedRoot 'Cargo.toml'
    $versionLine = Get-Content -LiteralPath $manifest | Where-Object { $_ -match '^version\s*=\s*"([^"]+)"\s*$' } | Select-Object -First 1
    if ($null -eq $versionLine) {
        throw 'could not resolve package version from Cargo.toml'
    }
    $manifestVersion = ([regex]::Match($versionLine, '^version\s*=\s*"([^"]+)"\s*$')).Groups[1].Value
    if ($manifestVersion -ne $ExpectedVersion) {
        throw "candidate version mismatch: expected=$ExpectedVersion actual=$manifestVersion"
    }
    $result.CANDIDATE_SHA = $actualSha
    $result.VERSION = $manifestVersion

    New-Item -ItemType Directory -Path $lab | Out-Null
    $target = Join-Path $lab 'target'
    $cargoHome = Join-Path $lab 'cargo-home'
    $upgradeReceipt = Join-Path $lab 'upgrade-receipt.json'
    New-Item -ItemType Directory -Path $target, $cargoHome | Out-Null
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_BUILD_JOBS = '1'
    $env:RUSTFLAGS = '-C debuginfo=0'
    Remove-Item Env:RUSTUP_HOME -ErrorAction SilentlyContinue
    # Publish dry-run never needs an Owner registry credential. Keep its Cargo
    # home disposable so it cannot inspect or use a real credential store.
    $env:CARGO_HOME = $cargoHome

    $packageScript = Join-Path $resolvedRoot 'scripts/release/verify-package.ps1'
    $packageOutput = Invoke-CheckedNative -FailureMessage 'existing package verification failed' -Command {
        & pwsh -NoProfile -File $packageScript -RepositoryRoot $resolvedRoot -RustToolchain $RustToolchain -ExpectedVersion $ExpectedVersion
    }
    $packageText = $packageOutput -join "`n"
    foreach ($required in @(
        'PACKAGE_ARCHIVE_SELF_CONTAINED=true',
        'FRESH_INSTALL_REQUIRED_PACKAGED_BINARIES=true',
        'FRESH_INSTALL_TRANSPARENT_SHIM_ADMISSION=qualified_not_admitted_performance',
        "FRESH_INSTALL_VERSION=$ExpectedVersion",
        'FRESH_INSTALL_REPORT_SMOKE=true',
        'FRESH_INSTALL_DOCTOR_SMOKE=true',
        'FRESH_INSTALL_TUI_FRAME_SMOKE=true'
    )) {
        if ($packageText -notmatch [regex]::Escape($required)) {
            throw 'existing package verification omitted a required receipt field'
        }
    }
    $result.PACKAGE = 'PASS'
    $result.FRESH_INSTALL = 'PASS'

    Push-Location $resolvedRoot
    try {
        Invoke-CheckedNative -FailureMessage 'cargo publish --dry-run failed' -Command {
            & rustup run $RustToolchain cargo publish --dry-run --locked
        } | Out-Null
        $result.PUBLISH_DRY_RUN = 'PASS'

        $upgradeScript = Join-Path $resolvedRoot 'scripts/release/verify-upgrade.ps1'
        $upgradeOutput = Invoke-CheckedNative -FailureMessage 'upgrade fixture failed' -Command {
            & pwsh -NoProfile -File $upgradeScript -CandidateSha $actualSha -RepositoryRoot $resolvedRoot -RustToolchain $RustToolchain -OutputPath $upgradeReceipt
        }
        $upgrade = Get-Content -LiteralPath $upgradeReceipt -Raw | ConvertFrom-Json
        if ($upgrade.OVERALL -ne 'PASS' -or $upgrade.UPGRADE_031_TO_040 -ne 'PASS') {
            throw 'upgrade fixture did not produce a passing receipt'
        }
        foreach ($field in @(
            'LEDGER_PRESERVED',
            'LEGACY_EVIDENCE_PRESERVED',
            'PREFERENCES_PRESERVED',
            'V031_PUBLIC_BINARY',
            'V031_RECEIPT_SPOOL_RECONCILED',
            'V031_RECEIPT_JOURNAL_PRESERVED',
            'CANDIDATE_RECEIPT_HISTORY_PRESERVED'
        )) {
            if ($upgrade.$field -ne $true) {
                throw "upgrade fixture did not prove $field"
            }
        }
        if ($upgrade.RUNTIME_PRESENTATION_PERSISTED -ne $false) {
            throw 'upgrade fixture did not prove runtime presentation remains ephemeral'
        }
        $result.UPGRADE_031_TO_040 = 'PASS'
        $result.UPGRADE_FROM_PUBLIC_BASELINE = 'PASS'
        $result.LEDGER_PRESERVED = 'PASS'
        $result.LEGACY_EVIDENCE_PRESERVED = 'PASS'
        $result.PREFERENCES_PRESERVED = 'PASS'
        $result.RUNTIME_PRESENTATION_PERSISTED = 'false'

        Invoke-CheckedNative -FailureMessage 'deterministic TUI visual and real-wire gate failed' -Command {
            & rustup run $RustToolchain cargo test --locked --lib tui_visual -- --nocapture
        } | Out-Null
        $result.TUI_DETERMINISTIC_TESTS = 'PASS'
        $result.TUI_GOLDEN_BASELINES = 'PASS'
        $result.TUI_STRUCTURAL_INVARIANTS = 'PASS'
        $result.REAL_WIRE_TO_FRAME_E2E = 'PASS'
        $result.REPORT_SMOKE = 'PASS'
        $result.DOCTOR_SMOKE = 'PASS'
    }
    finally {
        Pop-Location
    }

    $result.OVERALL = 'PASS'
    Write-Result -Result $result
}
catch {
    Write-Result -Result $result
    throw 'release gate failed; inspect the bounded command output above without persisting it as a receipt'
}
finally {
    if (-not $KeepLab -and (Test-Path -LiteralPath $lab)) {
        $resolvedLab = (Resolve-Path -LiteralPath $lab).Path
        if (-not $resolvedLab.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'refusing to remove a release-gate path outside the temporary root'
        }
        Remove-Item -LiteralPath $resolvedLab -Recurse -Force
    }
}
