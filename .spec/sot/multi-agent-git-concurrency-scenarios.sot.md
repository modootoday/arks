---
id: sot:root/multi-agent-git-concurrency-scenarios
title: Multi-agent Git concurrency scenario catalog
status: active
created: 2026-11-09T02:31:18Z
updated: 2026-11-10T14:00:00Z
tags: [concurrency, multi-agent, lease, git-compat, performance, scenarios, g17, g18, m19, m20, m21, cx, cd, dz, deferred-zero]
verify: manual
rel:
  depends_on:
    - sot:root/lease-concurrency-intent
    - sot:root/arks-lease
    - sot:root/git-compat-surface
    - sot:root/storage-celir-boundary
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - plan:root/20261110010000
    - plan:root/20261110030000
    - plan:root/20261110070000
    - plan:root/20261110080000
    - plan:root/20261110090000
    - plan:root/20261110140000
    - adr:root/20261108232800
    - research:root/20261108083502
    - research:root/20261109023117
    - research:root/20261110021546
  evidence:
    - evidence:root/20261110010000
    - evidence:root/20261110020000
    - evidence:root/20261110030000
    - evidence:root/20261110070000
    - evidence:root/20261110080000
    - evidence:root/20261110090000
---

# Multi-agent Git concurrency scenario catalog

Version **0.1.0**. Authoritative **scenario ID catalog** for multi-agent / high-N Git concurrency failure modes, stock-Git behavior, and arks opportunity classification. Evidence and citations: [[research:root/20261109023117]]. Product coordination intent: [[sot:root/lease-concurrency-intent]]. Unit runtime: [[sot:root/arks-lease]].

## Rule

### Scope

1. This SoT **catalogs** concurrency issue scenarios that arise when **dozens–hundreds of agents** use Git (or Git-compatible tooling) concurrently.
2. Each row has a stable **Scenario ID**, **class**, failure mode, symptoms, stock Git behavior summary, severity, qualitative frequency-at-N, high-level reproduction sketch, and **arks opportunity class**.
3. Opportunity classes (normative vocabulary):
   - **`mitigate-with-compat`** — improve reliability/UX while still honoring documented Git lock/bytes/semantics on Git-compat paths.
   - **`differentiate-beyond-git`** — product capability stock Git lacks or handles poorly (leases, sharded/batched writers, agent profiles, opt-in L3 pipeline shapes) — still subject to dual-path / export rules in [[sot:root/lease-concurrency-intent]].
   - **`out-of-scope`** — acknowledge but do not own as arks product semantics (e.g. cloud-sync folders, forge-specific shadow refs).

### Explicit non-claims (honesty)

1. This SoT does **not** redefine **M9 / M10 / M11** production exits ([[sot:root/program-roadmap]], [[sot:root/production-residual-register]]).
2. This SoT does **not** claim absolute latency (ms) or throughput SLOs. Comparative ≥20% is **M14/G19 only**, and **only after** M12/M13 issue-resolution closes — not parallel Done with the issue track.
3. **CELIR** remains **excluded** from product Cargo deps unless a future SoT opens it ([[sot:root/storage-celir-boundary]]).
4. **Lease default vs L3 opt-in** honesty is preserved: L1/L2 leases = default product capability (ship timing M10); L3 N-way high-concurrency pipeline = **opt-in** and currently **refuse-closed** for production bars ([[adr:root/20261108232800]], [[sot:root/lease-concurrency-intent]]).
5. Catalog rows are **not** automatic implementation requirements until claimed. Selecting a row for build needs an owning plan/ADR and bottom-up readiness. Claimed rows schedule into **M12 (mitigate / G17)**, **M13 (differentiate / G18)**, expansion **M19/M20 (Cx/Cd)**, or **M21 (Dz / Deferred=0)** per [[sot:root/program-roadmap]]. The **221** count is a research taxonomy — marketing “all races fixed” is still forbidden; **M21 Done** requires every ID terminal with **Deferred=0**.
6. Do **not** invent Git lock-file dialects on Git-compat paths to “fix” a row.
7. **Issue-resolution Done** for a claimed row requires Docker clean-room **G17/G18**: induce fault; stock **git** exhibits pathology; **arks** meets SoT resolved criteria. SoT/code claim or host demo alone is insufficient.
8. **Terminal dispositions only (M21 Done bar):** **Claimed+clean-room-closed**, **Refuse-closed (L3-class only, ADR+SoT)**, or **N/A-out-of-scope (primary-doc cite)**. **Closed-deferred is not a terminal success disposition** — it is an in-progress work-queue label only. Forbidden premature stops: bucket B exhausted, Docker-induce-thin, policy/G8 later, keep-active-for-humans ([[plan:root/20261110140000]]).

### Claimed-set ledger (honesty)

| Milestone / wave | Gate | Claimed IDs (count) | Status |
|------------------|------|---------------------|--------|
| M12 Cm-1..Cm-3 | G17 | AGENT-006/IDX-008, LOCK-002, WT-001, SUB-001, IDX-001, HOOK-001/HOOK-006, PACK-004/MAINT-003, AGENT-003, IDX-002, AGENT-005, AGENT-007, CORR-001, FS-006 (**13**) | Done |
| M13 Cd-1..Cd-3 | G18 | REF-001, REF-006, REF-011, AGENT-003, LOCK-002, WT-001, SUB-001, AGENT-007, HOOK-001/HOOK-006, PACK-004/MAINT-003, AGENT-002, IDX-001 (**12**) | Done |
| M19 Cx-1 | G17 | REF-007, LOCK-001, WT-002, WT-004, SEQ-001, SEQ-002, SEQ-003, REF-018, MISC-012, TOCTOU-005 (**10**) | Done — [[evidence:root/20261110010000]] |
| M19 Cx-2 | G17 | BISECT-001, BISECT-002, SEQ-006, SEQ-007, MISC-007, STASH-001, STASH-002, REF-021, TOCTOU-003, MISC-005, CORR-002, LOCK-004 (**12**) | Done — [[evidence:root/20261110020000]] |
| M20 Cx-3 | G17 | REF-012, REF-016, IDX-014, SPARSE-001, MISC-018, SUB-008, TOCTOU-007, PACK-009, MAINT-002, NET-009, CFG-004, MISC-017 (**12**) | Done — [[evidence:root/20261110030000]] |
| M20 Cx-4 | G17 | REF-009, REF-022, IDX-013, IDX-010, HOOK-002, HOOK-005, MISC-014, MISC-011, SHAL-001, WT-006, SUB-006, PACK-008 (**12**) | Done — [[evidence:root/20261110040000]] |
| M20 Cx-5 | G17 | IDX-004, IDX-011, HOOK-003, MISC-019, OBJ-001, OBJ-005, PACK-001, WT-005, LOCK-003, MISC-002, MISC-003, SHAL-003 (**12**) | Done — [[evidence:root/20261110050000]] |
| M20 Cx-6 | G17 | OBJ-008, PACK-002, PACK-003, WT-009, WT-010, SUB-003, SUB-009, SEQ-004, CORR-004, CORR-005, LOCK-007, MISC-020 (**12**) | Done — [[evidence:root/20261110060000]] |
| M20 Cx-7 | G17 | FSM-001, FSM-002, FSM-004, IDX-005, MISC-008, SHAL-004, SPARSE-002, PACK-005, MAINT-005, CORR-003, CI-001, FS-008 (**12**) | Done — [[evidence:root/20261110070000]] |
| M20 Cx-8 | G17 | CI-005, IDX-006, FSM-003, REF-003, MAINT-006, MISC-001, MISC-009, OBJ-006, CORR-006, CORR-007, NET-010, PACK-006 (**12**) | Done — [[evidence:root/20261110080000]] |
| M20 Cd-4 | G18 | AGENT-001, AGENT-010, AGENT-011, REF-002, REF-004, REF-008, REF-010, IDX-003, IDX-007, SEQ-005, WT-003, MAINT-001 (**12**) | Done — [[evidence:root/20261110090000]] |
| M20 Cd-5 | G18 | AGENT-008, AGENT-009, AGENT-012, REF-005, REF-014, REF-015, IDX-009, IDX-012, WT-007, WT-008, CFG-001, TOCTOU-001 (**12**) | Done — [[evidence:root/20261110100000]] |
| M20 Cd-6 | G18 | HOOK-004, CFG-003, REF-019, REF-023, OBJ-002, OBJ-003, SUB-002, SUB-004, SUB-007, TOCTOU-002, TOCTOU-006, MISC-004 (**12**) | Done — [[evidence:root/20261110110000]] |
| M20 Cd-7 | G18 | MISC-010, MISC-015, MISC-016, SUB-010, SHAL-002, PACK-007, CI-003, CI-004, NET-001, NET-002, NET-007, LFS-002 (**12**) | Done — [[evidence:root/20261110120000]] |
| M20 Cd-8 | G18 | PERF-001..PERF-010, CFG-002, AGENT-004 (**12**) | Done — [[evidence:root/20261110130000]] |
| M21 Dz-1 | G17 | FS-001..FS-005, FS-009..FS-013, WIN-001, WIN-002 (**12**) | Done — [[evidence:root/20261110141000]] |
| M21 Dz-2 | G17 | LOCK-005/006/008/009, REF-013/020, OBJ-007/009, SUB-005, MISC-013, HOOK-007, CI-002 (**12**) | Done — [[evidence:root/20261110142000]] |
| M21 Dz-3 | G17 | CRED-001/002, NET-003/004/005/008/011, LFS-001/003/004/005/006 (**12**) | Done — [[evidence:root/20261110143000]] |
| M21 Dz-4 | G17 | CI-006, TOCTOU-004 (**2**) | Done — [[evidence:root/20261110144000]] |

**Unique claimed IDs (G17∪G18):** **212**. Suite totals: G17 **145**/145 dual PASS after Dz-4; G18 **72**/72 dual PASS after Cd-8. **M21 Deferred=0** achieved.

### Catalog disposition register (no bare unclaimed Opens)

Every catalog ID has exactly one disposition. **Dispose ≠ “all Git races fixed.”** Only Claimed+clean-room-closed rows may be marketed as issue-resolved under G17/G18 Done rules. Promotion map: [[research:root/20261110021546]]. **M21 Done bar:** [[plan:root/20261110140000]].

| Disposition | Count (M21 exit) | Meaning |
|-------------|------:|---------|
| **Claimed+clean-room-closed** | **212** | Explicit wave claim + Docker G17/G18 dual PASS — **terminal** |
| **N/A-out-of-scope** | **9** | Opportunity `out-of-scope` + primary-doc cite — **terminal** |
| **Refuse-closed** | **0** catalog rows (+ L3 product valve) | **L3-class only** per [[adr:root/20261108232800]] / [[sot:root/lease-concurrency-intent]] — **terminal** |
| **Closed-deferred** | **0** | **M21 exit satisfied** — not used as terminal success |

Honesty: `Deferred=0 Claimed=212 Refuse=0 N/A=9` (catalog rows; L3 product valve remains refuse-closed separately).

#### N/A-out-of-scope (9) — historical M21 exit list

`FS-007`, `FS-014`, `HOOK-008`, `MAINT-004`, `MISC-006`, `NET-006`, `OBJ-004`, `REF-017`, `WIN-003`

**FS reopen honesty (does not invent M21 gate PASS):** residual [[sot:root/production-residual-register]] + [[adr:root/20261110214000]] / [[adr:root/20261110214800]] Claimed slices for `HOOK-008` / `MAINT-004` / `MISC-006` / `NET-006` / `OBJ-004` (incl. sha256 porcelain) / `REF-017` / FW-LFS (incl. network batch/server). Remaining true host/env **Impossible**: `FS-007`, `FS-014`, `WIN-003`. M21 arithmetic above stays historical.

#### M21 work queue remaining

*(empty — Deferred=0)*

**Bare unclaimed Open count:** **0**. **Deferred count:** **0**.

### Severity legend

| Code | Meaning |
|------|---------|
| **S0** | Corruption / dual-writer / object loss risk |
| **S1** | Wrong state / lost update / inconsistent view |
| **S2** | Hard fail (fatal / rejected op) |
| **S3** | Soft fail / retryable / degraded correctness signal |
| **S4** | Performance / throughput only |

### Frequency-at-N legend (qualitative)

| Label | Meaning at N≈dozens–hundreds |
|-------|------------------------------|
| **dominant** | Expected primary bottleneck or failure mode |
| **common** | Regularly observed under dense agent load |
| **occasional** | Needs specific timing/FS/feature combo |
| **rare** | Edge / historical / hostile |

### Counts (v0.1.0)

| Metric | Value |
|--------|------:|
| Total scenarios | 221 |

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

| Opportunity class | Count |
|-------------------|------:|
| `differentiate-beyond-git` | 84 |
| `mitigate-with-compat` | 128 |
| `out-of-scope` | 9 |

### Top differentiation candidates (non-exhaustive)


| Rank | IDs | Why it differentiates |
|-----:|-----|-----------------------|
| 1 | IDX-001, IDX-008, IDX-002, AGENT-006 | Single `index.lock` + optional status locks dominate multi-agent local write/read storms; stock Git has no sharded index writer path |
| 2 | REF-001, REF-006, REF-011, NET-002, AGENT-002 | Same-branch tip CAS / lock storms and uncoordinated agent ref namespaces; no first-class lease/namespace scheduler |
| 3 | LOCK-002, AGENT-005, FS-006 | Stale locks after hard kill / container eviction / dual host+container writers — stock cleanup is process-local atexit |
| 4 | PERF-001, PERF-006, AGENT-003 | Process-per-command + blind retry herds; native engine + coordinated backoff/batch are product-shaped |
| 5 | WT-001, SUB-001, AGENT-007 | Worktree-per-agent still shares common store; submodule multiplies lock domains — composition poorly supported |
| 6 | HOOK-001, HOOK-006, AGENT-008 | Hook amplification inside exclusive critical sections |
| 7 | PACK-004, MAINT-003, CORR-001, OBJ-003 | auto-gc / prune coupling under write storms — coordination with writers is weak |
| 8 | NET-001, NET-007, SHAL-002 | Fetch/promisor/credential stampedes at N agents |
| 9 | AGENT-001, SEQ-005, MISC-004 | Shared worktree as blackboard / conflict / clean races — needs grants beyond porcelain |
| 10 | LFS-002 + IDX/REF family | Content locking vs Git metadata locking mismatch for large-asset agent workflows |


### Scenario catalog

### Class `agent` (8)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| AGENT-001 | agent | Shared checkout as multi-agent blackboard | Dozens of agents use one WT as message bus | Index/ref/file chaos; non-reproducible state | Stock Git has no agent coordination primitive | S1 | dominant | Multi-agent coding framework; one clone | differentiate-beyond-git |
| AGENT-002 | agent | Branch-per-agent without merge coordinator | Fan-in merge conflicts explode | Human/bot merge queue collapse | Git supports branches; not agent merge scheduler | S2 | dominant | Hundreds of agent branches merge to main | differentiate-beyond-git |
| AGENT-003 | agent | Optimistic concurrency without leases | Retry loops amplify lock storms | Thundering herd on index.lock/ref.lock | No backoff protocol in porcelain | S4 | dominant | Agents retry commit on lock fail immediately | differentiate-beyond-git |
| AGENT-004 | agent | Cross-host agents on network FS repo | Combined FS + lock + NFS issues | Corruption + stale locks | Git not designed for multi-host WT on NFS | S0 | occasional | Two hosts mount same repo NFS | mitigate-with-compat |
| AGENT-005 | agent | Ephemeral container agents leave stale locks | Container killed; lock remains on volume | Next pod cannot write | Lock cleanup assumes process atexit on same node | S2 | common | K8s kill pod mid-git; PVC keeps lock | differentiate-beyond-git |
| AGENT-006 | agent | Read-only mirror agents still take optional locks | status refresh writes index | Writers fail due to readers | --no-optional-locks exists but not default for agents | S2 | dominant | Many status-only agents without flag | differentiate-beyond-git |
| AGENT-007 | agent | Submodule + worktree + agent matrix | Combination multiplies lock domains | Hard-to-debug multi-lock failures | Composition of WT-*/SUB-* | S2 | common | Worktree-per-agent with submodules | differentiate-beyond-git |
| AGENT-008 | agent | Policy hooks that call network from pre-commit | Hook latency × N agents | Global throughput collapse | Hooks are sync in critical path | S4 | common | pre-commit curls policy service | differentiate-beyond-git |

### Class `bisect` (2)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| BISECT-001 | bisect | Concurrent bisect state mutation | BISECT_* refs and meta files shared | Broken bisect session | git-bisect state in $GIT_DIR | S1 | rare | Two bisects same repo | mitigate-with-compat |
| BISECT-002 | bisect | Bisect checkout vs agent dirty tree | Checkout fails mid-bisect | Stuck bisect | bisect + dirty worktree rules | S2 | occasional | Agent writes files during bisect | mitigate-with-compat |

### Class `ci` (4)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| CI-001 | ci | Matrix jobs share cloned workspace (misconfig) | Parallel jobs same checkout dir | index.lock storms; corruption | CI hygiene; not Git feature | S0 | occasional | Reusable workspace without isolation | mitigate-with-compat |
| CI-002 | ci | Cache restore of .git across concurrent jobs | Partial cache extract while job runs | Corrupt packs/refs | CI cache patterns | S0 | occasional | Warm cache mid-job | mitigate-with-compat |
| CI-003 | ci | Parallel test suites init same fixture bare | Shared bare remote fixture | Push CAS fails flaky tests | Test design | S2 | common | pytest-xdist style shared remote | differentiate-beyond-git |
| CI-004 | ci | Namespace collision of agent branch names in PR bots | Bots use same temp branch | Force-push races; lost reviews | Hosting + ref naming | S1 | common | Many bots push refs/heads/bot/fix | differentiate-beyond-git |

### Class `config` (4)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| CFG-001 | config | Concurrent config file rewrite | git config writes via lock; last writer wins logically | Lost updates to different keys if non-transactional merge absent | git-config file locking | S1 | common | Agents set different keys concurrently | differentiate-beyond-git |
| CFG-002 | config | IncludeIf / conditional config TOCTOU | Config view changes between read and use | Unexpected identity/remote | includeIf | S1 | occasional | Change branch; includeIf by branch; concurrent ops | mitigate-with-compat |
| CFG-003 | config | Global ~/.gitconfig races across agents on host | Shared user config mutated | Cross-agent identity bleed | system/global/local precedence | S1 | common | Agents rewrite user.name concurrently | differentiate-beyond-git |
| CFG-004 | config | extensions.* unknown causes hard refuse | One agent enables extension others cannot read | Other processes MUST NOT proceed (format v1) | repository format version 1 rules | S2 | rare | Write unknown extension; second tool opens repo | mitigate-with-compat |

### Class `corruption` (5)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| CORR-001 | corruption | gc prune=now with concurrent writers | Prune deletes object another process will reference | Missing objects; fsck fail; possible silent corruption | git-gc NOTES; --prune=now warning | S0 | occasional | gc --prune=now during commits | differentiate-beyond-git |
| CORR-002 | corruption | Forced overlapping gc (--force) | Two gc instances run | Pack/prune interference | git-gc --force | S0 | rare | Two agents gc --force | mitigate-with-compat |
| CORR-003 | corruption | Manual lock removal during live writer | See LOCK-008 amplified | Torn index/refs | Operational anti-pattern | S0 | occasional | Delete lock under live writer + second writer | differentiate-beyond-git |
| CORR-004 | corruption | Partial clone + prune misunderstanding | Local gc removes objects thought unreachable but needed lazily | Later fetch fails; broken checkout | partial clone + gc interactions | S1 | occasional | Aggressive gc on partial clone under agents | mitigate-with-compat |
| CORR-005 | corruption | Shallow file concurrent deepen | Concurrent fetch --deepen rewrite shallow | Corrupt/inconsistent shallow boundary | layout shallow file | S1 | occasional | Parallel deepen on same repo | mitigate-with-compat |

### Class `credential` (2)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| CRED-001 | credential | osxkeychain / wincred concurrent store | Platform store locking | Helper errors; duplicate entries | credential helpers | S3 | occasional | N agents approve same host | mitigate-with-compat |
| CRED-002 | credential | cache helper socket contention | credential-cache daemon overload | Timeouts; fallback prompt | git-credential-cache | S3 | common | Burst auth from agents | mitigate-with-compat |

### Class `fs` (11)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| FS-001 | fs | NFS non-atomic rename / lock semantics | NFS may break lock/rename assumptions | Stale locks; torn files; phantom exists | Community/ops knowledge; lockfile assumes POSIX local | S0 | occasional | Repo on NFS; concurrent writers | mitigate-with-compat |
| FS-002 | fs | SMB/CIFS lock residue after network blip | Lock create succeeds; unlock delete lost | Permanent index.lock / ref.lock | Observed SMB Git lock residue reports | S2 | common | SMB mount; disconnect mid-op | mitigate-with-compat |
| FS-003 | fs | Windows mandatory file locking / sharing violation | Cannot replace/delete file while handle open | Access denied; sharing violation; stuck LFS/index | Win32 sharing rules; LFS issues | S2 | dominant | Antivirus holds handle during rename | differentiate-beyond-git |
| FS-004 | fs | Antivirus / Windows Defender scan during rename | AV locks `.lock` or target during commit_lock_file | Stale locks; flaky CI | Widely reported AV+Git interactions | S2 | common | Real-time AV on .git; parallel ops | mitigate-with-compat |
| FS-005 | fs | WSL↔NTFS metadata mismatch | Linux Git on /mnt/c sees weird mtimes/permissions | Racy status; chmod noise; lock weirdness | WSL filesystem notes | S3 | common | Agents in WSL on NTFS working tree | mitigate-with-compat |
| FS-006 | fs | Docker bind-mount lock visibility | Locks on host vs container namespaces | Dual writers if host+container both operate | Bind mount semantics | S0 | occasional | Host git and container git same bind mount | differentiate-beyond-git |
| FS-007 | fs | Cloud sync (OneDrive/Dropbox) on .git | Sync conflicts duplicate/lock files | Repo corruption | Strong community prohibition | S0 | occasional | Repo inside synced folder; multi device | out-of-scope |
| FS-008 | fs | case-insensitive FS path collisions | Agents create Foo vs foo | Checkout failures; silent overwrite | core.ignorecase | S1 | occasional | Parallel adds differing only by case on Windows/macOS | mitigate-with-compat |
| FS-009 | fs | Unicode normalization (NFD/NFC) races | macOS NFD vs NFC path names | Duplicate logical paths; fsmonitor issues | macOS Unicode; fsmonitor NFC notes | S1 | occasional | Agents create differently normalized names | mitigate-with-compat |
| FS-010 | fs | Disk full during lock commit rename | rename fails; rollback | Failed op; possible leftover lock | api-lockfile commit failure path | S2 | rare | Fill disk; concurrent commits | mitigate-with-compat |
| FS-011 | fs | Container overlayfs / copy-up costs | Many small object creates amplify overlay | Extreme latency; ENOSPC on upper | overlayfs behavior | S4 | common | Hundreds agents in overlay-backed clone | mitigate-with-compat |

### Class `fsmonitor` (3)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| FSM-001 | fsmonitor | Daemon start races | Multiple clients start daemon | Brief failures; duplicate start attempts | fsmonitor--daemon auto-start | S3 | occasional | Cold start; N status | mitigate-with-compat |
| FSM-002 | fsmonitor | Hook-based fsmonitor vs builtin daemon mix | Mixed clients disagree | Wrong dirty set | core.fsmonitor hook vs true | S1 | rare | Some agents set hook path; others true | mitigate-with-compat |
| FSM-003 | fsmonitor | Event overflow under write storm | Kernel/queue drops events | False clean; next command full scan | platform FS events limits | S3 | common | Generate > queue capacity changes | mitigate-with-compat |

### Class `hook` (6)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| HOOK-001 | hook | pre-commit storm under N agents | Each commit runs hooks; CPU/IO amplification | Queueing; timeouts; false flake | githooks pre-commit | S4 | dominant | N concurrent commits with heavy lint hook | differentiate-beyond-git |
| HOOK-002 | hook | pre-auto-gc hook during auto housekeeping | gc --auto invokes pre-auto-gc | Hook failure blocks housekeeping; surprises agents | git-gc HOOKS; githooks | S3 | occasional | Trigger gc.auto with failing pre-auto-gc | mitigate-with-compat |
| HOOK-003 | hook | post-index-change hook races | Fired on index write; concurrent invocations | External indexers thrash | githooks post-index-change | S4 | common | Enable hook; parallel add | mitigate-with-compat |
| HOOK-004 | hook | Server update/pre-receive hook serialization | receive-pack runs hooks per push | Push latency; hook-side locking deadlocks | githooks update / pre-receive | S2 | common | Parallel pushes to bare with heavy update hook | differentiate-beyond-git |
| HOOK-005 | hook | Hook modifies index while another writer active | Hook runs git add; contends index.lock | Nested lock failure or deadlock-like waits | Hook environment + index lock | S2 | occasional | pre-commit that stages files under concurrent agents | mitigate-with-compat |
| HOOK-006 | hook | reference-transaction hook storms | Every ref update notifies hook | Hook process storms at N agents | githooks reference-transaction | S4 | common | Enable reference-transaction; many updates | differentiate-beyond-git |

### Class `index` (12)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| IDX-001 | index | index.lock exclusive writer | Only one process may rewrite index | `Unable to create index.lock` | api-lockfile + index write path | S2 | dominant | Parallel git add/commit/status-refresh | differentiate-beyond-git |
| IDX-002 | index | Background status optional lock conflict | status refreshes index and takes lock | Foreground writers fail; background status holds lock | git-status BACKGROUND REFRESH; --no-optional-locks | S2 | dominant | IDE/agent loops `git status` while other agents add | differentiate-beyond-git |
| IDX-003 | index | Temporary index files (GIT_INDEX_FILE) | Agents share default index instead of private temp | Cross-contamination of staged sets | GIT_INDEX_FILE / read-tree patterns | S1 | common | Two agents stage different paths without private index | differentiate-beyond-git |
| IDX-004 | index | Split-index sharedindex races | Concurrent updates of sharedindex.* and index | Split-index desync; fallback/full rewrite storms | layout sharedindex.*; split-index mode | S1 | occasional | Enable splitIndex; N concurrent adds | mitigate-with-compat |
| IDX-005 | index | Sparse-index expand under concurrency | One command expands sparse index while another writes | index.lock + ensure_full_index races; fsmonitor reset | sparse-index + fsmonitor integration notes | S2 | common | Sparse cone; agents touch outside cone concurrently | mitigate-with-compat |
| IDX-006 | index | Untracked-cache / racy-git false dirtiness | Concurrent mtime updates fool racy-git checks | Spurious modified files; thrashing refresh | racy-git / untracked cache docs | S3 | common | Many agents touch files then status | mitigate-with-compat |
| IDX-007 | index | Parallel add of same path | Two stages of same path last-writer-wins after serial locks | Lost intended stage content | index entry replace under lock serialization | S1 | common | Agents write different content then add same path | differentiate-beyond-git |
| IDX-008 | index | Parallel add of disjoint paths still serialize | Disjoint path updates share one index.lock | Throughput collapse despite no logical conflict | Single index file lock model | S4 | dominant | N agents add unique files | differentiate-beyond-git |
| IDX-009 | index | commit holds index.lock across hooks | Long pre-commit extends exclusive critical section | Other agents blocked for entire hook duration | commit sequence + githooks | S4 | common | Slow pre-commit; parallel status/add | differentiate-beyond-git |
| IDX-010 | index | Skip-worktree / assume-unchanged bit races | Concurrent bit flips lost | Unexpected checkout/skip behavior | index flags + update-index | S1 | occasional | Agents toggle skip-worktree on overlapping paths | mitigate-with-compat |
| IDX-011 | index | Index version / extension rewrite races | Concurrent writers disagree on extensions present | Extension drop/add churn | index format extensions | S1 | rare | Mix tools with/without fsmonitor extension | mitigate-with-compat |
| IDX-012 | index | Per-worktree index isolation vs shared object store | Agents in different worktrees OK on index; still contend on refs/objects | Index OK but tip CAS fails | worktree private index; common objects/refs | S2 | common | Worktree-per-agent; shared branch tip | differentiate-beyond-git |

### Class `lfs` (4)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| LFS-001 | lfs | Shared lfs.storage concurrent download race | Multiple processes write same OID object | Sharing violation; smudge errors (esp Windows) | git-lfs issues #2825 / PRs #3873 #6215 | S2 | common | Shared lfs.storage; parallel checkout/fetch | mitigate-with-compat |
| LFS-002 | lfs | LFS lock API vs Git ref locks confusion | File locks on server ≠ Git index.lock | Agents assume exclusive content lock incorrectly | git-lfs locks feature | S1 | occasional | Two agents edit same LFS-tracked path | differentiate-beyond-git |
| LFS-003 | lfs | Filter process / long-running filter concurrency | Single filter process multiplex; backpressure | Hang; protocol errors | long-running process filter | S2 | common | Parallel checkout of many LFS files | mitigate-with-compat |
| LFS-004 | lfs | LFS prune while another checkout | Prune removes local object needed by peer | Smudge re-download storms; errors | git lfs prune | S1 | occasional | lfs prune + concurrent checkout | mitigate-with-compat |

### Class `lock` (8)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| LOCK-001 | lock | Exclusive O_CREAT\|O_EXCL writer lock | Second writer cannot create `.lock` while first holds it | `Unable to create '…/.lock': File exists`; fatal exit | api-lockfile: mutual exclusion via exclusive create; readers not blocked | S2 | dominant | N writers call update that takes same path lock concurrently | differentiate-beyond-git |
| LOCK-002 | lock | Stale lock after crash before atexit cleanup | Process killed hard leaves `.lock` behind | All subsequent writers fail until manual lock removal | atexit/signal cleanup only if process dies gracefully; SIGKILL leaves cruft | S2 | common | Kill -9 mid-commit/add; retry same command | differentiate-beyond-git |
| LOCK-003 | lock | Readers observe old-or-new only (rename atomicity) | Reader mid-rename may see either version; never partial if FS atomic | Usually silent; inconsistent tips if non-atomic FS | api-lockfile: readers see old or new assuming atomic rename(2) | S1 | occasional | Concurrent reader of refs/index during writer commit_lock_file | mitigate-with-compat |
| LOCK-004 | lock | LOCK_DIE_ON_ERROR vs soft -1 | Caller dies vs returns error depending on flag | Hard fatal vs recoverable failure across commands | api-lockfile flags LOCK_DIE_ON_ERROR | S3 | common | Compare porcelain that die vs plumbing that returns | mitigate-with-compat |
| LOCK-005 | lock | core.lockfilePid diagnostics | Without PID sidecar, agents cannot identify lock holder | Opaque 'File exists'; optional `index~pid.lock` when enabled | git-config core.lockfilePid (default false) | S3 | common | Enable lockfilePid; contend; inspect pid file | differentiate-beyond-git |
| LOCK-006 | lock | Lockfile only blocks writers | Readers proceed during write; agents assume mutual exclusion for reads | TOCTOU between read decision and later write | api-lockfile: locks block writers only | S1 | common | Agent A reads tip; agent B updates; A acts on stale tip | differentiate-beyond-git |
| LOCK-007 | lock | Same-filesystem rename requirement | commit_lock_file_to fails across mounts | Rename errno; rollback | api-lockfile: path must be same filesystem | S2 | rare | GIT_DIR on different FS than intended final path | mitigate-with-compat |
| LOCK-008 | lock | Manual lock deletion while holder alive | Operator deletes lock; second writer proceeds → dual writers | Corruption / torn index or refs | Documented folklore; Git assumes exclusive lock holder | S0 | occasional | Delete index.lock while writer still writing; second writer starts | differentiate-beyond-git |

### Class `maintenance` (4)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| MAINT-001 | maintenance | git maintenance start background vs agents | Scheduled tasks lock packs/refs | Foreground command failures | git-maintenance | S3 | common | Enable maintenance; agent storm | differentiate-beyond-git |
| MAINT-002 | maintenance | gc.log suppresses auto-gc unexpectedly | Prior gc failure leaves gc.log | Auto-gc skipped; repo grows forever under agents | gc.logExpiry | S4 | occasional | Fail gc; continue agent writes | mitigate-with-compat |
| MAINT-003 | maintenance | Detached auto-gc surprises | gc.autoDetach backgrounds work | Hidden CPU/IO; lock contention unexplained | gc.autoDetach default true | S3 | common | Trigger auto-gc; continue committing | differentiate-beyond-git |
| MAINT-004 | maintenance | update-server-info races on dumb HTTP publish | Concurrent info/refs rewrite | Torn dumb-HTTP advertisements | update-server-info | S1 | rare | Parallel pushes with update hook | out-of-scope |

### Class `misc` (15)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| MISC-001 | misc | Parallel describe / pack-refs heavy read during write | Readers OK but expensive under write locks | Latency spikes | Readers unlocked; writers locked | S4 | common | N describe while pack-refs | mitigate-with-compat |
| MISC-002 | misc | Parallel archive / checkout-index to same prefix | Output path collisions | Torn export dirs | Not Git-locked output paths | S1 | occasional | Agents archive to same folder | mitigate-with-compat |
| MISC-003 | misc | Bundle create while refs move | Bundle tip set unstable | Bundle missing objects relative to advertised tips | git-bundle | S1 | occasional | bundle create during push storm | mitigate-with-compat |
| MISC-004 | misc | Parallel clean -fdx | Deletes other agents' untracked outputs | Lost build artifacts / agent scratch | git-clean | S1 | common | One agent clean while others write untracked | differentiate-beyond-git |
| MISC-005 | misc | Parallel reset --hard | Worktree thrash; index rewrite | Lost WIP; lock fails | reset --hard | S1 | common | Two reset --hard same wt | mitigate-with-compat |
| MISC-006 | misc | Namespace refs/pull or hosting shadow refs | Hosting creates refs agents also write | Permission / hook rejects | Host-specific ref namespaces | S2 | occasional | Agents write refs/pull/* locally mirrored | out-of-scope |
| MISC-007 | misc | Parallel apply / am mailbox | Shared .git/rebase-apply style state | Corrupt apply state | git-am | S1 | rare | Two am in same wt | mitigate-with-compat |
| MISC-008 | misc | Sparse-checkout + skip-worktree vs concurrent materialize | One agent materializes; another sparsifies | Unexpected present/absent files | sparse + skip-worktree | S1 | occasional | Cone toggle vs checkout | mitigate-with-compat |
| MISC-009 | misc | Parallel clone --reference to shared reference repo | Reference repo under gc while clones borrow | Missing alternate objects mid-clone | clone --reference + alternates | S1 | occasional | gc reference bare; N clones --reference | mitigate-with-compat |
| MISC-010 | misc | Agent identity (author/committer) env races on host | Shared env mutation | Misattributed commits | GIT_AUTHOR_* env | S1 | common | Export author in shared shell profile concurrently | differentiate-beyond-git |
| MISC-011 | misc | Hooks installed by one agent while others run | Mid-flight hook swap | Non-atomic policy change | hooks/ directory files | S1 | occasional | Deploy hooks while commits run | mitigate-with-compat |
| MISC-012 | misc | Parallel update-index --refresh | Refresh takes index.lock | Blocks adds; optional lock issues | update-index | S3 | common | N refresh with writers | mitigate-with-compat |
| MISC-013 | misc | Server-side quarantine + parallel push same OID fan-in | Many pushes of related objects | Quarantine disk pressure | receive-pack quarantine | S4 | common | N pushes large packs to bare | mitigate-with-compat |
| MISC-014 | misc | Reflog expire concurrent with updates | expire rewrites logs while append | Lost lines; lock fails | git-reflog expire | S1 | occasional | gc reflog expire during commits | mitigate-with-compat |
| MISC-015 | misc | Parallel worktree add from bare | Admin dir create races | Failed add; partial worktrees/ | git-worktree add from bare | S2 | occasional | N worktree add from same bare | differentiate-beyond-git |

### Class `network` (8)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| NET-001 | network | Parallel fetch same remote | Multiple fetches download overlapping packs | Bandwidth waste; local pack/ref lock contention | git-fetch | S4 | dominant | N agents fetch origin concurrently | differentiate-beyond-git |
| NET-002 | network | Parallel push same branch | CAS / non-ff rejects | rejected non-fast-forward; lock errors | git-push + receive-pack | S2 | dominant | N agents push main | differentiate-beyond-git |
| NET-003 | network | HTTP keep-alive / connection pool exhaustion | Hundreds of agents open smart HTTP sessions | 429/503; TLS handshake storms | HTTP transport; host limits | S2 | common | CI matrix hammer same host | mitigate-with-compat |
| NET-004 | network | SSH multiplexing races | ControlMaster socket contention | ssh failures; hung mux | ssh ControlPath practices | S2 | occasional | N ssh fetches with shared ControlMaster | mitigate-with-compat |
| NET-005 | network | Protocol v2 ls-refs vs concurrent tip move | Advertisement stale by apply time | Negotiation mismatch; empty fetch | gitprotocol-v2 | S3 | common | ls-refs then long gap then fetch-pack | mitigate-with-compat |
| NET-006 | network | Bundle URI / clone bundle races | Parallel clones hit same bundle source | Partial downloads; checksum fails | bundle URI docs | S2 | occasional | N clones with bundle URI | mitigate-with-compat |
| NET-007 | network | Credential helper cache stampede | Many helpers fill/ask concurrently | Auth prompts; helper lock files; token thrash | gitcredentials / helper protocols | S2 | common | N agents fetch private remote first time | differentiate-beyond-git |
| NET-008 | network | askPass / interactive credential under agents | No TTY; hung waits | Processes block forever | GIT_ASKPASS / core.askPass | S2 | common | Agent without helper hits HTTPS auth | mitigate-with-compat |

### Class `object` (7)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| OBJ-001 | object | Concurrent loose object create same OID | Identical content hashed by many agents | Usually OK (same bytes); rare create races on Windows | objects/xx/… loose create | S3 | common | N agents hash identical blob | mitigate-with-compat |
| OBJ-002 | object | Concurrent distinct loose creates fan-out dir pressure | Hundreds of writers create many loose objects | Directory entry storms; auto-gc threshold hit | layout objects/[0-9a-f]{2} | S4 | dominant | N agents commit unique blobs without packing | differentiate-beyond-git |
| OBJ-003 | object | Object write without reachable ref then prune | Writer creates objects; gc prunes before ref update | Missing object; corruption if ref added later | git-gc NOTES concurrent prune risk | S0 | occasional | Long-running writer; concurrent gc --prune | differentiate-beyond-git |
| OBJ-004 | object | Hash collision pretends (SHA-1 ecosystem) | Theoretical collision attacks / mismatched content | Wrong object accepted | SHA-1 transition / sha256 repos | S0 | rare | Hostile content in multi-agent untrusted inputs | mitigate-with-compat |
| OBJ-005 | object | Partial object file on crash | Writer dies mid-loose write | Corrupt loose object; fsck errors | Write then rename patterns vary by path | S0 | occasional | Kill mid-hash-object write | mitigate-with-compat |
| OBJ-006 | object | Alternates borrow race | Alternate store packs/prunes while borrower reads | Missing object from alternates | objects/info/alternates | S1 | occasional | Shared alternate ODB; gc on donor; read on borrower | mitigate-with-compat |
| OBJ-007 | object | Quarantine dir (receive-pack) vs premature advertise | Objects in quarantine not yet promoted | Concurrent local readers miss quarantine objects | receive-pack quarantine | S1 | rare | Push in quarantine; local fsck concurrently | mitigate-with-compat |

### Class `pack` (7)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| PACK-001 | pack | Concurrent pack-objects / index-pack | Multiple pack writers in objects/pack | Temp pack names collide; .keep files; disk thrash | pack-objects / index-pack / .keep | S2 | common | Parallel fetch + local repack | mitigate-with-compat |
| PACK-002 | pack | Multi-pack-index (MIDX) rewrite under readers | MIDX rewrite while pack reads | Stale MIDX; fallback to slow path; rare inconsistency | multi-pack-index docs | S3 | occasional | maintenance write-midx during heavy read | mitigate-with-compat |
| PACK-003 | pack | Bitmap generation vs concurrent pack change | Bitmap for pack that is replaced mid-write | Bitmap ignored or mismatched | pack bitmap docs | S3 | occasional | gc writes bitmap while fetch adds packs | mitigate-with-compat |
| PACK-004 | pack | gc.autoPackLimit consolidation storm | Many small packs from agents trigger auto repack | Sudden long exclusive maintenance; lock storms | gc.autoPackLimit | S4 | dominant | N agents each create packs via fetch/commit | differentiate-beyond-git |
| PACK-005 | pack | Cruft pack concurrent with writers | Cruft packing moves unreachable objects | Writer may race prune window | gc cruft packs | S1 | occasional | gc --cruft during object creation storms | mitigate-with-compat |
| PACK-006 | pack | Window/memory blowup under parallel pack-objects | N pack-objects each allocate large windows | OOM; host thrashing | pack.windowMemory / threads | S2 | common | CI matrix parallel pack on same host | mitigate-with-compat |
| PACK-007 | pack | Promisor pack / partial clone fetch races | Concurrent demand-fetches of missing objects | Duplicate fetches; promisor remote lock/queue storms | partial clone / promisor remote | S2 | common | N agents checkout paths needing promisor objects | differentiate-beyond-git |

### Class `perf` (7)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| PERF-001 | perf | Single-writer lock throughput ceiling | All mutations serialize on locks | Latency ≈ N × critical_section | Inherent exclusive lock design | S4 | dominant | Measure commits/sec vs N agents same branch | differentiate-beyond-git |
| PERF-002 | perf | status full tree walk without fsmonitor | N agents each walk entire tree | CPU/IO meltdown | git-status; fsmonitor optional | S4 | dominant | Disable fsmonitor; N status loops | differentiate-beyond-git |
| PERF-003 | perf | fsmonitor daemon single WD limit | One daemon per working directory | Event queue backlog under write storms | git-fsmonitor--daemon | S4 | common | Many writers; one daemon; status clients | mitigate-with-compat |
| PERF-004 | perf | commit-graph rewrite during walks | gc writes commit-graph while agents rev-list | Transient slowdown; rare read issues | gc.writeCommitGraph | S3 | occasional | maintenance commit-graph during log storms | mitigate-with-compat |
| PERF-005 | perf | Loose object explosion before pack | gc.auto threshold lag | Slow object access; many inodes | gc.auto default 6700 | S4 | dominant | Burst create < threshold then read-heavy | differentiate-beyond-git |
| PERF-006 | perf | Allocator / process spawn overhead (stock git) | Each agent operation is new process | Fork/exec dominates vs in-process engine | Process-per-command model | S4 | dominant | N agents × M short git commands | differentiate-beyond-git |
| PERF-007 | perf | Delta cache / mmap thrash under parallel readers | Many pack readers compete for RAM | Page cache thrash | pack mmap windows | S4 | common | Parallel blame/log on huge packs | differentiate-beyond-git |

### Class `ref` (20)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| REF-001 | ref | Per-ref loose `.lock` contention | Two updates to same loose ref collide | `cannot lock ref 'refs/heads/X': Unable to create '….lock'` | git-update-ref + lockfile per loose ref | S2 | dominant | N agents commit to same branch tip concurrently | differentiate-beyond-git |
| REF-002 | ref | packed-refs.lock global serialization | Any packed-refs rewrite holds one lock for all packed refs | Spurious failures updating unrelated refs; timeout after packedRefsTimeout | core.packedRefsTimeout retry (default ~1s); Haggerty/GitHub lore patches | S2 | dominant | Parallel pack-refs / delete / update across many packed refs | differentiate-beyond-git |
| REF-003 | ref | Loose vs packed delete race (historical) | Loose deleted before packed removal → temporary/permanent wrong tip | Readers see stale packed OID; possible GC of needed object | Fixed via packed-ref transactions; older Git versions vulnerable | S0 | rare | Hold packed-refs.lock externally during delete on old Git | mitigate-with-compat |
| REF-004 | ref | Multi-ref transaction prepare failure | Cannot lock all refs in --stdin transaction → abort all | No partial apply (unless --batch-updates user errors) | git-update-ref --stdin start/prepare/commit | S2 | common | Transaction locking many refs while another holds one | differentiate-beyond-git |
| REF-005 | ref | Concurrent reader sees subset of multi-ref updates | Atomic per-ref but not snapshot across refs | Inconsistent cross-ref view mid-transaction finish | git-update-ref NOTES: concurrent reader may see subset | S1 | occasional | Reader lists refs during multi-update commit | differentiate-beyond-git |
| REF-006 | ref | Compare-and-swap old OID mismatch | Expected old tip no longer matches | `fatal: cannot lock ref … is at X but expected Y` | update-ref three-arg CAS | S2 | dominant | Two agents commit; second loses CAS | differentiate-beyond-git |
| REF-007 | ref | HEAD.lock contention | HEAD updates serialize (commit/checkout/merge) | Unable to create HEAD.lock | layout + update-ref lock of HEAD | S2 | common | Parallel commit and checkout on same worktree | mitigate-with-compat |
| REF-008 | ref | Symref vs target race | HEAD points to branch; branch tip moves under agent | Commit lands on unexpected tip; detach surprises | Symref deref semantics in update-ref | S1 | common | Agent A commits; agent B retargets HEAD; A finishes | differentiate-beyond-git |
| REF-009 | ref | Reflog append concurrency | Concurrent appends to same logs/refs/… file | Missing/garbled reflog lines; update failure if cannot append | update-ref LOGGING UPDATES; append model | S1 | occasional | N agents update same branch with reflog on | mitigate-with-compat |
| REF-010 | ref | Illegal concurrent create of same new ref | Two agents create identical new branch name | One wins; other fails create | update-ref create / check-ref-format | S2 | common | Agents invent same refs/heads/agent-feature | differentiate-beyond-git |
| REF-011 | ref | Namespace collision under refs/arks/ or agent prefixes | Agents share namespace without partitioning | CAS storms; accidental overwrite | No Git-native agent namespace lease | S2 | dominant | Hundreds of agents write refs/heads/tmp/* | differentiate-beyond-git |
| REF-012 | ref | pack-refs while live updates | pack-refs rewrites packed-refs under load | Lock timeouts; temporary ref visibility oddities | git-pack-refs + packed-refs.lock | S2 | common | gc/pack-refs during agent commit storm | mitigate-with-compat |
| REF-013 | ref | reftable stack concurrent writers | Multiple writers contend on reftable stack | Transaction failures; stack compaction races | reftable technical doc + extensions.refStorage | S2 | occasional | Enable reftable; N concurrent update-ref | mitigate-with-compat |
| REF-014 | ref | Remote-tracking ref fan-in | Parallel fetch updating many refs/remotes/* | packed-refs.lock storms; fetch failures | fetch ref updates via transaction | S2 | common | N agents fetch same remote concurrently | differentiate-beyond-git |
| REF-015 | ref | Server receive-pack multi-push to same ref | Two pushes CAS same branch on bare remote | non-fast-forward / lock failure on one | receive-pack + update hook + ref locks | S2 | dominant | N agents push same branch to shared bare | differentiate-beyond-git |
| REF-016 | ref | Notes ref concurrent append | refs/notes/* updates serialize | notes add failures under contention | git-notes + update-ref | S2 | occasional | Agents annotate same commit concurrently | mitigate-with-compat |
| REF-017 | ref | Replace refs concurrent edit | refs/replace collisions | Replace tip CAS fail | git-replace / layout refs/replace | S2 | rare | Two agents replace same OID | out-of-scope |
| REF-018 | ref | Pseudoref races (ORIG_HEAD, MERGE_HEAD, CHERRY_PICK_HEAD) | State files/refs overwritten by parallel porcelain | Wrong merge/rebase continuation | layout + sequencer state | S1 | common | Two merges in same worktree | mitigate-with-compat |
| REF-019 | ref | Worktree-private vs common refs split | Agents confuse per-worktree vs common refs | Unexpected shared tip moves across worktrees | gitrepository-layout GIT_COMMON_DIR rules | S1 | common | Linked worktrees; agents update shared branch | differentiate-beyond-git |
| REF-020 | ref | Dangling lock after NFS disconnect mid-update-ref | Network FS leaves `.lock` after client disconnect | Permanent lock until admin cleanup | Lockfile assumes local FS semantics | S2 | occasional | Update-ref on SMB/NFS; kill network mid-lock | mitigate-with-compat |

### Class `sequencer` (5)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| SEQ-001 | sequencer | Parallel rebase in same worktree | Shared sequencer state under .git | Corrupted todo; abort impossible cleanly | sequencer / rebase-merge / rebase-apply dirs | S1 | common | Two rebases same wt | mitigate-with-compat |
| SEQ-002 | sequencer | rebase vs merge state collision | MERGE_HEAD and rebase dirs coexist wrongly | Porcelain confusion; hard reset needed | State machine exclusivity assumed | S1 | occasional | Start merge while rebase in progress | mitigate-with-compat |
| SEQ-003 | sequencer | cherry-pick sequence across agents | Shared CHERRY_PICK_HEAD | Wrong pick applied | sequencer | S1 | occasional | Two cherry-picks same wt | mitigate-with-compat |
| SEQ-004 | sequencer | rerere shared rr-cache writes | Concurrent conflict resolutions | rr-cache entry races | git-rerere; gc.rerere* | S1 | occasional | Parallel merges with rerere enabled | mitigate-with-compat |
| SEQ-005 | sequencer | merge conflict marker files edited by two agents | Dual resolution of same conflicted path | Lost resolution; invalid merge | Working tree files uncoordinated | S1 | common | Two agents edit conflicted file | differentiate-beyond-git |

### Class `shallow` (3)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| SHAL-001 | shallow | Parallel shallow fetch rewrite | shallow file locked/rewritten | Inconsistent depth; fetch fail | shallow file in common dir | S1 | occasional | Two deepen fetches | mitigate-with-compat |
| SHAL-002 | shallow | Partial clone promisor remote queue pileup | On-demand OID fetches stampeded | Latency explosion; remote rate limits | promisor remote | S4 | dominant | N agents blame/checkout cold partial clone | differentiate-beyond-git |
| SHAL-003 | shallow | Filter clone object filter mismatch across agents | Different filters expected | Missing blobs; confusing errors | partial clone filters | S1 | rare | Agents assume full objects on filtered clone | mitigate-with-compat |

### Class `sparse` (2)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| SPARSE-001 | sparse | Concurrent sparse-checkout cone edits | info/sparse-checkout rewrite races | Wrong cone; missing files | sparse-checkout file + index | S1 | occasional | Agents set different cones | mitigate-with-compat |
| SPARSE-002 | sparse | Sparse index expand stampede | Many commands expand simultaneously | index.lock + CPU spike | sparse-index ensure_full_index | S4 | common | Outside-cone ops × N agents | differentiate-beyond-git |

### Class `stash` (2)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| STASH-001 | stash | Concurrent stash push | refs/stash reflog updates contend | Lost stash entries; lock fail | git-stash + reflog | S1 | occasional | N agents stash -u | mitigate-with-compat |
| STASH-002 | stash | stash pop while another modifies index | Pop applies against moving index/WT | Conflict storms; partial apply | stash pop sequence | S1 | occasional | stash pop + concurrent add | mitigate-with-compat |

### Class `submodule` (8)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| SUB-001 | submodule | Nested index.lock in submodule + superproject | Update holds locks in both repos | Lock failures in either layer abort | gitsubmodules + submodule update | S2 | common | Parallel submodule update and super commit | differentiate-beyond-git |
| SUB-002 | submodule | Concurrent submodule update same path | Two updates checkout different gitlink SHAs | Torn working tree; index mismatch | submodule update serialization absent by default | S1 | common | N agents submodule update --init | differentiate-beyond-git |
| SUB-003 | submodule | modules/ absorbgitdirs races | Absorb while another clones into path | gitfile/modules inconsistency | submodule absorbgitdirs | S1 | occasional | Absorb concurrent with status/update | mitigate-with-compat |
| SUB-004 | submodule | Recursive status serial cost | Submodule status examined serially | Status latency dominates at N submodules × M agents | RFC parallel submodule status lore | S4 | dominant | Many submodules; agents loop status | differentiate-beyond-git |
| SUB-005 | submodule | fsmonitor superproject noise from submodule edits | Daemon reports submodule path events to super | Perf hit; client filters correctness OK | git-fsmonitor--daemon docs | S4 | common | fsmonitor on; agents edit inside submodule | mitigate-with-compat |
| SUB-006 | submodule | Gitlink stage vs nested checkout mismatch | Super index records SHA A; worktree at B | `+` dirty submodule status storms | gitlink 160000 semantics | S3 | common | Agent commits in submodule without super update | mitigate-with-compat |
| SUB-007 | submodule | Parallel clone into .git/modules | Two clones of same submodule name | Directory create races; partial clones | modules/ layout | S2 | occasional | Agents submodule update --init same name | differentiate-beyond-git |
| SUB-008 | submodule | Worktree containing submodules cannot move | move refused with submodules | Command failure | git-worktree move notes | S3 | rare | Move linked wt with submodules | mitigate-with-compat |

### Class `toctou` (5)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| TOCTOU-001 | toctou | status then commit decision race | Clean status becomes dirty before commit | Unexpected files in commit or empty commit surprises | No transactional snapshot across porcelain | S1 | dominant | Agent status; another writes; agent commits | differentiate-beyond-git |
| TOCTOU-002 | toctou | rev-parse tip then update-ref without CAS | Read tip; compute; write without old OID | Silent lost update | Must use old OID; agents often omit | S1 | common | Script updates ref without expected OID | differentiate-beyond-git |
| TOCTOU-003 | toctou | Worktree file edit vs checkout | Checkout overwrites agent edits | Lost worktree changes | checkout/restore vs dirty tree | S1 | common | Agent edits file; another checkout path | mitigate-with-compat |
| TOCTOU-004 | toctou | Credential exists check then store | Two helpers store different tokens | Wrong token wins | credential approve races | S1 | occasional | Parallel first auth | mitigate-with-compat |
| TOCTOU-005 | toctou | Lock absence check then create (userspace) | Agents poll for lock file then proceed wrongly | Dual writers | Must use O_EXCL; polling is racy | S0 | occasional | Custom scripts test -f index.lock | mitigate-with-compat |

### Class `windows` (3)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| WIN-001 | windows | MAX_PATH deep worktree + LFS temp names | Deep paths exceed limits | CreateFile failures | Win32 MAX_PATH; LFS short temp pattern | S2 | occasional | Deep sparse tree; LFS smudge | mitigate-with-compat |
| WIN-002 | windows | Mandatory delete-pending window | Remove then O_EXCL create fails while handle open | Access denied (LFS CVE fix fallout) | Win32 pending delete; LFS PR 6215 | S2 | common | Parallel LFS smudge same OID on Server 2019 | mitigate-with-compat |
| WIN-003 | windows | OneDrive Files On-Demand placeholders | Placeholder files confuse lstat | False dirty; checkout errors | Cloud files API | S1 | occasional | Repo under OneDrive | out-of-scope |

### Class `worktree` (8)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| WT-001 | worktree | Shared common dir object/ref contention | Linked worktrees share $GIT_COMMON_DIR | Ref/index (per-wt) OK but common packed-refs/objects contend | git-worktree + layout commondir | S2 | dominant | Worktree-per-agent; all commit same shared branch | differentiate-beyond-git |
| WT-002 | worktree | Same branch checked out in two worktrees | add fails if branch already checked out | `already used by worktree at …` | git-worktree add fail-closed | S2 | common | Two agents add worktrees for same branch | mitigate-with-compat |
| WT-003 | worktree | worktree prune while agent uses path | Prune removes admin entry for live worktree | Broken gitfile; repair needed | worktree prune / gc.worktreePruneExpire | S1 | occasional | gc prune during ephemeral agent worktree | differentiate-beyond-git |
| WT-004 | worktree | Locked worktree prevents remove/move | Agent cannot clean up locked wt | remove/move fail; locked file reason | worktree lock / locked file | S3 | occasional | Lock portable wt; another agent tries remove | mitigate-with-compat |
| WT-005 | worktree | Main worktree move breaks linked gitdir pointers | Absolute paths in worktrees/*/gitdir stale | Linked wt cannot find common dir | worktree repair | S1 | rare | Move main repo path under agents | mitigate-with-compat |
| WT-006 | worktree | Per-worktree config.worktree races | Concurrent edits to config.worktree | Lost config keys | extensions.worktreeConfig | S1 | occasional | Agents set core.* in worktree config concurrently | mitigate-with-compat |
| WT-007 | worktree | Private HEAD/index but shared hooks/config | Hook storm from all worktrees sharing hooks/ | Serialized hook CPU; side effects collide | layout: hooks under common | S4 | common | N worktrees commit → shared pre-commit | differentiate-beyond-git |
| WT-008 | worktree | Concurrent worktree add name collision | Same worktree id directory | add fails / corrupt admin dir | worktrees/<id>/ | S2 | occasional | Agents choose same worktree name | differentiate-beyond-git |


### Class `addendum` (v0.1.0 expansion)

| ID | Class | Scenario | Failure mode | Observable symptoms | Git stock behavior | Severity | Freq@N | Repro sketch | Arks opportunity |
|----|-------|----------|--------------|---------------------|--------------------|----------|--------|--------------|------------------|
| OBJ-008 | object | Concurrent hash-object --stdin-paths | Many agents hash huge inputs | CPU saturation; temp file pressure | hash-object | S4 | common | N hash-object large files | mitigate-with-compat |
| OBJ-009 | object | Cat-file --batch-check stampede | Parallel cat-file batches | FD/CPU contention | cat-file --batch | S4 | common | N agents batch-check OIDs | differentiate-beyond-git |
| PACK-008 | pack | Concurrent index-pack --fix | Same pack filename race | Failed index-pack; leftover .tmp | index-pack | S2 | occasional | Parallel fetch same pack name | mitigate-with-compat |
| PACK-009 | pack | Keep-file forgotten after crash | .keep left forever | Pack never GC'd; disk growth | pack .keep protocol | S4 | occasional | Kill during fetch; .keep remains | mitigate-with-compat |
| REF-021 | ref | Concurrent symbolic-ref retarget | HEAD/other symref rewrite race | Unexpected branch; lock fail | git-symbolic-ref | S1 | occasional | Parallel symbolic-ref HEAD | mitigate-with-compat |
| REF-022 | ref | For-each-ref during pack-refs rewrite | Reader iteration mid-rewrite | Rare inconsistent listing | for-each-ref + packed-refs | S3 | common | N for-each-ref during pack-refs | mitigate-with-compat |
| REF-023 | ref | Branch rename while agents commit old name | refs/heads/A.lock vs rename to B | Lost commits / fail | branch -m | S1 | occasional | Rename branch under writers | differentiate-beyond-git |
| IDX-013 | index | Concurrent chmod / update-index --chmod | Mode bit races | Wrong executable bit | update-index --chmod | S1 | occasional | Parallel chmod+add | mitigate-with-compat |
| IDX-014 | index | Parallel read-tree -m merges into index | Multi-way read-tree contend | index.lock fail; torn merge stage | read-tree | S2 | occasional | Two read-tree -m same wt | mitigate-with-compat |
| HOOK-007 | hook | prep-receive / post-receive side effects on shared FS | Server hooks write shared scratch | Cross-push interference | receive hooks | S1 | occasional | Parallel pushes; hook writes shared scratch | mitigate-with-compat |
| HOOK-008 | hook | proc-receive / push-options races | Concurrent pushes with options | Hook sees interleaved options | proc-receive | S1 | rare | Parallel push --push-option | out-of-scope |
| NET-009 | network | Concurrent clone of same URL to shared dest | Destination path collision | Failed clone; partial dir | git-clone | S2 | common | N clones same path | mitigate-with-compat |
| NET-010 | network | Partial clone filter negotiation mismatch | Agents disagree on filter | Unexpected missing blobs | protocol filter | S1 | occasional | Mixed filter clones then fetch | mitigate-with-compat |
| NET-011 | network | Push atomic vs non-atomic mixed clients | Partial apply visibility | Cross-ref inconsistency window | push --atomic | S1 | occasional | Mix atomic/non-atomic pushes | mitigate-with-compat |
| FS-012 | fs | FUSE / virtiofs lock latency | High lock acquire latency | Timeouts; false stale locks | FUSE semantics | S2 | occasional | Repo on virtiofs; N writers | mitigate-with-compat |
| FS-013 | fs | inode / direntry exhaustion in objects/ | Too many loose objects | ENOSPC/ENFILE | loose object fanout | S2 | occasional | Extreme loose create without gc | mitigate-with-compat |
| FS-014 | fs | SELinux/AppArmor deny rename of .lock | Policy blocks commit_lock_file | Mysterious EACCES | LSM policy | S2 | rare | Confined agent cannot rename lock | out-of-scope |
| PERF-008 | perf | Parallel blame on same pack | Heavy delta walks | CPU meltdown | git-blame | S4 | common | N blame hot files | differentiate-beyond-git |
| PERF-009 | perf | Parallel grep across huge trees | Tree walk storms | IO saturation | git-grep | S4 | common | N grep without fsmonitor help | differentiate-beyond-git |
| PERF-010 | perf | Parallel diff --stat after large fetch | Diff setup cost x N | Latency spike | git-diff | S4 | common | Post-fetch status/diff herd | differentiate-beyond-git |
| TOCTOU-006 | toctou | Fetch then rebase without rebase onto CAS | Upstream moved after fetch | Rebase wrong base | fetch+rebase scripts | S1 | common | Agent fetch; peer push; agent rebase | differentiate-beyond-git |
| TOCTOU-007 | toctou | Worktree add then prune race | Add incomplete; prune removes | Broken wt | worktree add/prune | S1 | occasional | Parallel add and prune | mitigate-with-compat |
| SEQ-006 | sequencer | Interactive rebase todo edited by two agents | Shared todo file | Divergent todo; corrupt sequencer | rebase -i | S1 | rare | Two editors on todo | mitigate-with-compat |
| SEQ-007 | sequencer | Parallel am --3way | Shared apply state | Failed am; dirty state | git-am | S1 | rare | Two am --3way | mitigate-with-compat |
| SUB-009 | submodule | sync config while update runs | .gitmodules/config rewrite | Wrong URL mid-clone | submodule sync/update | S1 | occasional | sync + update parallel | mitigate-with-compat |
| SUB-010 | submodule | deinit while another agent builds in path | Working tree removed under build | Lost outputs; errors | submodule deinit | S1 | occasional | deinit vs agent compile in sub | differentiate-beyond-git |
| WT-009 | worktree | Concurrent repair after path move | Dual repair rewrites pointers | Divergent gitdir paths | worktree repair | S1 | rare | Two repair after move | mitigate-with-compat |
| WT-010 | worktree | extensions.worktreeConfig false vs true mix | Tools disagree on config scope | Divergent effective config | worktreeConfig extension | S1 | rare | Mixed clients | mitigate-with-compat |
| LFS-005 | lfs | Concurrent lfs push same OID | Upload races | Duplicate uploads; server rejects | git lfs push | S3 | common | N push after parallel commits | mitigate-with-compat |
| LFS-006 | lfs | Pointer checkout without LFS installed | Filter missing under agents | Pointer files committed as content | clean/smudge filters | S1 | occasional | Agent env missing LFS | mitigate-with-compat |
| SHAL-004 | shallow | Unshallow while peers fetch shallow | Shallow file rewrite | Peer fetch confusion | fetch --unshallow | S1 | rare | Unshallow + shallow fetch | mitigate-with-compat |
| FSM-004 | fsmonitor | Daemon crash mid-status | Clients fall back to full scan | Stampede full scans | fsmonitor IPC | S4 | occasional | Kill daemon under N status | mitigate-with-compat |
| CI-005 | ci | Reusable Actions workspace without clean | Leftover locks across jobs | Flaky index.lock | Actions workspace reuse | S2 | common | Concurrent jobs same runner workdir | mitigate-with-compat |
| CI-006 | ci | Parallel checkout action on monorepo sparse | Sparse cone fights | Missing paths; lock fails | sparse + actions/checkout | S2 | common | Matrix sparse paths overlap | mitigate-with-compat |
| MAINT-005 | maintenance | commit-graph verify under writers | Verify vs rewrite | Transient verify fail | commit-graph | S3 | occasional | verify during gc write | mitigate-with-compat |
| MAINT-006 | maintenance | Incremental multi-pack-index under pack add | MIDX incremental race | Fallback; rebuild cost | MIDX incremental | S3 | occasional | fetch + maintenance midx | mitigate-with-compat |
| AGENT-009 | agent | Shared bare as only coordination channel | All sync via push/fetch | Ref lock storms on bare | bare remote model | S2 | dominant | Agents only communicate by push | differentiate-beyond-git |
| AGENT-010 | agent | Tooling wraps git without error taxonomy | Treat all fatals as retry | Amplifies lock herds | Agent frameworks | S4 | dominant | Blind retry on any nonzero | differentiate-beyond-git |
| AGENT-011 | agent | Parallel formatters rewriting same files | Content merge conflicts | Constant dirty tree | Editor/format agents | S1 | dominant | N formatters same paths | differentiate-beyond-git |
| AGENT-012 | agent | Secret scanning hook x N commits | Hook network x commits | Throughput collapse | pre-receive/pre-commit scanners | S4 | common | Secret scan hook under storm | differentiate-beyond-git |
| MISC-016 | misc | Parallel update-ref --stdin huge batches | Giant transactions | Long lock hold; timeouts | update-ref --stdin | S2 | occasional | Mega transaction vs live writers | differentiate-beyond-git |
| MISC-017 | misc | Concurrent notes merge | notes tree merge races | Failed notes merge | git notes merge | S2 | rare | Parallel notes merge | mitigate-with-compat |
| MISC-018 | misc | Parallel sparse-checkout reapply | Cone reapply + index | Lock + unexpected skip-worktree | sparse-checkout reapply | S2 | occasional | reapply under writers | mitigate-with-compat |
| MISC-019 | misc | Attribute macro / merge driver concurrent install | gitattributes rewrite | Divergent merge behavior | gitattributes | S1 | occasional | Agents edit attributes | mitigate-with-compat |
| MISC-020 | misc | Parallel checkout-index --prefix | Shared prefix collision | Torn export | checkout-index | S1 | occasional | Same prefix exports | mitigate-with-compat |
| CORR-006 | corruption | Alternates cycle / dangling under GC | Borrower points at pruned donor | Missing objects appear as corruption | alternates | S0 | rare | GC donor used by live clones | mitigate-with-compat |
| CORR-007 | corruption | Replace refs + concurrent prune | Replace target pruned | Unexpected object resolution | replace + gc | S0 | rare | replace then aggressive prune | mitigate-with-compat |
| LOCK-009 | lock | Lock held across NFS lease steal | Client thinks it holds lock; server grants another | Dual writers | NFS lock/lease semantics | S0 | occasional | NFS lock reclaim under writers | mitigate-with-compat |

### Cross-links

- Research method + primary URLs: [[research:root/20261109023117]]
- Lease / dual-path / L3 refuse-close: [[sot:root/lease-concurrency-intent]], [[adr:root/20261108232800]]
- Residual register (bars unchanged): [[sot:root/production-residual-register]]
- Parallel residual **worktree/submodule implement** plan is a separate inventory track — this catalog may **cite** worktree/submodule concurrency scenarios but must **not** silently widen that plan’s claimed CLI surface.

## Why

Multi-agent Git usage exposes a large surface beyond “index.lock exists.” An ID’d catalog lets future performance uplift and lease/L3 design point at **named scenarios** instead of folklore, while keeping milestone honesty and CELIR/lease boundaries intact.

## Verification

1. `spec check` clean for this SoT and [[research:root/20261109023117]].
2. Every row has ID, class, opportunity class ∈ {mitigate-with-compat, differentiate-beyond-git, out-of-scope}.
3. No row is treated as shipping L3 or redefining M9/M10/M11 without a superseding ADR/plan.
4. Primary sources for Git semantics remain cited from research; this SoT does not paste copyrighted primary text.
5. Future implementation PRs that claim a scenario mitigated/differentiated MUST name scenario IDs in the plan/evidence.
