# Optional cloud Authenticode signing. This script is also Inno Setup's signing callback.
param(
    [Parameter(Mandatory)][string]$Path,
    [ValidateSet('none','azure')][string]$Mode = 'none'
)
$ErrorActionPreference = 'Stop'
$target = (Resolve-Path -LiteralPath $Path).Path
if ($Mode -eq 'none') { return }
foreach ($name in @('WINDOWS_SIGNING_ENDPOINT','WINDOWS_SIGNING_ACCOUNT','WINDOWS_SIGNING_PROFILE')) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) { throw "Missing signing configuration: $name" }
}
$endpoint = [uri]$env:WINDOWS_SIGNING_ENDPOINT
if ($endpoint.Scheme -ne 'https' -or -not $endpoint.Host.EndsWith('.codesigning.azure.net')) { throw 'Expected an HTTPS Azure Artifact Signing endpoint' }
# Parameters/version verified against Azure/artifact-signing-action v2 action.yml.
Import-Module ArtifactSigning -RequiredVersion 0.1.8 -ErrorAction Stop
Invoke-ArtifactSigning -Endpoint $endpoint.AbsoluteUri -CodeSigningAccountName $env:WINDOWS_SIGNING_ACCOUNT `
    -CertificateProfileName $env:WINDOWS_SIGNING_PROFILE -Files $target `
    -FileDigest SHA256 -TimestampRfc3161 'http://timestamp.acs.microsoft.com' -TimestampDigest SHA256 `
    -ExcludeEnvironmentCredential $true -ExcludeWorkloadIdentityCredential $true -ExcludeManagedIdentityCredential $true `
    -ExcludeSharedTokenCacheCredential $true -ExcludeVisualStudioCredential $true -ExcludeVisualStudioCodeCredential $true `
    -ExcludeAzureCliCredential $false -ExcludeAzurePowerShellCredential $true -ExcludeAzureDeveloperCliCredential $true `
    -ExcludeInteractiveBrowserCredential $true -ErrorAction Stop | Out-Host
$signature = Get-AuthenticodeSignature -LiteralPath $target
if ($signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate) { throw "Signing or timestamp verification failed: $target" }
