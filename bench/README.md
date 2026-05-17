# Benchmarks

These scripts compare `npm-run-all` with `run-all-now` across startup, stress, memory, and flamegraph workflows.

## Requirements

- Rust stable
- Node.js 18+
- npm
- hyperfine
- cargo-flamegraph for flamegraphs

## Commands

```bash
bash bench/startup.sh
bash bench/stress.sh
bash bench/memory.sh
bash bench/flamegraph.sh
```

Never publish scaffold numbers. Add the commit SHA, OS, CPU, disk type, Node version, Rust version, and fixture size to every public benchmark.
