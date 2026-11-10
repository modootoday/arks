---
id: sot:root/arks
title: arks CLI Wave L+H+Pa+Pb+Pc
status: active
created: 2026-11-08T10:39:43Z
updated: 2026-11-10T09:45:00Z
tags: [m5, m6, m7, m8, m9, unit, cli, wave-l, wave-h, wave-pa, wave-pb, wave-pc, ring-7, phase-l, phase-h, phase-pa, phase-pb, phase-pc, production, alias]
verify: manual
rel:
  governs: [crates/arks]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090650
    - plan:root/20261108090651
    - plan:root/20261108183810
    - plan:root/20261108183811
    - plan:root/20261108183812
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-m9
    - sot:root/crate-architecture
    - sot:root/product-naming
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/remote-host-compatibility-surface
    - sot:root/public-surface
    - sot:root/arks-remote
    - sot:root/arks-object
    - sot:root/arks-store
    - sot:root/arks-refs
    - sot:root/arks-index
    - sot:root/arks-repo
    - sot:root/arks-pack
    - research:root/20261108071913
    - research:root/20261108071916
    - research:root/20261108183800
    - adr:root/20261108083507
    - evidence:root/20261108104837
    - evidence:root/20261108112000
    - evidence:root/20261108133000
    - evidence:root/20261108143000
  evidence: [evidence:root/20261108104837, evidence:root/20261108112000, evidence:root/20261108133000, evidence:root/20261108143000, evidence:root/20261108150000]
---

# arks CLI Wave L+H+Pa+Pb+Pc

Version **0.5.0**. Unit SoT for Ring 7 primary package + binary `arks` ([[sot:root/product-naming]]) delivering [[sot:root/cli-surface]] **Wave L** (M5), **Wave H** (M6), **Wave Pa** (M7), **Wave Pb** (M8), and **Wave Pc** (M9) under roadmap Phases L/H/Pa/Pb/Pc. **M9 / Wave Pc is the CLI-parity production claim tier** (same `.git` + inventory closure + flag-parity + alias-ready) when clean-room **G14** is green with a new evidence receipt.

**Entry (Wave Pc):** M8 exit proven — [[evidence:root/20261108143000]]. Frozen checklist: [[plan:root/20261108183812]]. Matrix: `verify/matrices/matrix-m009-clean-room-cli-parity.md`. Flag-parity ledger: [[sot:root/cli-flag-parity-m9]]. Alias how-to: `docs/alias-git-arks.md`. Remote client lib: [[sot:root/arks-remote]].

## Rule

### Role

1. **Thin CLI compose.** `arks` wraps proven libraries; MUST NOT invent object/ref/index/wire semantics ([[sot:root/crate-architecture]] Ring 7; [[adr:root/20261108083507]]). Prefer helpers inside `crates/arks` over new crates unless dependency direction requires a split.
2. **Binary name `arks`.** No `git` substring in package/bin names ([[sot:root/product-naming]]). Drop-in is via user `alias git=arks` / PATH shim ([[sot:root/cli-surface]]).
3. **Help honesty.** `arks --help` lists **only** implemented Wave L+H+Pa+Pb+Pc (+ residual) commands for this claim tier. Refuse-closed rows (`repack`, `sparse-checkout`, `maintenance`, `backfill`, `range-diff`, `notes`, `filter-branch`) and unimplemented options **refuse** (non-zero; no repo mutation). Residual implement: `worktree` / `submodule` per [[plan:root/20261109022956]] ([[sot:root/cli-surface]]).
4. **No lease / CELIR / server** in M9 product paths. Remotes remain **client** only ([[sot:root/arks-remote]]). No L1+ lease runtime (**M10**). No Git server (**M11**).

### Wave L command → library map (retained)

| Command | Owning libs / APIs (compose) | Claimed subset | Refuse |
|---------|------------------------------|----------------|--------|
| `init` | `arks-repo` layout | Non-bare `.git/` + `HEAD` → `main` | Bare/`--template`/shared/`--separate-git-dir` |
| `hash-object` | object/store/hash/oid | `-w` blob; stdin or path; SHA-1 | other `-t`; filters |
| `cat-file` | store/object/repo | `-t` / `-s` / `-p` | Batch; `--textconv` |
| `update-ref` | `arks-refs` | Create/update/delete + optional old | `--stdin` batch; reftable |
| `rev-parse` | refs/repo | `HEAD`, full refnames, full OID hex | Peel chains; `--parseopt` |
| `add` | object/index/repo | Stage file paths into index v2 | `--all`/`-A`/`-u` |
| `status` | index/repo/worktree | Short staged/unstaged/untracked; `--no-optional-locks` / agent profile (never takes optional index.lock) | porcelain v2 full |
| `lock` | git_dir/common_dir `*.lock` | M12 Cm-1: `inspect` / `clear --stale` (refuse live PID; lease ≠ skip locks) | auto-evict / dialect invention |
| `commit` | object/store/index/refs + `ssh-keygen -Y` (SSH sign) | Tree+commit; `-m` required; append reflog; **`-S`/`--gpg-sign`** / `commit.gpgSign` when `gpg.format=ssh` (FW-GPGSIG) | OpenPGP/X.509 `-S`; empty without `--allow-empty` |
| `verify-commit` | object + `ssh-keygen -Y verify` | Cryptographic verify for SSH `gpgsig` commits ([git-verify-commit](https://git-scm.com/docs/git-verify-commit)) | OpenPGP/X.509; `verify-tag` |
| `log` | object/refs | First-parent from HEAD | `--graph`; full decorate |
| `show` | object/repo | Commit summary or blob pretty | Diff patch beyond Pa deepen |

### Wave H command → library map (retained)

| Command | Owning libs / APIs | Claimed subset | Refuse |
|---------|--------------------|----------------|--------|
| `remote` | `arks-remote` config | `add` / list / `remove` | SSH-only workflows; unlisted subcommands |
| `fetch` | `arks-remote` | Smart HTTP/SSH fetch; optional `--depth N` (G9/G35) | unclaimed filters / promisor |
| `push` | `arks-remote` | Smart HTTP push of claimed ref updates | SSH; unlisted force options |
| `clone` | `arks-remote` + checkout | Smart HTTP/SSH/local clone; optional `--depth N` on pack transports | bare; local `--depth`; filter/promisor |

### Wave Pa command → library map (retained; M7)

| Command | Owning libs / APIs | Claimed subset (M7→Pc) | Refuse |
|---------|--------------------|------------------------|--------|
| `branch` | `arks-refs` | List/create/delete `-d`/`-D`/rename `-m` | `-a`/`-r`; upstream extras |
| `checkout` | refs/index/object/store + checkout helper | Switch; `-b`; path restore from HEAD; reflog append | `--orphan`; unclaimed |
| `switch` | same | Switch; `-c`/`-C`; reflog append | `--detach` |
| `restore` | index/object | Paths WT and/or `--staged` | `--overlay` |
| `reset` | refs/index/checkout | `--soft`/`--mixed`/`--hard`; reflog append | `--keep`/`--merge` |
| `diff` | index/object | WT↔index; `--cached`; commit pair unified text | binary/word-diff |
| `merge` | refs/object | Fast-forward; `--ff-only`; reflog append | Non-FF; `--no-ff`; other strategies |
| `rm` / `mv` / `clean` | index/worktree | `rm`/`--cached`; `mv`; `clean -f` files; **Pc:** `clean -d` | `-x` ignored |
| `tag` | refs/object/store | Lightweight create/list/`-d`; **Pc:** annotated `-a -m` | Signed `-s`/`-u` |
| `describe` / `shortlog` | refs/object | Nearest tag; author shortlog first-parent | `--dirty`; unclaimed |
| `config` | local `.git/config` parse/write | `--get`/`--set`/`--list`/`--unset` | `--global`/`--system` |
| `grep` | worktree/index | Basic/fixed pattern on tracked paths | `-P` |
| `stash` | objects/refs/index | `push`/`list`/`pop` minimal | Other subcommands |
| `pull` (FF) | `arks-remote` + merge | fetch + merge `--ff-only` | Non-FF merge pull |

### Wave Pb command → library map (retained; M8)

| Command | Owning libs / APIs | Claimed subset (M8→Pc) | Refuse |
|---------|--------------------|------------------------|--------|
| `cherry-pick` | object/refs/index + checkout + tree apply | Single commit; clean path apply; reflog | Ranges; conflicted apply |
| `revert` | same | Single commit; clean inverse; reflog | Ranges; `-n`; conflicted |
| `rebase` | same | Non-interactive onto `<upstream>` linear; reflog | `-i`; conflicted |
| `format-patch` | object + unified diff emit | `-1` / `A..B`; `-o <dir>` | `--cover-letter`; binary |
| `am` | patch parse + tree apply + commit | One `.patch`/mbox from claimed format-patch | `--3way`; interactive |
| `bisect` | refs + first-parent walk | `start` / `bad` / `good` / `reset` | `run`/`skip`/`visualize` |
| `bundle` | refs + `arks-pack` | create/`list-heads`/unbundle; **Pc:** `verify` | Thin-pack edges |
| `pull --rebase` | remote + rebase | fetch + claimed rebase onto upstream | Other pull strategies |

Apply rule: path-level clean apply only (HEAD blob must match base); else refuse non-zero.

### Wave Pc command → library map (M9 — production tier)

| Command | Owning libs / APIs | Claimed subset (M9) | Refuse |
|---------|--------------------|---------------------|--------|
| `archive` | object/store + ustar emit | `--format=tar` (default); `-o <file>`; tree-ish default HEAD | `zip`; `--remote`; `tar.gz` unless claimed |
| `reflog` | filesystem `.git/logs` | `reflog` / `reflog show [<ref>]`; append on claimed HEAD/ref moves | `expire` / `delete` / `exists` |
| `tag` deepen | object (`tag` kind) + refs | Annotated `-a -m <msg> <name> [<commit>]` | `-s` / `-u` |
| `bundle` deepen | pack + bundle header | `bundle verify <file>` | Unclaimed thin edges |
| `clean` deepen | worktree walk | `-d` untracked directories | `-x` |
| `worktree` | `arks-repo` common-dir + checkout | Residual: `add`/`list`/`remove`/`prune`/`move` ([[plan:root/20261109022956]]) | `--porcelain`; `lock`/`unlock`; `--detach`/`--orphan` |
| `submodule` | gitlink + `.gitmodules` + local clone | Residual: `add`/`init`/`update`/`status`/`deinit` | `--recursive`; `--remote`; non-local URL add |
| `repack` | store + pack + `maint:gc` | **Historical M9 refuse** — **Claimed M15** Fs-3 (`-a`/`-ad`) | `--window` / geometric / cruft |
| `sparse-checkout` / `maintenance` | index + worktree / maint path | **Historical M9 refuse** — **Claimed M15** (cone sparse; `maintenance run`) | non-cone extras; `backfill` still **exclude** |
| `range-diff` / `notes` | object/refs | **Historical M9 refuse** — **Claimed M15** Fs-5 / notes subset | `filter-branch` still **exclude** |

**Current** command/flag authority after M9: [[sot:root/cli-flag-parity-full]] (M15) + [[sot:root/cli-flag-parity-complete]] (M16). Do not treat the historical M9 refuse column above as tip honesty.

Flag-parity rows for every inventory command: [[sot:root/cli-flag-parity-m9]] (historical) / M15–M16 ledgers. Alias drop-in: `docs/alias-git-arks.md` + G14 shim tests.

### Out of scope

- Git server plumbing — **M11**.
- CELIR; L1+ lease APIs — **M10**.
- Permanent `gix`; forge APIs (B); GUI / LFS / scalar.
- Calling M10/M11 “CLI-parity production.”

## Why

Wave Pc closes remaining inventory with honest implement/refuse classification, flag-parity audit, and alias-ready workflows on the same `.git`, unlocking **CLI-parity production** at M9 when G14 is clean-room green.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks` and `cargo run -p arks -- --help`.
2. Clean-room **G14** + G13/G12/G7/G6 regression via `verify/matrices/matrix-m009-clean-room-cli-parity.md` — authoritative for M9 / CLI-parity production. Prior M8 receipt [[evidence:root/20261108143000]] does not alone carry M9.
3. `spec check` clean for this SoT.
4. Help lists Wave L+H+Pa+Pb+Pc claimed commands; refuse-closed commands refuse and are omitted from help; refuse under alias stays non-zero.
5. Workspace `cargo metadata`: inward deps only; zero `gix`/gitoxide; zero `celir`.
