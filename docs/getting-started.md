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

## Build the native CLI

1. Install a recent Rust toolchain (**1.85+**).
2. Clone and build the release binary.
3. Put `target/release/arks` on your `PATH`, or invoke via `cargo run`.

`file:build.sh`

```bash
git clone https://github.com/modootoday/arks.git
cd arks
cargo build -p arks --release
```

Without installing:

`file:help.sh`

```bash
cargo run -p arks -- --help
```

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
3. Treat unfinished commands as fail-closed — see [Alias `git` → `arks`](../alias-git-arks/).

`file:alias.sh`

```bash
alias git=arks
```

## Next

[Common workflows](../workflows/) [Concurrent sessions](../concurrent-sessions/) [Limits](../limits/)
