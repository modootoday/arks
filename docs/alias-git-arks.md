---
title: Alias git to arks
description: How to adopt arks with alias git=arks — the symmetric CLI path
nav_order: 40
sections:
  - why
  - {c: tabs, name: shell}
  - shim
  - {c: accordion, name: honesty}
  - {c: cta, name: next}
---

# Alias `git` → `arks`

arks ships a native binary named **`arks`**. The product goal is a **fully symmetric CLI** so anyone can adopt immediately: keep muscle memory, change the binary underneath. Drop-in usage is a **shell alias** or a **PATH shim**, not a renamed binary and not a container.

## Why alias matters here

Multi-session / multi-agent environments already teach people `git …`. A separate command vocabulary would block adoption. Alias (or shim) is the intended day-one path for claimed commands — while unfinished flags still fail closed.

## Shell alias

### bash / zsh

```bash
alias git=arks
```

### Git Bash (Windows)

```bash
alias git=arks.exe
```

### fish

```fish
alias git=arks
```

Familiar commands (`git status`, `git commit -m …`) then invoke arks with the same arguments after the command name.

## PATH shim (scripts)

Place an executable named `git` earlier on `PATH` that runs the arks binary with `"$@"`. Shell aliases do not apply inside many scripts and CI jobs, so a shim is the reliable non-interactive option — including agent runners that shell out to `git`.

> [!TIP]
> Use the alias for interactive evaluation; use a PATH shim when orchestrators or CI must call `git` without a shell profile.

## Honesty

### Coverage

The alias is for evaluation and everyday claimed commands — not a warranty that every Git subcommand and flag exists today. Unimplemented or explicitly excluded commands exit **non-zero** and must not change the repository.

### Out of the client alias story

GUI tools (`gitk` / `gui` / `citool`), Git LFS, `scalar`, and forge host APIs are not part of the client alias story.

### Credentials

Credential fill includes `credential.helper` / `GIT_ASKPASS` / TTY prompts and first-party OS wallets (`wincred` / `libsecret` / `osxkeychain`); forge OAuth stays out.

### Separate capabilities

Optional concurrency leases and the Git **server** role are separate capabilities; ordinary client alias use does not require them — see [Concurrent sessions](../concurrent-sessions/). The published binary remains `arks`; do not publish a `git`-named crate or binary.

## Next

[Getting started](../getting-started/) [Workflows](../workflows/) [Limits](../limits/)
