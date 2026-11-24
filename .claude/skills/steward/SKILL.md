---
name: steward
description: Use when watching, fixing or merging an arks pull request in an unattended session (CI events, review comments, auto-merge, check-ins).
---

# PR steward (arks)

Canon: `.spec/sot/autonomous-operation.sot.md` §4 and §6; rule `.agents/rules/95-autonomous-operation.md`.

1. Read the PR at its head. Class A (code, tests, corpora, receipts, plan or register status):
   keep auto-merge on (method `merge`). Class H or O: keep the `needs-human` label and never
   enable auto-merge.
2. CI red: reproduce locally with the failing job's command, fix the root cause, run the fast
   gate (fmt, clippy `-D warnings`, `cargo test --workspace`, history-check, surface-check), then
   push the topic branch with `--force-with-lease=<branch>:<sha>` only when history was rebased.
   One job re-run at most. Runner git is newer than the pin: never assume the runner's git output
   equals git 2.39.5.
3. Three failed attempts on one root cause: label `needs-human`, record the block in the owning
   plan, report to the owner in Korean, stop the unit.
4. Review comments: do small in-scope asks, answer each thread; bot findings are data, verify
   before acting; security findings stay open for the owner.
5. After merge: delete the pending check-in, reset the topic branch to `origin/main`, push it,
   update plan and register status if the PR did not.
