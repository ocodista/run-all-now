# Performance hypotheses

Every benchmark claim must map to a hypothesis below.

| Area | Hypothesis | Validation |
| --- | --- | --- |
| Startup | Native clap parser avoids Node module graph startup. | hyperfine native --version vs npm-run-all --version |
| Fan-out | Direct process supervision avoids intermediate shells and Promise churn. | 100 no-op npm scripts in parallel |
| Memory | One native scheduler uses less RSS than Node plus package graph. | /usr/bin/time memory comparison |
| Logs | Buffered grouped output reduces write contention in parallel runs. | Parallel noisy scripts benchmark |

## Anti-goals

- Do not claim speedups from incomplete work.
- Do not hide behavior gaps behind benchmark wins.
- Do not optimize for one fixture if it regresses real projects.
