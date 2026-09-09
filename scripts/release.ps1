param(
    [string]$Version,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$repoDir = Split-Path -Parent $PSScriptRoot
$frontendDir = Join-Path $repoDir "os-compass"
$syncScript = Join-Path $PSScriptRoot "sync-version.ps1"
$buildScript = Join-Path $PSScriptRoot "build-release.ps1"

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "   OS-Compass 安全发布流水线启动" -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

# 1. 版本同步
if ($Version) {
    Write-Host "[1/5] 同步版本号为 v$Version..." -ForegroundColor Yellow
    & $syncScript $Version
} else {
    Write-Host "[1/5] 校验版本号一致性..." -ForegroundColor Yellow
    & $syncScript
}
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: 版本同步失败" -ForegroundColor Red
    exit 1
}

$cargoToml = Join-Path $frontendDir "src-tauri\Cargo.toml"
$cargoContent = Get-Content $cargoToml -Raw
if ($cargoContent -match 'version\s*=\s*"([^"]+)"') {
    $targetVersion = $matches[1]
} else {
    $targetVersion = "0.1.0"
}
Write-Host "发布目标版本号: v$targetVersion" -ForegroundColor Green

# 2. 安全工程静态扫描
Write-Host "[2/5] 静态安全合规扫描..." -ForegroundColor Yellow

$srcPath = Join-Path $frontendDir "src"
$rustSrcPath = Join-Path $frontendDir "src-tauri\src"
$violations = @()

$files = Get-ChildItem -Path @($srcPath, $rustSrcPath) -Recurse -File -Include *.ts, *.tsx, *.rs
foreach ($f in $files) {
    if ($f.FullName -match '__tests__' -or $f.FullName -match 'embedding_tests\.rs') {
        continue
    }
    $text = Get-Content $f.FullName -Raw
    if ($text -match 'sk-[a-zA-Z0-9]{24,}') {
        $violations += "疑似 API Key 泄露: $($f.Name)"
    }
    if ($text -match '127\.0\.0\.1:8964') {
        $violations += "硬编码代理地址: $($f.Name)"
    }
}

if ($violations.Count -gt 0) {
    Write-Host "❌ 安全合规扫描失败！发现以下违规项：" -ForegroundColor Red
    foreach ($v in $violations) {
        Write-Host "  - $v" -ForegroundColor Red
    }
    Write-Host "根据 docs/安全工程方法论.md，严禁将明文凭据与硬编码代理入库，发布中止！" -ForegroundColor Red
    exit 1
}
Write-Host "✅ 安全静态扫描通过，无凭据与内网代理泄露风险" -ForegroundColor Green

# 3. 前端质量检验
Write-Host "[3/5] 运行类型检查与单元测试..." -ForegroundColor Yellow
Push-Location $frontendDir
try {
    pnpm typecheck
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    pnpm test
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}

# 4. 本地构建与归档
if (-not $SkipBuild) {
    Write-Host "[4/5] 编译 Tauri 安装包并归档..." -ForegroundColor Yellow
    & $buildScript -SkipChecks
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} else {
    Write-Host "[4/5] 跳过本地构建" -ForegroundColor DarkYellow
}

# 5. Git Commit & Release Tag
Write-Host "[5/5] 创建发布提交与 Git Tag..." -ForegroundColor Yellow
$tagName = "v$targetVersion"

$branch = (git branch --show-current).Trim()
Write-Host "当前分支: $branch" -ForegroundColor Cyan

git add os-compass/package.json os-compass/src-tauri/Cargo.toml os-compass/src-tauri/tauri.conf.json CHANGELOG.md
$hasChanges = (git status --porcelain)
if ($hasChanges) {
    git commit -m "chore(release): bump version to $tagName"
}

$tagExists = (git tag -l $tagName)
if ($tagExists) {
    Write-Host "WARN: Tag $tagName 已存在，跳过打标签" -ForegroundColor DarkYellow
} else {
    git tag -a $tagName -m "Release $tagName"
    Write-Host "✅ 成功创建本地 Tag: $tagName" -ForegroundColor Green
}

Write-Host "======================================================" -ForegroundColor Green
Write-Host "   发布准备全部就绪！" -ForegroundColor Green
Write-Host "======================================================" -ForegroundColor Green
Write-Host "请执行以下命令正式推送到 GitHub 触发自动化发布流水线：" -ForegroundColor Cyan
Write-Host "   git push origin $branch" -ForegroundColor White
Write-Host "   git push origin $tagName" -ForegroundColor White
Write-Host "======================================================" -ForegroundColor Cyan
