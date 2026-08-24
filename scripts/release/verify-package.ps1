[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1',
    [switch]$KeepLab
)

$ErrorActionPreference = 'Stop'
$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-package-verify-' + [guid]::NewGuid().ToString('N'))
$target = Join-Path $lab 'target'
$unpacked = Join-Path $lab 'unpacked'
$installRoot = Join-Path $lab 'install-root'

function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    & rustup run $RustToolchain cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

try {
    New-Item -ItemType Directory -Path $lab, $target, $unpacked | Out-Null
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_BUILD_JOBS = '1'
    $env:RUSTFLAGS = '-C debuginfo=0'
    Remove-Item Env:RUSTUP_HOME -ErrorAction SilentlyContinue
    Remove-Item Env:CARGO_HOME -ErrorAction SilentlyContinue

    Push-Location $resolvedRoot
    try {
        Invoke-Cargo package --locked
    }
    finally {
        Pop-Location
    }

    $crate = Get-ChildItem -LiteralPath (Join-Path $target 'package') -Filter 'hookstat-*.crate' -File |
        Select-Object -First 1
    if ($null -eq $crate) {
        throw 'cargo package did not produce a hookstat .crate artifact'
    }
    & tar -xzf $crate.FullName -C $unpacked
    if ($LASTEXITCODE -ne 0) {
        throw "tar extraction failed with exit code $LASTEXITCODE"
    }
    $packageRoot = Get-ChildItem -LiteralPath $unpacked -Directory | Select-Object -First 1
    if ($null -eq $packageRoot) {
        throw 'package archive did not contain a crate root'
    }
    $manifest = Join-Path $packageRoot.FullName 'Cargo.toml'
    $dependencySection = $false
    $pathDependency = $false
    foreach ($line in Get-Content -LiteralPath $manifest) {
        if ($line -match '^\s*\[(.+)\]\s*$') {
            $dependencySection = $Matches[1] -match '(?:^|\.)(?:build-|dev-)?dependencies$'
            continue
        }
        if ($dependencySection -and $line -match '\bpath\s*=') {
            $pathDependency = $true
            break
        }
    }
    if ($pathDependency) {
        throw 'packaged manifest retains a path dependency or path-only package reference'
    }

    Invoke-Cargo build --manifest-path $manifest --locked --bins
    Invoke-Cargo install --path $packageRoot.FullName --locked --root $installRoot

    $extension = if ($IsWindows) { '.exe' } else { '' }
    foreach ($binary in 'hookstat', 'hookstat-hook', 'hookstat-ipc-broker') {
        $candidate = Join-Path $installRoot ("bin/{0}{1}" -f $binary, $extension)
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            throw "required production binary missing from fresh install: $binary"
        }
    }
    $fixture = Join-Path $installRoot ("bin/hookstat-shim-fixture{0}" -f $extension)
    if (Test-Path -LiteralPath $fixture -PathType Leaf) {
        throw 'test-only shim fixture leaked into ordinary fresh installation'
    }

    'PACKAGE_ARCHIVE_SELF_CONTAINED=true'
    'FRESH_INSTALL_REQUIRED_BINARIES=true'
}
finally {
    if (-not $KeepLab -and (Test-Path -LiteralPath $lab)) {
        $resolvedLab = (Resolve-Path -LiteralPath $lab).Path
        if (-not $resolvedLab.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw 'refusing to remove a release-verification path outside the temporary root'
        }
        Remove-Item -LiteralPath $resolvedLab -Recurse -Force
    }
}
