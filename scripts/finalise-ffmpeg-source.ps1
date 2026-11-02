# Finalise notices and source recipe metadata; does not compile or promote binaries.
param([Parameter(Mandatory)][string]$WorkDirectory, [Parameter(Mandatory)][string]$DownloadDirectory)
$ErrorActionPreference="Stop"
$repo=Split-Path $PSScriptRoot -Parent
$work=[IO.Path]::GetFullPath((Join-Path $repo $WorkDirectory))
$downloads=[IO.Path]::GetFullPath((Join-Path $repo $DownloadDirectory))
$stage=Join-Path $work "source-package"
$receipt=Get-Content "$work/output/BUILD_RECEIPT.json" -Raw | ConvertFrom-Json
if ($receipt.recipeSha256 -ne (Get-FileHash "$repo/scripts/ffmpeg/build.sh").Hash.ToLowerInvariant()) {throw "Cannot finalise a package for another build recipe"}
function Extract-Notice([string]$entry,[string]$target) {
    $info=[Diagnostics.ProcessStartInfo]::new("tar")
    $info.UseShellExecute=$false
    $info.CreateNoWindow=$true
    $info.RedirectStandardOutput=$true
    $info.RedirectStandardError=$true
    foreach($argValue in @("-xOf","$downloads/llvm-project-18.1.8.tar.gz",$entry)) {$info.ArgumentList.Add($argValue)}
    $p=[Diagnostics.Process]::Start($info)
    $err=$p.StandardError.ReadToEndAsync()
    $out=[IO.File]::Create($target)
    try {$p.StandardOutput.BaseStream.CopyTo($out)} finally {$out.Dispose()}
    $p.WaitForExit()
    if($p.ExitCode -ne 0) {throw $err.GetAwaiter().GetResult()}
    $p.Dispose()
}
Extract-Notice "llvm-project-llvmorg-18.1.8/compiler-rt/LICENSE.TXT" "$stage/licenses/compiler-rt-LICENSE.TXT"
Extract-Notice "llvm-project-llvmorg-18.1.8/compiler-rt/CREDITS.TXT" "$stage/licenses/compiler-rt-CREDITS.TXT"
Extract-Notice "llvm-project-llvmorg-18.1.8/libunwind/LICENSE.TXT" "$stage/licenses/libunwind-LICENSE.TXT"
foreach($file in @("build-ffmpeg.ps1","finalise-ffmpeg-source.ps1","test-ffmpeg.ps1","test-ffmpeg-app.ps1","review-ffmpeg.ps1","check-ffmpeg-distribution.ps1")) {
    Copy-Item -LiteralPath "$repo/scripts/$file" -Destination "$stage/recipe/scripts/$file" -Force
}
Copy-Item -LiteralPath "$repo/scripts/ffmpeg/trace-link.sh","$repo/scripts/ffmpeg/POLICY.md" -Destination "$stage/recipe/scripts/ffmpeg/" -Force
$receipt | Add-Member -NotePropertyName traceRecipeSha256 -NotePropertyValue ((Get-FileHash "$repo/scripts/ffmpeg/trace-link.sh").Hash.ToLowerInvariant()) -Force
$receipt | ConvertTo-Json -Depth 8 | Set-Content "$stage/BUILD_RECEIPT.json" -Encoding utf8
Copy-Item -LiteralPath "$stage/BUILD_RECEIPT.json" -Destination "$work/output/BUILD_RECEIPT.json" -Force
Copy-Item -Path "$work/logs/*" -Destination "$stage/logs/" -Force
Copy-Item -LiteralPath "$work/logs/x264-config.mak","$work/logs/x264-config.h","$work/logs/zlib.pc" -Destination "$stage/logs/" -Force
New-Item -ItemType Directory -Force "$work/output/licenses" | Out-Null
Copy-Item -Path "$stage/licenses/*" -Destination "$work/output/licenses/" -Force
Copy-Item -LiteralPath "$stage/licenses/THIRD-PARTY-NOTICES.txt" -Destination "$work/output/" -Force
$bundle="$work/output/ffmpeg-complete-source.zip"
# Preserve the previous, build-produced package rather than deleting it.
$previous="$work/source-package-before-finalisation-$([Guid]::NewGuid().ToString('N')).zip"
Move-Item -LiteralPath $bundle -Destination $previous
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip=[IO.Compression.ZipFile]::Open($bundle,[IO.Compression.ZipArchiveMode]::Create)
try {
 foreach($file in Get-ChildItem -LiteralPath $stage -Recurse -File | Sort-Object FullName) {
  $name=[IO.Path]::GetRelativePath($stage,$file.FullName).Replace("\","/")
  $entry=$zip.CreateEntry($name,[IO.Compression.CompressionLevel]::Fastest)
  $entry.LastWriteTime=[DateTimeOffset]::new(2024,6,19,0,0,0,[TimeSpan]::Zero)
  $inputStream=[IO.File]::OpenRead($file.FullName); $outputStream=$entry.Open()
  try {$inputStream.CopyTo($outputStream)} finally {$inputStream.Dispose();$outputStream.Dispose()}
 }
} finally {$zip.Dispose()}
Write-Output "Source package finalised with original compiler-rt notices: $bundle"
