function Assert-HookStatSafeScratchPath {
    [CmdletBinding()]
    param([Parameter(Mandatory = $true)][string]$Candidate)

    $normalized = $Candidate.Replace('/', '\')
    if ($normalized -notmatch '^(?i:c):\\') {
        return
    }

    $components = [System.Collections.Generic.List[string]]::new()
    foreach ($component in $normalized.Substring(3).Split('\')) {
        if ([string]::IsNullOrEmpty($component) -or $component -eq '.') {
            continue
        }
        if ($component -eq '..') {
            if ($components.Count -gt 0) {
                $components.RemoveAt($components.Count - 1)
            }
            continue
        }
        $components.Add($component)
    }

    if ($components.Count -le 1) {
        throw "HookStat scratch destination '$Candidate' is prohibited: direct C:\ project/temp roots are prohibited; choose a normal temporary or build location"
    }
}
