---
id: research:root/20261113194123
title: Behavioral DoD renew — git porcelain / daemon / gh peer parity (not Missing=0)
status: active
created: 2026-11-13T10:41:23Z
updated: 2026-11-13T10:41:23Z
tags: [research, dod, behavioral, symmetry, porcelain, git-2.39.5, daemon, gh, no-invent-claimed]
verify: manual
rel:
  depends_on:
    - sot:root/verify-cleanroom-symmetry-matrix
    - sot:root/arks-dod-optimized
    - sot:root/cli-git-help-matrix
    - sot:root/arks-cli-output-exit-contract
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-daemon-git-command-matrix
    - sot:root/arks-hub-gh-command-matrix
    - sot:root/production-residual-register
    - research:root/20261113143029
  related:
    - sot:root/arks-dod-behavioral
---

# Behavioral DoD renew — research receipt (2026-11-13)

English-only. Tip surveyed: **`e9704e3`**. Design ≠ Claimed until owning SoT + Docker receipts.

## Owner correction

Prior optimized DoD ([[sot:root/arks-dod-optimized]] / [[research:root/20261113143029]]) treated **Missing=0** (+ bounded OD-* smokes) as complete and cut **Partial×117** / hub **terminal Partial×6** out of DoD. Owner rejected that bar:

> CLI와 **동작까지** 대칭되어야지 — stubs do not count. Re-canonize DoD and continue.

**Law:** inventory Missing=0 alone ≠ complete. Stub / exit-2 / help-only for a DoD-IN command = **DoD gap**.

## Web / peer contracts (pin Git **2.39.5**)

Sources fetched 2026-11-13:

| Peer surface | Source | Contract used for DoD |
|--------------|--------|------------------------|
| Everyday porcelain set | [giteveryday](https://git-scm.com/docs/giteveryday.html) (stable across 2.28–2.56; applies to pin 2.39.5) | Bounded “~20 cmds” standalone + participant remotes — **not** every `list-cmds` Partial |
| Status / porcelain stdout | [git-status](https://git-scm.com/docs/git-status.html) (2.39.1–2.39.5 unchanged) | Human long form + `--porcelain`/`=v1` script form; color/relativePaths ignored in porcelain |
| Exit / stream classes | [api-error-handling](https://git-scm.com/docs/api-error-handling) + porcelain man pages | Success **0**; ordinary failure **1**; usage/fatal often **128**/usage class; `diff` may use **1** = differences when Claimed; diagnostics → stderr; payload → stdout |
| User manual commit loop | [user-manual 2.39.0](https://git-scm.com/docs/user-manual/2.39.0.html) | `add` → index; `diff`/`diff --cached`; `commit`; `status` summarizes |
| git-daemon flags | [git-daemon](https://git-scm.com/docs/git-daemon.html) (+ daemon.c flag table) | Wire/archive behavior changers: `--export-all`, `--listen`/`--port`, `--base-path`, `--strict-paths`, `--reuseaddr`, `--enable`/`--disable` (`upload-pack` default on; `upload-archive`/`receive-pack` default off), timeouts, `--max-connections`, `--verbose`, directory args |

**Hub / gh:** surveyed pin `gh` **2.103.3** leaves already in [[sot:root/arks-hub-gh-command-matrix]]. Claimed rows must be **real REST/metadata paths** (not clap-accept stubs). Terminal Partial×6 that cannot graduate MUST become explicit **Refuse** with reason — invent stub-OK as DoD is forbidden.

## What “behavioral parity” means here

For personality **`git`** (CI drop-in) on a named porcelain leaf:

1. **Action:** the command performs the peer-comparable work (index/object/ref/worktree/remote wire as applicable) — not merely `--help` registration.
2. **Exit class:** matches pin git success/failure **class** for the SoT scenario ([[sot:root/arks-cli-output-exit-contract]] `ExitGitFaithful`).
3. **Stdout/stderr class:** peer-shaped payload vs diagnostics; porcelain/script forms stable where SoT asserts `--porcelain` / machine flags.
4. **Byte identity:** **not** required unless a clean-room gate asserts `cmp` / exact bytes.
5. **Product MUST NOT spawn `git`/`gh`.** Verify MAY invoke pin peers as oracles under Docker.

Personality **`gh`:** same for Claimed hub leaves (peer-shaped stdout + exit). Personality **`arks`:** unified overlap honesty remains OD-UNIFIED (carrying); not a substitute for git drop-in parity.

## Bounded porcelain set (DoD-IN) — justify bounds

**IN (named BP set)** — everyday workflows from giteveryday + owner list:

| Bucket | Commands |
|--------|----------|
| Local core | `init`, `status`, `add`, `commit`, `log`, `show`, `diff`, `branch`, `checkout`, `switch`, `restore`, `reset`, `rm`, `mv`, `merge`, `rebase`, `tag`, `stash` |
| Remotes | `clone`, `fetch`, `pull`, `push`, `remote` (list / add / remove\|rm / show / get-url / set-url) |

**OUT of behavioral DoD (still honest inventory):**

- Full CLI survey Partial remainder / exotic man-page flag depth (**≠** BP core workflows).
- GUI / interactive TUI / mergetool / gitk ([[sot:root/cli-git-help-matrix]] Refuse).
- Hub `phase=later` Deferred×119; hub OUT Refuse groups.
- Daemon SysV double-fork, privilege-drop, inetd, and other documented Refuse flags.
- Impossible / L9 / live G8 latency SLO.
- Agent fourth pillar.

**Bounds rationale:** peer “everyday Git” + remotes that scripts/`alias git=arks` need; cutting thrash while restoring **action** symmetry. Partial×117 stampede remains non-DoD.

## Tip honesty probe (host, advisory)

At tip `e9704e3` host debug binary: `init`/`add`/`commit`/`status`/`log`/`tag`/`stash list`/`remote` (list) perform real work under `ARKS_PERSONALITY=git`. **Gaps observed:** `remote show`, `remote get-url` → `NotImplemented` (stub) — DoD-IN under BP remotes. Broader Docker STRICT peer-compare still required for Claimed.

## Daemon / hub carry

- Daemon living CSV **Parity=20** / Missing=0 — peer-mapped wire/archive flags already Parity-carrying stay IN; Refuse rows stay OUT.
- Hub v1 Claimed×94 must remain **real**; terminal Partial×6 → **graduate with evidence** or **Refuse** (Spec amend + CSV).

## Supersession

[[sot:root/arks-dod-optimized]] “DoD met” / Missing=0 complete story is **superseded** for public “behavioral DoD met.” Optimized IN rows (P1–P3 Missing=0 floors, R-HOST, R-B, OD-*) remain **necessary floors**, not sufficient. Normative renew: [[sot:root/arks-dod-behavioral]].
