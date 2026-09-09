param(
    [string]$Url = "https://example.com",
    [string]$Name = "smoke-note",
    [string]$OutputDir = "test-output/smoke/public"
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outputPath = Join-Path $repoRoot $OutputDir
$binary = Join-Path $repoRoot "web2md.exe"

if (-not (Test-Path $binary)) {
    Push-Location $repoRoot
    go build -o web2md.exe .
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        exit $LASTEXITCODE
    }
    Pop-Location
}

New-Item -ItemType Directory -Force $outputPath | Out-Null

Push-Location $outputPath
& $binary $Url -n $Name
$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -ne 0) {
    exit $exitCode
}

if (-not (Test-Path (Join-Path $outputPath "$Name.md"))) {
    Write-Error "Expected $Name.md to be created in $outputPath"
    exit 1
}

Write-Host "Smoke output created: $(Join-Path $outputPath "$Name.md")"
