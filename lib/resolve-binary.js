"use strict";

const fs = require("node:fs");
const path = require("node:path");

function binaryName() {
  return process.platform === "win32" ? "run-all-now.exe" : "run-all-now";
}

function platformPackageName() {
  const platform = process.platform;
  const arch = process.arch;
  const supportedPlatforms = new Set(["darwin", "linux", "win32"]);
  const supportedArchitectures = new Set(["x64", "arm64"]);

  if (!supportedPlatforms.has(platform) || !supportedArchitectures.has(arch)) {
    throw new Error(`Unsupported platform for run-all-now: ${platform}-${arch}`);
  }

  return `@run-all-now/${platform}-${arch}`;
}

function existingFile(candidate) {
  return candidate && fs.existsSync(candidate) ? candidate : null;
}

function resolveFromPlatformPackage() {
  const packageName = platformPackageName();
  try {
    const packageJson = require.resolve(`${packageName}/package.json`);
    return existingFile(path.join(path.dirname(packageJson), "bin", binaryName()));
  } catch {
    return null;
  }
}

function resolveBinary() {
  const candidates = [
    process.env.RUN_ALL_NOW_BINARY,
    resolveFromPlatformPackage(),
    path.join(__dirname, "..", "bin", binaryName()),
    path.join(__dirname, "..", "target", "release", binaryName()),
    path.join(__dirname, "..", "target", "debug", binaryName())
  ];

  for (const candidate of candidates) {
    const resolved = existingFile(candidate);
    if (resolved) {
      return resolved;
    }
  }

  throw new Error(
    `Could not find the run-all-now native binary for ${process.platform}-${process.arch}. ` +
      "Install the matching @run-all-now/* package or build with `cargo build --release`."
  );
}

module.exports = { resolveBinary };
