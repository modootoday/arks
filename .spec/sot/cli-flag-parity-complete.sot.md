---
id: sot:root/cli-flag-parity-complete
title: CLI flag-parity complete symmetry
status: active
created: 2026-11-09T04:39:10Z
updated: 2026-11-10T01:17:00Z
tags: [m16, cli, flag-parity, complete-symmetry, alias, wave-sy, g21, no-refuse, credential, g33, m25]
verify: manual
rel:
  depends_on:
    - sot:root/cli-surface
    - sot:root/cli-flag-parity-full
    - sot:root/cli-flag-parity-m9
    - sot:root/arks
    - sot:root/arks-remote
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/lease-concurrency-intent
    - sot:root/storage-celir-boundary
    - sot:root/remote-host-compatibility-surface
    - sot:root/credential-helper-askpass
    - plan:root/20261109043910
    - plan:root/20261109035335
  governs: [crates/arks, crates/arks-remote]
  evidence:
    - evidence:root/20261109044908
    - evidence:root/20261109045705
    - evidence:root/20261109050238
    - evidence:root/20261109054500
    - evidence:root/20261109061600
---

# CLI flag-parity complete symmetry (M16 / Phase Sy)

Version **0.1.1**. **Active** alias/`git=arks` honesty ledger after M15. Historical ledgers stay frozen: [[sot:root/cli-flag-parity-m9]] (M9), [[sot:root/cli-flag-parity-full]] (M15 inventory close). This SoT **supersedes them for production honesty**. Server transport N/A rows below must not contradict [[sot:root/arks-server]] **M25 G29–G32 Claimed**.

Primary inventory: [git(1) 2.39.0](https://git-scm.com/docs/git/2.39.0) porcelain + ancillary + plumbing required for scripting/alias muscle memory. Pin: clean-room Git **2.39.x**. Per-command options: matching `git-<cmd>(1)` for the pin.

## Rule

### Production classification (normative — no leftover refuse)

1. Every pin-Git CLI **command** in the inventory MUST be classified as either:
   - **implement** — claimed forms/flags behave with Git semantics vs pin on the same `.git` layout; or
   - **N/A exclusion** — documented product exception with explicit rationale in §Product N/A exclusions.
2. Every **documented option** on an **implement** command for the pin MUST be either **implement** or listed under that command’s **N/A** column. Silent ignore is forbidden. Production **refuse** / **partial** / “Open until later residual” rows are **forbidden** on the active alias surface.
3. Historical M9/M15 refuse rows remain valid as **history** of those exits; they MUST NOT be cited as the active production bar.
4. Unknown options (not on the pin man page) still exit non-zero (parse honesty) — that is not a production “refuse leftover.”
5. Hooks: claimed porcelain that runs hooks under stock Git MUST run **sync** hook scripts from `$GIT_DIR/hooks`. No L3 async hook scheduler. `--no-verify` only when the pin documents it.
6. Engine: zero `gix`; CELIR out of product deps; lease ≠ skip Git locks ([[sot:root/lease-concurrency-intent]]).

### Product N/A exclusions (only allowed leftovers)

| Exclusion | Rationale | Cite |
|-----------|-----------|------|
| GUI / browser (`gitk`, `git-gui`, `git-citool`, `gitweb`, `instaweb`) | Not CLI replacement surface | git(1); [[sot:root/cli-surface]] |
| `scalar` | Separate large-repo suite | git(1) |
| Git LFS | Separate project | product non-goal |
| Forge PR / Checks / Apps UX | Remote-host scope (B) | [[sot:root/remote-host-compatibility-surface]] |
| CELIR admin UX | Cancelled / non-product | [[sot:root/storage-celir-boundary]] |
| Lease admin / L3 N-way valves | Product lease SoT; L3 refuse-closed until superseding ADR | [[sot:root/lease-concurrency-intent]], [[adr:root/20261108232800]] |
| Forge OAuth UX + Git Credential Manager as bundled first-party binary | External GCM/OAuth helpers MAY still be configured by path/name; product ships first-party `wincred`/`libsecret`/`osxkeychain` + TTY (G33) | [[sot:root/credential-helper-askpass]] |
| Product SSHD / key CA (arks as sshd) | Ops wrap ForceCommand / `git-shell`; arks speaks pack protocol on stdio only | [[sot:root/arks-server]] |
| Dumb-HTTP-**only** as sole claimed server path | Forbidden — smart HTTP remains required; dumb is additive (**G29 Claimed**) | [[sot:root/arks-server]], [[evidence:root/20261109221500]] |
| Full curses chrome / `add -i` menu / `add -e` / `stash -p` / mergetool **GUI** identity | FS3+ / FS7 under [[research:root/20261110212414]]; **FS1 Claimed**: TTY `add -p` prompts + `rebase -i` editor fallback (not a lasting agent-N/A) | gitcli(7); [[adr:root/20261110212430]], [[adr:root/20261110210300]] |
| GPG/SSH **signing** beyond claimed SSH commit subset (`tag -s`/`-u`; `verify-tag`; OpenPGP/`gpg.format=openpgp` + X.509 `commit -S` / `verify-commit`) | External crypto UX outside claimed `gpg.format=ssh` commit create/verify ([[sot:root/arks-object]] FW-GPGSIG) | git-tag / git-commit / git-verify-commit / gitformat-signature |
| Foreign SCM bridges (`svn`, `p4`, `cvs*`, `archimport`, `quiltimport`, `imap-send`, `send-email` SMTP identity) | Not Git core object/CLI symmetry | git(1) interacting-with-others |
| `filter-branch` | Successor honesty — external history rewrite | git-filter-branch; M15 exclude |
| `backfill` / partial-clone promisor depth product | Not claimed without dedicated SoT | git-compat / G9 |
| `gc --prune=now` | CORR-001 / PACK-004 writer-race honesty — use `repack -ad` / `gc` without prune=now | [[sot:root/lease-concurrency-intent]] |
| Byte-identical stderr / advice text | Unless a gate asserts it | [[sot:root/cli-surface]] |
| Hardlink/`--shared` local clone alternates | Optional later; local path/`file://` object-dir copy remains | [[sot:root/arks-remote]] |

**Client SSH** (`git@` / `ssh://` for clone/fetch/push/submodule add) is **implement** under M16 Sy-4 — not an exclusion. **Server** transports Claimed under M25: smart HTTP (G15), dumb HTTP additive (**G29**), SSH command-mode stdio (**G30**), protocol v2 advertise (**G31**), `git://` daemon (**G32**) — [[sot:root/arks-server]]; not N/A.

### Wave Sy claim ledgers

Wave Done = Docker **G21** PASS for that wave’s matrix IDs + G14/G20 regression green + evidence. Plan: [[plan:root/20261109043910]].

#### Sy-1 — Everyday porcelain flag closure

| Command | Implement (claimed) | N/A on this command |
|---------|---------------------|---------------------|
| `status` | human; `-s`/`--porcelain` (v1); **`-b`** (branch header / porcelain `##`) | porcelain v2 full schema extras beyond claimed |
| `log` / `show` | default; **`--oneline`**; **`--graph`** (ASCII); `-n`/`-<n>`; path limiters as already claimed | `--pretty=format:` full custom; pager identity |
| `branch` | existing + **`-a`/`--all`**; **`-r`/`--remotes`**; `-v` short | `--sort` exotic; `--format` full |
| `add` | paths; `-A`/`-u`; **`--chmod=+x`/`--chmod=-x`**; **`-p`/`--patch`** file-granularity (**TTY prompts** + `ARKS_ADD_PATCH_ANSWERS` / non-TTY stdin; FS1) | `-i` menu; `-e` hunk edit; hunk-level `-p` (FS3) |
| `clean` | `-f` `-d`; **`-x`** (ignore ignore rules) | `-e` exclude pathspec extras beyond pin subset |
| `describe` | `--tags`; **`--dirty[=<mark>]`** | `--match` globs beyond claimed |
| `switch` | `-c`/`-C`; **`--detach`** | `--orphan` (→ Sy-2 with checkout) |
| `checkout` | existing; **`--detach`**; **`--orphan`** (Sy-2 may finish) | — |
| `config` | `--get`/`--set`/`--list`/`--unset`; **`--local`**; **`--global`** (home file); **`--system`** when path resolvable | `--worktree` multi-wt edge beyond claimed |
| `reset` | soft/mixed/hard; **`--keep`** | `--merge` conflicted restore edge if unclaimed → Sy-5 |
| `diff` | `--cached`/`--staged`; **`--stat`**; **`--name-only`**/`--name-status` | `--word-diff` / external difftool GUI (**N/A**) |
| `rm`/`mv` | existing + **`-f`** force claimed edges | — |
| `tag` | `-d` `-a` `-m` `-l`; lightweight | `-s`/`-u` (**global GPG N/A**) |
| `grep` | pattern; `-n`; `-i` | `-P` PCRE (**N/A** without PCRE dep claim) |
| `init` | path; **`--bare`** | template/shared exotic |
| `hash-object` | `-w`; **`-t` blob/tree/commit/tag** | `--stdin-paths` batch extras → Sy-3 |
| `cat-file` | `-t` `-s` `-p`; **`--batch-check`** minimal | textconv filters |
| `update-ref` | `-d`; **`--stdin`** line protocol subset | — |
| `rev-parse` | revs; **`--verify`**; **`--symbolic-full-name`**; **`--abbrev-ref`** | `--parseopt` shell helper |

#### Sy-2 — Worktree porcelain + submodule recursive

| Command | Implement | N/A |
|---------|-----------|-----|
| `worktree` | existing + **`list --porcelain`**; **`lock`/`unlock`**; **`add --detach`**; `-v` | main-worktree remove |
| `submodule` | existing + **`update --recursive`**; **`status --recursive`**; **`--init --recursive`**; **`update --remote`** (tracking branch tip when configured) | custom `!command` update; absorbgitdirs exotic |
| `checkout`/`switch` | finish **`--orphan`** | — |

#### Sy-3 — Plumbing breadth (scripting)

| Command | Implement | N/A |
|---------|-----------|-----|
| `update-index` | `--add`/`--remove`/`--cacheinfo`; `--refresh` | `--stdin` full + chmod skip-worktree edges beyond claimed |
| `read-tree` | single tree into index; `-m` two-tree merge-index baseline | 3-tree full merge-index |
| `diff-tree` | two trees; `--name-status` | recursive combine |
| `for-each-ref` | pattern; `--format=%(refname)` minimal | color/atom full set |
| `rev-list` | commit list; `--count`; `--max-count` | `--objects` full graph extras beyond claimed |
| `show-ref` | heads/tags; `--heads`/`--tags`/`--verify` | — |
| `pack-objects` / `index-pack` | stdin OID list → pack+idx; index existing pack | thin-pack delta reuse exotic |
| `ls-files` | tracked; **`-c`/`-o`/`-d`/`-m`/`-u`** claimed | `--debug` |
| `ls-tree` | tree-ish; **`-r`**; **`--name-only`** | — |
| `merge-base` | two commits; **`--all`** | octopus >2 beyond claimed |
| `check-ignore` | paths → ignored? | non-zero exit classes per pin |
| `name-rev` | OID → describe-ish name | — |

#### Sy-4 — Client SSH transport (as claimed)

| Surface | Implement | N/A |
|---------|-----------|-----|
| `clone`/`fetch`/`push` URL `ssh://` and `git@host:path` | OpenSSH `ssh` subprocess speaking upload-pack/receive-pack pkt-line (no product-path `git` binary) | Product SSHD/key CA (**global N/A** above); server command-mode stdio is **Claimed G30** — [[sot:root/arks-server]] |
| `submodule add` over SSH URL | Same client SSH | — |
| Auth (SSH) | `ssh` agent / key files via OpenSSH defaults | — |
| Auth (HTTPS) + `credential` plumbing | URL userinfo; `ARKS_REMOTE_USERNAME`/`PASSWORD`; `credential.helper` get/store/erase incl. first-party `wincred`/`libsecret`/`osxkeychain`; `GIT_ASKPASS`/`core.askPass`; TTY when allowed; `arks credential fill\|approve\|reject` | Forge OAuth UX / bundled GCM (**global N/A** above); Gate **G33** |

Primary: [git-clone](https://git-scm.com/docs/git-clone), [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack); OpenSSH as transport helper (not gix).

#### Sy-5 — Remaining depth + ancillary interrogators

| Command / form | Implement | N/A |
|----------------|-----------|-----|
| `stash` | push/list/pop + **`show`/`apply`/`drop`/`clear`** | `-p` interactive |
| `cherry-pick` / `revert` | single + **`A..B` ranges** (clean applies); **`--no-commit`** | conflicted continue/abort sequencer full → must implement or N/A with reason; prefer implement clean range + refuse only true conflict needing editor |
| `bisect` | start/bad/good/reset + **`run <cmd>`** | `visualize` GUI (**N/A**) |
| `notes` | add/show + **`append`/`remove`/`list`** | `merge`/`edit` interactive |
| `sparse-checkout` | cone + **`disable`**; **`reapply`** | non-cone full |
| `range-diff` | claimed + **symmetric `A...B`** form | merge-commit dual diff full emit |
| `format-patch` | existing + **`--cover-letter`**; `-o` | binary patch exotic |
| `am` | patch file + **`--3way`** when blobs resolvable | interactive |
| `blame` / `annotate` | path line attribution (minimal) | `-L` range exotic → implement basic |
| `fsck` | connectivity check claimed subset | full unreachable progress UI |
| `count-objects` | `-v` | — |
| `pack-refs` | `--all` | — |
| `version` / `help` | version string; help list honesty | man-page viewer identity |
| `merge` | FF / `--no-ff` / diverged + **`-m`**; strategy **`ort`/`recursive` default only** | octopus; `resolve`/`octopus`/`ours`/`subtree` strategies **N/A** until claimed |
| `pull` | FF / rebase / three-way as claimed; HTTPS creds via Claimed helper/askpass/TTY/wallet path | Forge OAuth browser UX |
| Detached `commit` / `commit --amend` | implement | — |
| Sequencer `cherry-pick`/`rebase` conflict | implement markers + non-zero; **`--continue`/`--abort`** when sequencer state present | — |

### Active inventory completeness

M16 exit requires: every main-porcelain + required ancillary/plumbing row from git(1) 2.39.0 is either covered by Sy-1..Sy-5 **implement** rows or appears in §Product N/A exclusions (or a command-local N/A column). No production “Still refuse / Open” columns remain after M16 exit.

## Why

M15 closed command inventory while leaving per-flag refuse gaps that break true `alias git=arks` honesty. Complete symmetry means users never hit a production “not claimed yet” on a pin-documented option unless the product explicitly N/A’d it.

## Verification

1. Clean-room **G21** (`verify/matrices/matrix-g21.md`) per Sy wave.
2. G14 + G20 regression green.
3. `spec check` clean.
4. Host `cargo test` advisory only.
5. Public alias docs cite this SoT as active bar; M9/M15 as historical.
