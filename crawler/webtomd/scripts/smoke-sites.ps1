param(
    [string]$OutputDir = "test-output/sites"
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outputRoot = Join-Path $repoRoot $OutputDir
$binary = Join-Path $repoRoot "web2md.exe"

Push-Location $repoRoot
go build -o web2md.exe .
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    exit $LASTEXITCODE
}
Pop-Location

$cases = @(
    @{
        Site = "example"
        Name = "example"
        Url  = "https://example.com"
    },
    @{
        Site = "go-blog"
        Name = "go-blog"
        Url  = "https://go.dev/blog/go1.22"
    },
    @{
        Site = "wechat"
        Name = "wechat-khazix"
        Url  = "https://mp.weixin.qq.com/s/Y_uRMYBmdLWUPnz_ac7jWA"
    }
)

$failed = 0
foreach ($case in $cases) {
    $caseDir = Join-Path $outputRoot $case.Site
    New-Item -ItemType Directory -Force $caseDir | Out-Null

    Write-Host "==> $($case.Site): $($case.Url)"
    Push-Location $caseDir
    & $binary $case.Url -n $case.Name
    $exitCode = $LASTEXITCODE
    Pop-Location

    if ($exitCode -ne 0) {
        Write-Warning "Smoke failed for $($case.Site) with exit code $exitCode"
        $failed++
        continue
    }

    $outputFile = Join-Path $caseDir "$($case.Name).md"
    if (-not (Test-Path $outputFile)) {
        Write-Warning "Smoke did not create expected file: $outputFile"
        $failed++
        continue
    }

    Write-Host "Created: $outputFile"
}

if ($failed -gt 0) {
    Write-Error "$failed smoke case(s) failed"
    exit 1
}
