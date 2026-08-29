[CmdletBinding()]
param(
    [ValidateRange(2, 10)][int]$CpuSamples = 3,
    [ValidateRange(0, 15)][int]$SampleIntervalSeconds = 2,
    [ValidateRange(80, 100)][int]$ExtremeCpuPercent = 90,
    [ValidateRange(2, 10)][int]$MinimumExtremeSamples = 2,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($MinimumExtremeSamples -gt $CpuSamples) {
    throw 'MinimumExtremeSamples must not exceed CpuSamples'
}

function Write-Receipt {
    param([Parameter(Mandatory = $true)][System.Collections.IDictionary]$Receipt)

    foreach ($entry in $Receipt.GetEnumerator()) {
        $value = if ($entry.Value -is [bool]) {
            $entry.Value.ToString().ToLowerInvariant()
        }
        elseif ($entry.Value -is [System.Collections.IEnumerable] -and $entry.Value -isnot [string]) {
            (@($entry.Value) -join ',')
        }
        else {
            [string]$entry.Value
        }
        "{0}={1}" -f $entry.Key, $value
    }

    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
        $parent = Split-Path -Parent $resolvedOutput
        if ([string]::IsNullOrWhiteSpace($parent) -or -not (Test-Path -LiteralPath $parent -PathType Container)) {
            throw 'OutputPath parent must already exist'
        }
        [pscustomobject]$Receipt | ConvertTo-Json -Depth 4 |
            Set-Content -LiteralPath $resolvedOutput -Encoding utf8NoBOM
    }
}

$receipt = [ordered]@{
    PREFLIGHT_VERSION                    = 1
    ENVIRONMENT_ADMITTED                 = $false
    ENVIRONMENT_BUSY                     = $true
    MUTATION                             = $false
    PERFORMANCE_CLAIM                    = 'NONE'
    DISPOSITION                          = 'ENVIRONMENT_BUSY'
    TOTAL_CPU_SAMPLE_PERCENT             = @()
    TOTAL_CPU_MAX_PERCENT                = 'UNAVAILABLE'
    SUSTAINED_EXTREME_CPU                = $false
    EXTREME_CPU_PERCENT_THRESHOLD        = $ExtremeCpuPercent
    EXTREME_CPU_MINIMUM_SAMPLES          = $MinimumExtremeSamples
    UNRELATED_CARGO_COUNT                = 0
    UNRELATED_RUSTC_COUNT                = 0
    OTHER_HOOKSTAT_QUALIFICATION_COUNT   = 0
    CPU_SAMPLING_AVAILABLE               = $false
}

if (-not $IsWindows) {
    $receipt.DISPOSITION = 'UNSUPPORTED_PLATFORM'
    Write-Receipt -Receipt $receipt
    exit 2
}

try {
    # This is observation only. The emitted receipt deliberately contains only
    # aggregate counts and CPU values: no command lines, paths, or process IDs.
    $processes = @(Get-CimInstance -ClassName Win32_Process)
    $receipt.UNRELATED_CARGO_COUNT = @(
        $processes | Where-Object {
            $_.ProcessId -ne $PID -and $_.Name -match '^(?i:cargo)(?:\.exe)?$'
        }
    ).Count
    $receipt.UNRELATED_RUSTC_COUNT = @(
        $processes | Where-Object {
            $_.ProcessId -ne $PID -and $_.Name -match '^(?i:rustc)(?:\.exe)?$'
        }
    ).Count
    $receipt.OTHER_HOOKSTAT_QUALIFICATION_COUNT = @(
        $processes | Where-Object {
            $_.ProcessId -ne $PID -and
            $_.Name -match '^(?i:hookstat)(?:[-_].*)?(?:\.exe)?$' -and
            $_.CommandLine -match '(?i:perf|qualif|benchmark)'
        }
    ).Count

    $samples = [System.Collections.Generic.List[double]]::new()
    for ($index = 0; $index -lt $CpuSamples; $index++) {
        $counter = Get-Counter -Counter '\Processor(_Total)\% Processor Time' -ErrorAction Stop
        $value = [double](($counter.CounterSamples | Select-Object -First 1).CookedValue)
        $samples.Add([math]::Round($value, 1))
        if ($index -lt ($CpuSamples - 1) -and $SampleIntervalSeconds -gt 0) {
            Start-Sleep -Seconds $SampleIntervalSeconds
        }
    }
    $receipt.CPU_SAMPLING_AVAILABLE = $true
    $receipt.TOTAL_CPU_SAMPLE_PERCENT = @($samples)
    $receipt.TOTAL_CPU_MAX_PERCENT = [math]::Round(($samples | Measure-Object -Maximum).Maximum, 1)
    $extremeCount = @($samples | Where-Object { $_ -ge $ExtremeCpuPercent }).Count
    $receipt.SUSTAINED_EXTREME_CPU = $extremeCount -ge $MinimumExtremeSamples

    $processBusy = $receipt.UNRELATED_CARGO_COUNT -gt 0 -or
        $receipt.UNRELATED_RUSTC_COUNT -gt 0 -or
        $receipt.OTHER_HOOKSTAT_QUALIFICATION_COUNT -gt 0
    $receipt.ENVIRONMENT_BUSY = $processBusy -or $receipt.SUSTAINED_EXTREME_CPU
    if (-not $receipt.ENVIRONMENT_BUSY) {
        $receipt.ENVIRONMENT_ADMITTED = $true
        $receipt.DISPOSITION = 'PASS'
    }

    Write-Receipt -Receipt $receipt
    if ($receipt.ENVIRONMENT_ADMITTED) {
        exit 0
    }
    exit 2
}
catch {
    # An unavailable process/counter observation is not a safe admission.
    $receipt.DISPOSITION = 'PREFLIGHT_UNAVAILABLE'
    $receipt.ENVIRONMENT_BUSY = $true
    $receipt.ENVIRONMENT_ADMITTED = $false
    Write-Receipt -Receipt $receipt
    exit 2
}
