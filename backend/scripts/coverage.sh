#!/usr/bin/env bash
set -euo pipefail

# The full report is kept alongside the rubric scope so exclusions stay visible.
mkdir -p coverage
cargo llvm-cov --all-features --workspace --lcov --output-path coverage/lcov.info
cargo llvm-cov report --summary-only | tee coverage/full-summary.txt

# Scope: JWT, RBAC, donations, scanner, admin bootstrap and local/hybrid matching.
# Groq, CEO metrics and demo seed belong to later activities. CLI entrypoints and
# main wiring are separate from the module under evaluation.
scope_exclusions='(src/bin/|src/main\.rs|src/ai/groq\.rs|src/api/(metrics|seed)\.rs|rustc-.*src/library/)'
cargo llvm-cov report --summary-only --ignore-filename-regex "$scope_exclusions" | tee coverage/module-summary.txt
awk '$1 == "TOTAL" { lines = $10 + 0; found = 1 } END { if (!found || lines < 80) exit 1 }' coverage/module-summary.txt
