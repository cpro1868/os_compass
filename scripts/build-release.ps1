#!/usr/bin/env pwsh
param(
    [string]$Version,
    [switch]$SkipChecks
)

$ErrorActionPreference = "Stop"
$repoDir = Split-Path -Parent $PSScriptRoot
$frontendDir = Join-Path $repoDir "os-compass"
$releaseDir = Join-Path $frontendDir "src-tauri\target\release"
$previousDir = Join-Path $repoDir "Previous"
$syncScript = Join-Path $PSScriptRoot "sync-version.ps1"
$cargoToml = Join-Path $repoDir "os-compass\src-tauri\Cargo.toml"

if ($Version) {
    & $syncScript $Version
} else {
    & $syncScript
}
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Push-Location $frontendDir
try {
    if (-not $SkipChecks) {
        Write-Host "[1/4] TypeScript typecheck..." -ForegroundColor Yellow
        pnpm typecheck
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        Write-Host "[2/4] Frontend tests..." -ForegroundColor Yellow
        pnpm test
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } else {
        Write-Host "[1/4] Checks skipped" -ForegroundColor DarkYellow
    }

    Write-Host "[3/4] Building Tauri bundles..." -ForegroundColor Yellow
    pnpm tauri build
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

$executable = Join-Path $releaseDir "os-compass.exe"
$loader = Join-Path $releaseDir "WebView2Loader.dll"
$msiDir = Join-Path $releaseDir "bundle\msi"
$nsisDir = Join-Path $releaseDir "bundle\nsis"

$requiredFiles = @($executable, $loader)
foreach ($file in $requiredFiles) {
    if (-not (Test-Path $file)) {
        Write-Host "ERROR: 构建产物不存在: $file" -ForegroundColor Red
        exit 1
    }
}
if (-not (Get-ChildItem $msiDir -Filter "*.msi" -File -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: MSI 产物不存在" -ForegroundColor Red
    exit 1
}
if (-not (Get-ChildItem $nsisDir -Filter "*.exe" -File -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: NSIS 产物不存在" -ForegroundColor Red
    exit 1
}

New-Item -ItemType Directory -Path $previousDir -Force | Out-Null
Copy-Item $executable (Join-Path $previousDir "os-compass.exe") -Force
Copy-Item $loader (Join-Path $previousDir "WebView2Loader.dll") -Force

$releaseStageDir = Join-Path $repoDir "release"
New-Item -ItemType Directory -Path $releaseStageDir -Force | Out-Null
foreach ($file in @(Get-ChildItem $msiDir -Filter "*.msi" -File)) {
    Copy-Item $file.FullName (Join-Path $releaseStageDir $file.Name) -Force
}
foreach ($file in @(Get-ChildItem $nsisDir -Filter "*.exe" -File)) {
    Copy-Item $file.FullName (Join-Path $releaseStageDir $file.Name) -Force
}

Write-Host "[4/4] Release artifacts verified" -ForegroundColor Green
Write-Host "MSI: $msiDir" -ForegroundColor Cyan
Write-Host "NSIS: $nsisDir" -ForegroundColor Cyan
Write-Host "Previous updated: $previousDir" -ForegroundColor Cyan
Write-Host "Staged: $releaseStageDir" -ForegroundColor Cyan
