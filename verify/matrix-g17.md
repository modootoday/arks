# Concurrency issue-compare matrix (G17) — M12 Phase Cm + M19/M20 Phase Cx

Owning SoT: `.spec/sot/interop-test-matrix.sot.md` (G17). Catalog: `.spec/sot/multi-agent-git-concurrency-scenarios.sot.md`. Plans: `.spec/plan/20261109024634-m12-concurrency-compat-mitigation.plan.md` (Cm), `.spec/plan/20261110010000-m19-concurrency-claim-expansion.plan.md` (Cx-1/2), `.spec/plan/20261110030000-m20-concurrency-cx3-claim-expansion.plan.md` (Cx-3), `.spec/plan/20261110040000-m20-concurrency-cx4-claim-expansion.plan.md` (Cx-4). Roadmap Done rule: `.spec/sot/program-roadmap.sot.md`.

Authoritative venue: Docker clean-room service `g17` (`verify/scripts/run-g17.sh`). Host runs are advisory only.

Protocol per claimed ID: **induce → stock `git` pathology oracle → `arks` resolved oracle → compare receipt**. Timings MAY be recorded; absolute / ≥20% pass-fail is **G19 / M14 only** (out of scope here). Cx waves do **not** claim the ~221 catalog is solved.

## Wave Cm-1 (claimed set)

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| AGENT-006 / IDX-008 | Status-class optional-lock contention vs writer | Held/optional `index.lock` blocks `git add` (writer fail class) | Agent/profile / `--no-optional-locks` status does **not** take optional locks; writer proceeds when no live lock | yes | `verify/g17` |
| LOCK-002 | Plant stale `index.lock` (no live holder) | Stale lock blocks subsequent `git add` | `arks lock inspect` reports; `arks lock clear --stale` safe path; writes still honor live locks (no skip) | yes | `verify/g17` |
| WT-001 | Linked worktree + common-dir lock planted | Common-store lock fails git writer from linked wt | Claimed worktree ops fail-closed with lock-domain honesty; locked `remove` refuse; no invented dialect | yes | `verify/g17` |
| SUB-001 | Superproject + submodule; plant submodule `index.lock` | Submodule lock-domain blocks induced git path | Claimed submodule matrix fail-closed / dual-domain honesty | yes | `verify/g17` |

Wave Cm-1 **Done** = Docker G17 PASS for **all four** rows + comparative evidence receipt. Unlisted catalog IDs remain **Open**.

## Wave Cm-2 (claimed set)

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| IDX-001 | Plant held index.lock; concurrent writer | git add fails under exclusive lock | rks add fail-closed with exclusive-lock + retry guidance (no sharded writer) | yes | erify/g17 |
| HOOK-001 / HOOK-006 | Install sync pre-commit / 
eference-transaction hook | Hook runs in critical section (amplification class) | Refuse inventing async hook runners; --no-verify escape; sync honesty | yes | erify/g17 |
| PACK-004 / MAINT-003 | Invoke gc / maintenance under writer load class | Stock gc/auto-detach/pack consolidation path available | rks gc/
epack/maintenance refuse-closed (safer than concurrent prune) | yes | erify/g17 |
| AGENT-003 | Contended lock + immediate retry herd sketch | Blind retries keep failing / amplify | Lock fail messages + rks lock note teach backoff/jitter; no M12 lease scheduler | yes | erify/g17 |

Wave Cm-2 **Done** = Docker G17 PASS for **all four** Cm-2 rows + Cm-1 regression green.

## Wave Cm-3 (claimed set)

Chosen Open mitigate IDs: IDX-002, AGENT-005, AGENT-007, CORR-001, FS-006 (roadmap ranks 1/3/5/7; preferred next wave).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| IDX-002 | Background/optional status lock vs foreground writer | Held optional-class `index.lock` blocks `git add` | Default `arks status` takes no optional locks; writer proceeds when unlocked | yes | `verify/g17` |
| AGENT-005 | Plant dead-PID lock (evicted container class) | Stale lock blocks next `git add` | `lock inspect` + `clear --stale` recover; AGENT-005 eviction honesty; live refuse | yes | `verify/g17` |
| AGENT-007 | Linked worktree + submodule; plant multi-domain locks | Writers fail across composed lock domains | Fail-closed on each domain; inspect multi-domain AGENT-007 honesty | yes | `verify/g17` |
| CORR-001 | `git gc --prune=now` under writer-risk class | Stock prune=now path available | `arks gc`/`repack`/`maintenance` refuse-closed citing CORR-001 | yes | `verify/g17` |
| FS-006 | Shared-path lock (bind-mount dual-writer class) | Lock on shared `.git` blocks stock writer | Inspect FS-006 bind-mount / cross-ns note; write path honors locks; clear only dead | yes | `verify/g17` |

Wave Cm-3 **Done** = Docker G17 PASS for **all five** Cm-3 rows + Cm-1/Cm-2 regression green.

## Wave Cx-1 (M19 claimed set)

Local filesystem / ref / worktree / sequencer races still unclaimed after M12/M13. Plan: `plan:root/20261110010000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| REF-007 | Plant `HEAD.lock` | HEAD update / commit class fails under lock | Fail-closed; `lock inspect` cites REF-007; lease ≠ skip | yes | `verify/g17` |
| LOCK-001 | Plant exclusive `index.lock` (O_CREAT class) | Second writer fails (`Unable to create` class) | Fail-closed; inspect LOCK-001 exclusive-create honesty | yes | `verify/g17` |
| WT-002 | Second `worktree add` same branch | Stock fails already-checked-out | Same fail-closed | yes | `verify/g17` |
| WT-004 | `worktree lock` then `remove` | Stock remove refuses locked wt | Same refuse; unlock then remove OK | yes | `verify/g17` |
| SEQ-001 | Plant `rebase-merge` (or apply) dir | Second rebase fails / refuses | arks rebase refuse with SEQ-001 honesty | yes | `verify/g17` |
| SEQ-002 | Plant `MERGE_HEAD` then start merge | Stock refuses “not concluded” / MERGE_HEAD class | arks rebase refuse SEQ-002 | yes | `verify/g17` |
| SEQ-003 | Plant `CHERRY_PICK_HEAD` | Second cherry-pick fails | arks cherry-pick refuse SEQ-003 | yes | `verify/g17` |
| REF-018 | Plant `MERGE_HEAD` then conflicting merge start | Stock refuses MERGE_HEAD / not-concluded class | arks refuse REF-018 on conflicting porcelain | yes | `verify/g17` |
| MISC-012 | Plant `index.lock`; `update-index --refresh` | Stock refresh fails under lock | arks refresh fail-closed under lock; unlocked OK | yes | `verify/g17` |
| TOCTOU-005 | Held lock + poll-absence temptation | Stock still fails under lock (O_EXCL) | arks never clears/skips after poll; inspect TOCTOU-005 | yes | `verify/g17` |

Wave Cx-1 **Done** = Docker G17 PASS for **all ten** Cx-1 rows + Cm-1..Cm-3 regression green.

## Wave Cx-2 (M19 claimed set)

Local mitigate-with-compat races still unclaimed after Cx-1. Plan: `plan:root/20261110010000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| BISECT-001 | Second `bisect start` | Stock refuses concurrent bisect | Refuse citing BISECT-001 | yes | `verify/g17` |
| BISECT-002 | Dirty WT mid-bisect | Checkout/mark fails dirty class | `bisect good/bad` refuse BISECT-002 | yes | `verify/g17` |
| SEQ-006 | Plant interactive `git-rebase-todo` | Second rebase fails | arks rebase refuse SEQ-006/SEQ-001 | yes | `verify/g17` |
| SEQ-007 | Plant `rebase-apply`; `am` | Stock am fails | arks am refuse SEQ-007 | yes | `verify/g17` |
| MISC-007 | Same apply-state class | Stock am fails | arks cite MISC-007 | yes | `verify/g17` |
| STASH-001 | Plant `refs/stash.lock` | Stock stash fails | arks refuse STASH-001 | yes | `verify/g17` |
| STASH-002 | Plant `index.lock`; stash pop | Stock pop fails | arks refuse STASH-002 | yes | `verify/g17` |
| REF-021 | Plant `HEAD.lock`; symbolic-ref | Stock fails | arks fail-closed REF-021 | yes | `verify/g17` |
| TOCTOU-003 | Dirty WT; checkout other | Stock overwrite refuse | arks refuse TOCTOU-003 | yes | `verify/g17` |
| MISC-005 | Plant `index.lock`; reset --hard | Stock fails | arks refuse MISC-005 | yes | `verify/g17` |
| CORR-002 | `gc --force` available | Stock offers --force | arks refuse CORR-002 | yes | `verify/g17` |
| LOCK-004 | Plant lock; porcelain add | Stock die-class | arks die-class + LOCK-004 inspect | yes | `verify/g17` |

Wave Cx-2 **Done** = Docker G17 PASS for **all twelve** Cx-2 rows + Cm-1..Cm-3 + Cx-1 regression green (**35**/35).

## Wave Cx-3 (M20 claimed set)

Research: `.spec/research/20261110021546-git-concurrency-contention-resolution.research.md`. Plan: `plan:root/20261110030000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| REF-012 | Plant `packed-refs.lock` | `pack-refs` fails | Fail-closed REF-012 | yes | `verify/g17` |
| REF-016 | Plant `refs/notes/commits.lock` | `notes add` fails | Fail-closed REF-016 | yes | `verify/g17` |
| IDX-014 | Plant `index.lock` | `read-tree` fails | Fail-closed IDX-014 | yes | `verify/g17` |
| SPARSE-001 | Plant `info/sparse-checkout.lock` | `sparse-checkout set` fails / race | Fail-closed SPARSE-001 | yes | `verify/g17` |
| MISC-018 | Plant `index.lock`; reapply | reapply fails | Fail-closed MISC-018 | yes | `verify/g17` |
| SUB-008 | `modules/` + move | stock move refuses | Refuse SUB-008 | yes | `verify/g17` |
| TOCTOU-007 | Incomplete admin + prune | race class | Refuse TOCTOU-007 | yes | `verify/g17` |
| PACK-009 | Plant `*.keep` | keep blocks GC class | Inspect PACK-009 | yes | `verify/g17` |
| MAINT-002 | Plant `gc.log` | auto-gc suppress class | Inspect MAINT-002 | yes | `verify/g17` |
| NET-009 | Clone into non-empty dest | clone fails | Refuse NET-009 | yes | `verify/g17` |
| CFG-004 | Format v1 + unknown extension | open/status refuse | Refuse CFG-004 | yes | `verify/g17` |
| MISC-017 | Plant `NOTES_MERGE_*` | notes mutate blocked class | Refuse MISC-017 | yes | `verify/g17` |

Wave Cx-3 **Done** = Docker G17 PASS for **all twelve** + prior regression (**47**/47).

## Wave Cx-4 (M20 claimed set)

Research: `.spec/research/20261110021546-git-concurrency-contention-resolution.research.md`. Plan: `plan:root/20261110040000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| REF-009 | Plant `logs/HEAD.lock` | commit/update-ref class fails / lock race | Refuse REF-009 | yes | `verify/g17` |
| REF-022 | Plant `packed-refs.lock` | for-each-ref reader-safe | Reader OK + inspect REF-022 | yes | `verify/g17` |
| IDX-013 | Plant `index.lock`; `--chmod` | chmod fails | Refuse IDX-013 | yes | `verify/g17` |
| IDX-010 | Plant `index.lock`; `--skip-worktree` | flip fails | Refuse IDX-010 | yes | `verify/g17` |
| HOOK-002 | pre-auto-gc / inspect | hook can block auto-gc | gc/inspect cites HOOK-002 | yes | `verify/g17` |
| HOOK-005 | Plant `index.lock`; nested add | add fails | Refuse HOOK-005 | yes | `verify/g17` |
| MISC-014 | Plant logs lock; `reflog expire` | expire fails | Refuse MISC-014 | yes | `verify/g17` |
| MISC-011 | Mid-flight hooks swap marker | non-atomic hooks deploy class | Inspect MISC-011 | yes | `verify/g17` |
| SHAL-001 | Plant `shallow.lock` | deepen/fetch class fails | Refuse SHAL-001 | yes | `verify/g17` |
| WT-006 | Plant `config.worktree.lock` | config --worktree fails | Refuse WT-006 | yes | `verify/g17` |
| SUB-006 | Gitlink ≠ nested HEAD | `+` status class | Status honesty SUB-006 | yes | `verify/g17` |
| PACK-008 | Plant pack `.lock` / tmp | index-pack collision class | Refuse PACK-008 | yes | `verify/g17` |

Wave Cx-4 **Done** = Docker G17 PASS for **all twelve** + prior regression (**59**/59).

## Wave Cx-5 (M20 claimed set)

Plan: `plan:root/20261110050000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| IDX-004 | Plant `sharedindex.lock` | split-index race class | Refuse IDX-004 | yes | `verify/g17` |
| IDX-011 | Plant `index.lock`; update-index | extension rewrite fails | Refuse IDX-011 | yes | `verify/g17` |
| HOOK-003 | post-index-change present | hook thrash class | Inspect HOOK-003 | yes | `verify/g17` |
| MISC-019 | Plant `.gitattributes.lock` | concurrent attrs class | Refuse MISC-019 | yes | `verify/g17` |
| OBJ-001 | Dual `hash-object -w` same bytes | idempotent create | Idempotent + OBJ-001 note | yes | `verify/g17` |
| OBJ-005 | Plant truncated loose | cat-file fails | Refuse OBJ-005 | yes | `verify/g17` |
| PACK-001 | Plant pack `.lock` | concurrent pack class | Refuse PACK-001 | yes | `verify/g17` |
| WT-005 | Stale linked `gitdir` | broken pointer class | Repair refuse WT-005 | yes | `verify/g17` |
| LOCK-003 | Inspect rename atomicity | old-or-new readers | Inspect LOCK-003 | yes | `verify/g17` |
| MISC-002 | Archive `-o` occupied | shared prefix risk | Refuse MISC-002 | yes | `verify/g17` |
| MISC-003 | Bundle under `HEAD.lock` | tip-unstable class | Refuse MISC-003 | yes | `verify/g17` |
| SHAL-003 | Plant filter info | filter mismatch class | Inspect SHAL-003 | yes | `verify/g17` |

Wave Cx-5 **Done** = Docker G17 PASS for **all twelve** + prior regression (**71**/71).

## Wave Cx-6 (M20 claimed set)

Plan: `plan:root/20261110060000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| OBJ-008 | Heavy hash-object | load class | Honesty OBJ-008 | yes | `verify/g17` |
| PACK-002 | Plant MIDX | MIDX present | Inspect PACK-002 | yes | `verify/g17` |
| PACK-003 | Plant bitmap | bitmap present | Inspect PACK-003 | yes | `verify/g17` |
| WT-009 | Plant repair.lock | dual repair class | Refuse WT-009 | yes | `verify/g17` |
| WT-010 | worktreeConfig mix | config diverge | Honesty WT-010 | yes | `verify/g17` |
| SUB-003 | modules/ present | absorb class | Honesty SUB-003 | yes | `verify/g17` |
| SUB-009 | `.gitmodules.lock` | sync fails | Refuse SUB-009 | yes | `verify/g17` |
| SEQ-004 | Plant rr-cache.lock | merge/rr class | Refuse SEQ-004 | yes | `verify/g17` |
| CORR-004 | filters + prune=now | prune class | Refuse CORR-004 | yes | `verify/g17` |
| CORR-005 | shallow.lock | deepen class | Refuse CORR-005 | yes | `verify/g17` |
| LOCK-007 | same-FS rename | rename class | Inspect LOCK-007 | yes | `verify/g17` |
| MISC-020 | occupied --prefix | collision class | Refuse MISC-020 | yes | `verify/g17` |

Wave Cx-6 **Done** = Docker G17 PASS for **all twelve** + prior regression (**83**/83).

## Wave Cx-7 (M20 claimed set)

Plan: `plan:root/20261110070000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| FSM-001 | lock inspect | daemon start class | Inspect FSM-001 | yes | `verify/g17` |
| FSM-002 | Plant FSM transition marker | transition class | Refuse FSM-002 | yes | `verify/g17` |
| FSM-004 | Plant incomplete FSM marker | crash/stampede class | Refuse FSM-004 | yes | `verify/g17` |
| IDX-005 | Plant untracked-cache.lock | expand race class | Refuse IDX-005 | yes | `verify/g17` |
| MISC-008 | Plant sparse-checkout.lock; add | materialize race | Refuse MISC-008 | yes | `verify/g17` |
| SHAL-004 | Plant shallow + grafts | boundary class | Inspect SHAL-004 | yes | `verify/g17` |
| SPARSE-002 | Non-cone sparse init | non-cone / expand | Refuse SPARSE-002 | yes | `verify/g17` |
| PACK-005 | Plant *.keep / cruft | maint race class | Inspect PACK-005 | yes | `verify/g17` |
| MAINT-005 | Plant commit-graph.lock | verify-vs-rewrite | Refuse MAINT-005 | yes | `verify/g17` |
| CORR-003 | Plant refs/replace.lock | replace-refs class | Refuse CORR-003 | yes | `verify/g17` |
| CI-001 | Plant index.lock (shared WS) | lock storm class | Inspect CI-001 | yes | `verify/g17` |
| FS-008 | lock inspect (ignorecase) | case-fold class | Inspect FS-008 | yes | `verify/g17` |

Wave Cx-7 **Done** = Docker G17 PASS for **all twelve** + prior regression (**95**/95).

## Wave Cx-8 (M20 claimed set)

Plan: `plan:root/20261110080000`.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| CI-005 | Leftover index.lock | reusable WS class | Inspect CI-005 | yes | `verify/g17` |
| IDX-006 | lock inspect | racy-git class | Inspect IDX-006 | yes | `verify/g17` |
| FSM-003 | lock inspect | overflow class | Inspect FSM-003 | yes | `verify/g17` |
| REF-003 | packed-refs.lock + delete | delete race | Refuse REF-003 | yes | `verify/g17` |
| MAINT-006 | multi-pack-index.lock | MIDX race | Refuse MAINT-006 | yes | `verify/g17` |
| MISC-001 | describe | reader class | Honesty MISC-001 | yes | `verify/g17` |
| MISC-009 | clone --reference + donor gc.lock | missing alt class | Refuse MISC-009 | yes | `verify/g17` |
| OBJ-006 | plant alternates | borrow class | Inspect OBJ-006 | yes | `verify/g17` |
| CORR-006 | alternates + prune=now | dangling class | Refuse CORR-006 | yes | `verify/g17` |
| CORR-007 | replace refs + prune=now | replace prune | Refuse CORR-007 | yes | `verify/g17` |
| NET-010 | plant filters | filter mismatch | Inspect NET-010 | yes | `verify/g17` |
| PACK-006 | lock inspect | window/memory | Inspect PACK-006 | yes | `verify/g17` |

Wave Cx-8 **Done** = Docker G17 PASS for **all twelve** + prior regression (**107**/107).

## Wave Dz-1 (M21 claimed set)

Plan: `plan:root/20261110140000`. Host-FS / platform honesty (Docker-limited; lockfile same-FS cite).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| FS-001 | lock inspect | NFS lock/rename class | Inspect FS-001 | yes | `verify/g17` |
| FS-002 | lock inspect | SMB lock residue class | Inspect FS-002 | yes | `verify/g17` |
| FS-003 | lock inspect | Win sharing class | Inspect FS-003 | yes | `verify/g17` |
| FS-004 | lock inspect | AV rename-hold class | Inspect FS-004 | yes | `verify/g17` |
| FS-005 | lock inspect | WSL/NTFS class | Inspect FS-005 | yes | `verify/g17` |
| FS-009 | lock inspect | NFD/NFC class | Inspect FS-009 | yes | `verify/g17` |
| FS-010 | lock inspect | disk-full rename class | Inspect FS-010 | yes | `verify/g17` |
| FS-011 | lock inspect | overlayfs class | Inspect FS-011 | yes | `verify/g17` |
| FS-012 | lock inspect | FUSE/virtiofs class | Inspect FS-012 | yes | `verify/g17` |
| FS-013 | lock inspect | inode exhaustion class | Inspect FS-013 | yes | `verify/g17` |
| WIN-001 | lock inspect | MAX_PATH class | Inspect WIN-001 | yes | `verify/g17` |
| WIN-002 | lock inspect | delete-pending class | Inspect WIN-002 | yes | `verify/g17` |

Wave Dz-1 **Done** = Docker G17 PASS for **all twelve** + prior regression (**119**/119).

## Wave Dz-2 (M21 claimed set)

Plan: `plan:root/20261110140000`. Lock/ref/object/sub/misc/hook/CI honesty.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| LOCK-005 | lock inspect | lockfilePid class | Inspect LOCK-005 | yes | `verify/g17` |
| LOCK-006 | lock inspect | writers-only class | Inspect LOCK-006 | yes | `verify/g17` |
| LOCK-008 | lock inspect | manual delete class | Inspect LOCK-008 | yes | `verify/g17` |
| LOCK-009 | lock inspect | NFS lease steal class | Inspect LOCK-009 | yes | `verify/g17` |
| REF-013 | lock inspect | reftable unclaimed class | Inspect REF-013 | yes | `verify/g17` |
| REF-020 | lock inspect | NFS dangling lock class | Inspect REF-020 | yes | `verify/g17` |
| OBJ-007 | lock inspect | quarantine class | Inspect OBJ-007 | yes | `verify/g17` |
| OBJ-009 | lock inspect | batch-check stampede | Inspect OBJ-009 | yes | `verify/g17` |
| SUB-005 | lock inspect | fsmonitor submodule noise | Inspect SUB-005 | yes | `verify/g17` |
| MISC-013 | lock inspect | quarantine fan-in | Inspect MISC-013 | yes | `verify/g17` |
| HOOK-007 | lock inspect | receive-hook scratch | Inspect HOOK-007 | yes | `verify/g17` |
| CI-002 | lock inspect | cache restore tear | Inspect CI-002 | yes | `verify/g17` |

Wave Dz-2 **Done** = Docker G17 PASS for **all twelve** + prior regression (**131**/131 intermediate; carrying **145**/145).

## Wave Dz-3 (M21 claimed set)

CRED/NET/LFS Docker-local honesty unlock (LFS CLI exclusion unchanged).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| CRED-001 | lock inspect | platform store race | Inspect CRED-001 | yes | `verify/g17` |
| CRED-002 | lock inspect | cache socket race | Inspect CRED-002 | yes | `verify/g17` |
| NET-003 | lock inspect | HTTP pool exhaustion | Inspect NET-003 | yes | `verify/g17` |
| NET-004 | lock inspect | SSH mux contention | Inspect NET-004 | yes | `verify/g17` |
| NET-005 | lock inspect | ls-refs stale tip | Inspect NET-005 | yes | `verify/g17` |
| NET-008 | lock inspect | askPass headless hang | Inspect NET-008 | yes | `verify/g17` |
| NET-011 | lock inspect | atomic vs non-atomic | Inspect NET-011 | yes | `verify/g17` |
| LFS-001 | lock inspect | shared lfs.storage | Inspect LFS-001 | yes | `verify/g17` |
| LFS-003 | lock inspect | filter backpressure | Inspect LFS-003 | yes | `verify/g17` |
| LFS-004 | lock inspect | prune vs checkout | Inspect LFS-004 | yes | `verify/g17` |
| LFS-005 | lock inspect | concurrent push OID | Inspect LFS-005 | yes | `verify/g17` |
| LFS-006 | lock inspect | missing git-lfs | Inspect LFS-006 | yes | `verify/g17` |

Wave Dz-3 **Done** = included in carrying **145**/145.

## Wave Dz-4 (M21 claimed set)

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| CI-006 | lock inspect | sparse checkout fight | Inspect CI-006 | yes | `verify/g17` |
| TOCTOU-004 | lock inspect | credential TOCTOU | Inspect TOCTOU-004 | yes | `verify/g17` |

Wave Dz-4 **Done** = Docker G17 PASS (**145**/145). Catalog **Deferred=0**.

## Explicit dispositions (not claimed as green)

See disposition register in `.spec/sot/multi-agent-git-concurrency-scenarios.sot.md`. M21 exit: **Deferred=0 Claimed=212 Refuse=0 N/A=9**. **Not** marketing “all races solved.”

## Evidence schema (receipt body)

Per ID: `scenario_id`, `git_outcome` (`pathology_pass` \| `fail`), `arks_outcome` (`resolved_pass` \| `fail`), `notes`. Aggregate: compose image pin, stock git version, tree SHA, gix-exit.
