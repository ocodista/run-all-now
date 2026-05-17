#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

if ! command -v cargo-flamegraph >/dev/null 2>&1 && ! cargo flamegraph --help >/dev/null 2>&1; then
  echo "Install flamegraph support: cargo install flamegraph" >&2
  exit 1
fi

cargo flamegraph --manifest-path "$ROOT/Cargo.toml" --bin "run-all-now" -- --internal-smoke-test
