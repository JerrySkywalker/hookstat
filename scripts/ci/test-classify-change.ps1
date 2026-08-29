[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$classifier = Join-Path $RepositoryRoot 'scripts/ci/classify-change.ps1'
if (-not (Test-Path -LiteralPath $classifier -PathType Leaf)) {
    throw "classifier not found: $classifier"
}

function Invoke-Classification {
    param([Parameter(Mandatory = $true)][string[]]$Paths)

    $json = @(& $classifier -ChangedFile $Paths -OutputFormat Json)
    if (-not $?) {
        throw "classifier failed for [$($Paths -join ', ')]"
    }
    return ($json -join "`n" | ConvertFrom-Json)
}

function Assert-Flag {
    param(
        [Parameter(Mandatory = $true)]$Result,
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][bool]$Expected
    )

    $actual = [bool]$Result.$Name
    if ($actual -ne $Expected) {
        throw "expected $Name=$Expected but got $actual"
    }
}

function Assert-Case {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string[]]$Paths,
        [Parameter(Mandatory = $true)][hashtable]$Expected
    )

    $result = Invoke-Classification -Paths $Paths
    foreach ($entry in $Expected.GetEnumerator()) {
        Assert-Flag -Result $result -Name $entry.Key -Expected ([bool]$entry.Value)
    }
    "CLASSIFIER_FIXTURE_{0}=PASS" -f $Name.ToUpperInvariant()
}

Assert-Case -Name 'docs_only' -Paths @('docs/process/guide.md') -Expected @{
    RISK_D = $true; LIGHTWEIGHT_ONLY = $true; RUN_FULL_WINDOWS = $false; RUN_FULL_UBUNTU = $false
}
Assert-Case -Name 'governance_only' -Paths @('dev_governance_files/FAST_LANE.md') -Expected @{
    RISK_D = $true; LIGHTWEIGHT_ONLY = $true; RUN_FULL_WINDOWS = $false
}
Assert-Case -Name 'readme_only' -Paths @('README.md') -Expected @{
    RISK_D = $true; LIGHTWEIGHT_ONLY = $true; RUN_FULL_UBUNTU = $false
}
Assert-Case -Name 'ordinary_rust' -Paths @('src/analytics.rs') -Expected @{
    RISK_C = $true; RUN_RUST_UBUNTU = $true; RUN_RUST_WINDOWS = $false; UNKNOWN_RISK = $false
}
Assert-Case -Name 'windows_ipc' -Paths @('src/ipc.rs') -Expected @{
    RISK_C = $true; RISK_E = $true; RISK_P = $true; WINDOWS_SENSITIVE = $true; RUN_RUST_WINDOWS = $true
}
Assert-Case -Name 'unix_sensitive' -Paths @('src/runtime/unix_transport.rs') -Expected @{
    RISK_C = $true; UNIX_SENSITIVE = $true; RUN_RUST_UBUNTU = $true; WINDOWS_SENSITIVE = $false
}
Assert-Case -Name 'ledger_schema' -Paths @('src/ledger.rs') -Expected @{
    RISK_C = $true; RISK_S = $true; RUN_RUST_UBUNTU = $true
}
Assert-Case -Name 'tui' -Paths @('src/tui/rendering.rs') -Expected @{
    RISK_C = $true; RISK_T = $true; RUN_RUST_UBUNTU = $true
}
Assert-Case -Name 'cargo_toml' -Paths @('Cargo.toml') -Expected @{
    RISK_C = $true; RISK_R = $true; PACKAGE_SURFACE_CHANGED = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}
Assert-Case -Name 'cargo_lock' -Paths @('Cargo.lock') -Expected @{
    RISK_C = $true; RISK_R = $true; PACKAGE_SURFACE_CHANGED = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}
Assert-Case -Name 'release_script' -Paths @('scripts/release/release-gate.ps1') -Expected @{
    RISK_R = $true; PACKAGE_SURFACE_CHANGED = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}
Assert-Case -Name 'workflow' -Paths @('.github/workflows/ci.yml') -Expected @{
    RISK_R = $true; WORKFLOW_CHANGED = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}
Assert-Case -Name 'mixed_docs_rust' -Paths @('docs/process/guide.md', 'src/report.rs') -Expected @{
    RISK_D = $true; RISK_C = $true; RUN_RUST_UBUNTU = $true; LIGHTWEIGHT_ONLY = $false
}
Assert-Case -Name 'unknown_source' -Paths @('src/future_subsystem.rs') -Expected @{
    UNKNOWN_RISK = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}
Assert-Case -Name 'unknown_test' -Paths @('tests/future_subsystem.rs') -Expected @{
    UNKNOWN_RISK = $true; RUN_FULL_WINDOWS = $true; RUN_FULL_UBUNTU = $true
}

'UNKNOWN_FAILS_SAFE=true'
'DOCS_ONLY_FULL_RUST_MATRIX=false'
'UNKNOWN_RISK_FULL_MATRIX=true'
'WINDOWS_SENSITIVE_WINDOWS_GATE=true'
'RELEASE_SENSITIVE_FULL_FINAL_GATE=true'
