# Fail-closed Windows package checks. Never changes local Defender settings/exclusions.
param([ValidateSet('none','azure')][string]$SigningMode='none')
$ErrorActionPreference='Stop'
Import-Module (Join-Path $PSScriptRoot 'WindowsSecurity.psm1') -Force
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$reports=Join-Path $repo 'work/package-security/windows'
New-Item -ItemType Directory -Force $reports | Out-Null
$report=[ordered]@{status='failed';commit=$env:GITHUB_SHA;timeUtc=[DateTime]::UtcNow.ToString('o');signingMode=$SigningMode;engine=$null;scans=@();files=@();error=$null}
try {
    $version=(Select-String -LiteralPath "$repo/Cargo.toml" -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
    $payload="$repo/work/packages/windows/LibreToon"
    $portable="$repo/dist/LibreToon-$version-Windows-x64-Portable.zip"
    $installer="$repo/dist/LibreToon-$version-Windows-x64-Setup.exe"
    $app="$payload/libretoon.exe"
    foreach ($path in @($app,$portable,$installer)) {if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {throw "Missing package: $path"}}
    foreach ($path in @($app,$installer)) {
        Assert-DistributionMetadata -Path $path -Version $version
        $sig=Get-DistributionSignature -Path $path -RequireSigned:($SigningMode -eq 'azure')
        $report.files+=@{name=[IO.Path]::GetFileName($path);sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant();signature=$sig}
    }
    if ($SigningMode -eq 'azure') {
        $uninstallers=@(Get-ChildItem -LiteralPath "$repo/work/packages/windows/signing-cache" -Filter '*.e32' -File -ErrorAction Stop)
        if ($uninstallers.Count -eq 0) {throw 'Signed Inno Setup uninstaller is missing'}
        foreach ($file in $uninstallers) {$null=Get-DistributionSignature -Path $file.FullName -RequireSigned}
    }
    $zip=[IO.Compression.ZipFile]::OpenRead($portable)
    try {
        $entry=$zip.GetEntry('LibreToon/libretoon.exe')
        if (-not $entry) {throw 'Portable application is missing'}
        $stream=$entry.Open()
        try {$algorithm=[Security.Cryptography.SHA256]::Create();try {$zipHash=[Convert]::ToHexString($algorithm.ComputeHash($stream)).ToLowerInvariant()} finally {$algorithm.Dispose()}} finally {$stream.Dispose()}
        if ($zipHash -ne (Get-FileHash -LiteralPath $app).Hash.ToLowerInvariant()) {throw 'Portable executable differs from the checked application'}
    } finally {$zip.Dispose()}
    $manifest=Get-Content -LiteralPath "$repo/dist/SHA256SUMS.txt"
    foreach ($path in @($portable,$installer)) {
        $hash=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()
        if ("$hash  $([IO.Path]::GetFileName($path))" -notin $manifest) {throw 'Package checksum manifest mismatch'}
    }
    $service=Get-Service WinDefend -ErrorAction Stop
    if ($service.Status -ne 'Running' -and $env:GITHUB_ACTIONS -eq 'true') {
        Start-Service WinDefend -ErrorAction Stop
    }
    if ((Get-Service WinDefend).Status -ne 'Running') {throw 'Defender service is stopped; no scan was performed'}
    if (-not (Get-Command Get-MpComputerStatus -ErrorAction SilentlyContinue)) {Import-Module Defender -UseWindowsPowerShell -ErrorAction Stop}
    $candidates=@(Get-ChildItem "$env:ProgramData/Microsoft/Windows Defender/Platform/*/MpCmdRun.exe" -ErrorAction SilentlyContinue | Sort-Object FullName -Descending)
    $candidates+=@(Get-Item "$env:ProgramFiles/Windows Defender/MpCmdRun.exe" -ErrorAction SilentlyContinue)
    if ($candidates.Count -eq 0) {throw 'Microsoft Defender scanner not found'}
    $scanner=$candidates[0].FullName
    & $scanner -SignatureUpdate 2>&1 | Tee-Object -FilePath "$reports/intelligence-update.log" | Out-Host
    if ($LASTEXITCODE -ne 0) {throw 'Defender intelligence update failed'}
    $status=Get-MpComputerStatus -ErrorAction Stop
    Assert-DefenderReady $status
    $report.engine=@{name='Microsoft Defender';version=$status.AMProductVersion;intelligence=$status.AntivirusSignatureVersion;intelligenceUpdatedUtc=([DateTime]$status.AntivirusSignatureLastUpdated).ToUniversalTime().ToString('o')}
    foreach ($target in @($payload,"$repo/dist")) {
        $log=Join-Path $reports ("scan-"+$report.scans.Count+'.log')
        & $scanner -Scan -ScanType 3 -File $target -DisableRemediation 2>&1 | Tee-Object -FilePath $log | Out-Host
        $code=$LASTEXITCODE
        $report.scans+=@{target=[IO.Path]::GetFileName($target);exitCode=$code;archivesIncluded=$true;remediation=$false}
        Assert-ScannerExit $code
    }
    foreach ($path in @($portable,$installer)) {$report.files+=@{name=[IO.Path]::GetFileName($path);sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()}}
    # Verify nothing disappeared or changed during the scan.
    foreach ($file in $report.files) {
        $path=if ($file.name -eq 'libretoon.exe') {$app} else {Join-Path "$repo/dist" $file.name}
        if ($file.sha256 -ne (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()) {throw 'A package changed during antivirus analysis'}
    }
    $report.status='passed'
} catch {$report.error=$_.Exception.Message;throw}
finally {
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$reports/security-report.json" -Encoding utf8
    $text="Windows distribution checks: $($report.status). Signing mode: $SigningMode. See security-report.json for hashes, engine and signatures. A passing scan does not guarantee absence of malware or removal of browser warnings."
    $text | Set-Content -LiteralPath "$reports/README.txt" -Encoding utf8
    if ($env:GITHUB_STEP_SUMMARY) {$text | Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY}
}
