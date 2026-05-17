# Architecture

`run-all-now` optimizes for startup latency, throughput, and predictable CLI behavior.

## Modules

| Module | Responsibility |
| --- | --- |
| Script resolver | Read package.json once, expand npm-run-all patterns, and preserve package-manager semantics. |
| Scheduler | Run serial and parallel groups without shell fan-out unless the user explicitly asks for shell behavior. |
| Process supervisor | Spawn child processes directly, stream logs, propagate signals, and preserve exit-code rules. |
| Output renderer | Group logs by task with low-lock writers, colors, prefixes, and CI-safe fallback output. |
| Compatibility shim | Expose npm-run-all, run-p, and run-s bin aliases from one native binary. |

## Constraints

- Avoid unnecessary shell spawning.
- Avoid sync IO on hot paths.
- Avoid retaining full result sets when streaming works.
- Prefer bounded parallelism over unbounded fan-out.
- Preserve compatibility before publishing benchmark claims.

## Rust crates to evaluate

- `clap` for CLI parsing.
- `rayon` for CPU-bound fan-out.
- `tokio` for async process and IO flows.
- `ignore`, `walkdir`, and `globset` for filesystem-heavy tools.
- `crossterm` for portable terminal UX.
