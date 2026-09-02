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

function Invoke-ReportJson {
    param(
        [Parameter(Mandatory = $true)][string]$Binary,
        [Parameter(Mandatory = $true)][string]$DataRoot,
        [Parameter(Mandatory = $true)][string]$FailureMessage
    )

    $output = @(& $Binary report --json --data-root $DataRoot 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw $FailureMessage
    }
    try {
        return (($output -join "`n") | ConvertFrom-Json)
    }
    catch {
        throw $FailureMessage
    }
}

function Assert-LegacyReceiptReport {
    param([Parameter(Mandatory = $true)]$Report)

    $handlers = @($Report.handlers | Where-Object { $_.handler.key -eq 'hk_v031_upgrade' })
    if ($Report.report_kind -ne 'instrumented_codex' -or
        $Report.malformed_receipts -ne 0 -or
        $Report.incomplete_receipts -ne 1 -or
        $handlers.Count -ne 1 -or
        $handlers[0].runs -ne 2 -or
        $handlers[0].failure_sample_count -ne 1 -or
        $handlers[0].failed_runs -ne 0) {
        throw 'legacy receipt history report did not retain the completed and incomplete v0.3.1 invocations'
    }
}

function Write-JsonFixture {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$Value
    )

    $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText(
        $Path,
        ($Value | ConvertTo-Json -Compress -Depth 8),
        $utf8NoBom
    )
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
    UPGRADE_FIXTURE_VERSION          = 2
    CANDIDATE_SHA                    = $actualSha
    UPGRADE_031_TO_040               = 'FAIL'
    LEDGER_PRESERVED                  = $false
    LEGACY_EVIDENCE_PRESERVED         = $false
    PREFERENCES_PRESERVED             = $false
    RUNTIME_PRESENTATION_PERSISTED    = $true
    LEGACY_LEDGER_PRESERVED           = $false
    LEGACY_RECEIPT_HISTORY_PRESERVED  = $false
    COMPLETED_EVIDENCE_PRESERVED      = $false
    FAILED_EVIDENCE_PRESERVED         = $false
    INCOMPLETE_EVIDENCE_PRESERVED     = $false
    ALIASES_PRESERVED                 = $false
    REVISION_EPOCHS_PRESERVED         = $false
    INTERFACE_PREFERENCES_PRESERVED   = $false
    V031_PUBLIC_BINARY                = $false
    V031_RECEIPT_SPOOL_RECONCILED     = $false
    V031_RECEIPT_JOURNAL_PRESERVED    = $false
    CANDIDATE_RECEIPT_HISTORY_PRESERVED = $false
    OVERALL                           = 'FAIL'
}
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-upgrade-verify-' + [guid]::NewGuid().ToString('N'))

try {
    New-Item -ItemType Directory -Path $lab | Out-Null
    $legacyInstallRoot = Join-Path $lab 'v031-install'
    $candidateInstallRoot = Join-Path $lab 'candidate-install'
    $legacyDataRoot = Join-Path $lab 'v031-data'
    $recordsRoot = Join-Path $legacyDataRoot 'receipts/records'
    New-Item -ItemType Directory -Path $legacyInstallRoot, $candidateInstallRoot, $recordsRoot | Out-Null

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

        # The current runtime catalog is intentionally a non-serializable
        # in-memory value. Exercise the focused privacy regression before this
        # upgrade receipt can assert that an upgrade has not made it durable.
        & rustup run $RustToolchain cargo test --locked runtime_presentation::tests::preserves_codex_human_fields_only_in_memory
        if ($LASTEXITCODE -ne 0) {
            throw 'runtime presentation ephemerality fixture failed'
        }

        # Exercise the public v0.3.1 binary itself against its published receipt
        # contract. This uses a disposable Cargo/install root and data root; it
        # neither reads Owner state nor touches Codex configuration. The receipt
        # journal models a v0.3.1 user-data root processed by that public binary;
        # it is not presented as Owner history or a fabricated live observation.
        & rustup run $RustToolchain cargo install hookstat --version 0.3.1 --locked --root $legacyInstallRoot
        if ($LASTEXITCODE -ne 0) {
            throw 'public v0.3.1 HookStat installation failed'
        }
        $extension = if ($env:OS -eq 'Windows_NT') { '.exe' } else { '' }
        $legacyBinary = Join-Path $legacyInstallRoot ("bin/hookstat{0}" -f $extension)
        if (-not (Test-Path -LiteralPath $legacyBinary -PathType Leaf)) {
            throw 'public v0.3.1 HookStat binary was not installed'
        }
        $legacyVersion = @(& $legacyBinary --version)
        if ($LASTEXITCODE -ne 0 -or ($legacyVersion -join "`n").Trim() -ne 'hookstat 0.3.1') {
            throw 'installed public HookStat binary is not v0.3.1'
        }
        $receipt.V031_PUBLIC_BINARY = $true

        $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
        $handler = [ordered]@{
            key                 = 'hk_v031_upgrade'
            revision            = 'hr_v031_upgrade'
            label               = 'v031 upgrade fixture'
            source_kind         = 'user_hooks_json'
            event               = 'stop'
            matcher_identity    = 'any'
            structural_identity = 'g0_h0'
            execution_mode      = 'sync'
        }
        $completedId = 'v031_completed_receipt'
        $incompleteId = 'v031_incomplete_receipt'
        $completedStart = [ordered]@{
            schema_version      = 1
            invocation_id       = $completedId
            handler             = $handler
            source              = 'codex'
            started_at_unix_ms  = $now - 20
            coverage            = 'partial'
        }
        $completed = [ordered]@{
            schema_version       = 1
            invocation_id        = $completedId
            handler              = $handler
            source               = 'codex'
            started_at_unix_ms   = $now - 20
            completed_at_unix_ms = $now - 10
            duration_ms          = 10
            exit_code            = 0
            terminal_status      = 'completed'
            coverage             = 'partial'
        }
        $incompleteStart = [ordered]@{
            schema_version      = 1
            invocation_id       = $incompleteId
            handler             = $handler
            source              = 'codex'
            started_at_unix_ms  = $now - 5
            coverage            = 'partial'
        }
        Write-JsonFixture -Path (Join-Path $recordsRoot "$completedId.start.json") -Value $completedStart
        Write-JsonFixture -Path (Join-Path $recordsRoot "$completedId.complete.json") -Value $completed
        Write-JsonFixture -Path (Join-Path $recordsRoot "$incompleteId.start.json") -Value $incompleteStart
        $journal = @(
            [ordered]@{ schema_version = 1; invocation_id = $completedId; stage = 'start' },
            [ordered]@{ schema_version = 1; invocation_id = $completedId; stage = 'complete' },
            [ordered]@{ schema_version = 1; invocation_id = $incompleteId; stage = 'start' }
        ) | ForEach-Object { $_ | ConvertTo-Json -Compress }
        $journalPath = Join-Path $legacyDataRoot 'receipts/receipt-journal-v1.ndjson'
        $utf8NoBom = [System.Text.UTF8Encoding]::new($false)
        [System.IO.File]::WriteAllText($journalPath, (($journal -join "`n") + "`n"), $utf8NoBom)
        $journalHashBefore = (Get-FileHash -LiteralPath $journalPath -Algorithm SHA256).Hash
        $recordHashesBefore = @(
            Get-ChildItem -LiteralPath $recordsRoot -File |
                Sort-Object Name |
                ForEach-Object { "{0}:{1}" -f $_.Name, (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
        )

        $legacyReport = Invoke-ReportJson -Binary $legacyBinary -DataRoot $legacyDataRoot -FailureMessage 'public v0.3.1 receipt reconciliation failed'
        Assert-LegacyReceiptReport -Report $legacyReport
        $receipt.V031_RECEIPT_SPOOL_RECONCILED = $true

        & rustup run $RustToolchain cargo install --path $resolvedRoot --locked --root $candidateInstallRoot --bin hookstat
        if ($LASTEXITCODE -ne 0) {
            throw 'candidate HookStat installation for upgrade verification failed'
        }
        $candidateBinary = Join-Path $candidateInstallRoot ("bin/hookstat{0}" -f $extension)
        if (-not (Test-Path -LiteralPath $candidateBinary -PathType Leaf)) {
            throw 'candidate HookStat binary was not installed'
        }
        $candidateReport = Invoke-ReportJson -Binary $candidateBinary -DataRoot $legacyDataRoot -FailureMessage 'candidate receipt-history upgrade reconciliation failed'
        Assert-LegacyReceiptReport -Report $candidateReport
        $receipt.CANDIDATE_RECEIPT_HISTORY_PRESERVED = $true

        $journalHashAfter = (Get-FileHash -LiteralPath $journalPath -Algorithm SHA256).Hash
        $recordHashesAfter = @(
            Get-ChildItem -LiteralPath $recordsRoot -File |
                Sort-Object Name |
                ForEach-Object { "{0}:{1}" -f $_.Name, (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
        )
        if ($journalHashAfter -ne $journalHashBefore -or
            (Compare-Object -ReferenceObject $recordHashesBefore -DifferenceObject $recordHashesAfter)) {
            throw 'candidate altered the public v0.3.1 receipt journal or canonical receipt files'
        }
        $receipt.V031_RECEIPT_JOURNAL_PRESERVED = $true
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
    $receipt.LEDGER_PRESERVED = $true
    $receipt.LEGACY_EVIDENCE_PRESERVED = $true
    $receipt.PREFERENCES_PRESERVED = $true
    # RuntimePresentationSnapshot deliberately has no durable serialization
    # path; this release proof does not invoke Codex discovery and cannot write
    # runtime-owned command/source/matcher presentation material.
    $receipt.RUNTIME_PRESENTATION_PERSISTED = $false
    $receipt.UPGRADE_031_TO_040 = 'PASS'
    $receipt.OVERALL = 'PASS'
    Write-Receipt -Receipt $receipt
}
catch {
    Write-Receipt -Receipt $receipt
    throw 'v0.3.1-to-candidate upgrade verification failed'
}
finally {
    if (Test-Path -LiteralPath $lab) {
        $resolvedLab = (Resolve-Path -LiteralPath $lab).Path
        if (-not $resolvedLab.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'refusing to remove an upgrade-verification path outside the temporary root'
        }
        Remove-Item -LiteralPath $resolvedLab -Recurse -Force
    }
}
