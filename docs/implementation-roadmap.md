# Implementation roadmap

## Phase 0: Repository bootstrap

- [x] Private GitHub repo.
- [x] Rust CLI skeleton.
- [x] npm wrapper skeleton.
- [x] Benchmark and marketing folders.

## Phase 1: Compatibility research

- [ ] Pin `npm-run-all` versions used for comparison.
- [ ] Document observable CLI/API behavior.
- [ ] Build golden fixtures for edge cases.
- [ ] Decide which quirks are compatibility requirements.

## Phase 2: MVP engine

- [ ] Implement the smallest compatible path that does real work.
- [ ] Add unit tests for argument parsing and error behavior.
- [ ] Add integration tests for golden fixtures.
- [ ] Return accurate exit codes and helpful errors.

## Phase 3: Performance pass

- [ ] Add hyperfine baselines.
- [ ] Capture flamegraphs before optimizing.
- [ ] Remove unnecessary allocations and subprocesses.
- [ ] Add regression benchmarks for startup and stress scenarios.

## Phase 4: npm distribution

- [ ] Build prebuilt binaries for macOS arm64/x64 and Linux x64/arm64.
- [ ] Publish optional dependency packages.
- [ ] Keep source install fallback documented.
- [ ] Verify `npx run-all-now` works on clean machines.

## Phase 5: Launch

- [ ] Render side-by-side GIF.
- [ ] Publish benchmark table with raw scripts.
- [ ] Open public issues for compatibility gaps.
- [ ] Flip repo public only after the README is credible.

## Initial compatibility targets

| Feature | Status | Notes |
| --- | --- | --- |
| npm-run-all CLI | Planned | Support --parallel, --serial, --continue-on-error, --race, --print-label, --silent. |
| run-p / run-s aliases | Planned | Detect alias name and map defaults. |
| npm script lifecycle env | Planned | Preserve npm_lifecycle_event, INIT_CWD, PATH augmentation, and package-manager quirks. |
| Task globs | Research | Match npm-run-all pattern behavior before optimizing expansion. |
| Watch mode | Future | Native grouped logs plus restart policy after baseline parity. |
