[CmdletBinding()]
param(
    [string]$BaseSha,
    [string]$HeadSha,
    [string[]]$ChangedFile,
    [ValidateSet('Text', 'Json', 'GitHub')]
    [string]$OutputFormat = 'Text'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Set-True {
    param(
        [Parameter(Mandatory = $true)][System.Collections.IDictionary]$State,
        [Parameter(Mandatory = $true)][string[]]$Names
    )

    foreach ($name in $Names) {
        $State[$name] = $true
    }
}

function Set-FullMatrix {
    param([Parameter(Mandatory = $true)][System.Collections.IDictionary]$State)

    Set-True -State $State -Names @('RUN_FULL_WINDOWS', 'RUN_FULL_UBUNTU', 'RUN_RUST_WINDOWS', 'RUN_RUST_UBUNTU')
}

function Set-UnknownRisk {
    param([Parameter(Mandatory = $true)][System.Collections.IDictionary]$State)

    Set-True -State $State -Names @('UNKNOWN_RISK')
    Set-FullMatrix -State $State
}

$state = [ordered]@{
    RISK_D                     = $false
    RISK_C                     = $false
    RISK_E                     = $false
    RISK_S                     = $false
    RISK_T                     = $false
    RISK_P                     = $false
    RISK_R                     = $false
    WINDOWS_SENSITIVE          = $false
    UNIX_SENSITIVE             = $false
    PACKAGE_SURFACE_CHANGED    = $false
    PERFORMANCE_SURFACE_CHANGED = $false
    WORKFLOW_CHANGED           = $false
    UNKNOWN_RISK               = $false
    RUN_FULL_WINDOWS           = $false
    RUN_FULL_UBUNTU            = $false
    RUN_RUST_WINDOWS           = $false
    RUN_RUST_UBUNTU            = $false
    LIGHTWEIGHT_ONLY           = $false
    CHANGED_FILE_COUNT         = 0
}

if ($PSBoundParameters.ContainsKey('ChangedFile')) {
    $files = @($ChangedFile)
}
else {
    if ([string]::IsNullOrWhiteSpace($BaseSha)) {
        throw 'BaseSha is required when ChangedFile is not supplied'
    }
    if ([string]::IsNullOrWhiteSpace($HeadSha)) {
        $HeadSha = (& git rev-parse HEAD).Trim()
        if ($LASTEXITCODE -ne 0) {
            throw 'could not resolve HEAD for change classification'
        }
    }
    $files = @(& git diff --name-only --diff-filter=ACMR $BaseSha $HeadSha --)
    if ($LASTEXITCODE -ne 0) {
        throw "could not list changed files for base=$BaseSha head=$HeadSha"
    }
}

$normalizedFiles = @(
    $files |
        ForEach-Object { $_.Trim().Replace('\\', '/') -replace '^\./', '' } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Sort-Object -Unique
)
$state.CHANGED_FILE_COUNT = $normalizedFiles.Count

foreach ($path in $normalizedFiles) {
    if ($path.StartsWith('../', [System.StringComparison]::Ordinal) -or $path.Contains('/../')) {
        Set-UnknownRisk -State $state
        continue
    }

    # Documentation and governance are deliberately recognized first. Their
    # classification is safe only while they remain outside build/release paths.
    if ($path -match '^(docs/|dev_governance_files/)' -or
        $path -match '^(README|CHANGELOG|CONTRIBUTING|CODE_OF_CONDUCT|SECURITY)\.md$') {
        Set-True -State $state -Names @('RISK_D')
        continue
    }

    if ($path -eq '.github/pull_request_template.md') {
        Set-True -State $state -Names @('RISK_D')
        continue
    }

    if ($path -match '^\.github/workflows/') {
        Set-True -State $state -Names @('RISK_R', 'WORKFLOW_CHANGED')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^(Cargo\.toml|Cargo\.lock|build\.rs|\.cargo/|rust-toolchain)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_R', 'PACKAGE_SURFACE_CHANGED')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^scripts/release/') {
        Set-True -State $state -Names @('RISK_R', 'PACKAGE_SURFACE_CHANGED')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^scripts/(ci|review)/') {
        Set-True -State $state -Names @('RISK_R', 'WORKFLOW_CHANGED')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^scripts/qualification/') {
        Set-True -State $state -Names @('RISK_R')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^scripts/performance/') {
        Set-True -State $state -Names @('RISK_C', 'RISK_P', 'PERFORMANCE_SURFACE_CHANGED', 'WINDOWS_SENSITIVE')
        $state.RUN_RUST_WINDOWS = $true
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^scripts/') {
        Set-UnknownRisk -State $state
        continue
    }

    if ($path -match '^\.github/') {
        # GitHub metadata can alter workflow behavior through actions, permissions,
        # or reusable configuration even when it is not in workflows/.
        Set-True -State $state -Names @('RISK_R', 'WORKFLOW_CHANGED')
        Set-FullMatrix -State $state
        continue
    }

    if ($path -match '^src/(?:ipc(?:[_.].*)?|ipc/|bin/hookstat_(?:ipc|hook)|runtime/windows)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_E', 'RISK_P', 'WINDOWS_SENSITIVE')
        $state.RUN_RUST_WINDOWS = $true
        continue
    }

    if ($path -match '^src/.*(?:unix|linux|uds|socket)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_E', 'RISK_P', 'UNIX_SENSITIVE')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^src/(?:ledger|receipt|migration|storage)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_S')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^src/tui/') {
        Set-True -State $state -Names @('RISK_C', 'RISK_T')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^src/(?:admission|analytics|domain|evidence|identity|interface_preferences|lib|main|observability|render|report|workbench)\.rs$') {
        Set-True -State $state -Names @('RISK_C')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^src/') {
        Set-UnknownRisk -State $state
        continue
    }

    if ($path -match '^tests/.*(?:ipc|windows|named_pipe)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_E', 'RISK_P', 'WINDOWS_SENSITIVE')
        $state.RUN_RUST_WINDOWS = $true
        continue
    }

    if ($path -match '^tests/.*(?:unix|linux|uds|socket)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_E', 'RISK_P', 'UNIX_SENSITIVE')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^tests/.*(?:ledger|migration|receipt|storage)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_S')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^tests/.*(?:tui|render)') {
        Set-True -State $state -Names @('RISK_C', 'RISK_T')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^tests/(?:analytics|domain|evidence|identity|report|workbench).*\.rs$') {
        Set-True -State $state -Names @('RISK_C')
        $state.RUN_RUST_UBUNTU = $true
        continue
    }

    if ($path -match '^tests/') {
        Set-UnknownRisk -State $state
        continue
    }

    Set-UnknownRisk -State $state
}

if ($state.UNKNOWN_RISK) {
    Set-FullMatrix -State $state
}
elseif ($state.RISK_R) {
    # Release, packaging, and workflow changes affect the way every candidate
    # is proven. They always receive the current full final matrix.
    Set-FullMatrix -State $state
}
elseif ($state.WINDOWS_SENSITIVE -and $state.UNIX_SENSITIVE) {
    $state.RUN_RUST_WINDOWS = $true
    $state.RUN_RUST_UBUNTU = $true
}

if (-not $state.RISK_C -and -not $state.RISK_R -and -not $state.UNKNOWN_RISK) {
    $state.LIGHTWEIGHT_ONLY = $true
}

switch ($OutputFormat) {
    'Json' {
        [pscustomobject]$state | ConvertTo-Json -Compress
    }
    'GitHub' {
        if ([string]::IsNullOrWhiteSpace($env:GITHUB_OUTPUT)) {
            throw 'GitHub output format requires GITHUB_OUTPUT'
        }
        foreach ($entry in $state.GetEnumerator()) {
            $value = if ($entry.Value -is [bool]) {
                $entry.Value.ToString().ToLowerInvariant()
            }
            else {
                [string]$entry.Value
            }
            "{0}={1}" -f $entry.Key.ToLowerInvariant(), $value |
                Out-File -LiteralPath $env:GITHUB_OUTPUT -Append -Encoding utf8
        }
        foreach ($entry in $state.GetEnumerator()) {
            "{0}={1}" -f $entry.Key, $entry.Value.ToString().ToLowerInvariant()
        }
    }
    default {
        foreach ($entry in $state.GetEnumerator()) {
            "{0}={1}" -f $entry.Key, $entry.Value.ToString().ToLowerInvariant()
        }
    }
}
