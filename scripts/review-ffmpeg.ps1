# Review collected build evidence; never changes assets/tools.
param([Parameter(Mandatory)][string]$CandidateDirectory, [Parameter(Mandatory)][string]$ReproducedDirectory)
$ErrorActionPreference = "Stop"
$repo = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
$candidate = [IO.Path]::GetFullPath((Join-Path $repo $CandidateDirectory))
$reproduced = [IO.Path]::GetFullPath((Join-Path $repo $ReproducedDirectory))
function Hash([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
$receipt = Get-Content -LiteralPath "$candidate/BUILD_RECEIPT.json" -Raw | ConvertFrom-Json
$other = Get-Content -LiteralPath "$reproduced/BUILD_RECEIPT.json" -Raw | ConvertFrom-Json
$tests = Get-Content -LiteralPath "$candidate/EXPORT_VALIDATION.json" -Raw | ConvertFrom-Json
$app = Get-Content -LiteralPath "$candidate/APPLICATION_TEST.json" -Raw | ConvertFrom-Json
$lock = Get-Content -LiteralPath "$repo/scripts/ffmpeg/sources.lock.json" -Raw | ConvertFrom-Json
$binaryHash = Hash "$candidate/ffmpeg.exe"
if ($receipt.binarySha256 -ne $binaryHash -or $other.binarySha256 -ne $binaryHash -or (Hash "$reproduced/ffmpeg.exe") -ne $binaryHash) { throw "Independent ffmpeg reproduction failed" }
if ($receipt.probeSha256 -ne (Hash "$candidate/ffprobe.exe") -or $other.probeSha256 -ne $receipt.probeSha256 -or (Hash "$reproduced/ffprobe.exe") -ne $receipt.probeSha256) { throw "Independent ffprobe reproduction failed" }
foreach ($item in @($receipt, $other)) {
    if ($item.lockSha256 -ne (Hash "$repo/scripts/ffmpeg/sources.lock.json") -or $item.recipeSha256 -ne (Hash "$repo/scripts/ffmpeg/build.sh")) { throw "Build used a different lock or recipe" }
    if ($item.traceRecipeSha256 -ne (Hash "$repo/scripts/ffmpeg/trace-link.sh")) {throw "Link trace recipe changed"}
}
if ($tests.status -ne "passed" -or $tests.binarySha256 -ne $binaryHash -or $tests.cases.Count -ne 10 -or $tests.testScriptSha256 -ne (Hash "$repo/scripts/test-ffmpeg.ps1")) { throw "Export quality/contract evidence missing or stale" }
if ($app.status -ne "passed" -or $app.binarySha256 -ne $binaryHash -or $app.testExitCode -ne 0) { throw "LibreToon export integration has not passed" }
$sourceZip = "$candidate/ffmpeg-complete-source.zip"
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($sourceZip)
function EntryText([string]$name) {
    $entry = $zip.GetEntry($name)
    if (-not $entry) { throw "Required source evidence absent from ZIP: $name" }
    $reader = [IO.StreamReader]::new($entry.Open())
    try {$reader.ReadToEnd()} finally {$reader.Dispose()}
}
try {
    foreach ($inputFile in $lock.inputs | Where-Object {$_.role -ne "build-tool"}) {
        $entry = $zip.GetEntry("sources/$($inputFile.file)")
        if (-not $entry) { throw "Corresponding source component missing: $($inputFile.id)" }
        $stream = $entry.Open()
        try {$hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)).ToLowerInvariant()} finally {$stream.Dispose()}
        if ($hash -ne $inputFile.sha256) {throw "Source archive hash mismatch: $($inputFile.id)"}
    }
    $built = (EntryText "BUILD_RECEIPT.json") | ConvertFrom-Json
    if ($built.binarySha256 -ne $binaryHash) {throw "Source ZIP belongs to another binary"}
    $config = EntryText "logs/config.h"
    if ($config -notmatch "#define CONFIG_GPL 1" -or $config -notmatch "#define CONFIG_NONFREE 0") {throw "Unexpected FFmpeg licensing configuration"}
    $imports = @()
    $archives = @()
    foreach ($program in @("ffmpeg","ffprobe")) {
        $importEntry = if ($program -eq "ffmpeg") {"logs/pe-imports.txt"} else {"logs/pe-imports-probe.txt"}
        $imports += [regex]::Matches((EntryText $importEntry),'(?im)^\s*Name:\s*(\S+\.dll)') | ForEach-Object {$_.Groups[1].Value.ToLowerInvariant()}
        $archives += [regex]::Matches((EntryText "logs/$program-link-trace.txt"),'(?im)^ld\.lld: Reading .*?([^/\\\s]+\.a)\s*$') | ForEach-Object {$_.Groups[1].Value.ToLowerInvariant()}
    }
    $imports = @($imports | Sort-Object -Unique)
    $archives = @($archives | Sort-Object -Unique)
    if ($imports.Count -eq 0 -or $archives.Count -lt 4) {throw "Incomplete import/static-link evidence"}
    $allowedDlls = @("kernel32.dll","user32.dll","advapi32.dll","shell32.dll","ole32.dll","ws2_32.dll","bcrypt.dll","secur32.dll","crypt32.dll","psapi.dll","ucrtbase.dll","msvcrt.dll")
    foreach ($dll in $imports) {
        if ($dll -notin $allowedDlls -and $dll -notmatch "^api-ms-win-crt-[a-z0-9-]+\.dll$") {throw "Unreviewed DLL: $dll"}
    }
    $allowedArchives = @("libm.a","libpostproc.a","libunwind.dll.a","libavcodec.a","libavformat.a","libavutil.a","libavfilter.a","libavdevice.a","libswresample.a","libswscale.a","libx264.a","libz.a","libmingw32.a","libmingwex.a","libmoldname.a","libucrt.a","libucrtbase.a","libmsvcrt.a","libclang_rt.builtins-x86_64.a","libkernel32.a","libuser32.a","libadvapi32.a","libshell32.a","libole32.a","libws2_32.a","libbcrypt.a","libsecur32.a","libcrypt32.a","libpsapi.a","libuuid.a")
    foreach ($lib in $archives) {if ($lib -notin $allowedArchives) {throw "Unreviewed static library: $lib"}}
    foreach ($required in @("licenses/COPYING.GPLv3","licenses/LICENSE.md","licenses/x264-COPYING","licenses/x264-HEADER-NOTICE.txt","licenses/zlib-LICENSE","licenses/LLVM-LICENSE.TXT","licenses/compiler-rt-LICENSE.TXT","licenses/compiler-rt-CREDITS.TXT","licenses/libunwind-LICENSE.TXT","licenses/THIRD-PARTY-NOTICES.txt","recipe/scripts/ffmpeg/POLICY.md","REBUILD.txt")) {$null=EntryText $required}
} finally {$zip.Dispose()}
$coverage = foreach ($lib in $archives) {
    $component = if ($lib -match "^lib(av|sw|postproc)") {"ffmpeg"} elseif ($lib -eq "libx264.a") {"x264"} elseif ($lib -eq "libz.a") {"zlib"} elseif ($lib -match "clang_rt|unwind") {"llvm-source"} else {"mingw-source"}
    $source = $lock.inputs | Where-Object {$_.id -eq $component}
    @{library=$lib;sourceComponent=$component;sourceArchive=$source.file;sourceSha256=$source.sha256}
}
$evidence = @{
    status="passed";binarySha256=$binaryHash;sourceSha256=(Hash $sourceZip)
    policySha256=(Hash "$repo/scripts/ffmpeg/POLICY.md");recipeSha256=$receipt.recipeSha256;traceRecipeSha256=$receipt.traceRecipeSha256
    lockSha256=$receipt.lockSha256;reproducedBinarySha256=$other.binarySha256
    imports=$imports;staticArchives=$archives;sourceCoverage=$coverage;exportValidationSha256=(Hash "$candidate/EXPORT_VALIDATION.json")
    applicationTestSha256=(Hash "$candidate/APPLICATION_TEST.json")
    scope="Locked source coverage, link/import inventory, reproducibility and declared tests. Not a legal originality certificate or exhaustive visual study."
}
$evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$candidate/REVIEW.json" -Encoding utf8
@"
# FFmpeg corresponding-source review
Binary SHA-256: $binaryHash
Complete source ZIP SHA-256: $(Hash $sourceZip)
Independent fresh-directory reproduction: ffmpeg.exe and ffprobe.exe byte-identical.
Export cases: 10 passed; LibreToon's existing export integration test passed.
Source coverage and original licenses: scripts/ffmpeg/POLICY.md; detailed evidence in REVIEW.json.
The matching source ZIP is supplied in the same downloadable package as the executable.
This review records the checks performed, not an unrestricted legal certification.
"@ | Set-Content -LiteralPath "$candidate/REVIEW.md" -Encoding utf8
$manifest = @{
    schema=2;status="reviewed-complete";binarySha256=$binaryHash
    correspondingSourceBundle=[IO.Path]::GetRelativePath($repo,$sourceZip)
    correspondingSourceSha256=(Hash $sourceZip)
    reviewRecord=[IO.Path]::GetRelativePath($repo,"$candidate/REVIEW.md")
    reviewEvidence=[IO.Path]::GetRelativePath($repo,"$candidate/REVIEW.json")
    reviewEvidenceSha256=(Hash "$candidate/REVIEW.json")
    exportValidation=[IO.Path]::GetRelativePath($repo,"$candidate/EXPORT_VALIDATION.json")
    applicationTest=[IO.Path]::GetRelativePath($repo,"$candidate/APPLICATION_TEST.json")
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath "$candidate/SOURCE_STATUS.json" -Encoding utf8
Write-Output "Source/build evidence review passed. Distribution gate must still validate this manifest."
