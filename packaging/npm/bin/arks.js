#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const PLATFORM_MAP = {
  "linux:x64": { pkg: "arks-linux-x64", bin: "bin/arks" },
  "darwin:arm64": { pkg: "arks-darwin-arm64", bin: "bin/arks" },
  "win32:x64": { pkg: "arks-win32-x64", bin: "bin/arks.exe" },
};

const key = `${process.platform}:${process.arch}`;
const entry = PLATFORM_MAP[key];

if (!entry) {
  console.error(
    `arks: unsupported platform ${process.platform}/${process.arch}. ` +
      `Prebuilt npm binaries are available for linux/x64, darwin/arm64, and win32/x64. ` +
      `Install from source via cargo or download a release binary from https://github.com/modootoday/arks/releases.`
  );
  process.exit(1);
}

let binPath;
try {
  binPath = require.resolve(`${entry.pkg}/${entry.bin}`);
} catch {
  console.error(
    `arks: platform binary package "${entry.pkg}" is not installed. ` +
      `Reinstall without --omit=optional, or download from https://github.com/modootoday/arks/releases.`
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`arks: failed to execute ${binPath}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 1);
