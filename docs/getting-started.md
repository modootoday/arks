---
title: Getting started
description: Build the native arks CLI and run your first Git-compatible workflows
sections:
  - build
  - first
  - remotes
  - alias
  - {c: cta, name: next}
---

# Getting started

Install **arks** as a normal local CLI. You do not need Docker to use the product — containers are for maintainer verification only.

If you landed here from multi-agent orchestration pain: start with a single-session smoke test below, then read [Concurrent sessions](../concurrent-sessions/) and [Alias `git` → `arks`](../alias-git-arks/) for the adopt path.

## Build the native CLI

You need a recent Rust toolchain (**1.85+**).

```bash
git clone https://github.com/modootoday/arks.git
cd arks
cargo build -p arks --release
```

Put `target/release/arks` on your `PATH`, or run without installing:

```bash
cargo run -p arks -- --help
```

## First repository

```bash
arks init demo
cd demo
echo "hello from arks" > hello.txt
arks add hello.txt
arks commit -m "Add hello.txt"
arks status
arks log
```

Open the same directory with stock Git if you want a second opinion — the `.git` directory is ordinary Git data.

## Remotes

Smart HTTP remotes use the usual commands:

```bash
arks clone https://example.com/repo.git
cd repo
arks fetch origin
arks pull
arks push
```

## Alias `git` → `arks`

```bash
alias git=arks
```

Familiar commands then invoke arks. Shell aliases do not apply inside many scripts — use a PATH shim when you need non-interactive drop-in behavior. Full honesty notes: [Alias `git` → `arks`](../alias-git-arks/).

## Next

[Common workflows](../workflows/) [Concurrent sessions](../concurrent-sessions/) [Limits](../limits/)
