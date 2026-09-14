#!/usr/bin/env bash
set -euo pipefail

fail=0

if [[ -e src/structural_combine.rs ]]; then
  echo "ERROR: obsolete structural_combine.rs still exists"
  fail=1
fi

# Production code may only insert bonds through the COMBINE runtime authority.
# contact::try_add_bond is the low-level admission primitive and must not be
# called directly by construction, reproduction, or other simulation modules.
while IFS= read -r file; do
  case "$file" in
    src/contact.rs|src/structure.rs|src/combine_runtime.rs) continue ;;
  esac
  if awk '/#\[cfg\(test\)\]/{exit} /(\.add_bond|::add_bond|try_add_bond)[[:space:]]*\(/ {print; found=1} END{exit found ? 0 : 1}' "$file"; then
    echo "ERROR: production raw bond-admission path found in $file"
    fail=1
  fi
  if awk '/#\[cfg\(test\)\]/{exit} /\bBond[[:space:]]*\{/ {print; found=1} END{exit found ? 0 : 1}' "$file"; then
    echo "ERROR: production Bond construction found outside COMBINE runtime in $file"
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
if grep -nE 'potential_energy.*-.*potential_energy|exponential_influence\(|formation_threshold\(' src/combine_runtime.rs; then
  echo "ERROR: COMBINE physics equation appears in runtime"
  fail=1
fi

if ! grep -q 'pub fn try_add_bond' src/contact.rs; then
  echo "ERROR: contact::try_add_bond() authority is missing"
  fail=1
fi

if ! grep -q 'pub(crate) fn combine_specific_pair' src/combine_runtime.rs; then
  echo "ERROR: COMBINE runtime construction boundary is missing"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then exit 1; fi
echo "COMBINE architecture guards passed."
