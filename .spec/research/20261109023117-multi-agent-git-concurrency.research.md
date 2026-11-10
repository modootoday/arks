---
id: research:root/20261109023117
title: Multi-agent Git concurrency issue scenarios
status: active
created: 2026-11-09T02:31:17Z
updated: 2026-11-09T02:55:00Z
tags: [git, concurrency, multi-agent, locks, index, refs, worktree, submodule, lfs, performance]
rel:
  depends_on: [sot:root/lease-concurrency-intent, sot:root/git-compat-surface, sot:root/multi-agent-git-concurrency-scenarios, research:root/20261108083502, research:root/20261108071914, research:root/20261108071915]
---

# Multi-agent Git concurrency issue scenarios

## Question

When **dozens to hundreds of agents** run Git operations concurrently against a shared logical repository (same worktree, linked worktrees, shared bare remote, CI matrix, containers, or network FS), what **issue scenarios** occur — lock, ref, index, object, pack, worktree, submodule, hook, config, network, filesystem, performance, corruption, TOCTOU, sequencer, and related classes — and which of those are **stock-Git limitations** versus opportunities for arks to **mitigate in compat mode** or **differentiate beyond Git**?

## Method

1. Start from product pain already recorded in [[sot:root/lease-concurrency-intent]] and [[research:root/20261108083502]]: exclusive Git locks serialize dense multi-writer storms.
2. Enumerate scenario classes without a quantity cap; assign stable **scenario IDs** (`CLASS-NNN`).
3. Prefer **primary / near-primary** sources: git-scm docs (layout, update-ref, gc, worktree, status, hooks, lockfile API, config), Git mailing-list / public-inbox design notes (packed-refs lock retries, ref transactions), and documented extension issues (git-lfs concurrency). Summarize + cite URLs; do **not** paste large copyrighted text.
4. Record qualitative **frequency-at-N** (dozens–hundreds of agents) and map each row to an **arks opportunity class** for the normative catalog [[sot:root/multi-agent-git-concurrency-scenarios]].
5. Explicit honesty: this research does **not** redefine M9/M10/M11 exits; does **not** claim absolute millisecond budgets; CELIR remains excluded from product deps; lease default vs L3 opt-in / refuse-close remains per lease SoT + [[adr:root/20261108232800]].

Research snapshot date: **2026-11-09**.

## Findings

### Primary mechanism (stock Git)

Git’s documented concurrency control is **pessimistic lockfiles**: create `path.lock` with `O_CREAT|O_EXCL`, write, then `rename` into place. Lockfiles **block other writers only**; readers are expected to see old-or-new if rename is atomic ([api-lockfile](https://git-scm.com/docs/api-lockfile)).

Critical shared hot spots under multi-agent load:

| Hot spot | Typical lock / artifact | Why it hurts at N agents |
|---------|-------------------------|--------------------------|
| Index | `$GIT_DIR/index.lock` | One lock for all path stages; even disjoint `add` serializes |
| Loose ref | `refs/…/*.lock` | Same-branch commit storms CAS-fail or lock-fail |
| Packed refs | `packed-refs.lock` | One lock for many refs; historically needed retries (`core.packedRefsTimeout`) |
| HEAD / pseudorefs | `HEAD.lock`, merge/rebase state | Porcelain assumes single actor per worktree |
| Objects / packs | loose dirs, `*.keep`, gc | Write+prune races; auto-gc storms |
| Common dir | shared among worktrees | Worktree-per-agent still contends on common store |

### Source catalog (cite, do not copy)

| Topic | URL | Scope used |
|-------|-----|------------|
| Lockfile API | https://git-scm.com/docs/api-lockfile | Exclusive create, rename commit, readers vs writers, atexit cleanup |
| Repository layout | https://git-scm.com/docs/gitrepository-layout | objects, refs, packed-refs, index, hooks, modules, worktrees, commondir, shallow, alternates |
| update-ref | https://git-scm.com/docs/git-update-ref | CAS, `--stdin` transactions, prepare/commit, concurrent reader subset note, reflog append |
| status optional locks | https://git-scm.com/docs/git-status | Background refresh vs writers; `--no-optional-locks` |
| git / config | https://git-scm.com/docs/git-config | `core.lockfilePid`, `core.logAllRefUpdates`, extensions, gc.* keys |
| gc | https://git-scm.com/docs/git-gc | auto heuristics, prune concurrency NOTES, `--force`, hooks |
| worktree | https://git-scm.com/docs/git-worktree | linked wt, common dir, lock/prune/repair |
| hooks | https://git-scm.com/docs/githooks | pre-commit, pre-auto-gc, reference-transaction, post-index-change |
| fsmonitor daemon | https://git-scm.com/docs/git-fsmonitor--daemon | single WD daemon; submodule event noise |
| pack-refs | https://git-scm.com/docs/git-pack-refs | packed-refs rewrite |
| reftable | https://git-scm.com/docs/reftable | alternate ref storage |
| Refs research (arks) | [[research:root/20261108071914]] | packed-refs / reftable / naming already cataloged |
| Index research (arks) | [[research:root/20261108071915]] | index lock surface |
| packed-refs lock retries (lore) | http://public-inbox.org/git/xmqq38337zy1.fsf@gitster.dls.corp.google.com/T/ | Why packed-refs needs timeout retries (GitHub-scale contention) |
| Packed ref transactions (lore) | http://public-inbox.org/git/810bbfa2-4486-e3dd-8243-ea6116f21b2c@alum.mit.edu/T/ | Historical delete/pack races; transaction prepare lock |
| Parallel submodule status (lore) | http://public-inbox.org/git/CAPig+cQvyScjiWe1ghFrv9B=v2+JxkkErdCyYFSA_8dTrWu60g@mail.gmail.com/T/ | Serial submodule status cost |
| git-lfs shared storage races | https://github.com/git-lfs/git-lfs/issues/2825 | Concurrent download/replace races |
| git-lfs Windows rename | https://github.com/git-lfs/git-lfs/pull/3873 | Sharing violation under parallel smudge |
| git-lfs pending-delete | https://github.com/git-lfs/git-lfs/pull/6215 | AV/handle races on Windows Server |

### Taxonomy counts (this snapshot)

**Total scenarios: 221** (normative rows live in [[sot:root/multi-agent-git-concurrency-scenarios]]).

| Class | Count |
|-------|------:|
| `agent` | 12 |
| `bisect` | 2 |
| `ci` | 6 |
| `config` | 4 |
| `corruption` | 7 |
| `credential` | 2 |
| `fs` | 14 |
| `fsmonitor` | 4 |
| `hook` | 8 |
| `index` | 14 |
| `lfs` | 6 |
| `lock` | 9 |
| `maintenance` | 6 |
| `misc` | 20 |
| `network` | 11 |
| `object` | 9 |
| `pack` | 9 |
| `perf` | 10 |
| `ref` | 23 |
| `sequencer` | 7 |
| `shallow` | 4 |
| `sparse` | 2 |
| `stash` | 2 |
| `submodule` | 10 |
| `toctou` | 7 |
| `windows` | 3 |
| `worktree` | 10 |

### ID index by class

- **agent**: AGENT-001, AGENT-002, AGENT-003, AGENT-004, AGENT-005, AGENT-006, AGENT-007, AGENT-008
- **bisect**: BISECT-001, BISECT-002
- **ci**: CI-001, CI-002, CI-003, CI-004
- **config**: CFG-001, CFG-002, CFG-003, CFG-004
- **corruption**: CORR-001, CORR-002, CORR-003, CORR-004, CORR-005
- **credential**: CRED-001, CRED-002
- **fs**: FS-001, FS-002, FS-003, FS-004, FS-005, FS-006, FS-007, FS-008, FS-009, FS-010, FS-011
- **fsmonitor**: FSM-001, FSM-002, FSM-003
- **hook**: HOOK-001, HOOK-002, HOOK-003, HOOK-004, HOOK-005, HOOK-006
- **index**: IDX-001, IDX-002, IDX-003, IDX-004, IDX-005, IDX-006, IDX-007, IDX-008, IDX-009, IDX-010, IDX-011, IDX-012
- **lfs**: LFS-001, LFS-002, LFS-003, LFS-004
- **lock**: LOCK-001, LOCK-002, LOCK-003, LOCK-004, LOCK-005, LOCK-006, LOCK-007, LOCK-008
- **maintenance**: MAINT-001, MAINT-002, MAINT-003, MAINT-004
- **misc**: MISC-001, MISC-002, MISC-003, MISC-004, MISC-005, MISC-006, MISC-007, MISC-008, MISC-009, MISC-010, MISC-011, MISC-012, MISC-013, MISC-014, MISC-015
- **network**: NET-001, NET-002, NET-003, NET-004, NET-005, NET-006, NET-007, NET-008
- **object**: OBJ-001, OBJ-002, OBJ-003, OBJ-004, OBJ-005, OBJ-006, OBJ-007
- **pack**: PACK-001, PACK-002, PACK-003, PACK-004, PACK-005, PACK-006, PACK-007
- **perf**: PERF-001, PERF-002, PERF-003, PERF-004, PERF-005, PERF-006, PERF-007
- **ref**: REF-001, REF-002, REF-003, REF-004, REF-005, REF-006, REF-007, REF-008, REF-009, REF-010, REF-011, REF-012, REF-013, REF-014, REF-015, REF-016, REF-017, REF-018, REF-019, REF-020
- **sequencer**: SEQ-001, SEQ-002, SEQ-003, SEQ-004, SEQ-005
- **shallow**: SHAL-001, SHAL-002, SHAL-003
- **sparse**: SPARSE-001, SPARSE-002
- **stash**: STASH-001, STASH-002
- **submodule**: SUB-001, SUB-002, SUB-003, SUB-004, SUB-005, SUB-006, SUB-007, SUB-008
- **toctou**: TOCTOU-001, TOCTOU-002, TOCTOU-003, TOCTOU-004, TOCTOU-005
- **windows**: WIN-001, WIN-002, WIN-003
- **worktree**: WT-001, WT-002, WT-003, WT-004, WT-005, WT-006, WT-007, WT-008

### Opportunity skew (research observation)

- **differentiate-beyond-git** concentrates on: single-index serialization, same-ref tip CAS storms, optional-lock readers blocking writers, missing agent lease/namespace coordination, retry stampedes, in-process vs process-per-command overhead, worktree+common-dir composition, hook amplification.
- **mitigate-with-compat** concentrates on: honoring lock semantics correctly, `--no-optional-locks` defaults for agent status, safer gc/prune coupling, FS/AV guidance, packed-refs timeout honesty, worktree/submodule fail-closed rules.
- **out-of-scope** (examples): cloud-sync folders as repo storage, SHA-1 collision theater, host forge shadow refs, some bundle-URI CDN ops — document but do not invent product claims.

### Relation to lease / L3

Leases ([[sot:root/lease-concurrency-intent]]) are the **default product coordination** answer; L3 N-way pipeline remains **opt-in / refuse-closed** for current production bars ([[adr:root/20261108232800]]). This catalog feeds future uplift design; it does **not** ship L3 or redefine milestone exits.

### Expansion note

Addendum rows landed in the SoT under class `addendum` (plus scattered IDs); total **221**. Opportunity skew remains dominated by mitigate-with-compat + differentiate-beyond-git.

## Open decisions

1. Which differentiate rows become M10+ lease grant shapes (ref namespace, worktree, path set) vs deferred L3 pipeline mechanics?
2. Clean-room stress topology for multi-writer scenarios (synthetic N agents) when a future ADR claims L3.
3. Whether agent-oriented defaults (e.g. status without optional locks) are Git-compat CLI defaults or arks agent-profile overlays ([[sot:root/cli-surface]]).

## Links

- Normative catalog: [[sot:root/multi-agent-git-concurrency-scenarios]]
- Lease intent: [[sot:root/lease-concurrency-intent]]
- Allocator / pipeline principles: [[research:root/20261108083502]]
