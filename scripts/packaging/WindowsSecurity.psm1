# Testable assertions shared by the Windows distribution gate.
function Assert-ScannerExit {
    param([int]$Code)
    # DisableRemediation is mandatory: a default scan can return 0 after removing malware.
    if ($Code -ne 0) { throw "Antivirus scan failed or detected a threat (exit $Code); packages are blocked" }
}
function Assert-DefenderReady {
    param($Status)
    if (-not $Status.AMServiceEnabled -or -not $Status.AntivirusEnabled) { throw 'Microsoft Defender is unavailable; packages are blocked' }
    if (-not $Status.AntivirusSignatureLastUpdated -or [string]::IsNullOrWhiteSpace($Status.AntivirusSignatureVersion)) { throw 'Missing Defender intelligence metadata' }
    $age = ([DateTime]::UtcNow - ([DateTime]$Status.AntivirusSignatureLastUpdated).ToUniversalTime()).TotalHours
    if ($age -gt 48 -or $age -lt -24) { throw 'Defender intelligence is stale or its timestamp is invalid' }
}
function Get-DistributionSignature {
    param([string]$Path, [switch]$RequireSigned)
    $sig = Get-AuthenticodeSignature -LiteralPath $Path
    if ($RequireSigned -and ($sig.Status -ne 'Valid' -or -not $sig.TimeStamperCertificate)) { throw "A valid timestamped signature is required: $Path" }
    if ($sig.Status -notin @('Valid','NotSigned')) { throw "Invalid signature: $Path ($($sig.Status))" }
    [pscustomobject]@{status=$sig.Status.ToString();subject=$sig.SignerCertificate.Subject;thumbprint=$sig.SignerCertificate.Thumbprint;timestamped=[bool]$sig.TimeStamperCertificate}
}
function Assert-DistributionMetadata {
    param([string]$Path, [string]$Version)
    $info = [Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
    if ($info.ProductName.Trim() -ne 'LibreToon' -or $info.CompanyName.Trim() -ne 'alvillafm' -or $info.ProductVersion.Trim() -ne $Version -or [string]::IsNullOrWhiteSpace($info.FileDescription)) {
        throw "Missing/incorrect LibreToon product, publisher or version metadata: $Path"
    }
}
Export-ModuleMember -Function Assert-ScannerExit,Assert-DefenderReady,Get-DistributionSignature,Assert-DistributionMetadata
