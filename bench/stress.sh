#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

require_tool hyperfine
ensure_native
ensure_node_deps
bash "$ROOT/bench/setup-fixtures.sh"

echo "Scaffold warning: do not publish these numbers until the MVP supports this scenario."

hyperfine --warmup 3 --runs 10 \
  "cd '$FIXTURES' && npm exec -- npm-run-all --parallel task:a task:b task:c task:d" \
  "'$NATIVE' --parallel task:a task:b task:c task:d"
