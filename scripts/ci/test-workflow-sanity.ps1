[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$workflow = Join-Path $RepositoryRoot '.github/workflows/ci.yml'
$content = Get-Content -LiteralPath $workflow -Raw

function Require-Pattern {
    param(
        [Parameter(Mandatory = $true)][string]$Pattern,
        [Parameter(Mandatory = $true)][string]$Description
    )

    if ($content -notmatch $Pattern) {
        throw "workflow sanity missing: $Description"
    }
}

# GitHub parses the authoritative YAML on dispatch. This local guard validates
# the compatibility-critical shape before the remote run and refuses tab-based
# indentation, which YAML permits inconsistently across parsers.
if ($content -match "`t") {
    throw 'workflow YAML contains tab indentation'
}
Require-Pattern '(?m)^name:\s+CI\s*$' 'workflow name CI'
Require-Pattern '(?m)^\s{2}workflow_dispatch:\s*$' 'manual candidate dispatch'
Require-Pattern '(?m)^\s{2}changes:\s*$' 'change classifier job'
Require-Pattern '(?m)^\s{2}rust:\s*$' 'legacy rust job id'
Require-Pattern '(?m)^\s{2}tui-visual:\s*$' 'dedicated TUI visual job id'
Require-Pattern '(?m)^\s{4}name:\s+tui-visual\s*$' 'dedicated TUI visual check name'
Require-Pattern '(?ms)^\s{2}tui-visual:\s*\n\s{4}name:\s+tui-visual\s*\n\s{4}if:\s+\$\{\{ always\(\) \}\}' 'visual check always reports after classifier failure'
Require-Pattern '(?ms)^\s{2}rust:\s*\n\s{4}if:\s+\$\{\{ always\(\) \}\}' 'legacy contexts always run after classifier failure'
Require-Pattern 'os:\s*\[windows-latest, ubuntu-latest\]' 'legacy matrix values'
Require-Pattern '(?ms)^\s{2}rust:\s*.*?fetch-depth:\s*0' 'full history for lightweight rust contexts'
Require-Pattern 'classify-change\.ps1' 'repository-owned classifier'
Require-Pattern 'run_rust_windows' 'Windows route output'
Require-Pattern 'run_rust_ubuntu' 'Ubuntu route output'
Require-Pattern 'run_tui_visual' 'TUI visual route output'
Require-Pattern 'cargo test --locked --lib tui_visual -- --nocapture' 'check-only Ratatui cell-golden command'
Require-Pattern 'CANDIDATE_FULL_MATRIX_OVERRIDE=true' 'monotonic candidate full-matrix override'
Require-Pattern 'RUST_TOOLCHAIN_NOT_INSTALLED=true' 'lightweight required context'
Require-Pattern "needs\.changes\.result != 'success'" 'classifier failure propagation'
Require-Pattern 'Validate visual classifier outputs' 'explicit visual classifier output validation'
Require-Pattern "CLASSIFIER_RUN_TUI_VISUAL -notin @\('true', 'false'\)" 'fail-closed visual route value validation'
Require-Pattern "CLASSIFIER_BASE_SHA -notmatch '\^\[0-9a-f\]\{40\}\$'" 'fail-closed visual base SHA validation'
Require-Pattern "CLASSIFIER_HEAD_SHA -notmatch '\^\[0-9a-f\]\{40\}\$'" 'fail-closed visual head SHA validation'

if ($content -match 'HOOKSTAT_UPDATE_VISUAL_BASELINES|--ignored') {
    throw 'workflow must never contain the explicit local baseline-update mechanism'
}
if ($content -match '(?i)PrintWindow|pixel.?screenshot') {
    throw 'pixel screenshot must not become the primary TUI visual gate'
}

'WORKFLOW_STRUCTURE_SANITY=PASS'
'EXISTING_CHECK_CONTEXTS_PRESERVED=true'
