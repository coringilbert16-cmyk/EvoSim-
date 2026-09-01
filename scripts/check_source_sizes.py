from pathlib import Path

# Keep source files comfortably below common review/LLM truncation limits.
# Warning threshold is intentionally lower than the hard failure threshold.
WARNING_LINES = 900
FAIL_LINES = 1200

SOURCE_SUFFIXES = {".rs", ".ts", ".tsx", ".js", ".jsx"}
SKIP_DIRS = {"target", "node_modules", ".git"}

violations = []
warnings = []

for path in Path(".").rglob("*"):
    if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
        continue
    if any(part in SKIP_DIRS for part in path.parts):
        continue

    lines = sum(1 for _ in path.open("r", encoding="utf-8"))
    if lines >= FAIL_LINES:
        violations.append((path, lines))
    elif lines >= WARNING_LINES:
        warnings.append((path, lines))

for path, lines in sorted(warnings, key=lambda item: item[1], reverse=True):
    print(f"WARNING: {path}: {lines} lines (decomposition recommended)")

for path, lines in sorted(violations, key=lambda item: item[1], reverse=True):
    print(f"ERROR: {path}: {lines} lines (must be decomposed)")

if violations:
    raise SystemExit(1)

print("Source-size guard passed.")
