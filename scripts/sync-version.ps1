#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"
$repoDir = Split-Path -Parent $PSScriptRoot
$cargoToml = Join-Path $repoDir "os-compass\src-tauri\Cargo.toml"
$tauriConf = Join-Path $repoDir "os-compass\src-tauri\tauri.conf.json"
$packageJson = Join-Path $repoDir "os-compass\package.json"

if ($args.Count -gt 0) {
    $newVersion = $args[0]
    if ($newVersion -notmatch '^\d+\.\d+\.\d+') {
        Write-Host "ERROR: 版本号格式无效（应为 x.y.z）: $newVersion" -ForegroundColor Red
        exit 1
    }
    $content = Get-Content $cargoToml -Raw -Encoding UTF8
    $content = $content -replace '(?m)^version = ".*"$', "version = `"$newVersion`""
    Set-Content $cargoToml $content -Encoding UTF8 -NoNewline
    Write-Host "Cargo.toml -> $newVersion" -ForegroundColor Green
} else {
    $newVersion = (Select-String -Path $cargoToml -Pattern '^version = "(.*)"').Matches[0].Groups[1].Value
    Write-Host "Cargo.toml 当前版本: $newVersion" -ForegroundColor Cyan
}

$conf = Get-Content $tauriConf -Raw -Encoding UTF8 | ConvertFrom-Json
if ($conf.version -ne $newVersion) {
    $conf.version = $newVersion
    $conf | ConvertTo-Json -Depth 10 | Set-Content $tauriConf -Encoding UTF8
    Write-Host "tauri.conf.json -> $newVersion" -ForegroundColor Green
} else {
    Write-Host "tauri.conf.json 已同步" -ForegroundColor DarkGray
}

$pkg = Get-Content $packageJson -Raw -Encoding UTF8 | ConvertFrom-Json
if ($pkg.version -ne $newVersion) {
    $pkg.version = $newVersion
    $pkg | ConvertTo-Json -Depth 10 | Set-Content $packageJson -Encoding UTF8
    Write-Host "package.json -> $newVersion" -ForegroundColor Green
} else {
    Write-Host "package.json 已同步" -ForegroundColor DarkGray
}

Write-Host "`n版本同步完成: $newVersion" -ForegroundColor Cyan
