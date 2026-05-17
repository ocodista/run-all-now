"use strict";

const { spawn } = require("node:child_process");
const { resolveBinary } = require("../lib/resolve-binary");

function run(binName) {
  const child = spawn(resolveBinary(), process.argv.slice(2), {
    stdio: "inherit",
    env: {
      ...process.env,
      RUN_ALL_NOW_BIN_NAME: binName,
      RUN_ALL_NOW_NODE_PATH: process.execPath
    }
  });

  child.on("error", (error) => {
    console.error(`ERROR: ${error.message}`);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code == null ? 1 : code);
  });
}

module.exports = { run };
