# Automated release build script for M-RustHub
[CmdletBinding()]
param(
    [switch]$Test = $false
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
Set-Location $ProjectRoot

Write-Host "==> [1/3] Building MCHOSE G3 V2 release binary..." -ForegroundColor Cyan

if ($Test) {
    Write-Host "--> Running tests before build..." -ForegroundColor Yellow
    cargo test
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Tests failed! Aborting build."
        exit $LASTEXITCODE
    }
}

cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed!"
    exit $LASTEXITCODE
}

Write-Host "==> [2/3] Preparing clean 'release' directory..." -ForegroundColor Cyan
$ReleaseDir = Join-Path $ProjectRoot "release"
if (-not (Test-Path $ReleaseDir)) {
    New-Item -ItemType Directory -Path $ReleaseDir | Out-Null
}

$SourceExe = Join-Path $ProjectRoot "target\release\m_rusthub.exe"
$DestExe = Join-Path $ReleaseDir "m_rusthub.exe"

Write-Host "==> [3/3] Copying clean executable without artifacts..." -ForegroundColor Cyan
Copy-Item -Path $SourceExe -Destination $DestExe -Force

$FileSize = (Get-Item $DestExe).Length
$FileSizeMB = [math]::Round($FileSize / 1MB, 2)

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " BUILD SUCCESSFUL! Clean release binary is ready:" -ForegroundColor Green
Write-Host " Destination: release\m_rusthub.exe" -ForegroundColor White
Write-Host " File size:   $FileSizeMB MB ($FileSize bytes)" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Green
