[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateSha,
    [string]$ExpectedVersion = '0.4.0',
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1',
    [string]$OutputPath,
    [string]$OwnerG45R,
    [string]$OwnerG45RReceiptId,
    [string]$OwnerG45RTestedMain,
    [string]$OwnerG45RNoHistoryPresentation,
    [string]$OwnerG45RLiveReliabilitySmoke,
    [string]$IndependentReviewResult,
    [string]$IndependentReviewReceiptId,
    [string]$IndependentReviewSha,
    [switch]$PreflightOnly,
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

function Assert-NonEmptyReceiptId {
    param(
        [AllowEmptyString()][string]$Value,
        [Parameter(Mandatory = $true)][string]$Name
    )

    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw "$Name must be a non-empty durable receipt identifier"
    }
}

function Assert-FullLowercaseSha {
    param(
        [AllowEmptyString()][string]$Value,
        [Parameter(Mandatory = $true)][string]$Name
    )

    if ($Value -notmatch '^[0-9a-f]{40}$') {
        throw "$Name must be a full lowercase Git SHA"
    }
}

$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$requiredOwnerG45RTestedMain = '6125734fdbc3edbe33712929abcd4cd1e0e07e1b'
$ownerDogfoodHumanSurfacePathspecs = @('src/tui', 'src/runtime_presentation.rs')
$result = [ordered]@{
    RELEASE_GATE_VERSION        = 3
    CANDIDATE_SHA               = $CandidateSha
    VERSION                     = 'UNVERIFIED'
    OWNER_G45R                  = 'NOT_RUN'
    OWNER_G45R_RECEIPT          = 'NOT_RUN'
    OWNER_G45R_TESTED_MAIN      = 'UNVERIFIED'
    OWNER_G45R_NO_HISTORY_PRESENTATION = 'UNVERIFIED'
    OWNER_G45R_LIVE_RELIABILITY_SMOKE = 'UNVERIFIED'
    OWNER_G45R_HUMAN_SURFACE    = 'UNVERIFIED'
    INDEPENDENT_REVIEW           = 'NOT_RUN'
    INDEPENDENT_REVIEW_RECEIPT   = 'NOT_RUN'
    INDEPENDENT_REVIEW_SHA       = 'UNVERIFIED'
    CARGO_HOME_ISOLATION         = 'NOT_RUN'
    RELEASE_GATE_CARGO_HOME_ISOLATED = 'false'
    VERIFY_PACKAGE_CARGO_HOME_ISOLATED = 'false'
    OWNER_CARGO_CREDENTIAL_STORE_USED = 'UNVERIFIED'
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
    PREFLIGHT_ONLY                = 'false'
    OVERALL                     = 'FAIL'
}
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-release-gate-' + [guid]::NewGuid().ToString('N'))

try {
    Assert-FullLowercaseSha -Value $CandidateSha -Name 'CandidateSha'
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

    if ([string]::IsNullOrWhiteSpace($OwnerG45R)) {
        throw 'OWNER_G45R=PASS external acceptance metadata is required'
    }
    $result.OWNER_G45R = $OwnerG45R
    if ($OwnerG45R -ne 'PASS') {
        throw 'OWNER_G45R must equal PASS'
    }
    Assert-NonEmptyReceiptId -Value $OwnerG45RReceiptId -Name 'OWNER_G45R_RECEIPT_ID'
    $result.OWNER_G45R_RECEIPT = 'PASS'
    Assert-FullLowercaseSha -Value $OwnerG45RTestedMain -Name 'OWNER_G45R_TESTED_MAIN'
    $result.OWNER_G45R_TESTED_MAIN = $OwnerG45RTestedMain
    if ($OwnerG45RTestedMain -ne $requiredOwnerG45RTestedMain) {
        throw "OWNER_G45R_TESTED_MAIN must equal $requiredOwnerG45RTestedMain"
    }
    & git -C $resolvedRoot merge-base --is-ancestor $OwnerG45RTestedMain $actualSha
    if ($LASTEXITCODE -ne 0) {
        throw 'OWNER_G45R_TESTED_MAIN must be an ancestor of CandidateSha'
    }
    $result.OWNER_G45R_NO_HISTORY_PRESENTATION = $OwnerG45RNoHistoryPresentation
    if ($OwnerG45RNoHistoryPresentation -ne 'PASS') {
        throw 'OWNER_G45R_NO_HISTORY_PRESENTATION must equal PASS'
    }
    $result.OWNER_G45R_LIVE_RELIABILITY_SMOKE = $OwnerG45RLiveReliabilitySmoke
    if ($OwnerG45RLiveReliabilitySmoke -ne 'BOUNDED_UNAVAILABLE_ACCEPTED') {
        throw 'OWNER_G45R_LIVE_RELIABILITY_SMOKE must equal BOUNDED_UNAVAILABLE_ACCEPTED'
    }
    $humanSurfaceChanges = @(& git -C $resolvedRoot diff --name-only "$OwnerG45RTestedMain..$actualSha" -- $ownerDogfoodHumanSurfacePathspecs)
    if ($LASTEXITCODE -ne 0) {
        throw 'could not determine whether CandidateSha changes the Owner-dogfood Human surface'
    }
    if ($humanSurfaceChanges.Count -ne 0) {
        $result.OWNER_G45R_HUMAN_SURFACE = 'RENEWED_DOGFOOD_REQUIRED'
        throw 'candidate changes the Owner-dogfood Human surface; renewed Owner G45R evidence is required'
    }
    $result.OWNER_G45R_HUMAN_SURFACE = 'PASS'

    if ([string]::IsNullOrWhiteSpace($IndependentReviewResult)) {
        throw 'INDEPENDENT_REVIEW=PASS external acceptance metadata is required'
    }
    $result.INDEPENDENT_REVIEW = $IndependentReviewResult
    if ($IndependentReviewResult -ne 'PASS') {
        throw 'INDEPENDENT_REVIEW must equal PASS'
    }
    Assert-NonEmptyReceiptId -Value $IndependentReviewReceiptId -Name 'INDEPENDENT_REVIEW_RECEIPT_ID'
    $result.INDEPENDENT_REVIEW_RECEIPT = 'PASS'
    Assert-FullLowercaseSha -Value $IndependentReviewSha -Name 'INDEPENDENT_REVIEW_SHA'
    $result.INDEPENDENT_REVIEW_SHA = $IndependentReviewSha
    if ($IndependentReviewSha -ne $actualSha) {
        throw 'INDEPENDENT_REVIEW_SHA must equal CandidateSha exactly'
    }

    New-Item -ItemType Directory -Path $lab | Out-Null
    $target = Join-Path $lab 'target'
    $cargoHome = Join-Path $lab 'cargo-home'
    $upgradeReceipt = Join-Path $lab 'upgrade-receipt.json'
    New-Item -ItemType Directory -Path $target | Out-Null
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_BUILD_JOBS = '1'
    $env:RUSTFLAGS = '-C debuginfo=0'
    Remove-Item Env:RUSTUP_HOME -ErrorAction SilentlyContinue
    # Publish dry-run never needs an Owner registry credential. Keep its Cargo
    # home disposable so it cannot inspect or use a real credential store.
    $env:CARGO_HOME = $cargoHome
    if (-not ([System.IO.Path]::GetFullPath($env:CARGO_HOME).Equals(
                [System.IO.Path]::GetFullPath($cargoHome),
                [System.StringComparison]::OrdinalIgnoreCase))) {
        throw 'release gate failed to bind its disposable Cargo home'
    }

    $packageScript = Join-Path $resolvedRoot 'scripts/release/verify-package.ps1'
    $packageArguments = @(
        '-NoProfile',
        '-File', $packageScript,
        '-RepositoryRoot', $resolvedRoot,
        '-RustToolchain', $RustToolchain,
        '-ExpectedVersion', $ExpectedVersion,
        '-CargoHome', $cargoHome
    )
    if ($PreflightOnly) {
        $packageArguments += '-IsolationProbeOnly'
    }
    $packageOutput = Invoke-CheckedNative -FailureMessage 'existing package verification failed' -Command {
        & pwsh @packageArguments
    }
    $packageText = $packageOutput -join "`n"
    foreach ($required in @(
        'VERIFY_PACKAGE_CARGO_HOME_ISOLATED=true',
        'OWNER_CARGO_CREDENTIAL_STORE_USED=false'
    )) {
        if ($packageText -notmatch [regex]::Escape($required)) {
            throw 'package verification omitted the required Cargo-home isolation receipt field'
        }
    }
    $result.CARGO_HOME_ISOLATION = 'PASS'
    $result.RELEASE_GATE_CARGO_HOME_ISOLATED = 'true'
    $result.VERIFY_PACKAGE_CARGO_HOME_ISOLATED = 'true'
    $result.OWNER_CARGO_CREDENTIAL_STORE_USED = 'false'
    if ($PreflightOnly) {
        $result.PREFLIGHT_ONLY = 'true'
        $result.OVERALL = 'NOT_RUN_PREFLIGHT_ONLY'
        Write-Result -Result $result
        return
    }
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
