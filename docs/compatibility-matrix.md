# Compatibility matrix

`run-all-now` will not claim drop-in status until these rows pass golden tests.

| Feature | Status | Notes |
| --- | --- | --- |
| npm-run-all CLI | Planned | Support --parallel, --serial, --continue-on-error, --race, --print-label, --silent. |
| run-p / run-s aliases | Planned | Detect alias name and map defaults. |
| npm script lifecycle env | Planned | Preserve npm_lifecycle_event, INIT_CWD, PATH augmentation, and package-manager quirks. |
| Task globs | Research | Match npm-run-all pattern behavior before optimizing expansion. |
| Watch mode | Future | Native grouped logs plus restart policy after baseline parity. |

## Status values

- `Research`: behavior is still being pinned.
- `Planned`: behavior is accepted for MVP.
- `Implemented`: code exists and has tests.
- `Verified`: behavior matches fixtures and benchmarks can be published.
- `Won't match`: documented incompatibility with migration guidance.
