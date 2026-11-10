---
id: sot:root/cli-surface
title: CLI surface
status: active
created: 2026-11-08T08:21:48Z
updated: 2026-11-10T20:15:00Z
tags: [cli, porcelain, ux, arks, parity, git-cli, alias, drop-in, server, worktree, submodule, m15, m16, full-symmetric, complete-symmetry, credential, personality]
verify: manual
rel:
  depends_on:
    - sot:root/product-naming
    - sot:root/git-compat-surface
    - sot:root/product-intent
    - sot:root/crate-architecture
    - sot:root/core-object-model
    - sot:root/product-verification-clean-room
    - sot:root/remote-host-compatibility-surface
    - sot:root/interop-test-matrix
    - sot:root/program-roadmap
    - sot:root/cli-flag-parity-m9
    - sot:root/cli-flag-parity-full
    - sot:root/cli-flag-parity-complete
    - sot:root/arks-cli-personality-factory
    - sot:root/arks-cli-output-exit-contract
    - plan:root/20261109035335
    - plan:root/20261109043910
    - plan:root/20261109043916
    - sot:root/clean-room-git-pin-advance
    - sot:root/production-residual-register
    - plan:root/20261108082148
    - plan:root/20261108082110
    - plan:root/20261108183812
    - plan:root/20261109022956
    - research:root/20261108183800
  evidence:
    - research:root/20261108071859
---

# CLI surface

Version **0.5.0**. Normative **porcelain / plumbing CLI intent** for the `arks` binary ([[sot:root/product-naming]]). On-disk and wire semantics remain owned by [[sot:root/git-compat-surface]] and domain research — this SoT defines **Git CLI symmetric compatibility**, **`git`→`arks` alias drop-in**, how command/flag parity is judged, refuse-vs-implement rules, version pinning, phased delivery waves (including **Wave Sv** server plumbing, **Wave Fs** M15 inventory residual, and **Wave Sy** M16 complete flag symmetry), and honesty constraints.

**End-state (normative):** `arks` is a **symmetric replacement** for the stock `git` CLI for the **full** git(1) inventory minus explicit **N/A exclusions** — same subcommand names, documented options/flags for the **clean-room-pinned Git version**, interchangeable repository/worktree results on the **same `.git` layout**, and **argv suitability for shell alias substitution** (e.g. `alias git=arks`) — implemented **in-engine** (**never** spawn `git` or `gh`). **M9** closed an implement/refuse ledger (historical). **M15** closed inventory-level full symmetric residual (historical; still allowed per-flag refuse). **Active alias honesty** is **M16 / Wave Sy** — every pin command/flag is **implement** or **N/A exclusion**; **no production partial/refuse leftovers** ([[sot:root/program-roadmap]], [[plan:root/20261109043910]], [[sot:root/cli-flag-parity-complete]]). Everyday/partial/refuse subset MUST NOT be treated as the lasting production bar.

## Rule

### Identity and invocation

1. **Binary name is `arks`.** Do not ship a bin whose name contains the substring `git` ([[sot:root/product-naming]]).
2. **Porcelain composes plumbing.** CLI commands MUST wrap library crates from [[sot:root/crate-architecture]]; they MUST NOT invent on-disk or wire semantics. Format/protocol fidelity is owned by [[sot:root/git-compat-surface]].
3. **Claims must be explicit.** Docs, `--help`, public narrative, and tests MUST name the **currently claimed** command/flag subset. Advertising a Git-compatible command implies the corresponding git-compat rows **and** the man-page option contract for the pin (below).
4. **Exit and messaging.** Non-zero exit on failure. Prefer clear refuse messages for unimplemented surface. Exact stderr string identity with stock Git is **not** required unless a clean-room gate explicitly asserts it; on-disk / exit-code / documented behavioral outcomes **are** required for claimed parity.
5. **Forge UX is not part of git alias DoD.** GitHub PR/Checks/Apps UX is remote-host scope (B), exposed as **flat forge leaves** (and transitional **`arks hub …`**) on the **same** published binary ([[sot:root/arks-hub-product-intent]] / [[sot:root/arks-hub-cli-surface]] / [[sot:root/arks-hub-gh-command-matrix]] — internal `arks-hub` lib; native API; never `gh`/`git` subprocess; **no** separate hub release). **LLM review / CEV bots** are likewise **not** part of git alias DoD — exposed as **`arks agent …`** ([[sot:root/arks-agent-product-intent]] / [[sot:root/arks-agent-cli-surface]]; internal `arks-agent`; never `gh`/`git` subprocess; **no** separate agent release). **Personality dispatch**, overlap unified I/O/exit, and CommandLayer factory: [[sot:root/arks-cli-personality-factory]] (**design / Partial** — not Claimed). Normative stdout/stderr/exit **classes**: [[sot:root/arks-cli-output-exit-contract]]. CI drop-in SHOULD use personality `git` / `gh`; bare `arks` unified overlap is not the CI drop-in path unless single-peer flags are set.

### Git CLI symmetric compatibility (definition)

Primary command taxonomy: [git(1)](https://git-scm.com/docs/git) — porcelain vs plumbing; command-line conventions: [gitcli(7)](https://git-scm.com/docs/gitcli); glossary: [gitglossary(7)](https://git-scm.com/docs/gitglossary) (*plumbing* / *porcelain*). Per-command option contracts: the matching `git-<cmd>(1)` page for the **pinned** Git version in the clean-room image ([[sot:root/product-verification-clean-room]]). Catalog entry points: [[research:root/20261108071859]]; CLI-parity research note: [[research:root/20261108183800]].

| Term | Normative meaning |
|------|-------------------|
| **Symmetric CLI** | For every command in the **parity inventory** (below), `arks <cmd>` exists; documented options for the pin are either **implemented with Git semantics** or **N/A exclusion** (active M16 bar) — historical M9/M15 may still document refuse rows as history; repository/worktree/ref/index/object outcomes match stock Git for the same inputs on the claimed surface |
| **Alias drop-in** | A user who previously invoked stock `git` can configure a shell (or equivalent) alias such as `alias git=arks` / `alias git=arks.exe` (or a documented PATH wrapper with the same argv contract) and continue muscle-memory workflows: same subcommand names, comparable exit codes for success/failure classes, and on-disk results on the same `.git` layout. Alias drop-in is a **first-class product requirement**, not a docs afterthought |
| **Parity inventory** | The clean-room-pinned Git’s **main porcelain** list from git(1), plus **ancillary** and **plumbing** commands required by the inventory tables in this SoT (and amendments), minus **explicit exclusions** |
| **Flag / option parity** | For each claimed command, every option documented on that command’s man page for the pin is classified: **implement** or **N/A exclusion** (active M16). Historical M9/M15 **refuse** rows are frozen history only. Silent ignore of a documented option is forbidden |
| **Version pin** | Parity is judged against the **in-image stock Git version** locked with the clean-room — not against the developer host’s `git`, and not against an unpinned “latest Git on the internet.” When the pin advances, a gap audit + plan amendment is required before claiming production still holds |
| **Peer oracle** | Stock `git` in clean-room is the behavioral/on-disk oracle; arks remains the system under test ([[adr:root/20261108083507]]) |

### Alias drop-in (`git` → `arks`) — normative

11. **Binary remains `arks`.** Product naming forbids shipping a bin whose name contains `git` ([[sot:root/product-naming]]). Drop-in is achieved by **user alias / PATH shim**, not by renaming the crate/bin to `git`.
12. **Argv contract under alias.** After `alias git=arks`, invocations of the form `git <subcommand> [args…]` MUST be equivalent to `arks <subcommand> [args…]` for the claimed inventory (global options that git(1) documents before the subcommand MUST be accepted or refused per the pin — not silently dropped).
13. **Muscle-memory workflows.** Subcommand names match Git’s porcelain/plumbing inventory names. Users MUST NOT need to learn an arks-specific command vocabulary for claimed inventory rows.
14. **Exit codes.** Success = 0; failure classes that scripts rely on MUST be non-zero. Where git(1) / command man pages document distinct exit codes for a claimed command, arks SHOULD match those classes for implemented behavior; where undocumented, prefer non-zero on any failure and never exit 0 after a refused/unimplemented option.
15. **Refuse under alias.** Unimplemented commands/options still refuse (non-zero, no repo mutation). Prefer messages that remain honest under alias (e.g. state not implemented) — do not pretend success. Exact stderr byte identity with stock Git remains **not** required unless a clean-room gate asserts it; **behavioral** suitability for alias substitution **is** required for production.
16. **Discovery under alias.** `git --help` / `git <cmd> --help` (via alias) MUST list only implemented surface for the claim tier — same honesty rules as `arks --help`.
17. **Documented limits.** README/`docs` MUST document how to set the alias **and** that alias drop-in is **intent until M9**, and must list exclusions (GUI, LFS, …) so users do not assume `gitk`/`git-lfs` appear via alias.
18. **Not a wrapper.** Alias drop-in MUST NOT be implemented by shelling out to stock `git` for product paths ([[adr:root/20261108083507]]).

### Refuse-vs-implement (historical delivery) → implement-or-N/A (active M16)

6. **During delivery waves:** Unimplemented-but-scheduled command/option MUST exit non-zero and MUST NOT mutate the repository (honest incomplete). Message SHOULD state not yet claimed for the active wave.
7. **At M16 exit:** No scheduled inventoriable command/option may remain in production refuse/partial state — each is **implement** or **N/A exclusion** ([[sot:root/cli-flag-parity-complete]]).
8. **Unknown option** (not on the pin’s man page): exit non-zero; do not invent option aliases that imply unsupported Git behavior.
9. **Prefer fail-closed over incompatible success.** Divergent on-disk results that stock Git would not produce for the same claimed invocation are defects, not “subset UX.”
10. **Help honesty:** `arks --help` / `arks <cmd> --help` list implemented commands/options for the current claim tier. Active production help lists the full parity inventory (minus N/A exclusions).

### Explicit exclusions (not required for CLI-parity production)

These remain **out of the default parity inventory** unless a future SoT amendment explicitly claims them:

| Exclusion | Rationale | Cite / owner |
|-----------|-----------|--------------|
| GUI / browser tools (`gitk`, `git-gui`, `git-citool`, and similar) | Not CLI replacement surface | git(1) porcelain list; product non-goal |
| `scalar` | Separate suite / managing large repos; not required for CLI symmetry bar | git(1) |
| **Git LFS** (`git-lfs` and LFS-aware porcelain) | Separate project; not core Git builtins | defer; claim only with dedicated SoT |
| Host product APIs (PRs/Checks/Apps) | Remote-host scope (B); `arks hub …` (same binary) | [[sot:root/remote-host-compatibility-surface]], [[sot:root/arks-hub-product-intent]] |
| LLM review / CEV bots | Explicit extension; `arks agent …` (same binary); not Git man-page inventory | [[sot:root/arks-agent-product-intent]], [[sot:root/arks-agent-cli-surface]] |
| Forge / GitHub App **as** arks server UX | Not CLI-parity; not production-server | [[sot:root/remote-host-compatibility-surface]] |
| Lease admin UX / high-concurrency product valves | **Not** Git man-page inventory. Leases = **default product capability** (ships M10); high-concurrency = **opt-in selectable** product mode — exact CLI/config flags owned by [[sot:root/lease-concurrency-intent]], not this parity inventory | [[sot:root/lease-concurrency-intent]], [[sot:root/program-roadmap]] |
| CELIR admin UX | **Cancelled / non-product** | [[sot:root/storage-celir-boundary]] |
| Exact pager / editor / mergetool **GUI** identity | May use documented helper protocols; pixel/TUI clone not required | gitcli / mergetool docs via research |
| Byte-identical stderr / advice text | Unless a gate asserts it | this SoT §Identity |

**Hooks:** When a claimed porcelain command would run hooks under stock Git, arks MUST honor the documented hook points for that command by running **sync** hook scripts from `$GIT_DIR/hooks` (hook *scripts* are user content). Inventing a parallel hook API or async/out-of-band hook scheduler is forbidden (no L3). Skipping hooks by default on claimed commands is forbidden unless the pin’s option (e.g. documented `--no-verify`) is used. “Refuse whenever any hook file exists” is **not** hook honor.

### Layers (CLI vs format)

| Layer | Owner | Role |
|-------|-------|------|
| Bytes / wire / **same `.git` layout** | [[sot:root/git-compat-surface]] | What repositories and sessions mean on disk |
| Command/flag UX + parity inventory + **alias drop-in** | **This SoT** | How humans/agents invoke that fidelity (including `alias git=arks`) |
| Production exit | [[sot:root/program-roadmap]] | When parity + alias-ready + remotes + clean-room close |

Implementing a CLI command does **not** authorize inventing formats; claiming a format does **not** by itself ship CLI parity.

### Delivery waves (phased; end-state is full inventory)

Delivery phases below are **program waves**, not a lowered end-state. Old “MVP subset = enough for production” is **revoked**. Mapping to milestones: [[sot:root/program-roadmap]].

#### Wave L — Local foundation (post-engine; milestone M5)

Minimum local daily-driver surface (still a **subset claim** until later waves close):

| Command | Intent | Compat note |
|---------|--------|-------------|
| `arks init` | Create repository layout stock Git can open | gitrepository-layout |
| `arks hash-object` | Write/read blob OID from content | Loose object format |
| `arks cat-file` | Type/size/content by OID | Object header + inflate |
| `arks update-ref` | Create/update/delete a ref | Refs + locking as documented |
| `arks rev-parse` | Resolve revisions / symbolic names (expand toward pin); **`--abbrev-ref`** short branch/tag name | Ref + object lookup; [[sot:root/cli-flag-parity-complete]] |
| `arks add` | Stage paths into the index | Index as claimed |
| `arks status` | Worktree vs index vs HEAD | Index + refs |
| `arks commit` | Write tree + commit; advance branch; optional `-S` SSH sign | Objects + refs + index; FW-GPGSIG SSH ([[sot:root/arks-object]]) |
| `arks log` | Commit history from HEAD | Commit objects |
| `arks show` | Show object / commit summary | Object read |

#### Wave H — Remote client (milestone M6)

Generic Git remote **client** for claimed protocols; GitHub-class hosts are the **representative verify peer** ([[sot:root/remote-host-compatibility-surface]]) — not GitHub-exclusive product scope.

| Command | Intent | Compat note |
|---------|--------|-------------|
| `arks remote` | Add/list/remove remotes | Config as Git documents |
| `arks fetch` | Download objects/refs | Pack protocol / HTTP smart / v2 as negotiated |
| `arks push` | Upload refs/objects; **`-u`/`--set-upstream`**; short tag name DWIM | receive-pack / dumb WebDAV path; `:<ref>` delete claimed (S3); branch upstream config; heads-then-tags DWIM; refuse other `src:dst` refspecs / unimplemented caps |
| `arks clone` | Init + fetch/copy + checkout baseline | HTTP smart + local path / same-host `file://` hardlink-or-copy ([[sot:root/arks-remote]]); not `--shared` |
| `arks pull` | Fetch + integrate (may land with Wave Pa/Pb if merge/rebase not yet claimed — if so, refuse incomplete pull forms) | Must not imply unimplemented merge/rebase |

#### Wave Pa — Porcelain parity A (milestone M7)

Main local branching / worktree / diff / integrate foundations from git(1) main porcelain, including at least: `branch`, `checkout`, `switch`, `restore`, `reset`, `diff`, `merge`, `rm`, `mv`, `tag`, `clean`, `config` (claimed subset→parity), `describe`, `shortlog`, `grep`, `stash` (minimal→full per plan), `worktree` (as claimed in plan), plus ancillary needed for those flows (`reflog`, `repack` / pack helpers as required). **Frozen** implement/refuse/exclude checklist: [[plan:root/20261108183810]] (M7); matrix `verify/matrices/matrix-m007-clean-room-porcelain-pa.md`. M7 defers `worktree` / `reflog` / `repack` with refuse; `merge` is FF/`--ff-only` only; annotated tags refuse to Pb/Pc.

#### Wave Pb — Porcelain parity B (milestone M8)

History-editing and patch workflows from git(1). **Frozen** implement/refuse/exclude checklist: [[plan:root/20261108183811]] (M8); matrix `verify/matrices/matrix-m008-clean-room-porcelain-pb.md`. Implement (claimed subset): `rebase` (non-interactive), `cherry-pick`, `revert`, `bisect` (start/bad/good/reset), `am`, `format-patch`, `bundle`, `pull --rebase`. **Refuse** to Pc (omit from help): `range-diff`, `notes`, `filter-branch`, interactive rebase. Conflicted applies refuse non-zero (no 3-way merge engine in M8).

#### Wave Pc — Porcelain parity C + inventory closure (milestone M9) — **production**

1. Close remaining main-porcelain and required ancillary/plumbing inventory rows against the pin (including sparse-checkout / maintenance / archive / backfill / etc. **unless** moved to exclusions by SoT amendment). **Post-M9 residual:** `worktree` + `submodule` claimed implement matrix in [[plan:root/20261109022956]] (does not reopen the M9 exit bar).
2. **Flag-parity audit** green in clean-room for every inventory command (implement or refuse classification recorded).
3. **Alias-ready audit** green: representative muscle-memory workflows succeed when the clean-room invokes the `arks` binary under a `git`→`arks` alias (or argv-equivalent shim); refuse paths remain non-zero under alias.
4. Satisfy [[sot:root/program-roadmap]] CLI-parity production checklist (same `.git` + remotes + engine + honesty + gates).

**Frozen** implement/refuse/exclude checklist: [[plan:root/20261108183812]] (M9); matrix `verify/matrices/matrix-m009-clean-room-cli-parity.md`; flag-parity ledger [[sot:root/cli-flag-parity-m9]]; alias how-to `docs/alias-git-arks.md`. Unit SoT: [[sot:root/arks]] Wave Pc.

#### Wave Fs — Full CLI symmetric residual (milestone M15) — **historical inventory close**

Post-M9 residual that raised claimed inventory from implement/**refuse** closure to **inventory-level** full symmetric replacement (− exclusions). Does **not** rewrite the M9 historical exit. Flag ledger (historical): [[sot:root/cli-flag-parity-full]]. Plan: [[plan:root/20261109035335]]. Gate: **G20**. Carrying: [[evidence:root/20261109203000]]. Per-flag refuse leftovers at M15 exit are **not** lasting production honesty — see Wave Sy.

#### Wave Sy — Complete CLI flag symmetry (milestone M16) — **active alias bar**

Raises active honesty from M15 inventory close to **complete flag/command symmetry**: every pin option is **implement** or **N/A exclusion**; **no production refuse/partial leftovers**. Ledger: [[sot:root/cli-flag-parity-complete]]. Plan: [[plan:root/20261109043910]]. Gate: **G21**. Functional Sy waves have priority over finishing M17/G19 expansion on those units.

| Wave | Focus |
|------|-------|
| Sy-1 | Everyday porcelain flags (`status -b`, `log --graph`, `branch -a/-r`, `add --chmod`, …) |
| Sy-2 | Worktree porcelain + submodule `--recursive`/`--remote` |
| Sy-3 | Plumbing breadth |
| Sy-4 | Client SSH transport (as claimed) |
| Sy-5 | Remaining depth (stash/sequencer/bisect/notes/blame/fsck/…) |

#### Wave Sv — Git server plumbing (milestone M11) — **production-server** (not M9)

Server plumbing is an **in-scope product goal** under [[sot:root/remote-host-compatibility-surface]] / [[sot:root/program-roadmap]] **M11**. It is **excluded from the M9 CLI-parity inventory closure** (users aliasing `git=arks` need client porcelain first; server binaries are invoked by transports / hooks / CGI). Primary man pages: [git-upload-pack](https://git-scm.com/docs/git-upload-pack), [git-receive-pack](https://git-scm.com/docs/git-receive-pack), [git-http-backend](https://git-scm.com/docs/git-http-backend), [git-daemon](https://git-scm.com/docs/git-daemon).

| Command | Intent | Compat note |
|---------|--------|-------------|
| `arks upload-pack` | Serve fetch/clone (upload-pack) | gitprotocol-pack duplex stdio (SSH command-mode / local); HTTP ads via serve-http; v2 when G31 Claimed |
| `arks receive-pack` | Serve push (receive-pack) | report-status (v1); githooks honor list `pre-receive` / `update` / `post-receive` — [[sot:root/arks-server]] |
| `arks http-backend` | Smart (+ dumb) HTTP CGI/service adapter | gitprotocol-http; git-http-backend |
| `arks serve` | GitHub-class self-host umbrella (default HTTP+SSH+forge API) | Prefer `--no-http`/`--no-ssh`/`--no-forge` opt-outs; `serve config` nested; SSH + forge-in-serve Partial/fail-closed until Claimed ([[sot:root/arks-self-host-server]]); HTTP smart+dumb + Basic (G36) |
| `arks serve-http` | Transitional HTTP-git spelling | Prefer ≡ `arks serve --no-ssh --no-forge`; not a permanent MUST alias |
| `arks update-server-info` | Refresh dumb HTTP advertisements | [git-update-server-info](https://git-scm.com/docs/git-update-server-info); G29 |
| `arks daemon` | `git://` TCP daemon | [git-daemon](https://git-scm.com/docs/git-daemon); internal crate `arks-daemon` ([[sot:root/arks-daemon-product-intent]]); Claimed on G32; **not** a separate release bin |

Refuse unimplemented server transports/options; never advertise unimplemented capabilities on the wire.

### UX principles (arks-specific, non-semantic)

11. **Git muscle memory without lying.** Same subcommand names as Git for inventory commands; never advertise full parity before Wave Pc exit.
12. **Agent-friendly + human parity.** Stable non-interactive / scripted defaults remain; **TTY interactive prompts and editor-driven flows that stock git exposes MUST be Claimed or impossible+slice** under [[research:root/20261110212414]] (FS1+) — “agent bar” convenience MUST NOT forever N/A real git features.
13. **Crate placement.** CLI crate is outer: depends inward; libraries MUST NOT depend on the CLI bin ([[sot:root/crate-architecture]]).

## Why

arks is a Git-compatible **replacement CLI** on the **same `.git`**. Formats alone are not enough: users who already type `git …` must be able to **alias to `arks`** and keep muscle memory. Raising the end-state to **symmetric CLI parity + alias drop-in** (pinned Git man pages + clean-room oracle) prevents “subset MVP forever” while still allowing bottom-up engine delivery and phased porcelain waves. Server plumbing (**Wave Sv / M11**) is a separate production-server bar so M9 stays an honest client alias target. Refuse-closed behavior avoids silent incompatibility under alias. Exclusions keep GUI/`scalar` from blocking the CLI-parity production bar. Non-git product CLI (`forge`, `host-env`, in-process `lfs`, CELIR Claimed path) is **trimmed** ([[research:root/20261111035700]]); `lease`/`lock` remain intentional asymmetry.

## Verification

Manual until harness exists. Authoritative product claims use [[sot:root/product-verification-clean-room]] and Gate IDs in [[sot:root/interop-test-matrix]].

1. `spec check` clean for this SoT.
2. Every claimed command maps to git-compat / research (no orphan UX).
3. Clean-room gates: foundation (G6), remotes (G7+), porcelain waves (G12–G14 including alias-ready), production closure (roadmap).
4. `--help` matches the claimed tier; production help matches inventory minus exclusions; alias docs honest.
5. Dependency direction: CLI → libs only.
6. Alias-ready: at least one clean-room recipe runs claimed workflows with `git` aliased to `arks` before M9 exit.

## Open decisions

| Decision | Status | Notes |
|----------|--------|-------|
| Exact Wave Pa/Pb/Pc command partition tables | **Pa frozen** in [[plan:root/20261108183810]]; **Pb frozen** in [[plan:root/20261108183811]]; Pc plan-owned | Amend plans without lowering production bar |
| Whether `pull` fully closes in M6 vs M7/M8 | **M7:** FF-only `pull`; **M8:** `pull --rebase` claimed; other strategies → Pc | Refuse incomplete forms until claimed |
| Plumbing inventory minimum beyond everyday scripting set | **Claimed under M15 / Fs-1+** | [[sot:root/cli-flag-parity-full]]; expand until porcelain does not depend on missing plumbing |
| Advancing clean-room Git pin | **Closed (process)** | MUST follow [[sot:root/clean-room-git-pin-advance]]; prior evidence does not carry silently |
| Credential helper / askpass parity depth | **Claimed (TTY + OS wallets)** | HTTPS Basic via `arks credential fill` + `credential.helper` + `GIT_ASKPASS`/`core.askPass` + TTY when `GIT_TERMINAL_PROMPT`≠`0` + first-party `wincred`/`libsecret`/`osxkeychain` — [[sot:root/credential-helper-askpass]]; Gate **G33** [[evidence:root/20261110224000]] [[evidence:root/20261110233000]]; forge OAuth out |
| Full inventory vs M9 refuse ledger | **M15 historical; M16 active** | M9/M15 receipts frozen; complete symmetry = M16 exit + G21 ([[sot:root/cli-flag-parity-complete]]) |
| Per-flag refuse leftovers after M15 | **Scheduled M16 Sy** | Must become implement or N/A — not lasting refuse |
