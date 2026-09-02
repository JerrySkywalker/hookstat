[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-ReceiptField {
    param(
        [Parameter(Mandatory = $true)][string]$Text,
        [Parameter(Mandatory = $true)][string]$Field
    )

    if ($Text -notmatch [regex]::Escape($Field)) {
        throw "release-gate regression test omitted $Field"
    }
}

function Invoke-ExpectedFailure {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    $output = @(& pwsh @Arguments 2>&1)
    if ($LASTEXITCODE -eq 0) {
        throw "$Name unexpectedly succeeded"
    }
    $text = $output -join "`n"
    if ($text -match 'OVERALL=PASS') {
        throw "$Name reported a final release-gate PASS"
    }
    "${Name}=PASS"
}

$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$candidateSha = (& git -C $resolvedRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $candidateSha -notmatch '^[0-9a-f]{40}$') {
    throw 'release-gate regression test could not resolve an exact candidate SHA'
}
$expectedOwnerMain = '6125734fdbc3edbe33712929abcd4cd1e0e07e1b'
$releaseGate = Join-Path $resolvedRoot 'scripts/release/release-gate.ps1'
$verifyPackage = Join-Path $resolvedRoot 'scripts/release/verify-package.ps1'
$testLab = Join-Path ([System.IO.Path]::GetTempPath()) ('hookstat-release-gate-test-' + [guid]::NewGuid().ToString('N'))

try {
    New-Item -ItemType Directory -Path $testLab | Out-Null
    # This is a synthetic poison marker, never an Owner credential. The probe
    # must create and bind a different disposable Cargo home without reading it.
    $poisonCargoHome = Join-Path $testLab 'poisoned-owner-cargo-home'
    New-Item -ItemType Directory -Path $poisonCargoHome | Out-Null
    Set-Content -LiteralPath (Join-Path $poisonCargoHome 'credentials.toml') -Value 'token = "synthetic-poison-not-a-credential"' -Encoding utf8NoBOM
    $env:CARGO_HOME = $poisonCargoHome

    $packageProbe = @(& pwsh -NoProfile -File $verifyPackage -RepositoryRoot $resolvedRoot -RustToolchain $RustToolchain -IsolationProbeOnly 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw 'verify-package Cargo-home isolation probe failed'
    }
    $packageProbeText = $packageProbe -join "`n"
    Assert-ReceiptField -Text $packageProbeText -Field 'VERIFY_PACKAGE_CARGO_HOME_ISOLATED=true'
    Assert-ReceiptField -Text $packageProbeText -Field 'OWNER_CARGO_CREDENTIAL_STORE_USED=false'

    $ownerMetadata = @(
        '-OwnerG45R', 'PASS',
        '-OwnerG45RReceiptId', 'test-owner-g45r-receipt',
        '-OwnerG45RTestedMain', $expectedOwnerMain,
        '-OwnerG45RNoHistoryPresentation', 'PASS',
        '-OwnerG45RLiveReliabilitySmoke', 'BOUNDED_UNAVAILABLE_ACCEPTED'
    )
    $reviewMetadata = @(
        '-IndependentReviewResult', 'PASS',
        '-IndependentReviewReceiptId', 'test-independent-review-receipt',
        '-IndependentReviewSha', $candidateSha
    )
    $baseArguments = @(
        '-NoProfile',
        '-File', $releaseGate,
        '-CandidateSha', $candidateSha,
        '-RepositoryRoot', $resolvedRoot,
        '-RustToolchain', $RustToolchain,
        '-PreflightOnly'
    )

    Invoke-ExpectedFailure -Name 'NO_OWNER_RECEIPT_FAIL_CLOSED' -Arguments $baseArguments
    Invoke-ExpectedFailure -Name 'NO_INDEPENDENT_REVIEW_FAIL_CLOSED' -Arguments @($baseArguments + $ownerMetadata)
    Invoke-ExpectedFailure -Name 'WRONG_REVIEW_SHA_FAIL_CLOSED' -Arguments @(
        $baseArguments + $ownerMetadata + @(
            '-IndependentReviewResult', 'PASS',
            '-IndependentReviewReceiptId', 'test-independent-review-receipt',
            '-IndependentReviewSha', ('0' * 40)
        )
    )
    Invoke-ExpectedFailure -Name 'REVIEW_RESULT_FINDINGS_FAIL_CLOSED' -Arguments @(
        $baseArguments + $ownerMetadata + @(
            '-IndependentReviewResult', 'FINDINGS',
            '-IndependentReviewReceiptId', 'test-independent-review-receipt',
            '-IndependentReviewSha', $candidateSha
        )
    )

    $preflight = @(& pwsh @($baseArguments + $ownerMetadata + $reviewMetadata) 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw 'valid synthetic preflight metadata did not reach the controlled downstream isolation path'
    }
    $preflightText = $preflight -join "`n"
    foreach ($field in @(
        'OWNER_G45R=PASS',
        "OWNER_G45R_TESTED_MAIN=$expectedOwnerMain",
        "INDEPENDENT_REVIEW_SHA=$candidateSha",
        'RELEASE_GATE_CARGO_HOME_ISOLATED=true',
        'VERIFY_PACKAGE_CARGO_HOME_ISOLATED=true',
        'OWNER_CARGO_CREDENTIAL_STORE_USED=false',
        'OVERALL=NOT_RUN_PREFLIGHT_ONLY'
    )) {
        Assert-ReceiptField -Text $preflightText -Field $field
    }

    'VERIFY_PACKAGE_CARGO_HOME_ISOLATED=true'
    'RELEASE_GATE_CARGO_HOME_ISOLATED=true'
    'OWNER_CARGO_CREDENTIAL_STORE_USED=false'
    'NO_OWNER_RECEIPT_FAIL_CLOSED=true'
    'NO_INDEPENDENT_REVIEW_FAIL_CLOSED=true'
    'WRONG_REVIEW_SHA_FAIL_CLOSED=true'
    'REVIEW_RESULT_FINDINGS_FAIL_CLOSED=true'
    'VALID_SYNTHETIC_PREFLIGHT_ONLY=true'
}
finally {
    if (Test-Path -LiteralPath $testLab) {
        $resolvedLab = (Resolve-Path -LiteralPath $testLab).Path
        $tempRoot = [System.IO.Path]::GetTempPath()
        if (-not $resolvedLab.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'refusing to remove a release-gate regression path outside the temporary root'
        }
        Remove-Item -LiteralPath $resolvedLab -Recurse -Force
    }
}
