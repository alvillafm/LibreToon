# Run LibreToon's unmodified export test against the candidate via executable-local assets.
param([Parameter(Mandatory)][string]$CandidateDirectory)
$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
$candidate = [IO.Path]::GetFullPath((Join-Path $repo $CandidateDirectory))
Push-Location $repo
try {
    $records = & cargo test --release --locked --no-run --message-format=json
    if ($LASTEXITCODE -ne 0) {throw "Cannot compile LibreToon export integration test"}
    $artifact = $records | ForEach-Object { try {$_ | ConvertFrom-Json} catch {} } |
        Where-Object {$_.reason -eq "compiler-artifact" -and $_.profile.test -and $_.executable -and $_.target.name -eq "libretoon"} |
        Select-Object -Last 1
    if (-not $artifact) {throw "No LibreToon test executable found"}
    $isolated = Join-Path $candidate "application-test"
    New-Item -ItemType Directory -Force "$isolated/assets/tools" | Out-Null
    $testName = if ($IsWindows) {"libretoon-tests.exe"} else {"libretoon-tests"}
    $binaryName = if ($IsWindows) {"ffmpeg.exe"} else {"ffmpeg"}
    Copy-Item -LiteralPath $artifact.executable -Destination "$isolated/$testName"
    Copy-Item -LiteralPath "$candidate/$binaryName" -Destination "$isolated/assets/tools/$binaryName"
    if (-not $IsWindows) { & chmod +x "$isolated/$testName" "$isolated/assets/tools/$binaryName" }
    & "$isolated/$testName" export_all_containers_with_ffmpeg --ignored --nocapture
    $code = $LASTEXITCODE
    if ($code -ne 0) {throw "LibreToon export integration failed ($code)"}
    @{
        status="passed";testExitCode=$code
        binarySha256=(Get-FileHash -LiteralPath "$candidate/$binaryName").Hash.ToLowerInvariant()
        testBinarySha256=(Get-FileHash -LiteralPath "$isolated/$testName").Hash.ToLowerInvariant()
        exporterSha256=(Get-FileHash -LiteralPath "$repo/src/exporter.rs").Hash.ToLowerInvariant()
        testScriptSha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash.ToLowerInvariant()
        command="export_all_containers_with_ffmpeg --ignored --nocapture"
    } | ConvertTo-Json | Set-Content -LiteralPath "$candidate/APPLICATION_TEST.json" -Encoding utf8
} finally {Pop-Location}
