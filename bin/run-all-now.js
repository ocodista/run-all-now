#!/usr/bin/env node
'use strict';

const { existsSync } = require('node:fs');
const { spawnSync } = require('node:child_process');
const { join } = require('node:path');

const packageRoot = join(__dirname, '..');
const binaryName = process.platform === 'win32' ? 'run-all-now.exe' : 'run-all-now';
const platformArch = `${process.platform}-${process.arch}`;

const candidates = [
  join(packageRoot, 'native', platformArch, binaryName),
  join(packageRoot, 'target', 'release', binaryName),
  join(packageRoot, 'target', 'debug', binaryName),
];

const binary = candidates.find((candidate) => existsSync(candidate));

if (!binary) {
  console.error('run-all-now: native binary not found.');
  console.error('Run `npm run build:native` from the package root during alpha.');
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  console.error(`run-all-now: failed to launch native binary: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
