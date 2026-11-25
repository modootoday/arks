---
id: adr:root/20261124000000
title: Run arks unattended until final completion under decision classes A / H / O
status: active
created: 2026-11-24T00:00:00Z
updated: 2026-11-24T00:00:00Z
tags: [governance, autonomy, unattended, permissions, merge, auto-merge, escalation]
rel:
  depends_on:
    - sot:root/agent-governance
    - adr:root/20261119220000
---

# Run arks unattended until final completion under decision classes A / H / O

## Context

The owner directed (2026-11-24) that arks be operated fully automatically, without a human in
the loop, until final completion, with detailed governance and optimal permission settings.
Earlier directives already let agents merge their own PRs through auto-merge (2026-11-20:
"merge it yourself, follow the recommendations") and set the argv-symmetry ladder, which closed
with the A_final receipt ([[evidence:root/20261120175548-argv-inventory-final]]).

The always-on rules predate that mode. Rule `agent-governance` §12, rule `90` item 4 and rule `91`
item 5 still say agents MUST NOT push or create branches without an explicit order, which
contradicts the PR flow every session has used since PR #1. No document said which changes an
agent may merge on its own, when it must stop, how it picks the next work, or what "complete"
means. The tracked `.claude/settings.json` carried only the `playpilot guard` hook, so every
unattended session prompted for routine commands or ran with whatever its launcher allowed.

## Decision

1. Adopt [[sot:root/autonomous-operation]] as the process canon for unattended work: a fixed
   operating loop, a work-selection order, a PR lifecycle with a retry budget, verification
   tiers, stop signals, reporting, and a testable final-completion condition.
2. Sort every change into **class A** (agent merges through auto-merge), **class H** (agent
   prepares, owner merges; label `needs-human`) or **class O** (owner only). Permission and
   `.github/` changes are never class A.
3. Replace the stale "no push" wording: pushing the session's own `claude/*` topic branch and
   opening a PR is the normal loop; pushing `main`, tags or other branches stays forbidden.
4. Track a permission baseline in `.claude/settings.json`: an allow list for the loop's commands
   and a deny list for pushes to protected refs, unleased force-pushes, identity or global git
   configuration, secret reads, publishing, privilege escalation and edits to the permission
   files themselves. The `playpilot guard` hook stays.
5. Mirror the playpilot canonical labels in `.github/labels.json` and add the `Public surface` check
   to the required checks of `.github/rulesets/main-protected.json` (both applied by the owner).
6. The kill switches are the fleet's: pause the scheduled task, `ops.paused` on the fleet Project
   or Session, or revoke the mailbox token (amended 2026-11-24 to drop an arks-only pause label).

7. **Amendment (2026-11-25):** the fleet adopted the playpilot governance canon (G1–G10). Classes
   A / H / O become the canon levels **A0** (auto) / **A1** (measured, `needs-human`, then
   auto-merge) / **A2** (owner queue); the retry budget becomes two attempts per step; blocked
   A2 items go to the coordinator's Owner queue through the mailbox.

## Consequences

- Unattended sessions run the loop without prompts for routine commands and cannot widen their
  own permissions: the permission files are deny-listed for editing and permission changes are
  class H (narrowing) or class O (widening).
- Merges of product code need green required checks; governance, CI and permission changes wait
  for the owner, so a faulty agent cannot relax its own guard rails.
- The fleet side (a `Project` and `Session` for arks in the private fleet repository) is class O
  under that repository's own rules; it is prepared as a patch for the owner, not applied here.
- "Complete" is checkable from `main`: no active non-maintenance plan, no unreasoned Partial
  residual, zero unreasoned corpus DIFFs in the latest receipts, green required checks.

## Alternatives considered

- **Bypass mode for unattended sessions:** rejected; it removes the deny list and lets an agent
  push protected refs if the guard hook is ever missing.
- **Owner review of every PR:** rejected; it stalls the loop on routine product work, which the
  required checks and receipts already gate.
- **Agent-merged governance changes:** rejected; an agent could relax its own constraints.
