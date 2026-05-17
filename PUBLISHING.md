# Publishing

Publish all packages at the same version.

## Check

```sh
npm whoami
npm run publish:check
```

## Publish native packages first

```sh
cargo build --release --target <rust-target>
node scripts/prepare-platform-package.js packages/<platform-package> target/<rust-target>/release/run-all-now
cd packages/<platform-package>
npm publish --access public --provenance
```

Use `run-all-now.exe` for Windows targets.

| npm package | Rust target |
| --- | --- |
| `@run-all-now/darwin-arm64` | `aarch64-apple-darwin` |
| `@run-all-now/darwin-x64` | `x86_64-apple-darwin` |
| `@run-all-now/linux-arm64` | `aarch64-unknown-linux-gnu` |
| `@run-all-now/linux-x64` | `x86_64-unknown-linux-gnu` |
| `@run-all-now/win32-arm64` | `aarch64-pc-windows-msvc` |
| `@run-all-now/win32-x64` | `x86_64-pc-windows-msvc` |

## Publish main package last

```sh
npm publish --access public --provenance
```

## Smoke test

```sh
tmp=$(mktemp -d)
cd "$tmp"
npm init -y
npm i -D run-all-now
npx run-s --version
node -e 'console.log(typeof require("run-all-now"))'
```
