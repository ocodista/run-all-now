#!/usr/bin/env node
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const [, , packageDir, binaryPath] = process.argv;
if (!packageDir || !binaryPath) {
  console.error("Usage: prepare-platform-package <package-dir> <binary-path>");
  process.exit(1);
}

const packageJsonPath = path.join(packageDir, "package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
const isWindows = packageJson.os && packageJson.os.includes("win32");
const destinationDir = path.join(packageDir, "bin");
const destination = path.join(destinationDir, isWindows ? "run-all-now.exe" : "run-all-now");

fs.mkdirSync(destinationDir, { recursive: true });
fs.copyFileSync(binaryPath, destination);
fs.chmodSync(destination, 0o755);
console.log(`Copied ${binaryPath} -> ${destination}`);
