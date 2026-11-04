$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $Root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }
    $Out = Join-Path $Root 'target\release\husk.exe'
    if (-not (Test-Path $Out)) { throw "Expected binary not found: $Out" }
    Write-Host "Built: $Out" -ForegroundColor Green
} finally { Pop-Location }
