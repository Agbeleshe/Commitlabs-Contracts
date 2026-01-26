#!/bin/bash
# Clean up development log files and artifacts
# This script removes all .txt and .log files from the root directory
# that are generated during development, testing, and debugging

echo "Cleaning up development log files..."

# Get the root directory (parent of scripts/)
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Patterns to clean
patterns=(
    "*_error*.txt"
    "*_errors.txt"
    "clippy_*.txt"
    "test_*.txt"
    "*_test_*.txt"
    "workspace_*.txt"
    "check_*.txt"
    "*.log"
)

files_removed=0

for pattern in "${patterns[@]}"; do
    while IFS= read -r -d '' file; do
        echo "  Removing: $(basename "$file")"
        rm -f "$file"
        ((files_removed++))
    done < <(find "$ROOT_DIR" -maxdepth 1 -type f -name "$pattern" -print0 2>/dev/null)
done

if [ $files_removed -eq 0 ]; then
    echo "No log files found to clean."
else
    echo ""
    echo "Cleaned up $files_removed log file(s)."
fi
