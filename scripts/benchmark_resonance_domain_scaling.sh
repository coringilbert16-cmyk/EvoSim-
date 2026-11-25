#!/usr/bin/env bash
set -euo pipefail

cargo test --release --locked --lib benchmark_fixed_domain_domain_scaling -- --ignored --nocapture
