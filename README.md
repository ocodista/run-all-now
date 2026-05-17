# run-all-now

Ultra-fast native replacement for npm-run-all.

Run npm scripts in parallel with near-zero overhead.

`run-all-now` is a Rust-native npm package for developers who want old Node.js tooling to feel instant. The repo starts private while compatibility, benchmarks, and launch assets mature.

## Benchmarks

Do not publish scaffold numbers. Publish only after the compatibility matrix marks a scenario supported.

| Scenario | Original | Native target | Goal |
| --- | --- | --- | --- |
| Startup | `npm-run-all` cold start | `run-all-now --version` | 5-20x faster startup |
| Stress | npm script fan-out across a monorepo workspace | Native Rust engine | Lower wall time and RSS |
| Memory | `/usr/bin/time` RSS | `/usr/bin/time` RSS | Lower peak memory |
| Flamegraph | Node package stack | Rust hot path | Clear bottleneck story |

Run locally:

```bash
npm run build:native
bash bench/startup.sh
bash bench/stress.sh
bash bench/memory.sh
bash bench/flamegraph.sh
```

## GIF demo

The launch asset lives in `marketing/demo.tape` and renders to `marketing/demo.gif` with VHS.

```bash
vhs marketing/demo.tape
```

The GIF should show the original package on the left and `run-all-now` on the right with wall time, CPU, and memory visible.

## Installation

```bash
npm install -D run-all-now
npm run build:native
```

During alpha, the npm wrapper looks for a local Rust binary in `target/release/run-all-now`. Public releases will ship prebuilt binaries through npm optional dependencies.

## Usage

```bash
npx run-all-now --help
run-all-now --parallel lint test typecheck
```

## Compatibility notes

`run-all-now` aims for drop-in behavior where compatibility does not block major performance wins. See `docs/compatibility-matrix.md` before replacing `npm-run-all` in production.

## Architecture notes

The engine avoids unnecessary shell spawning, sync IO, excessive allocations, and avoidable subprocesses. See `docs/architecture.md` for the planned modules and bottleneck strategy.

## Roadmaps

- `docs/implementation-roadmap.md`
- `docs/benchmark-roadmap.md`
- `docs/performance-hypotheses.md`
- `docs/launch-checklist.md`
