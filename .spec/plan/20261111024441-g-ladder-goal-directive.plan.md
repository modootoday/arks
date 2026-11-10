---
id: plan:root/20261111024441
title: G-ladder goal directive (acceptance, Claimed vs Hold)
status: active
created: 2026-11-10T17:44:41Z
updated: 2026-11-10T17:44:41Z
tags:
  [
    goal-directive,
    g-ladder,
    acceptance,
    claimed,
    hold,
    docker,
    oom,
    no-invent-pass,
  ]
verify: manual
rel:
  depends_on:
    - plan:root/20261111024440
    - research:root/20261111022345
    - research:root/20261111012840
    - sot:root/program-roadmap
    - sot:root/public-surface
    - sot:root/product-verification-clean-room
    - sot:root/pf5-journey-seed-amortization
---

# G-ladder goal directive

English normative directive for agents executing [[plan:root/20261111024440]]. Owning research: [[research:root/20261111022345]].

## Mission

Execute **G1 → G_final** to the end: Specfold amend → code → evidence → e2e → docs → commit/push with CI green. Do not stop at planning.

## Acceptance criteria (all goals)

1. **No invent PASS.** SKIP, dry-run, local-forge `claimed-wire`, and host-only greens are not live or clean-room Claimed substitutes.
2. **Claimed vs Hold.** Promote to Claimed only with SoT amend + receipts for the stated slice. Keep Hold/Impossible/Non-goal rows honest forever where listed below.
3. **Docker-only heavy.** Product “verified” and G19 remasure require Docker clean-room. Host compile/tests are advisory.
4. **Halt OOM.** If Docker sole-runner OOM or host memory pressure threatens the machine, **stop** the heavy gate, file evidence of halt, keep tip honesty, do not thrash retries.
5. **Exactly one sole G19 for G7.** After a justified PATH/`exec` product cut, run **one** sole Docker G19. PASS ≤0.5× **or** retain honest `aggressive_na`. No second remasure after FAIL. No concurrent G19.
6. **L9.** Refresh reopen pack only. Agents MUST NOT flip `publish = true`, bind tokens, or run live `cargo publish` without owner SoT reopen.
7. **G1 Live G8.** If Environment `g8-live` secrets `total_count < 2`, refresh Blocked evidence and continue. Never invent latency medians.
8. **English only** for authored Specfold/docs/commit prose.
9. **No CELIR ObjectStore reverse; no `gix`; no GUI/scalar embed; no blanket “faster than Git.”**

## Claimed upgrade targets (this ladder)

| Goal | Honest Claimed outcome if pursued |
|------|-----------------------------------|
| G1 | Live G8 latency Done **only** with secrets + non-SKIP receipt; else Blocked refresh Claimed-as-honesty |
| G2 | Reopen pack current; L9 stays Blocked |
| G3 | Server `bundle-uri` advertise + serve path Claimed |
| G4 | LFS locks HTTP subset Claimed; tus/multi-tenant S3 remain handoff/Partial |
| G5 | Forge recipe deepen Claimed-closest (thin `gh` wrap) |
| G6 | `object:type=…` then `combine:` of already-Claimed children Claimed |
| G7 | ALIAS ≤0.5× Claimed **or** `aggressive_na` retained after one G19 |
| G_final | Tip + public docs + residual currency match evidence; new Claimed surfaces have e2e |

## Hold forever (do not schedule as Claimed)

| Item | Class |
|------|-------|
| Absolute SHA-1 collision immunity | Impossible / Non-goal |
| Full cloud-sync / OneDrive hydration / LSM bypass safety | Impossible |
| CELIR ObjectStore (reverse Refuse ADR) | Refuse / Impossible |
| Blanket “faster than Git” marketing | Forbidden / Hold |
| Multi-writer absolute warranty (“all races fixed”) | Impossible as warranty |
| GUI embed (`gitk` / `git-gui` / citool) / Scalar product embed | Non-goal |
| Full forge Checks/PR/Issues/merge-queue product UX | Non-goal |
| Embedded browser OAuth product | Impossible without AUTH SoT reopen |
| Mixed sha1↔sha256 remotes as full product | Non-claim / Hold |
| Dumb HTTP `push --force-with-lease` (unless peer forces reopen) | Low-ROI Hold (smart-only OK) |

## Ops / Docker rules

- Prefer sole-runner Docker for G14–G28 / G19 class work; avoid colliding live-GH agents.
- Evidence under `.spec/evidence/<ts>-…/` with index linking SoT/plan IDs.
- Push tip; require CI green or document residual CI failure honestly (no greenwash).

## Exit

G-ladder table in [[sot:root/program-roadmap]] tip currency shows each G1…G_final disposition with tip SHA + evidence links. Operator one-word remains honest (**PARTIAL** while G1 secrets or L9 owner blocks persist).
