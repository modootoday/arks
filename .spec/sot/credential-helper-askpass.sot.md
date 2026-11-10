---
id: sot:root/credential-helper-askpass
title: Credential helper / askpass (TTY + OS wallets)
status: active
created: 2026-11-10T22:40:00Z
updated: 2026-11-11T00:10:00Z
tags: [credential, askpass, https, auth, fw-cred-askpass, g33, tty, wincred, libsecret, osxkeychain]
verify: clean-room
rel:
  governs: [crates/arks-remote, crates/arks]
  depends_on:
    - sot:root/arks-remote
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-complete
    - sot:root/remote-host-compatibility-surface
    - sot:root/public-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - research:root/20261108073228
  evidence: [evidence:root/20261110224000, evidence:root/20261110233000]
---

# Credential helper / askpass (TTY + OS wallets)

Version **0.2.0**. Unlocks residual **FW-CRED-ASKPASS** as a **Claimed** HTTPS credential surface for `arks` / `arks-remote`: plumbing fill, askpass programs, **TTY terminal prompt**, and **first-party OS wallet helpers** (`wincred` / `libsecret` / `osxkeychain`). Supersedes v0.1.0 “minimal non-interactive / wallets N/A”.

Primary docs (cite; do not paste):

- [gitcredentials](https://git-scm.com/docs/gitcredentials) — helper selection, askpass order, terminal prompt, popular OS wallet helpers
- [git-credential](https://git-scm.com/docs/git-credential) — `fill` / `approve` / `reject`; INPUT/OUTPUT attribute format
- [git-config](https://git-scm.com/docs/git-config) — `credential.helper`, `credential.username`, `core.askPass`, `GIT_TERMINAL_PROMPT`

## Rule

### MUST (Claimed)

1. **Plumbing.** `arks credential fill|approve|reject` speaks the git-credential stdin/stdout attribute protocol (`protocol=`, `host=`, `path=`, `username=`, `password=`; blank-line terminated) for the claimed actions.
2. **Fill order.** When HTTPS needs credentials and URL userinfo is absent, fill MUST combine, in this order (gitcredentials requesting-credentials + helper loop):
   1. Existing attributes already present on the description
   2. Environment-backed values `ARKS_REMOTE_USERNAME` / `ARKS_REMOTE_PASSWORD` (arks product env helper; secrets never from the tree)
   3. Configured `credential.helper` entries — invoke with operation `get` / `store` / `erase` per gitcredentials custom-helper rules (`!` shell snippet, absolute path, or helper name). Helper names **`wincred`**, **`libsecret`**, and **`osxkeychain`** MUST resolve to **arks first-party** wallet helpers (in-process and/or `arks credential-<name>`), not as an N/A exclusion. Bang helpers that are a **simple argv** (including POSIX single-quoted paths with spaces, e.g. stock `gh auth setup-git` on Windows: `!'C:\Program Files\GitHub CLI\gh.exe' auth git-credential`) MUST spawn the executable directly with argv — do not require host `sh`, and MUST NOT leave Unix single quotes intact under `cmd /C` (git-for-Windows uses a shell that honors them; arks uses argv / cmd-safe rewrite for cross-OS parity).
   4. `GIT_ASKPASS`, else `core.askPass`, else `SSH_ASKPASS` — run the program with a prompt argv; read one line from stdout
   5. **TTY terminal prompt** when `GIT_TERMINAL_PROMPT` is unset or not `0` and a controlling terminal is available — prompt on the TTY for username (if missing) and password (gitcredentials step 4). When no TTY is available or `GIT_TERMINAL_PROMPT=0`, skip this step (fail closed if still incomplete).
3. **OS wallet helpers (first-party).** For helper names `wincred` / `libsecret` / `osxkeychain`:
   - `get` / `store` / `erase` MUST implement the credential-helper stdin/stdout protocol.
   - Target key shape MUST be `git:{protocol}://{host}` (path omitted unless `credential.useHttpPath` is true — default omit path, matching common Git wallet helpers).
   - **Persistence backends (all Claimed product paths, not residuals):**
     1. If `ARKS_CREDENTIAL_WALLET_DIR` (or config `credential.arksWalletDir`) is set → portable file wallet under that directory (clean-room / portable; intentional documented backend).
     2. Else on **Windows** + `wincred` → Windows Credential Manager (`CredRead` / `CredWrite` / `CredDelete`).
     3. Else on **macOS** + `osxkeychain` → macOS user keychain generic password item.
     4. Else on **Linux** + `libsecret` → Secret Service via `secret-tool` when present; else the portable file wallet under `$XDG_DATA_HOME/arks/credentials` (or `~/.local/share/arks/credentials`).
     5. Cross-OS helper name on a non-matching host without (1) → `get` returns empty; `store`/`erase` succeed as no-ops (silent), matching Git helper tolerance.
4. **HTTPS Basic.** Remote smart-HTTP client MUST attach `Authorization: Basic …` when fill yields username+password (or URL userinfo).
5. **Secrets hygiene.** Secrets MUST NOT enter tracked fixtures, evidence, or logs. Clean-room helpers MAY use process-env / runtime-generated passwords and `ARKS_CREDENTIAL_WALLET_DIR`.
6. **Fail closed.** If no source yields a password after the fill order above, the operation fails closed (non-zero / HTTP auth error) without hanging.

### MAY

- Forward `approve` / `reject` to configured helpers (`store` / `erase`), including first-party OS wallet helpers.
- Read `credential.username` / context-matched `credential.<url>.username` from discovered config files.
- Expose CLI entrypoints `arks credential-wincred|credential-libsecret|credential-osxkeychain` for PATH/`credential.helper` absolute-path use.

### Out of scope (still N/A / non-claim)

- Forge OAuth browser UX / host Apps auth (owned by AUTH residual / separate SoT when Claimed).
- `credential-cache` daemon product path.
- Git Credential Manager as a bundled first-party binary (external GCM MAY still be configured by path/name).
- SSH askpass as a required client-SSH path (OpenSSH agent/keys remain Sy-4; this SoT is HTTPS-focused).

## Why

Operators need standards-cited HTTPS Basic fill including interactive TTY and OS wallet helpers that Git documents as popular (`wincred` / `libsecret` / `osxkeychain`). Claiming those paths closes **FW-CRED-ASKPASS** without leaving TTY/wallets as residual N/A.

## Verification

1. Host unit tests in `arks-remote` / CLI — **advisory** (including Windows Cred* host tests when on Windows).
2. Clean-room **G33** (`verify/matrices/matrix-g33.md`, compose `g33`) — **authoritative** for Claimed (prior C01–C05 plus TTY + wallet rows).
3. `spec check` clean; residual register **FW-CRED-ASKPASS** → **Claimed** with evidence cite; no “부분/대기/N/A wallets” wording for this ID.
4. Public copy (README / alias docs) must not leave TTY / OS wallets as N/A after Claimed.
