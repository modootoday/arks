# Concurrency issue-compare matrix (G18) — M13 Phase Cd

Owning SoT: `.spec/sot/interop-test-matrix.sot.md` (G18). Catalog: `.spec/sot/multi-agent-git-concurrency-scenarios.sot.md`. Plan: `.spec/plan/20261109024635-m13-concurrency-differentiate-lease.plan.md`. Grant schema ADR: `.spec/adr/20261109032955-m13-cd1-lease-grant-resource-schema.adr.md`. Roadmap Done rule: `.spec/sot/program-roadmap.sot.md` v0.5.0+.

Authoritative venue: Docker clean-room service `g18` (`verify/scripts/run-g18.sh`). Host runs are advisory only.

Protocol per claimed ID: **induce → stock `git` pathology oracle → `arks` resolved oracle (lease uplift) → compare receipt**. Lease ≠ skip Git locks. L3 refuse-closed. Timings MAY be recorded; absolute / ≥20% pass-fail is **G19 / M14 only**.

## Wave Cd-1 (claimed set)

Top differentiate track not closed as lease-uplift under M12 mitigate-only: REF/CAS/namespace + retry-herd coordination via leases.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| REF-001 | Same-branch tip update under competing writers / planted `*.lock` | Stock `update-ref` fails under per-ref lock contention | Exclusive `refns:` lease coordinates; holder may update-ref; non-holder AlreadyHeld; Git locks still honored when present | yes | `verify/gates/g18` |
| REF-006 | Tip CAS race (expected old OID mismatch class) | Stock CAS / tip race fails (`is at … but expected …` class or lock collide) | Lease serializes tip writers before CAS; loser fails at lease layer with wait honesty (not blind CAS herd) | yes | `verify/gates/g18` |
| REF-011 | Unpartitioned agent tmp refs collide | Competing writers share `refs/heads/tmp/*` without namespace lease | Partitioned `refns:refs/heads/agents/<id>/` leases succeed side-by-side; overlapping prefix AlreadyHeld | yes | `verify/gates/g18` |
| AGENT-003 | Blind retry herd on contended lock | Immediate retries keep failing / amplify under held lock | Durable `index:` / lease grant: second holder AlreadyHeld + M13 wait/backoff note; write path still honors live `index.lock` | yes | `verify/gates/g18` |

Wave Cd-1 **Done** = Docker G18 PASS for **all four** rows + comparative evidence receipt ([[evidence:root/20261109153000]]).

## Wave Cd-2 (claimed set)

Lease fencing + worktree/submodule cross-domain leases (roadmap ranks 3/5 differentiate uplift beyond M12 mitigate).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| LOCK-002 | Stale/dead holder class; lease TTL fencing | Stale `index.lock` blocks writers | Short-TTL lease expires via `arks lease expire`; next holder grants; write path still honors live/stale Git locks | yes | `verify/gates/g18` |
| WT-001 | Linked worktree + common-dir contention | Common-store lock fails git writer from linked wt | Exclusive `domain:worktree:` lease; non-holder AlreadyHeld; Git locks still honored | yes | `verify/gates/g18` |
| SUB-001 | Superproject + submodule lock domain | Submodule lock-domain blocks induced git path | Exclusive `domain:submodule:` lease; non-holder AlreadyHeld; submodule locks still honored | yes | `verify/gates/g18` |
| AGENT-007 | Worktree + submodule composition | Writers fail across composed lock domains | Concurrent distinct domain leases; inspect lists both; same-id overlap AlreadyHeld; locks honored | yes | `verify/gates/g18` |

Wave Cd-2 **Done** = Docker G18 PASS for **all four** Cd-2 rows + Cd-1 regression green.

## Wave Cd-3 (claimed set)

Hook budgets, writer–gc leases, agent merge namespace, index path-set (roadmap ranks 1/2/6/7).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| HOOK-001/HOOK-006 | Sync hooks in critical section under N | Hook amplification / critical-section class | Exclusive `hook:critical` lease; non-holder AlreadyHeld; no invented async hook runner | yes | `verify/gates/g18` |
| PACK-004/MAINT-003 | gc/maintenance under writers | Stock gc path available under contention class | Exclusive `maint:gc` lease; writer holds; second AlreadyHeld; `arks gc` remains refuse-closed | yes | `verify/gates/g18` |
| AGENT-002 | Fan-in agent branches without merge coordinator | Uncoordinated merge/ref storm class | `refns:refs/heads/agents/` partitions + exclusive `refns:refs/heads/main` merge lease | yes | `verify/gates/g18` |
| IDX-001 | Exclusive index writer storm | `index.lock` blocks concurrent writers | Exclusive `index:paths:default` path-set lease; non-holder AlreadyHeld; locks still honored | yes | `verify/gates/g18` |

Wave Cd-3 **Done** = Docker G18 PASS for **all four** Cd-3 rows + Cd-1/Cd-2 regression green ([[evidence:root/20261109163000]]).

**M13 claimed-set exit** = Cd-1 ∪ Cd-2 ∪ Cd-3 (12 rows) all G18 PASS.

## Wave Cd-4 (M20 claimed set)

Plan: `plan:root/20261110090000`. Bucket B after Cx-8 (bucket A Docker-induce-thin).

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| AGENT-001 | Shared WT blackboard | lock/chaos class | `agent:blackboard` exclusive lease | yes | `verify/gates/g18` |
| AGENT-010 | Blind retry all fatals | lock herd class | AlreadyHeld taxonomy honesty | yes | `verify/gates/g18` |
| AGENT-011 | Parallel formatters | dirty-tree class | `index:paths:formatter` lease | yes | `verify/gates/g18` |
| REF-002 | packed-refs.lock | global serialize | `refns:packed-refs` lease | yes | `verify/gates/g18` |
| REF-004 | multi-ref txn | prepare abort | `refns:txn:multi` lease | yes | `verify/gates/g18` |
| REF-008 | HEAD.lock / symref | tip race | `refns:HEAD` lease | yes | `verify/gates/g18` |
| REF-010 | identical new-ref create | one wins | exclusive refns create lease | yes | `verify/gates/g18` |
| IDX-003 | shared default index | contamination | `index:private:agent` lease | yes | `verify/gates/g18` |
| IDX-007 | same-path parallel add | last-writer | `index:paths:same` lease | yes | `verify/gates/g18` |
| SEQ-005 | dual conflict edit | lost resolution | `seq:conflict:path` lease | yes | `verify/gates/g18` |
| WT-003 | prune vs live wt | broken gitfile | `domain:worktree:prune` lease | yes | `verify/gates/g18` |
| MAINT-001 | background maint | foreground fail | `maint:background` lease | yes | `verify/gates/g18` |

Wave Cd-4 **Done** = Docker G18 PASS for **all twelve** + Cd-1..Cd-3 regression (**24**/24).

## Wave Cd-5 (M20 claimed set)

Plan: `plan:root/20261110100000`. Bucket B continue after Cd-4.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| AGENT-008 | Sync policy-hook latency | hook/lock storm class | `hook:policy` exclusive lease | yes | `verify/gates/g18` |
| AGENT-009 | Bare-as-only-channel | tip lock storm on bare | `refns:refs/heads/agents/` + `refns:bare:coord` | yes | `verify/gates/g18` |
| AGENT-012 | Secret-scan hook × N | hook amplification | `hook:secret-scan` lease | yes | `verify/gates/g18` |
| REF-005 | Multi-ref reader subset | inconsistent cross-ref view | `refns:txn:snapshot` lease honesty | yes | `verify/gates/g18` |
| REF-014 | Remote-tracking fan-in | packed-refs.lock storms | `refns:refs/remotes/` lease | yes | `verify/gates/g18` |
| REF-015 | Multi-push same tip | CAS / lock fail | `refns:receive-pack` lease | yes | `verify/gates/g18` |
| IDX-009 | Long hook holds index | index.lock across hooks | `hook:critical` + `index:paths:default` | yes | `verify/gates/g18` |
| IDX-012 | Per-WT index vs shared tip | tip CAS under WT | `index:paths:worktree` lease | yes | `verify/gates/g18` |
| WT-007 | Shared hooks across WTs | hook storm | `domain:worktree:hooks` lease | yes | `verify/gates/g18` |
| WT-008 | Worktree add name collide | admin dir fail | `domain:worktree:add` lease | yes | `verify/gates/g18` |
| CFG-001 | Concurrent config rewrite | config.lock / lost updates | `config:local` lease | yes | `verify/gates/g18` |
| TOCTOU-001 | status then commit | dirty between decision | `toctou:status-commit` lease-before-lock | yes | `verify/gates/g18` |

Wave Cd-5 **Done** = Docker G18 PASS for **all twelve** + Cd-1..Cd-4 regression (**36**/36).

## Wave Cd-6 (M20 claimed set)

Plan: `plan:root/20261110110000`. Bucket B continue after Cd-5.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| HOOK-004 | Server update/pre-receive | tip/hook lock class | `hook:update` lease | yes | `verify/gates/g18` |
| CFG-003 | Concurrent global config | config.lock class | `config:global` lease | yes | `verify/gates/g18` |
| REF-019 | WT-private vs common refs | shared tip surprise | `refns:common` lease | yes | `verify/gates/g18` |
| REF-023 | Branch rename under writers | rename vs tip lock | `refns:rename` lease | yes | `verify/gates/g18` |
| OBJ-002 | Loose object write herd | fan-out pressure class | `obj:loose:write` lease | yes | `verify/gates/g18` |
| OBJ-003 | Write before reachable ref | prune race class | `obj:write-before-ref` + maint honesty | yes | `verify/gates/g18` |
| SUB-002 | Concurrent submodule update | torn checkout class | `domain:submodule:update` | yes | `verify/gates/g18` |
| SUB-004 | Recursive submodule status | status storm class | `domain:submodule:status` | yes | `verify/gates/g18` |
| SUB-007 | Parallel modules/ clone | dir create race | `domain:submodule:modules` | yes | `verify/gates/g18` |
| TOCTOU-002 | Tip read then write no CAS | lost update class | `toctou:tip-cas` | yes | `verify/gates/g18` |
| TOCTOU-006 | Fetch then rebase | wrong base class | `toctou:fetch-rebase` | yes | `verify/gates/g18` |
| MISC-004 | Parallel clean -fdx | scratch wipe class | `misc:clean` lease | yes | `verify/gates/g18` |

Wave Cd-6 **Done** = Docker G18 PASS for **all twelve** + Cd-1..Cd-5 regression (**48**/48).

## Wave Cd-7 (M20 claimed set)

Plan: `plan:root/20261110120000`. Bucket B continue after Cd-6.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| MISC-010 | Shared identity env | misattributed commits | `misc:identity-env` | yes | `verify/gates/g18` |
| MISC-015 | WT add from bare | admin dir race | `domain:worktree:add-from-bare` | yes | `verify/gates/g18` |
| MISC-016 | Huge update-ref stdin | long lock hold | `refns:txn:stdin` | yes | `verify/gates/g18` |
| SUB-010 | deinit vs build | path removed under build | `domain:submodule:deinit` | yes | `verify/gates/g18` |
| SHAL-002 | Promisor queue pileup | latency / rate-limit class | `shallow:promisor` | yes | `verify/gates/g18` |
| PACK-007 | Promisor demand-fetch | duplicate fetch storms | `pack:promisor:fetch` | yes | `verify/gates/g18` |
| CI-003 | Shared fixture bare | push CAS flake | `ci:fixture-bare` | yes | `verify/gates/g18` |
| CI-004 | Bot temp branch ns | force-push races | `refns:refs/heads/bot/` | yes | `verify/gates/g18` |
| NET-001 | Parallel fetch | packed-refs contention | `net:fetch:origin` | yes | `verify/gates/g18` |
| NET-002 | Parallel push | non-ff / lock | `net:push:branch` | yes | `verify/gates/g18` |
| NET-007 | Credential helper stampede | helper thrash | `net:credential` honesty | yes | `verify/gates/g18` |
| LFS-002 | LFS lock ≠ index.lock | wrong exclusivity assumption | `lfs:path-lock` honesty | yes | `verify/gates/g18` |

Wave Cd-7 **Done** = Docker G18 PASS for **all twelve** + Cd-1..Cd-6 regression (**60**/60).

## Wave Cd-8 (M20 claimed set — bucket B leftovers)

Plan: `plan:root/20261110130000`. Final researched bucket B promoteables (PERF coordination + CFG-002 + AGENT-004 NFS honesty). Absolute PERF ms / ≥20% remain M14/G19.

| ID | Induce sketch | git pathology oracle | arks resolved oracle | Required | Harness |
|----|---------------|----------------------|----------------------|----------|---------|
| PERF-001 | Lock serialize herds | N× critical section class | `perf:lock:serialize` (no ms SLO) | yes | `verify/gates/g18` |
| PERF-002 | status walk storms | CPU/IO class | `perf:status:walk` | yes | `verify/gates/g18` |
| PERF-003 | fsmonitor backlog | queue backlog class | `perf:fsmonitor` | yes | `verify/gates/g18` |
| PERF-004 | commit-graph vs walks | rewrite contention | `perf:commit-graph` | yes | `verify/gates/g18` |
| PERF-005 | Loose object burst | inode/access lag | `perf:loose:burst` | yes | `verify/gates/g18` |
| PERF-006 | Process-spawn herd | fork/exec class | `perf:process:spawn` | yes | `verify/gates/g18` |
| PERF-007 | Pack mmap thrash | page-cache thrash | `perf:pack:mmap` | yes | `verify/gates/g18` |
| PERF-008 | Parallel blame | CPU meltdown class | `perf:blame` | yes | `verify/gates/g18` |
| PERF-009 | Parallel grep | IO saturation class | `perf:grep` | yes | `verify/gates/g18` |
| PERF-010 | Parallel diff --stat | latency spike class | `perf:diff:stat` | yes | `verify/gates/g18` |
| CFG-002 | includeIf TOCTOU | config view race | `config:includeIf` | yes | `verify/gates/g18` |
| AGENT-004 | Cross-host NFS WT | multi-host lock/corruption class | `agent:nfs:refuse` honesty | yes | `verify/gates/g18` |

Wave Cd-8 **Done** = Docker G18 PASS for **all twelve** + Cd-1..Cd-7 regression (**72**/72). Researched bucket B exhausted.

## Explicit Opens (not in M13/M20 Cd-8 claimed set)

Remainder = policy/remote G8 (CRED/NET live-forge), host-FS/platform, L3 refuse, absolute PERF SLOs (M14/G19). Do not claim “221 solved.”

## Evidence schema (receipt body)

Per ID: `scenario_id`, `git_outcome` (`pathology_pass` \| `fail`), `arks_outcome` (`resolved_pass` \| `fail`), `notes`. Aggregate: compose image pin, stock git version, tree SHA, gix-exit, wave `Cd-8`.
