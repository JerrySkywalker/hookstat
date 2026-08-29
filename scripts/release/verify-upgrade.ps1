[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateSha,
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1',
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Write-Receipt {
    param([Parameter(Mandatory = $true)][System.Collections.IDictionary]$Receipt)

    foreach ($entry in $Receipt.GetEnumerator()) {
        $value = if ($entry.Value -is [bool]) { $entry.Value.ToString().ToLowerInvariant() } else { [string]$entry.Value }
        "{0}={1}" -f $entry.Key, $value
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
        $parent = Split-Path -Parent $resolvedOutput
        if ([string]::IsNullOrWhiteSpace($parent) -or -not (Test-Path -LiteralPath $parent -PathType Container)) {
            throw 'OutputPath parent must already exist'
        }
        [pscustomobject]$Receipt | ConvertTo-Json |
            Set-Content -LiteralPath $resolvedOutput -Encoding utf8NoBOM
    }
}

$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$actualSha = (& git -C $resolvedRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actualSha -notmatch '^[0-9a-f]{40}$') {
    throw 'upgrade verification could not resolve an exact source HEAD'
}
if ($CandidateSha -notmatch '^[0-9a-f]{40}$' -or $actualSha -ne $CandidateSha) {
    throw "upgrade verification requires HEAD to equal CandidateSha"
}

$receipt = [ordered]@{
    UPGRADE_FIXTURE_VERSION          = 1
    CANDIDATE_SHA                    = $actualSha
    UPGRADE_030_TO_031               = 'FAIL'
    LEGACY_LEDGER_PRESERVED           = $false
    LEGACY_RECEIPT_HISTORY_PRESERVED  = $false
    COMPLETED_EVIDENCE_PRESERVED      = $false
    FAILED_EVIDENCE_PRESERVED         = $false
    INCOMPLETE_EVIDENCE_PRESERVED     = $false
    ALIASES_PRESERVED                 = $false
    REVISION_EPOCHS_PRESERVED         = $false
    INTERFACE_PREFERENCES_PRESERVED   = $false
    OVERALL                           = 'FAIL'
}

try {
    Push-Location $resolvedRoot
    try {
        # This existing isolated fixture constructs a schema-v3 v0.3.0 ledger,
        # reopens it with the candidate, and checks additive schema-v5 migration,
        # completed/failed/incomplete entries, aliases, and ordered revisions.
        & rustup run $RustToolchain cargo test --locked --test g37_migration_shadow_identity v03_fixtures_migrate_additively_and_reopen_idempotently
        if ($LASTEXITCODE -ne 0) {
            throw 'v0.3.0 ledger migration fixture failed'
        }

        # Preferences are intentionally HookStat-owned state. This test retains
        # forward-compatible fields and rejects a stale overwrite in a disposable root.
        & rustup run $RustToolchain cargo test --locked interface_preferences::tests::save_preserves_unknown_fields_and_rejects_a_stale_snapshot
        if ($LASTEXITCODE -ne 0) {
            throw 'interface-preference preservation fixture failed'
        }
    }
    finally {
        Pop-Location
    }

    foreach ($field in @(
        'LEGACY_LEDGER_PRESERVED',
        'LEGACY_RECEIPT_HISTORY_PRESERVED',
        'COMPLETED_EVIDENCE_PRESERVED',
        'FAILED_EVIDENCE_PRESERVED',
        'INCOMPLETE_EVIDENCE_PRESERVED',
        'ALIASES_PRESERVED',
        'REVISION_EPOCHS_PRESERVED',
        'INTERFACE_PREFERENCES_PRESERVED'
    )) {
        $receipt[$field] = $true
    }
    $receipt.UPGRADE_030_TO_031 = 'PASS'
    $receipt.OVERALL = 'PASS'
    Write-Receipt -Receipt $receipt
}
catch {
    Write-Receipt -Receipt $receipt
    throw 'v0.3.0-to-candidate upgrade verification failed'
}
