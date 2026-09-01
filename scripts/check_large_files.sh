#!/usr/bin/env bash
set -euo pipefail

# GitHub's file/code views can truncate large source files. Keep source files
# below the review-safe threshold so future changes remain inspectable as a
# complete unit. This is an architecture guard, not a runtime limit.
MAX_BYTES=30000

status=0
while IFS= read -r -d '' file; do
    case "$file" in
        ./.git/*|./target/*) continue ;;
    esac

    size=$(wc -c < "$file")
    if (( size > MAX_BYTES )); then
        printf 'LARGE SOURCE FILE: %s (%s bytes; limit %s)\n' "$file" "$size" "$MAX_BYTES"
        status=1
    fi
done < <(find . -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' \) -print0)

exit "$status"
