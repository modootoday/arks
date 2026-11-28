#!/usr/bin/env node

// Build npm platform packages (`arks-linux-x64`, `arks-darwin-arm64`,
// `arks-win32-x64`) and the root `arks` launcher package from the exact
// prebuilt `dist` archives emitted by `.github/workflows/release.yml`.
//
// Cite: .spec/sot/external-distribution-and-registry-publishing.sot.md

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "..", "..");

const TARGETS = {
  "x86_64-unknown-linux-gnu": {
    pkgName: "arks-linux-x64",
    archive: "arks-x86_64-unknown-linux-gnu.tar.gz",
    binName: "arks",
    os: ["linux"],
    cpu: ["x64"],
  },
  "aarch64-apple-darwin": {
    pkgName: "arks-darwin-arm64",
    archive: "arks-aarch64-apple-darwin.tar.gz",
    binName: "arks",
    os: ["darwin"],
    cpu: ["arm64"],
  },
  "x86_64-pc-windows-msvc": {
    pkgName: "arks-win32-x64",
    archive: "arks-x86_64-pc-windows-msvc.zip",
    binName: "arks.exe",
    os: ["win32"],
    cpu: ["x64"],
  },
};

function readWorkspaceVersion() {
  const cargo = fs.readFileSync(path.join(REPO_ROOT, "Cargo.toml"), "utf8");
  const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) {
    throw new Error("unable to read workspace version from Cargo.toml");
  }
  return m[1];
}

function parseArgs(argv) {
  const opts = { version: null, artifactsDir: null, outDir: null, singleBinaries: {} };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--version") opts.version = argv[++i];
    else if (a === "--artifacts-dir") opts.artifactsDir = path.resolve(argv[++i]);
    else if (a === "--out-dir") opts.outDir = path.resolve(argv[++i]);
    else if (a === "--single-binary") {
      const spec = argv[++i];
      const idx = spec.indexOf("=");
      if (idx === -1) throw new Error(`invalid --single-binary: ${spec}`);
      opts.singleBinaries[spec.slice(0, idx)] = path.resolve(spec.slice(idx + 1));
    } else {
      throw new Error(`unknown argument: ${a}`);
    }
  }
  if (!opts.outDir) throw new Error("--out-dir is required");
  return opts;
}

function extractBinary(archivePath, binName, tmpDir) {
  fs.mkdirSync(tmpDir, { recursive: true });
  if (archivePath.endsWith(".zip")) {
    execFileSync("python3", [
      "-c",
      `import pathlib, sys, zipfile
zf = zipfile.ZipFile(sys.argv[1])
for name in zf.namelist():
    if pathlib.Path(name).name == sys.argv[2]:
        pathlib.Path(sys.argv[3]).write_bytes(zf.read(name))
        sys.exit(0)
sys.exit("binary not found in zip")`,
      archivePath,
      binName,
      path.join(tmpDir, binName),
    ]);
  } else {
    execFileSync("python3", [
      "-c",
      `import pathlib, sys, tarfile
with tarfile.open(sys.argv[1], "r:gz") as tf:
    for m in tf.getmembers():
        if m.isfile() and pathlib.Path(m.name).name == sys.argv[2]:
            pathlib.Path(sys.argv[3]).write_bytes(tf.extractfile(m).read())
            sys.exit(0)
sys.exit("binary not found in tar.gz")`,
      archivePath,
      binName,
      path.join(tmpDir, binName),
    ]);
  }
  return path.join(tmpDir, binName);
}

function copyCommonDocs(destDir) {
  for (const name of ["LICENSE", "LICENSE-APACHE", "LICENSE-MIT", "README.md"]) {
    const src = path.join(REPO_ROOT, name);
    if (fs.existsSync(src)) {
      fs.copyFileSync(src, path.join(destDir, name));
    }
  }
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  const version = opts.version || readWorkspaceVersion();
  fs.mkdirSync(opts.outDir, { recursive: true });

  const builtPlatformPkgs = [];

  for (const [target, spec] of Object.entries(TARGETS)) {
    let binSrc = opts.singleBinaries[target] || null;
    if (!binSrc && opts.artifactsDir) {
      const archivePath = path.join(opts.artifactsDir, spec.archive);
      if (fs.existsSync(archivePath)) {
        const tmpExtract = path.join(opts.outDir, ".tmp-extract", target);
        binSrc = extractBinary(archivePath, spec.binName, tmpExtract);
      }
    }
    if (!binSrc) continue;

    const pkgDir = path.join(opts.outDir, spec.pkgName);
    fs.rmSync(pkgDir, { recursive: true, force: true });
    fs.mkdirSync(path.join(pkgDir, "bin"), { recursive: true });

    const binDest = path.join(pkgDir, "bin", spec.binName);
    fs.copyFileSync(binSrc, binDest);
    fs.chmodSync(binDest, 0o755);
    copyCommonDocs(pkgDir);

    const pkgJson = {
      name: spec.pkgName,
      version,
      description: `Prebuilt arks binary for ${spec.os[0]}/${spec.cpu[0]}`,
      license: "Apache-2.0 OR MIT",
      homepage: "https://modootoday.github.io/arks/",
      repository: {
        type: "git",
        url: "https://github.com/modootoday/arks.git",
      },
      bugs: {
        url: "https://github.com/modootoday/arks/issues",
      },
      os: spec.os,
      cpu: spec.cpu,
      files: [`bin/${spec.binName}`, "LICENSE", "LICENSE-APACHE", "LICENSE-MIT", "README.md"],
      preferUnplugged: true,
    };
    fs.writeFileSync(path.join(pkgDir, "package.json"), JSON.stringify(pkgJson, null, 2) + "\n");
    builtPlatformPkgs.push(spec.pkgName);
    console.log(`built npm platform package ${spec.pkgName}@${version}`);
  }

  if (builtPlatformPkgs.length === 0) {
    throw new Error("no platform binaries found (pass --artifacts-dir or --single-binary)");
  }

  // Clean temporary extraction dir
  fs.rmSync(path.join(opts.outDir, ".tmp-extract"), { recursive: true, force: true });

  // Root `arks` package with optionalDependencies pointing to all 3 platform packages
  const rootDir = path.join(opts.outDir, "arks");
  fs.rmSync(rootDir, { recursive: true, force: true });
  fs.mkdirSync(path.join(rootDir, "bin"), { recursive: true });
  const shimDest = path.join(rootDir, "bin", "arks.js");
  fs.copyFileSync(path.join(__dirname, "bin", "arks.js"), shimDest);
  fs.chmodSync(shimDest, 0o755);
  copyCommonDocs(rootDir);

  const optionalDependencies = {};
  for (const spec of Object.values(TARGETS)) {
    optionalDependencies[spec.pkgName] = version;
  }

  const rootPkgJson = {
    name: "arks",
    version,
    description:
      "arks for Git — native Rust Git-compatible CLI, daemon, forge client, and self-hosted server",
    license: "Apache-2.0 OR MIT",
    homepage: "https://modootoday.github.io/arks/",
    repository: {
      type: "git",
      url: "https://github.com/modootoday/arks.git",
    },
    bugs: {
      url: "https://github.com/modootoday/arks/issues",
    },
    keywords: ["git", "vcs", "cli", "concurrency", "arks"],
    engines: {
      node: ">=18",
    },
    bin: {
      arks: "bin/arks.js",
    },
    files: ["bin/arks.js", "LICENSE", "LICENSE-APACHE", "LICENSE-MIT", "README.md"],
    optionalDependencies,
  };
  fs.writeFileSync(path.join(rootDir, "package.json"), JSON.stringify(rootPkgJson, null, 2) + "\n");
  console.log(`built npm root package arks@${version}`);
}

main();
