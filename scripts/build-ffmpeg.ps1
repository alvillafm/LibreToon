# Build in an isolated directory. Never writes assets/tools or the existing FFmpeg.
[CmdletBinding()]
param(
    [string]$WorkDirectory = "work/ffmpeg-owned",
    [string]$DownloadDirectory = "work/ffmpeg-downloads",
    [string]$ToolDirectory = "",
    [int]$Jobs = 6
)
$ErrorActionPreference = "Stop"
$repo = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
function Under-Repo([string]$value) {
    $path = [IO.Path]::GetFullPath((Join-Path $repo $value))
    if (-not $path.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Build paths must remain inside this checkout: $value"
    }
    return $path
}
function Native([string]$exe, [string[]]$arguments) {
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "$exe failed with exit code $LASTEXITCODE" }
}
$work = Under-Repo $WorkDirectory
$downloads = Under-Repo $DownloadDirectory
$tools = if ($ToolDirectory) { Under-Repo $ToolDirectory } else { Join-Path $work "tools" }
if (Test-Path -LiteralPath (Join-Path $work "build-ffmpeg")) {
    throw "Use a fresh WorkDirectory; existing compilation files are preserved: $work"
}
$lockPath = Join-Path $repo "scripts/ffmpeg/sources.lock.json"
$lock = Get-Content -LiteralPath $lockPath -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Force $work, $downloads, $tools | Out-Null
$byId = @{}
foreach ($inputFile in $lock.inputs) {
    $path = Join-Path $downloads $inputFile.file
    if (-not (Test-Path -LiteralPath $path)) { Invoke-WebRequest -Uri $inputFile.url -OutFile $path -TimeoutSec 600 }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $inputFile.sha256) {
        throw "Locked input checksum mismatch: $($inputFile.id). File preserved for investigation."
    }
    $byId[$inputFile.id] = $path
}
$toolchain = Join-Path $tools "llvm-mingw-20240619-ucrt-x86_64"
if (-not (Test-Path -LiteralPath $toolchain)) { Native tar @("-xf", $byId.toolchain, "-C", $tools) }
if (-not (Test-Path -LiteralPath (Join-Path $tools "msys64"))) {
    Native tar @("-xf", $byId.msys2, "-C", $tools)
    foreach ($id in @("make", "pkgconf", "diffutils")) { Native tar @("-xf", $byId[$id], "-C", (Join-Path $tools "msys64")) }
}
if (-not (Test-Path -LiteralPath (Join-Path $tools "nasm-2.16.03"))) { Native tar @("-xf", $byId.nasm, "-C", $tools) }
# Verify every toolchain file against the locked official ZIP, even when reusing tools.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($byId.toolchain)
try {
    foreach ($entry in $zip.Entries) {
        if ($entry.FullName.EndsWith("/")) { continue }
        $local = Join-Path $tools $entry.FullName
        if (-not (Test-Path -LiteralPath $local -PathType Leaf)) { throw "Toolchain file missing: $local" }
        $stream = $entry.Open()
        try { $expected = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)).ToLowerInvariant() } finally { $stream.Dispose() }
        if ((Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw "Modified toolchain file: $local" }
    }
} finally { $zip.Dispose() }
foreach ($id in @("ffmpeg", "x264", "zlib")) {
    $source = Join-Path $work "src/$id"
    New-Item -ItemType Directory -Force $source | Out-Null
    Native tar @("-xf", $byId[$id], "-C", $source, "--strip-components=1")
}
$env:FFMPEG_BUILD_ROOT = $work
$env:FFMPEG_TOOL_ROOT = $tools
$env:FFMPEG_BUILD_JOBS = "$Jobs"
Copy-Item -LiteralPath (Join-Path $repo "scripts/ffmpeg/build.sh") -Destination (Join-Path $work "build-recipe.sh")
Native (Join-Path $tools "msys64/usr/bin/bash.exe") @("--noprofile", "--norc", (Join-Path $work "build-recipe.sh"))
$preTrace = (Get-FileHash -LiteralPath "$work/output/ffmpeg.exe").Hash
Native (Join-Path $tools "msys64/usr/bin/bash.exe") @("--noprofile", "--norc", (Join-Path $repo "scripts/ffmpeg/trace-link.sh"))
if ((Get-FileHash -LiteralPath "$work/output/ffmpeg.exe").Hash -ne $preTrace) {throw "Link diagnostics changed the binary"}
$stage = Join-Path $work "source-package"
New-Item -ItemType Directory -Force "$stage/sources", "$stage/recipe/scripts/ffmpeg", "$stage/logs", "$stage/licenses" | Out-Null
foreach ($inputFile in $lock.inputs | Where-Object { $_.role -ne "build-tool" }) {
    Copy-Item -LiteralPath $byId[$inputFile.id] -Destination (Join-Path $stage "sources/$($inputFile.file)")
}
Copy-Item -LiteralPath $lockPath -Destination "$stage/recipe/scripts/ffmpeg/"
foreach ($path in @("scripts/build-ffmpeg.ps1", "scripts/test-ffmpeg.ps1", "scripts/test-ffmpeg-app.ps1", "scripts/review-ffmpeg.ps1", "scripts/check-ffmpeg-distribution.ps1", "scripts/ffmpeg/build.sh", "scripts/ffmpeg/trace-link.sh", "scripts/ffmpeg/POLICY.md")) {
    $target = Join-Path $stage "recipe/$path"
    New-Item -ItemType Directory -Force (Split-Path $target -Parent) | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination $target
}
Copy-Item -LiteralPath (Join-Path $repo "LICENSE") -Destination "$stage/recipe/LICENSE"
Copy-Item -LiteralPath "$work/build-recipe.sh" -Destination "$stage/recipe/scripts/ffmpeg/build.sh"
Copy-Item -Path "$work/logs/*" -Destination "$stage/logs/"
Copy-Item -Path "$work/src/ffmpeg/COPYING*", "$work/src/ffmpeg/LICENSE.md" -Destination "$stage/licenses/"
Copy-Item -LiteralPath "$work/src/x264/COPYING" -Destination "$stage/licenses/x264-COPYING"
Copy-Item -LiteralPath "$work/src/zlib/LICENSE" -Destination "$stage/licenses/zlib-LICENSE"
Copy-Item -LiteralPath "$toolchain/LICENSE.TXT" -Destination "$stage/licenses/LLVM-LICENSE.TXT"
$x264Notice = (Get-Content -LiteralPath "$work/src/x264/x264.h" -Raw).Split("#ifndef X264_X264_H")[0]
$x264Notice | Set-Content -LiteralPath "$stage/licenses/x264-HEADER-NOTICE.txt" -Encoding utf8
# Source archives remain byte-for-byte original; expose all MinGW notice files too.
$mingwNotices = & tar -tf $byId["mingw-source"] | Where-Object { $_ -match "/(COPYING[^/]*|LICENSE[^/]*)$" }
if ($LASTEXITCODE -ne 0) { throw "Cannot list MinGW notices" }
foreach ($entry in $mingwNotices) {
    $name = $entry.Substring($entry.IndexOf("/") + 1).Replace("/", "__")
    (& tar -xOf $byId["mingw-source"] $entry) | Set-Content -LiteralPath "$stage/licenses/mingw-$name" -Encoding utf8
    if ($LASTEXITCODE -ne 0) { throw "Cannot extract MinGW notice: $entry" }
}
@"
FFmpeg 7.1.5 + x264 + zlib, built for LibreToon as a separate GPLv3-or-later program.
LibreToon's own code and build orchestration remain MIT (recipe/LICENSE).
FFmpeg: Copyright (c) 2000-2026 the FFmpeg developers; see original source headers.
x264: see x264-HEADER-NOTICE.txt and x264-COPYING.
zlib: see zlib-LICENSE.
LLVM/compiler-rt and MinGW-w64 CRT/import libraries: see their notices and complete source archives.
This software is based in part on the work of the Independent JPEG Group.
FFmpeg libjpeg-derived source files have no project source patches.
All distributed source archives preserve their original notices and individual file licenses.
Build tools (compiler/MSYS2/make/pkgconf/diffutils/NASM) are not shipped as runtime programs.
Their versions, binary hashes and official download locations are recorded in sources.lock.json.
Only normal Windows system DLLs may be imported; the review step checks the actual PE imports.
"@ | Set-Content -LiteralPath "$stage/licenses/THIRD-PARTY-NOTICES.txt" -Encoding utf8
@"
Complete source inputs are in sources/; compilation and installation scripts are in recipe/.
To rebuild on Windows x64: extract recipe/ into a writable checkout; run PowerShell 7:
  ./scripts/build-ffmpeg.ps1 -WorkDirectory work/rebuilt
Before running, put the archives from sources/ in work/ffmpeg-downloads/ (matching locked names).
The recipe downloads only missing locked tool/input archives and checks every SHA-256.
The source package includes LLVM/compiler-rt, MinGW-w64 and the toolchain's exact build recipes.
No project source patches are applied to FFmpeg/x264/zlib.
zlib.pc is generated from its upstream template, x264 generates its configuration normally.
Only absolute build paths in generated FFmpeg config.h are normalised to /build for reproducibility.
Actual configure commands, generated configuration, link map and build logs are preserved in logs/.
The application integration and final artifact must separately pass tests and legal/source review.
This source package alone does not certify legal compliance of a different binary.
"@ | Set-Content -LiteralPath "$stage/REBUILD.txt" -Encoding utf8
$binary = Join-Path $work "output/ffmpeg.exe"
$receipt = [ordered]@{
    schema = 1
    binarySha256 = (Get-FileHash -LiteralPath $binary).Hash.ToLowerInvariant()
    probeSha256 = (Get-FileHash -LiteralPath "$work/output/ffprobe.exe").Hash.ToLowerInvariant()
    lockSha256 = (Get-FileHash -LiteralPath $lockPath).Hash.ToLowerInvariant()
    recipeSha256 = (Get-FileHash -LiteralPath "$work/build-recipe.sh").Hash.ToLowerInvariant()
    toolchainCompilerSha256 = (Get-FileHash -LiteralPath "$toolchain/bin/clang.exe").Hash.ToLowerInvariant()
    target = $lock.target
    status = "built-unreviewed"
}
$receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$stage/BUILD_RECEIPT.json" -Encoding utf8
Copy-Item -LiteralPath "$stage/BUILD_RECEIPT.json" -Destination "$work/output/"
# Fixed entry order/timestamps. Logs may contain host paths; EXE reproducibility is checked separately.
$bundle = Join-Path $work "output/ffmpeg-complete-source.zip"
$archive = [IO.Compression.ZipFile]::Open($bundle, [IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($file in Get-ChildItem -LiteralPath $stage -File -Recurse | Sort-Object FullName) {
        $relative = [IO.Path]::GetRelativePath($stage, $file.FullName).Replace("\", "/")
        $entry = $archive.CreateEntry($relative, [IO.Compression.CompressionLevel]::Fastest)
        $entry.LastWriteTime = [DateTimeOffset]::new(2024, 6, 19, 0, 0, 0, [TimeSpan]::Zero)
        $inputStream=[IO.File]::OpenRead($file.FullName); $outputStream=$entry.Open()
        try {$inputStream.CopyTo($outputStream)} finally {$inputStream.Dispose();$outputStream.Dispose()}
    }
} finally { $archive.Dispose() }
Copy-Item -LiteralPath "$stage/licenses/THIRD-PARTY-NOTICES.txt" -Destination "$work/output/"
Copy-Item -LiteralPath "$stage/licenses" -Destination "$work/output/licenses" -Recurse
Write-Output "Candidate built, NOT promoted or approved: $binary"
Write-Output "Matching source bundle: $bundle"

& (Join-Path $PSScriptRoot "finalise-ffmpeg-source.ps1") -WorkDirectory $WorkDirectory -DownloadDirectory $DownloadDirectory
