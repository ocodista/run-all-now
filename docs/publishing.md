# Publishing checklist

Do not publish from an unverified working tree. Do not publish the main package before all platform packages are available at the same version.

## 1. Confirm package metadata

```bash
npm whoami
npm view run-all-now version || true
npm view @run-all-now/darwin-arm64 version || true
node -p 'require("./package.json").version'
```

All packages must use the same version.

## 2. Verify locally

```bash
npm run publish:check
```

This runs formatting, clippy, Rust tests, Node integration tests, and an npm dry-run pack check.

## 3. Build and publish platform packages first

Build each Rust target, copy the native binary into its platform package, then publish from that platform package directory.

```bash
cargo build --release --target <rust-target>
node scripts/prepare-platform-package.js packages/<platform-package> target/<rust-target>/release/run-all-now
cd packages/<platform-package>
npm publish --access public --provenance
```

Windows uses `run-all-now.exe` as the source binary:

```bash
node scripts/prepare-platform-package.js packages/win32-x64 target/x86_64-pc-windows-msvc/release/run-all-now.exe
```

Platform packages:

| npm package | Rust target | Source binary |
| --- | --- | --- |
| `@run-all-now/darwin-arm64` | `aarch64-apple-darwin` | `run-all-now` |
| `@run-all-now/darwin-x64` | `x86_64-apple-darwin` | `run-all-now` |
| `@run-all-now/linux-arm64` | `aarch64-unknown-linux-gnu` | `run-all-now` |
| `@run-all-now/linux-x64` | `x86_64-unknown-linux-gnu` | `run-all-now` |
| `@run-all-now/win32-arm64` | `aarch64-pc-windows-msvc` | `run-all-now.exe` |
| `@run-all-now/win32-x64` | `x86_64-pc-windows-msvc` | `run-all-now.exe` |

## 4. Publish the main package last

```bash
npm publish --access public --provenance
```

The main package exposes JavaScript npm bins and the CommonJS API. Those entrypoints resolve the installed platform package and execute the Rust binary.

## 5. Smoke test from npm

```bash
tmp=$(mktemp -d)
cd "$tmp"
npm init -y
npm install --save-dev run-all-now
npx run-s --version
node -e 'const runAll = require("run-all-now"); console.log(typeof runAll)'
```

Expected output includes the published version and `function`.
