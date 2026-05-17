#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="run-all-now"
NATIVE="$ROOT/target/release/$PROJECT"
FIXTURES="$ROOT/bench/fixtures"
ORIGINAL_PACKAGE="npm-run-all"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required tool: $1" >&2
    echo "Install it before running this benchmark." >&2
    exit 1
  fi
}

ensure_native() {
  cargo build --release --manifest-path "$ROOT/Cargo.toml"
}

ensure_node_deps() {
  (cd "$FIXTURES" && npm install --silent)
}

measure_memory() {
  local label="$1"
  local command="$2"

  echo "## $label"
  if [[ "${OSTYPE:-}" == darwin* ]]; then
    /usr/bin/time -l bash -lc "$command"
  else
    /usr/bin/time -v bash -lc "$command"
  fi
}
