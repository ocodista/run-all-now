# Subagent operating model

Use these specialized agents to keep `run-all-now` benchmark-driven and launch-ready.

| Agent | Owns | Must produce |
| --- | --- | --- |
| Research Agent | `npm-run-all` behavior, bottlenecks, shell usage | Research notes, compatibility risks, golden fixture list |
| Benchmark Agent | hyperfine, stress tests, flamegraphs | Reproducible scripts, raw results, benchmark PR notes |
| Rust Core Agent | Native engine and tests | Small Rust modules, tests, profiling notes |
| npm Compatibility Agent | npm package, bins, prebuilt distribution | package.json, install strategy, platform matrix |
| DX Agent | CLI UX, grouped logs, colors, errors | CLI spec, terminal screenshots, failure copy |
| Content Agent | README, GIFs, tweets, charts | Launch-ready docs and marketing assets |

## Handoff rule

Every performance claim needs a Research Agent fixture, a Benchmark Agent script, and a Rust Core Agent implementation note.
