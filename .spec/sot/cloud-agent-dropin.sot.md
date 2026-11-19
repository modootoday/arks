---
id: sot:root/cloud-agent-dropin
title: Cloud agent drop-in (arks as git for cloud agent sessions)
status: active
created: 2026-11-19T18:00:00Z
updated: 2026-11-19T18:00:00Z
tags: [cloud, agent, dropin, proxy, tls, ca, config, signing, ssh-signing, personality, corpus, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/arks-dod-behavioral
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-remote
    - sot:root/credential-helper-askpass
    - sot:root/arks-object
    - sot:root/storage-celir-boundary
    - sot:root/product-verification-clean-room
    - sot:root/production-residual-register
    - adr:root/20261119180100
    - plan:root/20261119180000
---

# Cloud agent drop-in

Document revision **0.1.0** (SoT edit counter only — **not** a product/crate release).

Normative contract for running `arks` (personality `git`) as the version-control CLI of
hosted coding-agent sessions (for example Claude Code cloud containers). The
[[sot:root/arks-dod-behavioral]] bar proved peer actions for a named porcelain set in a
plain Docker network. Agent sessions additionally run behind a TLS-intercepting egress
proxy, read identity and signing settings from global config, and use a wider set of
option spellings. This SoT names that delta. Goal directive: [[plan:root/20261119180000]].
Decisions D1–D6: [[adr:root/20261119180100]].

## Baseline (host advisory, tip `b072a81`)

| Area | Observed | Class |
|------|----------|-------|
| HTTPS transport | `arks-remote` agent ignores `http.sslCAInfo` / `GIT_SSL_CAINFO` / `HTTPS_PROXY` → `UnknownIssuer` | Missing |
| Config | Global `~/.gitconfig` identity not used by `commit` (author `arks <arks@example.com>`) | Missing |
| Signing | `commit.gpgsign`, `gpg.format=ssh`, `gpg.ssh.program` ignored | Missing |
| Options | Corpus rows in §Corpus fail with `unsupported option` | Missing |
| Build | Fresh clone needs sibling `discrete-circuit` path deps | Build hygiene |

## Rule

### T — Transport (primary: [git-config `http.*`](https://git-scm.com/docs/git-config#Documentation/git-config.txt-httpproxy))

1. Trust anchors: `http.sslCAInfo` (config) and `GIT_SSL_CAINFO` (env, wins) name a PEM bundle used **in addition to** built-in web roots. `SSL_CERT_FILE` is honored as a fallback anchor file.
2. Proxy: `http.proxy` (config) then `HTTPS_PROXY` / `https_proxy` / `ALL_PROXY` env for `https://`; `NO_PROXY` / `no_proxy` host-suffix exclusions. HTTPS goes through HTTP `CONNECT`.
3. `url.<base>.insteadOf` rewrites apply before transport selection.
4. Same trust/proxy resolution is shared by `arks-remote` (smart/dumb HTTP) and `arks-hub` (forge API client).

### C — Config (primary: [git-config FILES + ENVIRONMENT](https://git-scm.com/docs/git-config#FILES))

1. Precedence low→high: system (`$(prefix)/etc/gitconfig`, honoring `GIT_CONFIG_NOSYSTEM`), global (`$XDG_CONFIG_HOME/git/config` then `~/.gitconfig`; `GIT_CONFIG_GLOBAL` overrides), repository `.git/config`, `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_<n>`/`GIT_CONFIG_VALUE_<n>`, command-line `-c`.
2. Identity: `GIT_AUTHOR_*` / `GIT_COMMITTER_*` env win over `user.name` / `user.email` (and `author.*` / `committer.*`) from merged config. A missing identity is an error, never a placeholder identity.

### S — Signing (primary: [gitformat-signature](https://git-scm.com/docs/gitformat-signature), [git-config `gpg.*`](https://git-scm.com/docs/git-config#Documentation/git-config.txt-gpgformat))

1. `commit.gpgsign=true` signs commits as `-S` would; `--no-gpg-sign` overrides.
2. `gpg.format=ssh` uses `gpg.ssh.program` (default `ssh-keygen`) with `-Y sign -n git -f <key>`; `user.signingKey` is a key path or a literal `key::` / `ssh-` public key.
3. Spawning the configured signing program is **not** a `git`/`gh` spawn (rule `30` unaffected).

### O — Corpus (option spellings agents use)

| Command | Spellings |
|---------|-----------|
| global | `-C <dir>`, `-c k=v`, `--git-dir`, `--work-tree`, `--no-pager` |
| `status` | `-s`, `-b`, `-sb`, `--short`, `--branch`, `--porcelain` |
| `diff` | `<rev>`, `<a> <b>`, `<a>..<b>`, `<a>...<b>`, `--stat`, `--name-only`, `--cached`/`--staged`, `-- <path>` |
| `log` | `--oneline`, `-n`/`-<n>`, `--format=`/`--pretty=format:` (`%H %h %s %an %ae %ad %b`), `<range>` |
| `show` | `--stat`, `--name-only`, `<rev>` |
| `branch` | `--show-current`, `-a`, `-r`, `-d`/`-D` |
| `remote` | `-v` |
| `commit` | bundled short options (`-am`), `--allow-empty`, `-S`, `--no-gpg-sign` |
| `checkout` | `-B <branch> [<start>]` |
| `merge` | `--no-edit`, `--ff-only`, `FETCH_HEAD` |
| `pull` | `--ff-only`, `--rebase` |
| `push` | `-u`/`--set-upstream`, `--dry-run` |

Bundled single-letter short options MUST be split parser-wide per [gitcli](https://git-scm.com/docs/gitcli).

### B — Build hygiene

Default workspace resolution MUST NOT require sibling trees. CELIR index stays opt-in, default OFF ([[sot:root/storage-celir-boundary]]).

### V — Verification

1. Claimed per area only with a Docker clean-room receipt whose fixture simulates the agent sandbox: TLS-intercepting proxy with a private CA, proxy-injected auth, global config with an SSH signing program, and `GIT_CONFIG_COUNT` env; peer = pinned stock git; stock `git fsck` + `git verify-commit` accept arks-written objects.
2. Host runs (including a live cloud-session probe) are **advisory** only.

## Why

Agent sessions are the primary multi-session contention users of arks. A drop-in that fails
behind the sandbox proxy or writes commits with a placeholder identity is not usable,
even when the behavioral DoD is met in a plain network.

## Acceptance

1. Each area T/C/S/O/B moves Missing → Partial (host advisory) → Claimed (Docker receipt) only via evidence.
2. Register row in [[sot:root/production-residual-register]] tracks state; no invented PASS.
