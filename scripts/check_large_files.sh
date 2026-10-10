#!/usr/bin/env bash
set -euo pipefail

# Report source files above the review-safe threshold for visibility.
# File size is advisory only and must never block CI or development.
MAX_BYTES=30000

while IFS= read -r -d '' file; do
    case "$file" in
        ./.git/*|./target/*) continue ;;
    esac

    size=$(wc -c < "$file")
    if (( size > MAX_BYTES )); then
        printf 'LARGE SOURCE FILE: %s (%s bytes; limit %s)\n' "$file" "$size" "$MAX_BYTES"
    fi
done < <(find . -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' \) -print0)

exit 0
