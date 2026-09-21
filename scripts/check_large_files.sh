#!/usr/bin/env bash
set -euo pipefail

# GitHub's file/code views can truncate large source files. Keep source files
# below the review-safe threshold so future changes remain inspectable as a
# complete unit. This is an architecture guard, not a runtime limit.
MAX_BYTES=30000

status=0
warned=0
while IFS= read -r -d '' file; do
    case "$file" in
        ./.git/*|./target/*) continue ;;
    esac

    size=$(wc -c < "$file")
    if (( size > MAX_BYTES )); then
        printf 'LARGE SOURCE FILE (review warning): %s (%s bytes; suggested limit %s)\n' "$file" "$size" "$MAX_BYTES"
        warned=1
    fi
done < <(find . -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.jsx' \) -print0)

if (( warned )); then
    printf 'Source file size check completed with warnings only.\n'
fi
exit 0
