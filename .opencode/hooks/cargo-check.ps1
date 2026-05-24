$data = [Console]::In.ReadToEnd() | ConvertFrom-Json
$filePath = $data.tool_input.file_path

# Only act on .rs files
if ($filePath -notlike "*.rs") { exit 0 }

# Determine which cargo workspace to check based on file location
$cwd = $data.cwd
if ($filePath -like "*\frontend\*" -or $filePath -like "*/frontend/*") {
    $cargoDir = Join-Path $cwd "frontend"
    $label = "frontend"
} else {
    $cargoDir = $cwd
    $label = "backend"
}

Write-Host "cargo check ($label)..." -ForegroundColor Cyan

Push-Location $cargoDir
$result = cargo check 2>&1
$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -eq 0) {
    Write-Host "OK" -ForegroundColor Green
    exit 0
} else {
    $result | ForEach-Object { Write-Host $_ }
    exit 2
}
