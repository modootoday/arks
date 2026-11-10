---
title: Alias git to arks
description: How to adopt arks with alias git=arks — the symmetric CLI path
sections:
  - why
  - shell
  - shim
  - honesty
  - {c: cta, name: next}
---

# Alias `git` → `arks`

arks ships a native binary named **`arks`**. The product goal is a **fully symmetric CLI** so anyone can adopt immediately: keep muscle memory, change the binary underneath. Drop-in usage is a **shell alias** or a **PATH shim**, not a renamed binary and not a container.

## Why alias matters here

Multi-session / multi-agent environments already teach people `git …`. A separate command vocabulary would block adoption. Alias (or shim) is the intended day-one path for claimed commands — while unfinished flags still fail closed.

## Shell alias

```bash
alias git=arks
# Windows Git Bash, if needed:
# alias git=arks.exe
```

Familiar commands (`git status`, `git commit -m …`) then invoke arks with the same arguments after the command name.

## PATH shim (scripts)

Place an executable named `git` earlier on `PATH` that runs the arks binary with `"$@"`. Shell aliases do not apply inside many scripts and CI jobs, so a shim is the reliable non-interactive option — including agent runners that shell out to `git`.

## Honesty

- The alias is for evaluation and everyday claimed commands — not a warranty that every Git subcommand and flag exists today.
- Unimplemented or explicitly excluded commands exit **non-zero** and must not change the repository (for example interactive-only flows without a non-interactive path).
- Not part of the client alias story: GUI tools (`gitk` / `gui` / `citool`), Git LFS, `scalar`, and forge host APIs.
- Credential fill includes `credential.helper` / `GIT_ASKPASS` / TTY prompts and first-party OS wallets (`wincred` / `libsecret` / `osxkeychain`); forge OAuth stays out.
- Optional concurrency leases and the Git **server** role are separate capabilities; ordinary client alias use does not require them — see [Concurrent sessions](../concurrent-sessions/).
- The published binary remains `arks`; do not publish a `git`-named crate or binary.

## Next

[Getting started](../getting-started/) [Workflows](../workflows/) [Limits](../limits/)
