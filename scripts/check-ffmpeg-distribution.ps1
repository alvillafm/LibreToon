# Fail closed: require original source inputs, reproduction and export evidence for this binary.
param(
    [string]$Destination = "dist",
    [string]$ManifestPath = "assets/tools/SOURCE_STATUS.json",
    [string]$BinaryPath = "assets/tools/ffmpeg.exe"
)
$ErrorActionPreference = "Stop"
$repoRoot = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
function Safe-File([string]$relative) {
    if ([IO.Path]::IsPathRooted($relative)) {throw "Source evidence must be repository-relative."}
    $path = [IO.Path]::GetFullPath((Join-Path $repoRoot $relative))
    if (-not $path.StartsWith($repoRoot + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) {throw "Source evidence escapes the repository."}
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {throw "Missing evidence: $relative"}
    return $path
}
function Hash([string]$path) {(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
$manifest = Get-Content -LiteralPath (Safe-File $ManifestPath) -Raw | ConvertFrom-Json
$binary = Safe-File $BinaryPath
if ((Hash $binary) -ne $manifest.binarySha256) {throw "FFmpeg hash changed: repeat source and licensing review."}
if ($manifest.status -ne "reviewed-complete") {throw "FFmpeg distribution BLOCKED: complete corresponding source and notices are unverified. See assets/tools/THIRD_PARTY.md."}
if ($manifest.schema -ne 2) {throw "Reviewed build evidence schema is required; a status flag alone is insufficient."}
foreach ($field in @("correspondingSourceBundle","correspondingSourceSha256","reviewRecord","reviewEvidence","reviewEvidenceSha256","exportValidation","applicationTest")) {
    if (-not $manifest.$field) {throw "Missing FFmpeg source/build evidence: $field"}
}
$bundle = Safe-File $manifest.correspondingSourceBundle
if ([IO.Path]::GetExtension($bundle) -ne ".zip" -or (Hash $bundle) -ne $manifest.correspondingSourceSha256) {throw "FFmpeg source ZIP checksum/type mismatch."}
$reviewPath = Safe-File $manifest.reviewEvidence
if ((Hash $reviewPath) -ne $manifest.reviewEvidenceSha256) {throw "Source review evidence checksum mismatch."}
$review = Get-Content -LiteralPath $reviewPath -Raw | ConvertFrom-Json
$exportsPath = Safe-File $manifest.exportValidation
$appPath = Safe-File $manifest.applicationTest
$exports = Get-Content -LiteralPath $exportsPath -Raw | ConvertFrom-Json
$app = Get-Content -LiteralPath $appPath -Raw | ConvertFrom-Json
if ($review.status -ne "passed" -or $review.binarySha256 -ne $manifest.binarySha256 -or $review.reproducedBinarySha256 -ne $manifest.binarySha256 -or $review.sourceSha256 -ne $manifest.correspondingSourceSha256) {throw "Binary/source/reproduction review failed."}
if ($review.recipeSha256 -ne (Hash "$repoRoot/scripts/ffmpeg/build.sh") -or $review.lockSha256 -ne (Hash "$repoRoot/scripts/ffmpeg/sources.lock.json") -or $review.policySha256 -ne (Hash "$repoRoot/scripts/ffmpeg/POLICY.md")) {throw "Build/source policy changed since review."}
if ($review.exportValidationSha256 -ne (Hash $exportsPath) -or $review.applicationTestSha256 -ne (Hash $appPath)) {throw "Validation evidence checksum mismatch."}
if ($exports.status -ne "passed" -or $exports.binarySha256 -ne $manifest.binarySha256 -or $exports.cases.Count -ne 10 -or $exports.testScriptSha256 -ne (Hash "$repoRoot/scripts/test-ffmpeg.ps1")) {throw "Export quality/contract validation absent or stale."}
if ($app.status -ne "passed" -or $app.binarySha256 -ne $manifest.binarySha256 -or $app.testExitCode -ne 0 -or $app.exporterSha256 -ne (Hash "$repoRoot/src/exporter.rs") -or $app.testScriptSha256 -ne (Hash "$repoRoot/scripts/test-ffmpeg-app.ps1")) {throw "Application export validation absent or stale."}
if ($review.traceRecipeSha256 -ne (Hash "$repoRoot/scripts/ffmpeg/trace-link.sh")) {throw "Link trace recipe changed since review."}
$lock = Get-Content -LiteralPath "$repoRoot/scripts/ffmpeg/sources.lock.json" -Raw | ConvertFrom-Json
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($bundle)
try {
    foreach ($inputFile in $lock.inputs | Where-Object {$_.role -ne "build-tool"}) {
        $entry = $zip.GetEntry("sources/$($inputFile.file)")
        if (-not $entry) {throw "Missing corresponding source: $($inputFile.id)"}
        $stream = $entry.Open()
        try {$hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)).ToLowerInvariant()} finally {$stream.Dispose()}
        if ($hash -ne $inputFile.sha256) {throw "Source checksum mismatch: $($inputFile.id)"}
    }
    $entry = $zip.GetEntry("BUILD_RECEIPT.json")
    if (-not $entry) {throw "Source build receipt missing"}
    $reader = [IO.StreamReader]::new($entry.Open())
    try {$built=$reader.ReadToEnd() | ConvertFrom-Json} finally {$reader.Dispose()}
    if ($built.binarySha256 -ne $manifest.binarySha256 -or $built.lockSha256 -ne $review.lockSha256 -or $built.recipeSha256 -ne $review.recipeSha256 -or $built.traceRecipeSha256 -ne $review.traceRecipeSha256) {throw "Source ZIP is not bound to this binary/recipe."}
} finally {$zip.Dispose()}
$sourceDir = Join-Path $Destination "corresponding-source"
New-Item -ItemType Directory -Force $sourceDir | Out-Null
Copy-Item -LiteralPath $bundle -Destination "$sourceDir/ffmpeg-complete-source.zip"
Copy-Item -LiteralPath (Safe-File $manifest.reviewRecord) -Destination "$sourceDir/REVIEW.md"
Copy-Item -LiteralPath $reviewPath, $exportsPath, $appPath -Destination $sourceDir
Copy-Item -LiteralPath (Safe-File $ManifestPath) -Destination "$sourceDir/SOURCE_STATUS.json"
@"
FFmpeg complete corresponding source is included in corresponding-source/ffmpeg-complete-source.zip.
It is supplied together with this executable download, with no additional charge.
Keep this source ZIP and the notices when redistributing the binary.
If publishing the executable elsewhere, publish the matching source ZIP alongside it for as long as the executable is offered.
See corresponding-source/REVIEW.md and assets/tools/licenses/ for source evidence and original notices.
LibreToon's own code remains under the root MIT LICENSE; FFmpeg is a separate GPLv3-or-later program.
"@ | Set-Content -LiteralPath (Join-Path $Destination "SOURCE-ACCESS.txt") -Encoding utf8
Write-Output "FFmpeg distribution checks passed; corresponding source included in $sourceDir"
