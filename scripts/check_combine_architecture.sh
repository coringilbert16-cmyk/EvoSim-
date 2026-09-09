#!/usr/bin/env bash
set -euo pipefail

fail=0

if [[ -e src/structural_combine.rs ]]; then
  echo "ERROR: obsolete structural_combine.rs still exists"
  fail=1
fi

# Production code may only insert bonds through contact::try_add_bond().
while IFS= read -r file; do
  [[ "$file" == "src/contact.rs" || "$file" == "src/structure.rs" ]] && continue
  if awk '/#\[cfg\(test\)\]/{exit} /add_bond\(/ {print; found=1} END{exit found ? 0 : 1}' "$file"; then
    echo "ERROR: production add_bond() path found in $file"
    fail=1
  fi
done < <(find src -name '*.rs' -type f | sort)

if grep -R -nE 'connection_count\([^)]*\)[[:space:]]*==[[:space:]]*0|connection_count\([^)]*\)[[:space:]]*<[[:space:]]*1' src --include='*.rs'; then
  echo "ERROR: connection_count() is being used as a capacity gate"
  fail=1
fi

if grep -R -nE 'MAX_CONNECTION|MAX_CONNECTIONS|CONNECTION_CAPACITY|connection_capacity' src --include='*.rs'; then
  echo "ERROR: fixed connection-capacity semantics found"
  fail=1
fi

if grep -R -n 'structural_combine' src --include='*.rs'; then
  echo "ERROR: obsolete structural_combine authority referenced"
  fail=1
fi

# Runtime must not contain an independent COMBINE physics equation.
if grep -R -nE 'potential_energy.*-.*potential_energy|exponential_influence\(|formation_threshold\(' src/combine_runtime.rs; then
  echo "ERROR: COMBINE physics equation appears in runtime"
  fail=1
fi

# The structural admission authority must remain the only production insertion call.
if ! grep -q 'pub fn try_add_bond' src/contact.rs; then
  echo "ERROR: contact::try_add_bond() authority is missing"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "COMBINE architecture guards passed."
