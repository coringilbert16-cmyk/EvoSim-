#!/usr/bin/env bash
set -euo pipefail

cargo test --release --locked --lib dense_uniform_kernel_matches_production_adaptive_uniform_operator -- --nocapture
cargo test --release --locked --lib benchmark_fixed_domain_domain_scaling -- --ignored --nocapture
