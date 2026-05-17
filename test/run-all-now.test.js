"use strict";

const assert = require("node:assert/strict");
const { execFileSync, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { Writable } = require("node:stream");
const test = require("node:test");

const root = path.resolve(__dirname, "..");
const binaryName = process.platform === "win32" ? "run-all-now.exe" : "run-all-now";
const debugBinary = path.join(root, "target", "debug", binaryName);
const npmRunAllBin = path.join(root, "bin", "npm-run-all.js");
const runSBin = path.join(root, "bin", "run-s.js");
const runPBin = path.join(root, "bin", "run-p.js");

function makeFixture() {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "run-all-now-fixture-"));
  fs.mkdirSync(path.join(directory, "scripts"));
  fs.writeFileSync(path.join(directory, "scripts", "a.js"), "console.log('A');\n");
  fs.writeFileSync(path.join(directory, "scripts", "b.js"), "console.log('B');\n");
  fs.writeFileSync(path.join(directory, "scripts", "fail.js"), "console.error('FAIL'); process.exit(7);\n");
  fs.writeFileSync(path.join(directory, "scripts", "arg.js"), "console.log(process.argv.slice(2).join(','));\n");
  fs.writeFileSync(path.join(directory, "scripts", "slow.js"), "setTimeout(() => console.log('SLOW'), 80);\n");
  fs.writeFileSync(path.join(directory, "scripts", "fast.js"), "setTimeout(() => console.log('FAST'), 10);\n");
  fs.writeFileSync(
    path.join(directory, "package.json"),
    JSON.stringify(
      {
        name: "fixture",
        version: "1.0.0",
        scripts: {
          "echo:a": "node scripts/a.js",
          "echo:b": "node scripts/b.js",
          fail: "node scripts/fail.js",
          arg: "node scripts/arg.js",
          slow: "node scripts/slow.js",
          fast: "node scripts/fast.js"
        }
      },
      null,
      2
    )
  );
  return directory;
}

function runNode(bin, args, cwd) {
  return spawnSync(process.execPath, [bin, ...args], {
    cwd,
    encoding: "utf8",
    env: {
      ...process.env,
      RUN_ALL_NOW_BINARY: debugBinary,
      RUN_ALL_NOW_NODE_PATH: process.execPath
    }
  });
}

test.before(() => {
  execFileSync("cargo", ["build"], { cwd: root, stdio: "inherit" });
  process.env.RUN_ALL_NOW_BINARY = debugBinary;
});

test("run-s JavaScript bin calls the Rust binary and runs matched scripts sequentially", () => {
  const cwd = makeFixture();
  const result = runNode(runSBin, ["--silent", "echo:*"], cwd);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^A\nB\n$/);
});

test("run-p JavaScript bin calls the Rust binary and runs matched scripts in parallel", () => {
  const cwd = makeFixture();
  const result = runNode(runPBin, ["--silent", "slow", "fast"], cwd);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /FAST/);
  assert.match(result.stdout, /SLOW/);
});

test("npm-run-all supports mixed sequential and parallel groups", () => {
  const cwd = makeFixture();
  const result = runNode(npmRunAllBin, ["--silent", "echo:a", "--parallel", "fast", "slow"], cwd);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /A/);
  assert.match(result.stdout, /FAST/);
  assert.match(result.stdout, /SLOW/);
});

test("argument placeholders are forwarded to scripts", () => {
  const cwd = makeFixture();
  const result = runNode(runSBin, ["--silent", "arg -- {1} {@} {*} {3:-fallback}", "--", "one", "two words"], cwd);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), "one,one,two words,one two words,fallback");
});

test("continue-on-error finishes remaining scripts and exits non-zero", () => {
  const cwd = makeFixture();
  const result = runNode(runSBin, ["--silent", "--continue-on-error", "echo:a", "fail", "echo:b"], cwd);
  assert.equal(result.status, 1);
  assert.match(result.stdout, /A/);
  assert.match(result.stdout, /B/);
});

test("node API resolves with npm-run-all compatible result objects", async () => {
  const cwd = makeFixture();
  const previousCwd = process.cwd();
  process.chdir(cwd);
  try {
    const chunks = [];
    const stdout = new Writable({
      write(chunk, _encoding, callback) {
        chunks.push(Buffer.from(chunk));
        callback();
      }
    });
    const runAll = require("..");
    const results = await runAll(["echo:*"], { stdout, parallel: false, silent: true });
    assert.deepEqual(results, [
      { name: "echo:a", code: 0 },
      { name: "echo:b", code: 0 }
    ]);
    assert.equal(Buffer.concat(chunks).toString("utf8"), "A\nB\n");
  } finally {
    process.chdir(previousCwd);
  }
});

test("ESM API resolves with the same runAll function", async () => {
  const cwd = makeFixture();
  const previousCwd = process.cwd();
  process.chdir(cwd);
  try {
    const api = await import("run-all-now");
    assert.equal(typeof api.default, "function");
    assert.equal(api.runAll, api.default);
    assert.equal(typeof api.NpmRunAllError, "function");
    const results = await api.default(["echo:a"], { parallel: false, silent: true });
    assert.deepEqual(results, [{ name: "echo:a", code: 0 }]);
  } finally {
    process.chdir(previousCwd);
  }
});

test("node API rejects with NpmRunAllError on failed scripts", async () => {
  const cwd = makeFixture();
  const previousCwd = process.cwd();
  process.chdir(cwd);
  try {
    const runAll = require("..");
    await assert.rejects(
      runAll(["fail"], { parallel: false, silent: true }),
      (error) => {
        assert.equal(error.name, "NpmRunAllError");
        assert.match(error.message, /fail/);
        assert.deepEqual(error.results, [{ name: "fail", code: 7 }]);
        return true;
      }
    );
  } finally {
    process.chdir(previousCwd);
  }
});
