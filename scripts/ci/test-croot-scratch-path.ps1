[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot '../Assert-HookStatScratchPath.ps1')

foreach ($candidate in @(
    'C:\',
    'C:\hookstat-review',
    'C:\hookstat-temp-lab',
    'C:\hookstat-target',
    'c:\hookstat-temp-lab',
    'C:/hookstat-temp-lab',
    'C:\.\hookstat-temp-lab'
)) {
    try {
        Assert-HookStatSafeScratchPath -Candidate $candidate
        throw "unsafe candidate was accepted: $candidate"
    }
    catch {
        if ($_.Exception.Message -notmatch 'direct C:\\ project/temp roots') {
            throw
        }
    }
}

foreach ($candidate in @(
    'C:\Users\test\AppData\Local\Temp\hookstat-lab',
    'D:\temp\hookstat-lab',
    'C:\hookstat-parent\child',
    'relative-test-root/hookstat-lab'
)) {
    Assert-HookStatSafeScratchPath -Candidate $candidate
}

'CROOT_SCRATCH_PATH_REJECTED_CASES=7'
'CROOT_SCRATCH_PATH_SAFE_CASES=4'
'CROOT_SCRATCH_PATH_NO_FILESYSTEM_MUTATION=true'
