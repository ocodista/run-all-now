# Research notes

## Original package

- Package: `npm-run-all`
- Replacement: `run-all-now`
- Initial scenario: npm script fan-out across a monorepo workspace

## Questions to answer

1. Which behaviors are users relying on by accident?
2. Where does the original package spawn shells or child processes?
3. Which dependencies dominate cold startup?
4. Which flags need exact parity for drop-in replacement?
5. Which quirks should become documented incompatibilities?

## First artifacts

- Dependency tree screenshot or text output.
- Flamegraph for original startup path where possible.
- Golden fixtures for common and weird inputs.
- List of known open issues in the original package that native code can solve.
