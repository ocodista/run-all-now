#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

ensure_native
ensure_node_deps
bash "$ROOT/bench/setup-fixtures.sh"

measure_memory "original: npm-run-all" "'$FIXTURES/node_modules/.bin/npm-run-all' --version"
measure_memory "native: $PROJECT" "'$NATIVE' --version"
