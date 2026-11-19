#!/usr/bin/env bash
set -euo pipefail

# Report larger source files as an advisory review signal only.
# This threshold is not an architectural limit and never blocks CI or development.
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
