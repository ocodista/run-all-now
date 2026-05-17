"use strict";

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { resolveBinary } = require("./lib/resolve-binary");

class NpmRunAllError extends Error {
  constructor(message, results) {
    super(message);
    this.name = "NpmRunAllError";
    this.results = results;
  }
}

function toArray(value, fieldName) {
  if (value == null) {
    return [];
  }
  if (Array.isArray(value)) {
    for (const entry of value) {
      if (typeof entry !== "string") {
        throw new TypeError(`${fieldName} entries must be strings`);
      }
    }
    return value.slice();
  }
  if (typeof value === "string") {
    return [value];
  }
  throw new TypeError(`${fieldName} must be a string or an array of strings`);
}

function toOptionalStringArray(value, fieldName) {
  if (value == null) {
    return null;
  }
  return toArray(value, fieldName);
}

function normalizeMaxParallel(value) {
  if (value == null || value === Number.POSITIVE_INFINITY) {
    return 0;
  }
  if (!Number.isFinite(value) || value < 0) {
    throw new TypeError("options.maxParallel must be a positive number");
  }
  return Math.floor(value);
}

function normalizeOptions(patterns, options) {
  const source = options || {};
  return {
    patterns: toArray(patterns, "patterns"),
    arguments: toArray(source.arguments, "options.arguments"),
    aggregateOutput: Boolean(source.aggregateOutput),
    continueOnError: Boolean(source.continueOnError),
    parallel: Boolean(source.parallel),
    maxParallel: normalizeMaxParallel(source.maxParallel),
    npmPath: typeof source.npmPath === "string" ? source.npmPath : null,
    config: source.config || null,
    packageConfig: source.packageConfig || null,
    printLabel: Boolean(source.printLabel),
    printName: Boolean(source.printName),
    race: Boolean(source.race),
    silent: Boolean(source.silent),
    taskList: toOptionalStringArray(source.taskList, "options.taskList")
  };
}

function pipeIfPresent(readable, writable) {
  if (!readable || !writable) {
    return;
  }
  readable.pipe(writable, { end: false });
}

function runAll(patterns, options) {
  let request;
  try {
    request = normalizeOptions(patterns, options);
  } catch (error) {
    return Promise.reject(error);
  }

  if (request.patterns.length === 0) {
    return Promise.resolve(null);
  }

  const source = options || {};
  return fs.promises.mkdtemp(path.join(os.tmpdir(), "run-all-now-")).then(async (directory) => {
    const optionsPath = path.join(directory, "options.json");
    const resultPath = path.join(directory, "result.json");
    await fs.promises.writeFile(optionsPath, JSON.stringify(request), "utf8");

    return new Promise((resolve, reject) => {
      const binary = resolveBinary();
      const stderrChunks = [];
      const child = spawn(binary, ["--run-all-now-api", optionsPath], {
        stdio: [source.stdin ? "pipe" : "ignore", source.stdout ? "pipe" : "ignore", "pipe"],
        env: {
          ...process.env,
          RUN_ALL_NOW_RESULT_FILE: resultPath,
          RUN_ALL_NOW_NODE_PATH: process.execPath
        }
      });

      if (source.stdin) {
        source.stdin.pipe(child.stdin);
      }
      pipeIfPresent(child.stdout, source.stdout);
      if (source.stderr) {
        pipeIfPresent(child.stderr, source.stderr);
      } else if (child.stderr) {
        child.stderr.on("data", (chunk) => stderrChunks.push(chunk));
      }

      child.on("error", (error) => reject(error));
      child.on("close", async (code) => {
        try {
          const payload = await readResult(resultPath);
          await fs.promises.rm(directory, { recursive: true, force: true });
          if (code === 0) {
            resolve(payload.results);
          } else {
            const stderr = Buffer.concat(stderrChunks).toString("utf8").trim();
            const message = payload.error || stderr || `run-all-now exited with code ${code}`;
            reject(new NpmRunAllError(message, payload.results));
          }
        } catch (error) {
          await fs.promises.rm(directory, { recursive: true, force: true }).catch(() => {});
          reject(error);
        }
      });
    });
  });
}

async function readResult(resultPath) {
  try {
    const raw = await fs.promises.readFile(resultPath, "utf8");
    const parsed = JSON.parse(raw);
    return {
      results: Array.isArray(parsed.results) ? parsed.results : [],
      error: typeof parsed.error === "string" ? parsed.error : null
    };
  } catch {
    return { results: [], error: null };
  }
}

runAll.NpmRunAllError = NpmRunAllError;
module.exports = runAll;
