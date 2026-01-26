# Clean up development log files and artifacts
# This script removes all .txt and .log files from the root directory
# that are generated during development, testing, and debugging

Write-Host "Cleaning up development log files..." -ForegroundColor Cyan

$rootDir = Split-Path -Parent $PSScriptRoot

# Patterns to clean
$patterns = @(
    "*_error*.txt",
    "*_errors.txt",
    "clippy_*.txt",
    "test_*.txt",
    "*_test_*.txt",
    "workspace_*.txt",
    "check_*.txt",
    "*.log"
)

$filesRemoved = 0

foreach ($pattern in $patterns) {
    $files = Get-ChildItem -Path $rootDir -Filter $pattern -File
    foreach ($file in $files) {
        Write-Host "  Removing: $($file.Name)" -ForegroundColor Yellow
        Remove-Item $file.FullName -Force
        $filesRemoved++
    }
}

if ($filesRemoved -eq 0) {
    Write-Host "No log files found to clean." -ForegroundColor Green
} else {
    Write-Host "`nCleaned up $filesRemoved log file(s)." -ForegroundColor Green
}
