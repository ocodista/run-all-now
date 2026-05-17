# Benchmark roadmap

## Principles

- Measure before optimizing.
- Publish scripts, fixtures, hardware, and commit SHAs.
- Compare against maintained and legacy versions of `npm-run-all` when behavior differs.
- Keep benchmark output reproducible in CI and locally.

## Required suites

| Suite | Metric | Tooling | Output |
| --- | --- | --- | --- |
| Startup | wall time, stddev | hyperfine | README table |
| Stress | wall time, throughput | hyperfine | benchmark artifact |
| Memory | max RSS | /usr/bin/time | markdown table |
| Flamegraph | hot path | cargo flamegraph, inferno | SVG artifact |
| Marketing | visible side-by-side timing | VHS/asciinema | GIF and cast |

## Scenarios

1. Cold startup on macOS arm64 and Linux x64.
2. Warm startup after filesystem cache is hot.
3. npm script fan-out across a monorepo workspace.
4. CI runner scenario with constrained CPU.
5. Monorepo scenario with thousands of files or scripts.

## Reporting template

```md
### Benchmark: <scenario>

- Tool: run-all-now <version> vs npm-run-all <version>
- Commit: <sha>
- OS/kernel: <value>
- CPU: <value>
- Disk: <value>
- Node/Rust: <versions>
- Fixture size: <files/scripts/processes>

| Command | Mean | Stddev | Max RSS | Notes |
| --- | ---: | ---: | ---: | --- |
| original |  |  |  |  |
| native |  |  |  |  |
```
