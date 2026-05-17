#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

require_tool hyperfine
ensure_native
ensure_node_deps

hyperfine --warmup 5 --runs 30 \
  "cd '$FIXTURES' && npm exec -- npm-run-all --version" \
  "'$NATIVE' --version"
