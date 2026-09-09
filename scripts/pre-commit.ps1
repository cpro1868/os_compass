#!/usr/bin/env pwsh
# Pre-commit check script for OS-Compass
# Run before committing to verify code quality

$ErrorActionPreference = "Stop"
$projectDir = Split-Path -Parent $PSScriptRoot
$frontendDir = Join-Path $projectDir "os-compass"

Write-Host "=== OS-Compass Pre-commit Check ===" -ForegroundColor Cyan

# 1. Frontend typecheck
Write-Host "`n[1/3] TypeScript typecheck..." -ForegroundColor Yellow
Push-Location $frontendDir
try {
    pnpm typecheck 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: TypeScript typecheck failed" -ForegroundColor Red
        pnpm typecheck
        exit 1
    }
    Write-Host "PASS: TypeScript typecheck" -ForegroundColor Green
} finally {
    Pop-Location
}

# 2. Frontend tests
Write-Host "`n[2/3] Frontend tests..." -ForegroundColor Yellow
Push-Location $frontendDir
try {
    pnpm test 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAIL: Frontend tests failed" -ForegroundColor Red
        pnpm test
        exit 1
    }
    Write-Host "PASS: Frontend tests" -ForegroundColor Green
} finally {
    Pop-Location
}

# 3. Check for sensitive files
Write-Host "`n[3/3] Sensitive file check..." -ForegroundColor Yellow
$sensitivePatterns = @("*.db", "*.cryptokey", ".env", "*.key")
$found = @()
foreach ($pattern in $sensitivePatterns) {
    $files = Get-ChildItem $projectDir -Filter $pattern -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch "node_modules|target|\.git" }
    if ($files) { $found += $files.Name }
}
if ($found.Count -gt 0) {
    Write-Host "WARN: Sensitive files found (ensure they are in .gitignore): $($found -join ', ')" -ForegroundColor Yellow
} else {
    Write-Host "PASS: No sensitive files in working tree" -ForegroundColor Green
}

Write-Host "`n=== All checks passed ===" -ForegroundColor Cyan
