[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path,
    [string]$RustToolchain = '1.97.1',
    [string]$ExpectedVersion = '0.4.0',
    [string]$CargoHome,
    [switch]$IsolationProbeOnly,
    [switch]$KeepLab
)

$ErrorActionPreference = 'Stop'
$resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$tempRoot = [System.IO.Path]::GetTempPath()
$lab = Join-Path $tempRoot ('hookstat-package-verify-' + [guid]::NewGuid().ToString('N'))
$target = Join-Path $lab 'target'
$unpacked = Join-Path $lab 'unpacked'
$installRoot = Join-Path $lab 'install-root'
$freshDataRoot = Join-Path $lab 'fresh-data'
$requestedCargoHome = if ([string]::IsNullOrWhiteSpace($CargoHome)) {
    [System.IO.Path]::GetFullPath((Join-Path $lab 'cargo-home'))
}
else {
    [System.IO.Path]::GetFullPath($CargoHome)
}
$ownerCargoHome = [System.IO.Path]::GetFullPath(
    (Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)) '.cargo')
)

if ($requestedCargoHome.Equals($ownerCargoHome, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'CargoHome must be a disposable lab path, not the Owner default Cargo home'
}
if (Test-Path -LiteralPath $requestedCargoHome) {
    throw 'CargoHome must not already exist; the verifier creates and owns its disposable Cargo home'
}

function Invoke-Cargo {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)

    & rustup run $RustToolchain cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

try {
    New-Item -ItemType Directory -Path $lab, $target, $unpacked, $freshDataRoot, $requestedCargoHome | Out-Null
    $resolvedCargoHome = (Resolve-Path -LiteralPath $requestedCargoHome).Path
    $env:CARGO_TARGET_DIR = $target
    $env:CARGO_BUILD_JOBS = '1'
    $env:RUSTFLAGS = '-C debuginfo=0'
    Remove-Item Env:RUSTUP_HOME -ErrorAction SilentlyContinue
    # Never fall back to the Owner/default Cargo home. The caller may supply a
    # fresh lab path; standalone verification creates its own lab-local home.
    $env:CARGO_HOME = $resolvedCargoHome

    'VERIFY_PACKAGE_CARGO_HOME_ISOLATED=true'
    'OWNER_CARGO_CREDENTIAL_STORE_USED=false'
    if ($IsolationProbeOnly) {
        return
    }

    Push-Location $resolvedRoot
    try {
        $sourceHead = (& git rev-parse HEAD).Trim()
        if ($LASTEXITCODE -ne 0 -or $sourceHead -notmatch '^[0-9a-f]{40}$') {
            throw 'package verification could not resolve an exact source HEAD'
        }
        $trackedStatus = @(& git status --porcelain=v1 --untracked-files=no)
        if ($LASTEXITCODE -ne 0 -or $trackedStatus.Count -ne 0) {
            throw 'package verification requires a tracked-clean exact source HEAD'
        }
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
    $crateSha256 = (Get-FileHash -LiteralPath $crate.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
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
    if (Test-Path -LiteralPath (Join-Path $packageRoot.FullName 'dev_proof')) {
        throw 'developer-only cooperative proof adapter leaked into the public package'
    }

    Invoke-Cargo build --manifest-path $manifest --locked --bins
    Invoke-Cargo install --path $packageRoot.FullName --locked --root $installRoot

    # `$IsWindows` is available in PowerShell Core but is not defined by
    # Windows PowerShell 5.1. Use the stable process environment instead so
    # this fresh-install gate validates the files Cargo actually installs.
    $extension = if ($env:OS -eq 'Windows_NT') { '.exe' } else { '' }
    foreach ($binary in 'hookstat', 'hookstat-hook', 'hookstat-ipc-broker') {
        $candidate = Join-Path $installRoot ("bin/{0}{1}" -f $binary, $extension)
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            throw "required packaged binary missing from fresh install: $binary"
        }
    }
    $fixture = Join-Path $installRoot ("bin/hookstat-shim-fixture{0}" -f $extension)
    if (Test-Path -LiteralPath $fixture -PathType Leaf) {
        throw 'test-only shim fixture leaked into ordinary fresh installation'
    }

    $shim = Join-Path $installRoot ("bin/hookstat-hook{0}" -f $extension)
    $shimAdmission = @(& $shim --admission-status)
    if ($LASTEXITCODE -ne 0 -or
        ($shimAdmission -join "`n").Trim() -ne
        'transparent_shim=qualified_not_admitted_performance production_admitted=false') {
        throw 'fresh-installed transparent shim did not report its non-production admission'
    }

    $hookstat = Join-Path $installRoot ("bin/hookstat{0}" -f $extension)
    $version = @(& $hookstat --version)
    if ($LASTEXITCODE -ne 0 -or ($version -join "`n").Trim() -ne "hookstat $ExpectedVersion") {
        throw "fresh-installed HookStat version did not equal $ExpectedVersion"
    }
    # A fresh install has no ledger yet. Normal report initialization is the
    # first-run contract; read-only inspection correctly rejects this empty
    # root and is covered separately for existing data roots.
    $report = @(& $hookstat report --json --data-root $freshDataRoot)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($report -join "`n"))) {
        throw 'fresh-installed HookStat report smoke failed'
    }
    $doctor = @(& $hookstat doctor --json --data-root $freshDataRoot)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($doctor -join "`n"))) {
        throw 'fresh-installed HookStat doctor smoke failed'
    }
    # The ordinary TUI needs an interactive terminal. This deterministic
    # packaged frame smoke exercises the same rendered home frame without
    # connecting to Owner data or depending on terminal input.
    $tuiFrame = @(& $hookstat preview-fixture)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace(($tuiFrame -join "`n"))) {
        throw 'fresh-installed HookStat deterministic TUI frame smoke failed'
    }

    'PACKAGE_ARCHIVE_SELF_CONTAINED=true'
    'FRESH_INSTALL_REQUIRED_PACKAGED_BINARIES=true'
    "FRESH_INSTALL_VERSION=$ExpectedVersion"
    'FRESH_INSTALL_REPORT_SMOKE=true'
    'FRESH_INSTALL_DOCTOR_SMOKE=true'
    'FRESH_INSTALL_TUI_FRAME_SMOKE=true'
    'FRESH_INSTALL_TRANSPARENT_SHIM_ADMISSION=qualified_not_admitted_performance'
    'FRESH_INSTALL_TRANSPARENT_SHIM_PRODUCTION_ADMITTED=false'
    "PACKAGE_SOURCE_GIT_HEAD=$sourceHead"
    "PACKAGE_ARCHIVE_SHA256=$crateSha256"
    foreach ($binary in 'hookstat', 'hookstat-hook', 'hookstat-ipc-broker') {
        $candidate = Join-Path $installRoot ("bin/{0}{1}" -f $binary, $extension)
        $binarySha256 = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
        "FRESH_INSTALL_BINARY_SHA256_{0}={1}" -f $binary.ToUpperInvariant().Replace('-', '_'), $binarySha256
    }
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
