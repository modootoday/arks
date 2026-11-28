---
title: Getting started
description: Build the native arks CLI and run your first Git-compatible workflows
nav_order: 30
sections:
  - {c: steps, name: build}
  - {c: tabs, name: first}
  - remotes
  - {c: steps, name: alias}
  - {c: cta, name: next}
---

# Getting started

Install **arks** as a normal local CLI. You do not need Docker to use the product — containers are for maintainer verification only.

If you landed here from multi-agent orchestration pain: start with a single-session smoke test below, then read [Concurrent sessions](../concurrent-sessions/) and [Alias `git` → `arks`](../alias-git-arks/) for the adopt path.

## Install or build the native CLI

1. **Prebuilt binary (recommended):** download the archive for Linux (`x86_64`), macOS (`arm64`), or Windows (`x86_64`) plus `SHA256SUMS` from [GitHub Releases (`v0.1.0`)](https://github.com/modootoday/arks/releases/tag/v0.1.0) and place `arks` on your `PATH`.
2. **From source (Rust 1.85+):** install directly from the release tag or build locally:

`file:build.sh`

```bash
# Direct install from release tag:
cargo install --git https://github.com/modootoday/arks --tag v0.1.0 arks

# Or build from a local checkout:
git clone https://github.com/modootoday/arks.git
cd arks
cargo build -p arks --release
```

Without installing:

`file:help.sh`

```bash
cargo run -p arks -- --help
```

Registry packages (`crates.io`, `PyPI`, `npm`, `RubyGems`) are packaged and dry-run verified; see [Release channels](../release/) for live registry activation status.

## First repository

### Init and commit

```bash
arks init demo
cd demo
echo "hello from arks" > hello.txt
arks add hello.txt
arks commit -m "Add hello.txt"
arks status
arks log
```

### Inspect with stock Git

```bash
git status
git log --oneline
```

Open the same directory with stock Git if you want a second opinion — the `.git` directory is ordinary Git data.

## Remotes

Smart HTTP remotes use the usual commands:

`file:remote.sh`

```bash
arks clone https://example.com/repo.git
cd repo
arks fetch origin
arks pull
arks push
```

## Alias `git` → `arks`

1. Prefer a shell alias for interactive evaluation.
2. Prefer a PATH shim when scripts or agent runners shell out to `git`.
3. Optional forge peer: `alias gh=arks` (personality `gh`) — see [CLI personality](../cli-personality/).
4. Treat unfinished commands as fail-closed — see [Alias `git` → `arks`](../alias-git-arks/).

`file:alias.sh`

```bash
alias git=arks
# optional:
alias gh=arks
```

## Self-host (optional)

HTTP git wire that is Claimed today (opt out of Partial modules):

```bash
arks serve --no-ssh --no-forge /path/to/repos
```

Full story: [Self-host](../self-host/) · [Ports](../ports/) · honesty bar: [Completeness](../completion/).

## Next

[Common workflows](../workflows/) [CLI personality](../cli-personality/) [Concurrent sessions](../concurrent-sessions/) [Limits](../limits/)
