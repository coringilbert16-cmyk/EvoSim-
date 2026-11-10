#!/usr/bin/env bash
set -euo pipefail

: "${BASE_SHA:?BASE_SHA must identify the PR base commit}"
: "${HEAD_SHA:?HEAD_SHA must identify the PR head commit}"

ROOT="$(git rev-parse --show-toplevel)"
RUNNER_TEMP="${RUNNER_TEMP:-/tmp}"
TARGET_DIR="$RUNNER_TEMP/pr335-shared-cargo-target"
BASE_DIR="$RUNNER_TEMP/pr335-baseline-worktree"
HEAD_DIR="$RUNNER_TEMP/pr335-candidate-worktree"
SUMMARY_TSV="$ROOT/pr335-performance-results.tsv"
SUMMARY_TXT="$ROOT/pr335-performance-results.txt"

cleanup() {
  git -C "$ROOT" worktree remove --force "$BASE_DIR" >/dev/null 2>&1 || true
  git -C "$ROOT" worktree remove --force "$HEAD_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# These files are the live production reaction/settlement path. A change here
# would mean this is no longer an isolated extraction and requires a new audit.
PRODUCTION_FILES=(
  src/chemistry/chemical_reaction.rs
  src/chemistry/environmental_combine.rs
  src/chemistry/environmental_displacement.rs
  src/simulation/simulation.rs
  src/simulation/state.rs
)
if ! git -C "$ROOT" diff --quiet "$BASE_SHA" "$HEAD_SHA" -- "${PRODUCTION_FILES[@]}"; then
  echo "PR #335 changed a production reaction or simulation file; stop and audit the new call path."
  exit 1
fi
if ! git -C "$ROOT" diff --quiet "$BASE_SHA" "$HEAD_SHA" -- Cargo.toml Cargo.lock; then
  echo "PR #335 changed dependencies; stop and audit the added runtime/build weight."
  exit 1
fi

# The newly registered modules may depend on one another, but no existing
# production source may call into them. This is the causal-isolation check.
if rg -n 'environmental_(broad_phase|interaction|snapshot_cursor)::' "$ROOT/src" \
  --glob '!**/chemistry/environmental_broad_phase.rs' \
  --glob '!**/chemistry/environmental_interaction.rs' \
  --glob '!**/chemistry/environmental_snapshot_cursor.rs' \
  --glob '!**/main.rs'; then
  echo "A source file outside the extracted modules references the new discovery modules."
  exit 1
else
  rg_status=$?
  if [[ "$rg_status" -ne 1 ]]; then
    echo "Could not complete the production call-site scan (rg exit $rg_status)."
    exit "$rg_status"
  fi
fi

for module in environmental_broad_phase environmental_interaction environmental_snapshot_cursor; do
  grep -q "mod $module;" "$ROOT/src/main.rs" || {
    echo "Module $module is not registered in src/main.rs; its tests may not execute."
    exit 1
  }
done

if [[ "${1:-}" == "--isolation-only" ]]; then
  echo "PASS: production reaction files and dependency manifests are unchanged."
  echo "PASS: no production source calls the extracted discovery modules."
  echo "PASS: all three extracted modules are registered for compilation and tests."
  exit 0
fi

mkdir -p "$TARGET_DIR"
git -C "$ROOT" worktree add --detach "$BASE_DIR" "$BASE_SHA"
git -C "$ROOT" worktree add --detach "$HEAD_DIR" "$HEAD_SHA"

echo "Building original-main baseline at $BASE_SHA"
(
  cd "$BASE_DIR"
  CARGO_TARGET_DIR="$TARGET_DIR" cargo build --locked --bin evosim
)
cp "$TARGET_DIR/debug/evosim" "$RUNNER_TEMP/evosim-main-baseline"

echo "Building PR #335 candidate at $HEAD_SHA"
(
  cd "$HEAD_DIR"
  CARGO_TARGET_DIR="$TARGET_DIR" cargo build --locked --bin evosim
)
cp "$TARGET_DIR/debug/evosim" "$RUNNER_TEMP/evosim-pr335-candidate"

{
  echo "base_sha=$BASE_SHA"
  echo "head_sha=$HEAD_SHA"
  echo "base_binary_bytes=$(stat -c '%s' "$RUNNER_TEMP/evosim-main-baseline")"
  echo "candidate_binary_bytes=$(stat -c '%s' "$RUNNER_TEMP/evosim-pr335-candidate")"
  echo "pairs=7 duration_seconds=60 seed=42 initial_resources=1200 initial_molecules=80"
  echo "pair variant start_record end_record"
} > "$SUMMARY_TSV"

run_case() {
  local pair="$1"
  local variant="$2"
  local binary="$3"
  local label="pair-${pair}-${variant}"
  local log="$RUNNER_TEMP/$label.log"
  local diagnostics="$RUNNER_TEMP/$label.jsonl"

  "$binary" --headless --environment-chemistry-experiment \
    --seconds 60 --seed 42 \
    --diagnostics "$diagnostics" --diagnostic-interval 100 > "$log"

  local start_line
  local end_line
  start_line="$(grep 'ENVIRONMENTAL CHEMISTRY START:' "$log" | tail -n 1)"
  end_line="$(grep 'ENVIRONMENTAL CHEMISTRY END:' "$log" | tail -n 1)"
  if [[ -z "$start_line" || -z "$end_line" ]]; then
    echo "Missing start/end record for $label"
    cat "$log"
    exit 1
  fi
  [[ "$start_line" == *"seed=42"* ]]
  [[ "$start_line" == *"independent_resources=1200 molecules=80 physical_objects=1280"* ]]
  [[ "$start_line" == *"duration_seconds=Some(60)"* ]]

  printf '%s\t%s\t%s\t%s\n' "$pair" "$variant" "$start_line" "$end_line" >> "$SUMMARY_TSV"
  rm -f "$log" "$diagnostics"
  echo "Completed $label: $end_line"
}

# Adjacent paired runs with alternating order reduce sensitivity to runner drift.
for pair in 1 2 3 4 5 6 7; do
  if (( pair % 2 == 1 )); then
    run_case "$pair" main "$RUNNER_TEMP/evosim-main-baseline"
    run_case "$pair" pr335 "$RUNNER_TEMP/evosim-pr335-candidate"
  else
    run_case "$pair" pr335 "$RUNNER_TEMP/evosim-pr335-candidate"
    run_case "$pair" main "$RUNNER_TEMP/evosim-main-baseline"
  fi
done

python3 - "$SUMMARY_TSV" "$SUMMARY_TXT" <<'PY'
import csv
import math
import random
import re
import statistics
import sys

tsv_path, txt_path = sys.argv[1:]
end_pattern = re.compile(
    r"tick=(?P<ticks>\d+) elapsed_wall_seconds=(?P<elapsed>[0-9.]+) "
    r"physical_resources=(?P<resources>\d+) constituents=(?P<constituents>\d+).* "
    r"energy_residual=(?P<residual>[^ ]+)"
)
runs = {}
with open(tsv_path, encoding="utf-8") as source:
    for line in source:
        parts = line.rstrip("\n").split("\t", 3)
        if len(parts) != 4:
            continue
        pair, variant, start, end = parts
        match = end_pattern.search(end)
        if not match:
            raise SystemExit(f"Could not parse end record: {end}")
        values = match.groupdict()
        ticks = int(values["ticks"])
        elapsed = float(values["elapsed"])
        if not 59.0 <= elapsed <= 62.0:
            raise SystemExit(f"Run duration outside 59–62 seconds: {elapsed}")
        if ticks <= 0:
            raise SystemExit(f"Run completed no ticks: {end}")
        runs[(int(pair), variant)] = {
            "ticks": ticks,
            "elapsed": elapsed,
            "rate": ticks / elapsed,
            "resources": int(values["resources"]),
            "constituents": int(values["constituents"]),
            "residual": values["residual"],
        }

ratios = []
lines = []
lines.append("PR #335 paired 60-second performance comparison")
lines.append("Baseline and candidate built with the same stable toolchain and shared Cargo target.")
lines.append("Workload: headless environmental chemistry, 60 seconds, seed 42, 1,200 independent resources + 80 molecules.")
lines.append("")
lines.append("Pair | main ticks/s | PR #335 ticks/s | paired change")
lines.append("---:|---:|---:|---:")
for pair in range(1, 8):
    base = runs[(pair, "main")]
    candidate = runs[(pair, "pr335")]
    ratio = candidate["rate"] / base["rate"]
    ratios.append(math.log(ratio))
    lines.append(
        f"{pair} | {base['rate']:.3f} | {candidate['rate']:.3f} | {(ratio - 1) * 100:+.2f}%"
    )
    lines.append(
        f"  final inventory: main {base['resources']} resources/{base['constituents']} constituents; "
        f"PR #335 {candidate['resources']} resources/{candidate['constituents']} constituents"
    )
    lines.append(
        f"  energy residual: main {base['residual']}; PR #335 {candidate['residual']}"
    )

median_log = statistics.median(ratios)
median_change = (math.exp(median_log) - 1) * 100
faster = sum(value > 0 for value in ratios)
slower = sum(value < 0 for value in ratios)

rng = random.Random(335)
bootstrap_medians = []
for _ in range(20000):
    sample = [rng.choice(ratios) for _ in ratios]
    bootstrap_medians.append(statistics.median(sample))
bootstrap_medians.sort()
low = bootstrap_medians[int(0.025 * len(bootstrap_medians))]
high = bootstrap_medians[min(len(bootstrap_medians) - 1, int(0.975 * len(bootstrap_medians)))]
low_pct = (math.exp(low) - 1) * 100
high_pct = (math.exp(high) - 1) * 100

lines.extend([
    "",
    f"Median paired throughput change: {median_change:+.2f}%",
    f"95% paired bootstrap interval: {low_pct:+.2f}% to {high_pct:+.2f}%",
    f"Pairs faster/slower: {faster}/{slower}",
])
if low_pct > 0:
    verdict = "REPRODUCIBLE IMPROVEMENT in this runner/workload"
elif high_pct < 0:
    verdict = "REPRODUCIBLE REGRESSION in this runner/workload"
else:
    verdict = "INCONCLUSIVE: the paired results do not establish a performance change"
lines.append(f"Verdict: {verdict}")
lines.extend([
    "",
    "Causal finding: the isolation checks verify that production reaction/simulation files are unchanged,",
    "dependency manifests are unchanged, and no production source calls the extracted modules.",
    "Therefore, any measured throughput difference cannot be attributed to execution of the new discovery code.",
    "A later production integration requires a separate before/after benchmark and reaction-parity validation.",
])
report = "\n".join(lines) + "\n"
print(report)
with open(txt_path, "w", encoding="utf-8") as target:
    target.write(report)
with open(tsv_path, "a", encoding="utf-8") as target:
    target.write(f"verdict\t{verdict}\n")
PY
