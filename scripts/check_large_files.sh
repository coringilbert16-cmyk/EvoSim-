#!/usr/bin/env bash
set -euo pipefail

# Source-file size is not an architectural constraint.
# Keep this CI hook as an explicit policy check so future changes do not
# reintroduce arbitrary byte/line thresholds as module-boundary requirements.
echo "Source module boundaries are responsibility-based; no file-size limit is enforced."
