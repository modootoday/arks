---
id: research:root/20261111072342
title: Remaining CLI-stub Missing inventory (post bf2b01f deepen)
status: active
created: 2026-11-10T22:23:42Z
updated: 2026-11-10T22:23:42Z
tags: [cli-stubs, missing, partial, ssh, hub, agent, daemon, serve-config, residual, no-invent-claimed, ffi, os-limits]
verify: manual
rel:
  depends_on:
    - sot:root/arks-self-host-server
    - sot:root/arks-server
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/arks-agent-product-intent
    - sot:root/production-residual-register
    - sot:root/public-surface
    - sot:root/language-english-only
  related:
    - docs/completion.md
    - .spec.local/goal-cli-stubs-complete.md
    - .spec.local/progress-post-spec-canon.md
    - .spec.local/sot/ssh-inprocess-ffi-partial.note.md
---

# Remaining CLI-stub Missing inventory (post `bf2b01f`)

Tip surveyed: **`bf2b01f`** (canon `C:\workspaces\gir`). English-only. **Design ≠ Claimed.** No invent Claimed. Product paths never spawn `git` / `gh`.

## Method

| Source | Use |
|--------|-----|
| Public honesty | `docs/completion.md`, `docs/self-host.md`, `docs/daemon.md`, `docs/agent.md`, `docs/arks-hub.md`, `docs/ports.md` |
| Local journey | `.spec.local/goal-cli-stubs-complete.md`, `progress-post-spec-canon.md`, SSH FFI note |
| Residual | [[sot:root/production-residual-register]] §F / post-terminal (no bare Opens; future-work terminal) |
| Code TODO / Missing / fail-closed | `crates/arks-server/src/ssh_serve.rs`, `daemon.rs`, `serve.rs`; `crates/arks/src/cmd/server.rs`; `crates/arks-hub`, `crates/arks-agent` |

## Inventory snapshot (five bullets)

| # | Area | Tip wiring | Still Missing (Claimed / exact) | Deps / FFI / OS |
|---|------|------------|----------------------------------|-----------------|
| 1 | SSH OpenSSH-exact / git-over-SSH e2e | Partial: bind + id + KEXINIT + curve25519 + ed25519 sign/NEWKEYS + **chacha20-poly1305 AEAD Partial** + authorized_keys ed25519 verify + session exec→pack dispatch; `serve ssh-smoke` banner Partial | OpenSSH-exact `chacha20-poly1305@openssh.com` / `aes256-gcm@openssh.com` **packet** interop; durable listener + rebind; Claimed SSH git e2e | `chacha20poly1305`, `x25519-dalek`, `ed25519-dalek`; **no** russh/libssh; `arks-server` `forbid(unsafe_code)`; OpenSSH client e2e needs Docker + keys + durable path |
| 2 | Hub HTTPS Claimed-path e2e + residual OUT | `hub smoke` HTTPS ureq Partial; local http fallback documents Missing; `extension`/`copilot` OUT refuse | Claimed forge remote HTTPS e2e; asset upload; CI execute; OUT residual **harness** (refuse proof) thin | `ureq` TLS; no `gh` spawn; in-process TLS terminate still Missing (ops reverse-proxy) |
| 3 | Agent LLM chatbot/review e2e | `provider smoke [ollama]` Partial; review/cev/serve proxy Partial | Claimed LLM chatbot / review e2e; HTTPS chat Partial not wired | Native TCP/HTTP only; never spawn `ollama` CLI; local daemon optional |
| 4 | Daemon archive / user / group / syslog | upload-archive path-filter **in stream** + tar/ustar honesty + protocol-body tests; `--user`/`--group`/`--syslog` **honest Missing refuse** | Claimed archive format parity vs stock `git-daemon`; classic double-fork detach; syslog facility parity; setuid/setgid Claimed | Unix: `libc` openlog/syslog + setuid/setgid (CLI crate); Windows: no setuid; Event Log ≠ syslog |
| 5 | Serve config Windows reload / SSH durable / zero-downtime | Unix SIGHUP + same-process HTTP/forge live rebind Partial; Windows signal **Missing**; SSH durable rebind **Missing** (probe-then-fail-closed) | Windows SIGHUP (N/A OS); Claimed zero-downtime hot-rebind; SSH durable listen rebind under SoT §17 tension | Unix `sigaction(SIGHUP)`; Windows: file-mtime in-process / console ctrl Partial possible; SSH durable conflicts with fail-closed-while-Partial unless SoT honesty amend |

## Classification

### Self-implementable (honest Partial this wave)

| ID | Item | Shape | Claimed boundary (must stay Missing) |
|----|------|-------|--------------------------------------|
| I1 | Windows serve-config **in-process file-mtime reload** Partial | Arm watcher thread inside running `arks serve` on non-unix; poll config mtime → apply/reload + HTTP/forge rebind when slots armed | Not SIGHUP; not Claimed zero-downtime |
| I2 | Daemon **`--syslog` unix Partial** | `openlog`/`syslog` via `libc` in `arks` CLI; stderr still mirrors; Windows stays documented Missing | Not Claimed facility/priority parity vs stock git-daemon |
| I3 | Daemon **`--user` / `--group` unix Partial** | Resolve name/uid + `setgid`/`setuid` after listen intent; refuse fail-closed on error; Windows stays Missing | Not Claimed privilege-drop / capability model |
| I4 | Hub **residual OUT refuse smoke** Partial | `arks hub smoke --out-refuse` proves `extension`/`copilot` exit 2 + OUT messaging without `gh` | Not Claimed forge remote e2e; OUT stays refuse (not graduated) |
| I5 | Agent **`review --smoke`** Partial | Built-in fixture prompt + optional local chat; documents Missing when daemon absent | Not Claimed LLM review e2e |

### Blocked / residual-docs only (this wave)

| ID | Item | Why blocked | Honesty action |
|----|------|-------------|----------------|
| B1 | OpenSSH-exact AEAD cipher interop | Needs OpenSSH binary-packet AEAD (len encryption, nonce/counter, MAC-less ChaCha OpenSSH variant) + live OpenSSH client evidence; Partial AEAD selfcheck ≠ interop | Keep Missing in docs/completion + SSH module notes |
| B2 | Claimed git-over-SSH e2e | Depends on B1 + durable Claimed SSH stack + clean-room receipt | Leave goal open for coordinator; no invent Claimed |
| B3 | Claimed hub forge remote HTTPS e2e | Needs real HTTPS forge + receipt; smoke ≠ Claimed | Residual / docs honesty |
| B4 | Claimed agent LLM chatbot/review e2e | Needs evidenced multi-turn product path; one-shot smoke ≠ e2e | Residual / docs honesty |
| B5 | Claimed daemon archive parity | Needs stock git-daemon upload-archive byte/stream compare gate | Residual / docs honesty |
| B6 | SSH **durable** listen + rebind | [[sot:root/arks-self-host-server]] §Fail-closed: enabled module not Claimed/ready → process fail-closed. Tip SSH is **probe-then-fail-closed** (2s). Durable Partial listen would change that valve — **needs SoT honesty amend** before code | SoT/docs residual only this wave unless amend lands |
| B7 | Claimed zero-downtime hot-rebind | Needs connection-drain / dual-bind evidence | Keep Missing |
| B8 | Windows **SIGHUP** | OS has no SIGHUP | Document N/A; I1 is the Partial substitute |
| B9 | In-process TLS terminate | Ops reverse-proxy pattern; no Claimed in-process TLS | Keep Missing |

## Deps / FFI / OS limits (cite)

- **Crypto Partial:** workspace `chacha20poly1305`, `x25519-dalek`, `ed25519-dalek`. `aes256-gcm@openssh.com` is **advertised** but not selected Partial — OpenSSH-exact still B1.
- **FFI:** unix `libc` already used in `arks` for SIGHUP + `setsid` detach. `arks-server` remains `forbid(unsafe_code)` — privilege/syslog stay in CLI.
- **Windows:** no setuid/syslog; serve reload Partial = file-mtime (I1) or future console-ctrl; not SIGHUP.
- **Docker:** advisory `cargo check -p arks-server -p arks-hub -p arks-agent -p arks -p arks-remote` (not product Verified).

## Goal honesty

Wiring Partial/Missing for all five bullets was already recorded at tip `bf2b01f`. **Claimed e2e rows remain Missing** (B1–B5). Per operator: **do not mark CreateGoal complete** while those Claimed e2e rows stay Missing — leave goal for coordinator.

## Non-goals

- Do not invent Claimed/PASS from Partial smokes.
- Do not spawn `git` / `gh` / `ssh` / `ollama` CLIs from product paths.
- Do not push.
- Do not graduate OUT `extension`/`copilot` to local Partial list.
