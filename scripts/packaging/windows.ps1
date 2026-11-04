# Package LibreToon only. FFmpeg is an optional user download, never redistributed here.
param([switch]$StageOnly, [ValidateSet('none','azure')][string]$SigningMode='none')
$ErrorActionPreference = "Stop"
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
Push-Location $repo
try {
    $payload = Join-Path $repo "work/packages/windows/LibreToon"
    if (Test-Path -LiteralPath $payload) { throw "Preserving existing staging directory: $payload" }
    if (Test-Path -LiteralPath "$repo/dist") {
        if (@(Get-ChildItem -LiteralPath "$repo/dist" -File).Count -gt 0) {throw 'Preserving existing distribution files; use a clean staging workspace'}
    }
    New-Item -ItemType Directory -Force $payload, (Join-Path $repo "dist") | Out-Null
    Copy-Item -LiteralPath "target/release/libretoon.exe" -Destination $payload
    New-Item -ItemType Directory -Force "$payload/assets" | Out-Null
    foreach ($resource in Get-ChildItem -LiteralPath "assets") {
        if ($resource.Name -ne "tools") {Copy-Item -LiteralPath $resource.FullName -Destination "$payload/assets/" -Recurse}
    }
    $font = [IO.Path]::GetFullPath("$payload/assets/fonts/NotoSansCJK-Regular.ttc")
    if (-not $font.StartsWith($payload + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {throw "Unsafe staging path"}
    if (Test-Path -LiteralPath $font) {Remove-Item -LiteralPath $font}
    Copy-Item -LiteralPath "LICENSE" -Destination $payload
    $forbidden = Get-ChildItem -LiteralPath $payload -File -Recurse | Where-Object { $_.Name -in @("ffmpeg.exe","ffprobe.exe","ffmpeg","ffprobe","ffmpeg-complete-source.zip") }
    if ($forbidden) {throw "An external FFmpeg executable/source bundle entered the lightweight payload"}
    $version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
    if ($version -notmatch '^\d+\.\d+\.\d+([-.][A-Za-z0-9.-]+)?$') {throw "Unsafe package version"}
    @"
LibreToon $version for Windows x64
Extract the whole portable ZIP, then open libretoon.exe.
Resources are included. FFmpeg is optional and is downloaded/configured on first video/GIF export, with consent.
Own code: LICENSE (MIT). Third-party notices: assets/THIRD_PARTY_NOTICES.md.
No FFmpeg executable or FFmpeg source bundle is shipped in this package.
Build commit: $env:GITHUB_SHA
"@ | Set-Content -LiteralPath "$payload/README.txt" -Encoding utf8
    & "$PSScriptRoot/windows-sign.ps1" -Path "$payload/libretoon.exe" -Mode $SigningMode
    "Signing mode: $SigningMode. Unsigned packages have no verified publisher signature." | Add-Content -LiteralPath "$payload/README.txt"
    Compress-Archive -LiteralPath $payload -DestinationPath "dist/LibreToon-$version-Windows-x64-Portable.zip"
    if ($StageOnly) {Write-Output "Portable ZIP staged and checked; installer compilation skipped"; return}
    $programFiles = [Environment]::GetFolderPath("ProgramFilesX86")
    $iscc = @("$programFiles/Inno Setup 6/ISCC.exe", "$programFiles/Inno Setup 7/ISCC.exe") |
        Where-Object {Test-Path -LiteralPath $_} | Select-Object -First 1
    if (-not $iscc) {throw "Inno Setup compiler is missing"}
    $arguments=@("/DPayload=$payload", "/DVersion=$version", "/DOutput=$repo/dist")
    if ($SigningMode -eq 'azure') {
        $cache=Join-Path $repo 'work/packages/windows/signing-cache'
        New-Item -ItemType Directory -Force $cache | Out-Null
        $pwsh=(Get-Command pwsh -ErrorAction Stop).Source
        # Inno's $q/$f placeholders quote paths without exposing credentials in arguments.
        $signCommand='$q{0}$q -NoProfile -NonInteractive -File $q{1}$q -Mode azure -Path $f' -f $pwsh, (Join-Path $PSScriptRoot 'windows-sign.ps1')
        $arguments+=@('/DSignBuild=1', "/DSignCache=$cache", ('/SLibreToon='+$signCommand))
    }
    & $iscc @arguments "$repo/scripts/packaging/windows.iss"
    if ($LASTEXITCODE -ne 0) {throw "Windows installer compilation failed"}
    $hashes=@()
    foreach ($file in Get-ChildItem -LiteralPath "$repo/dist" -File | Sort-Object Name) {
        if ($file.Extension -in @(".zip",".exe")) {
            $hashes+="$((Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant())  $($file.Name)"
        }
    }
    $hashes | Set-Content -LiteralPath "$repo/dist/SHA256SUMS.txt" -Encoding utf8
} finally {Pop-Location}
