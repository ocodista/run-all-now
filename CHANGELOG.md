# Changelog

## 1.0.1 - 2026-05-17

- Simplify repository docs and generated benchmark artifacts.
- Refresh README for npm users.
- Keep benchmark output on stdout instead of writing generated files.

## 1.0.0 - 2026-05-17

Initial stable release.

- Add compatible `npm-run-all`, `run-s`, and `run-p` commands.
- Add CommonJS Node API compatible with `npm-run-all` result objects and errors.
- Add zero-third-party-dependency npm package with Rust-powered orchestration.
- Add optional native packages for macOS, Linux, and Windows on x64/arm64.
- Add task glob matching, argument placeholders, parallel/sequential execution, race mode, labels, aggregate output, npm/yarn path support, and continue-on-error behavior.
- Add tests, CI, benchmark tooling, and npm publishing workflow.
