# Package only an already-built and already-reviewed candidate. Never build or promote it here.
param([string]$CandidateDirectory = "work/ffmpeg-owned-final/output")
$ErrorActionPreference = "Stop"
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))
Push-Location $repo
try {
    $payload = Join-Path $repo "work/packages/windows/LibreToon"
    if (Test-Path -LiteralPath $payload) { throw "Preserving existing staging directory: $payload" }
    New-Item -ItemType Directory -Force $payload, (Join-Path $repo "dist") | Out-Null
    Copy-Item -LiteralPath "target/release/libretoon.exe" -Destination $payload
    Copy-Item -LiteralPath "assets" -Destination "$payload/assets" -Recurse
    $font = [IO.Path]::GetFullPath("$payload/assets/fonts/NotoSansCJK-Regular.ttc")
    if (-not $font.StartsWith($payload + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {throw "Unsafe staging path"}
    if (Test-Path -LiteralPath $font) {Remove-Item -LiteralPath $font}
    Copy-Item -LiteralPath "LICENSE" -Destination $payload
    Copy-Item -LiteralPath "$CandidateDirectory/ffmpeg.exe" -Destination "$payload/assets/tools/ffmpeg.exe" -Force
    Copy-Item -LiteralPath "$CandidateDirectory/THIRD-PARTY-NOTICES.txt" -Destination "$payload/assets/tools/THIRD_PARTY.md" -Force
    Copy-Item -Path "$CandidateDirectory/licenses/*" -Destination "$payload/assets/tools/licenses/" -Force
    Copy-Item -LiteralPath "$CandidateDirectory/SOURCE_STATUS.json" -Destination "$payload/assets/tools/" -Force
    Copy-Item -LiteralPath "$CandidateDirectory/REVIEW.json" -Destination "$payload/assets/tools/BUILD_EVIDENCE.json" -Force
    & "$repo/scripts/check-ffmpeg-distribution.ps1" -Destination $payload -ManifestPath "$CandidateDirectory/SOURCE_STATUS.json" -BinaryPath "$CandidateDirectory/ffmpeg.exe"
    if ((Get-FileHash "$payload/assets/tools/ffmpeg.exe").Hash -ne (Get-FileHash "$CandidateDirectory/ffmpeg.exe").Hash) {throw "Staged FFmpeg differs from reviewed binary"}
    $version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
    if ($version -notmatch '^\d+\.\d+\.\d+([-.][A-Za-z0-9.-]+)?$') {throw "Unsafe package version"}
    @"
LibreToon $version for Windows x64
Extract the whole portable ZIP, then open libretoon.exe.
FFmpeg and resources are included; no manual FFmpeg installation is needed.
Own code: LICENSE (MIT). Third-party notices: assets/THIRD_PARTY_NOTICES.md.
Complete FFmpeg sources: corresponding-source/ffmpeg-complete-source.zip.
Build commit: $env:GITHUB_SHA
"@ | Set-Content -LiteralPath "$payload/README.txt" -Encoding utf8
    Compress-Archive -LiteralPath $payload -DestinationPath "dist/LibreToon-$version-Windows-x64-Portable.zip"
    $programFiles = [Environment]::GetFolderPath("ProgramFilesX86")
    $iscc = @("$programFiles/Inno Setup 6/ISCC.exe", "$programFiles/Inno Setup 7/ISCC.exe") |
        Where-Object {Test-Path -LiteralPath $_} | Select-Object -First 1
    if (-not $iscc) {throw "Inno Setup compiler is missing"}
    & $iscc "/DPayload=$payload" "/DVersion=$version" "/DOutput=$repo/dist" "$repo/scripts/packaging/windows.iss"
    if ($LASTEXITCODE -ne 0) {throw "Windows installer compilation failed"}
    foreach ($file in Get-ChildItem -LiteralPath "$repo/dist" -File) {
        if ($file.Extension -in @(".zip",".exe")) {
            "$((Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant())  $($file.Name)" | Add-Content -LiteralPath "$repo/dist/SHA256SUMS.txt"
        }
    }
} finally {Pop-Location}
