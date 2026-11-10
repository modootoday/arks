# G17 BUGSIM — concurrency issue-repro compare (Cm-1..Cm-3 + M19 Cx-1)

Induce → stock `git` pathology → `arks` resolved → compare.

| ID | Induce | Expected git | Expected arks |
|----|--------|--------------|---------------|
| AGENT-006 / IDX-008 | Plant held `index.lock` (optional-lock class) | `git add` fails | status agent/`--no-optional-locks` creates no lock; writer OK when lock absent |
| LOCK-002 | Plant stale `index.lock` (dead PID) | `git add` fails | `lock inspect` + `lock clear --stale`; then add OK; live PID refuse |
| WT-001 | Linked wt + common `HEAD.lock` / ref lock | git writer from wt fails | arks fail-closed; locked remove refuse |
| SUB-001 | Submodule `index.lock` planted | induced git path fails | arks fail-closed dual-domain honesty |
| IDX-001 | Held exclusive `index.lock` | `git add` fails | fail-closed + retry/exclusive honesty |
| HOOK-001 / HOOK-006 | Sync hooks installed | hook in critical section class | refuse inventing async; `--no-verify` escape |
| PACK-004 / MAINT-003 | `gc` / maintenance under writers | stock path available | refuse-closed gc/repack/maintenance |
| AGENT-003 | Contended lock + blind retry | retries keep failing | backoff/retry guidance + lock note |
| IDX-002 | Background/optional status lock vs writer | held lock blocks `git add` | default status takes no optional locks; writer OK |
| AGENT-005 | Dead/evicted PID lock on volume | next `git add` fails | inspect + `clear --stale` with AGENT-005 note |
| AGENT-007 | Worktree + submodule multi-domain locks | writers fail across domains | fail-closed + AGENT-007 multi-domain honesty |
| CORR-001 | `gc --prune=now` under writers | stock prune=now available | refuse-closed citing CORR-001 |
| FS-006 | Shared-path lock (bind-mount dual-writer class) | lock blocks stock writer | FS-006 inspect note; honor locks; clear only dead |
| REF-007 | Plant `HEAD.lock` | HEAD update fails | fail-closed; inspect REF-007 |
| LOCK-001 | Plant exclusive `index.lock` | second writer fails | fail-closed; inspect LOCK-001 |
| WT-002 | Second worktree same branch | already checked out | same fail-closed |
| WT-004 | Lock worktree then remove | remove refuses | refuse locked; unlock then remove |
| SEQ-001 | Plant `rebase-merge` | rebase fails | refuse SEQ-001 |
| SEQ-002 | Plant `MERGE_HEAD` then rebase | rebase fails | refuse SEQ-002 |
| SEQ-003 | Plant `CHERRY_PICK_HEAD` | cherry-pick fails | refuse SEQ-003 |
| REF-018 | Plant `MERGE_HEAD` then cherry-pick | conflicting porcelain fails | refuse REF-018 |
| MISC-012 | Plant `index.lock`; `update-index --refresh` | refresh fails | fail-closed MISC-012; unlocked OK |
| TOCTOU-005 | Held lock + poll temptation | add still fails | never poll-skip; inspect TOCTOU-005 |
