# run-all-now

Experimental Rust + npm scaffold for a future npm-run-all replacement.

Current alpha supports `--help`, `--version`, and scaffold output only. It does not run npm scripts yet.

The goal is a native command that runs npm scripts in parallel with low startup overhead after compatibility is implemented and verified.

## Status

Use this package for development and benchmark exploration only. Do not replace `npm-run-all` in CI or production projects yet.

Implemented today:

- Rust CLI skeleton with `--help` and `--version`.
- Default scaffold output from the native binary.
- Argument capture with a warning when compatibility execution is requested.
- npm wrapper that launches a local native binary from the package directory.

Not implemented yet:

- `package.json` script resolution.
- Parallel or serial task execution.
- `npm-run-all`, `run-p`, and `run-s` behavior parity.
- npm lifecycle environment handling.
- Exit-code, signal, log-prefix, and glob compatibility.

See [`docs/compatibility-matrix.md`](docs/compatibility-matrix.md) for the current compatibility target.

## Install from source/current alpha

Requirements:

- Rust stable.
- Node.js 18+ and npm.

Build from a source checkout:

```bash
git clone https://github.com/ocodista/run-all-now.git
cd run-all-now
npm install
npm run build:native
node bin/run-all-now.js --version
```

The npm wrapper searches for a binary in `native/<platform>-<arch>/`, then `target/release/`, then `target/debug/`. Current alpha releases do not ship prebuilt binaries.

## Current usage

These commands work today:

```bash
cargo run -- --help
cargo run -- --version
node bin/run-all-now.js --help
node bin/run-all-now.js --version
```

Passing npm-run-all-style arguments only prints scaffold output and a warning:

```bash
node bin/run-all-now.js --parallel lint test
```

Expected current behavior: the command exits without running `lint` or `test`.

## Planned compatibility

The target is npm-run-all-compatible behavior for common script workflows, once fixtures and tests verify each case.

Planned areas:

- `npm-run-all --parallel`, `--serial`, `--continue-on-error`, `--race`, `--print-label`, and `--silent`.
- `run-p` and `run-s` aliases.
- npm lifecycle environment variables and `PATH` behavior.
- Task glob expansion that matches npm-run-all.
- Exit-code and signal propagation.

Do not use it as a drop-in replacement until the compatibility matrix marks the relevant rows as verified.

## Benchmarks

Current benchmark coverage is startup-only. It compares version output for `npm-run-all` and `run-all-now`. These numbers do not measure script execution, scheduling throughput, memory under task fan-out, or compatibility.

Caveat: the native command does less work today. Treat these numbers as startup overhead data, not proof of replacement performance.

Reproduce the maintained startup benchmark:

```bash
npm run build:native
bash bench/startup.sh
```

Local startup measurement from 2026-05-17:

| Command | Mean | Min | Max | Runs |
| --- | ---: | ---: | ---: | ---: |
| `bench/fixtures/node_modules/.bin/npm-run-all --version` | 26.5 ms ± 1.2 ms | 24.4 ms | 28.7 ms | 30 |
| `target/release/run-all-now --version` | 1.4 ms ± 0.3 ms | 1.1 ms | 2.1 ms | 30 |

Environment:

- Fixture: [`bench/fixtures/package.json`](bench/fixtures/package.json) with `npm-run-all` v4.1.5 installed.
- Tooling: hyperfine 1.20.0, Node v24.15.0, npm 11.12.1, Rust 1.91.1.
- Machine: macOS/Darwin 25.2.0 arm64, Apple M2 Pro, 16 GB RAM.

Hyperfine warned that the native command took less than 5 ms, so shell timing precision can affect the exact native result.

See [`docs/benchmark-roadmap.md`](docs/benchmark-roadmap.md) and [`bench/README.md`](bench/README.md) for benchmark methodology.

## Architecture

Current code:

- [`src/main.rs`](src/main.rs) contains the Clap-based CLI scaffold.
- [`bin/run-all-now.js`](bin/run-all-now.js) locates and launches the native binary.
- [`bench/`](bench/) contains startup, stress, memory, and flamegraph scripts.

Planned native modules:

- Script resolver for `package.json` scripts and npm-run-all patterns.
- Scheduler for serial and parallel groups.
- Process supervisor for child processes, signals, and exit rules.
- Output renderer for grouped logs and CI-safe output.
- Compatibility shim for `npm-run-all`, `run-p`, and `run-s` aliases.

See [`docs/architecture.md`](docs/architecture.md) and [`docs/performance-hypotheses.md`](docs/performance-hypotheses.md) for the deeper design notes.

## Development

Useful commands:

```bash
npm run check:native
npm run build:native
cargo fmt --check
cargo clippy -- -D warnings
cargo test
npm run bench
npm run pack:dry
```

Before adding performance claims, add fixtures, raw commands, environment details, and a compatibility note. See [`docs/research-notes.md`](docs/research-notes.md).

## Roadmap

- Add golden compatibility fixtures for common npm-run-all workflows.
- Implement script resolution and npm lifecycle environment behavior.
- Implement serial and parallel execution.
- Match exit-code, signal, logging, and glob behavior.
- Add regression benchmarks after behavior parity exists.
- Ship prebuilt npm binaries for supported platforms.

Detailed plans live in [`docs/implementation-roadmap.md`](docs/implementation-roadmap.md), [`docs/benchmark-roadmap.md`](docs/benchmark-roadmap.md), and [`docs/launch-checklist.md`](docs/launch-checklist.md).

## License

MIT. See [`LICENSE`](LICENSE).
