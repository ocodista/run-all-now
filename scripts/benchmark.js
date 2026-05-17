#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const iterations = Number(process.env.BENCH_ITERATIONS || 12);
const taskCount = Number(process.env.BENCH_TASKS || 1);
const directory = fs.mkdtempSync(path.join(os.tmpdir(), "run-all-now-bench-"));
const nativeBinary = path.join(
  root,
  "target",
  "release",
  process.platform === "win32" ? "run-all-now.exe" : "run-all-now"
);

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: directory,
    encoding: "utf8",
    stdio: "pipe",
    env: {
      ...process.env,
      RUN_ALL_NOW_BINARY: nativeBinary,
      RUN_ALL_NOW_NODE_PATH: process.execPath,
      ...options.env
    }
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed\n${result.stderr}`);
  }
  return result;
}

function measure(label, command, args, options = {}) {
  const timings = [];
  for (let index = 0; index < iterations; index += 1) {
    const start = process.hrtime.bigint();
    run(command, args, options);
    const elapsedMs = Number(process.hrtime.bigint() - start) / 1_000_000;
    timings.push(elapsedMs);
  }
  const average = timings.reduce((total, value) => total + value, 0) / timings.length;
  const best = Math.min(...timings);
  const worst = Math.max(...timings);
  return { label, command: [command, ...args].join(" "), average, best, worst, timings };
}

function comparison(baseline, candidate) {
  const savedMs = baseline.average - candidate.average;
  return {
    savedMs,
    fasterPercent: (savedMs / baseline.average) * 100,
    speedup: baseline.average / candidate.average
  };
}

fs.writeFileSync(
  path.join(directory, "package.json"),
  JSON.stringify(
    {
      name: "bench",
      version: "1.0.0",
      scripts: Object.fromEntries(
        Array.from({ length: taskCount }, (_, index) => [`noop:${index}`, "node -e \"\""])
      )
    },
    null,
    2
  )
);

spawnSync("npm", ["install", "npm-run-all@4.1.5", "--silent"], { cwd: directory, stdio: "inherit" });
spawnSync("cargo", ["build", "--release"], { cwd: root, stdio: "inherit" });

const taskPattern = taskCount === 1 ? "noop:0" : "noop:*";
const npmRunAll = measure("npm-run-all", path.join(directory, "node_modules", ".bin", "run-s"), ["--silent", taskPattern]);
const runAllNow = measure("run-all-now (JS → Rust)", process.execPath, [path.join(root, "bin", "run-s.js"), "--silent", taskPattern]);
const runAllNowComparison = comparison(npmRunAll, runAllNow);
const summary = {
  taskCount,
  iterations,
  taskPattern,
  platform: `${process.platform}-${process.arch}`,
  npmRunAll: npmRunAll.average,
  runAllNow: runAllNow.average,
  savedMs: runAllNowComparison.savedMs,
  fasterPercent: runAllNowComparison.fasterPercent,
  commands: {
    npmRunAll,
    runAllNow
  },
  comparisons: {
    runAllNowVsNpmRunAll: runAllNowComparison
  }
};

console.log(JSON.stringify(summary, null, 2));
fs.mkdirSync(path.join(root, "assets"), { recursive: true });
fs.writeFileSync(path.join(root, "assets", "benchmark.json"), `${JSON.stringify(summary, null, 2)}\n`);
