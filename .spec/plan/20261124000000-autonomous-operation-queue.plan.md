---
id: plan:root/20261124000000
title: Autonomous operation queue — units after the argv-symmetry ladder
status: active
created: 2026-11-24T00:00:00Z
updated: 2026-11-24T00:00:00Z
tags: [autonomy, queue, residual, plan]
verify: manual
rel:
  depends_on:
    - sot:root/autonomous-operation
    - plan:root/20261120000000
    - sot:root/production-residual-register
---

# Autonomous operation queue

The work queue unattended sessions consume (tier 3 of [[sot:root/autonomous-operation]] §3).
Each unit is one PR. Status values: `open`, `in-progress`, `done` (with the PR), `owner-blocked`
(with the owner action that unblocks it).

| # | Unit | Class | Status | Exit |
|---|------|-------|--------|------|
| Q1 | `log -s --raw` prints nothing and `log --name-only -s` exits 128, as pin 2.39.5 | A | open | new corpus lines SAME in Docker |
| Q2 | `fsck --unreachable` output order (family-plumbing L428): derive the order from observed pin output or record Impossible with the reason | A | open | line SAME or reasoned Impossible in the register |
| Q3 | gh personality: unknown flag exits 1 like gh (arks exits 2) | A | open | gh argv inventory exit-code column matches |
| Q4 | Rulesets enforced on push to an `arks serve` forge (today stored, not enforced) | A | open | hub e2e journey refuses a push that a ruleset forbids |
| Q5 | `instaweb` serves gitweb-equivalent pages | A | open | corpus lines for page fetches SAME or reasoned |
| Q6 | `send-email` output against a captured SMTP session (no git-email oracle in the image) | A | open | receipt from a fixture SMTP server |
| Q7 | Daemon on Windows: syslog, inetd, detach, user and drive paths | A | open | Windows CI test plus honest refuse rows |
| Q8 | Real OS credential stores for `credential-manager` / `credential-wincred` on each OS | A | open | per-OS CI test where a store exists, honest refuse elsewhere |
| Q9 | TLS / CRAM-MD5 for `send-email` / `imap-send` against real servers | O | owner-blocked | owner supplies a test mail account (secret) |
| Q10 | Live G8 latency and L9 publish timing | O | owner-blocked | owner supplies forge secrets and publish policy |
| Q11 | Apply `.github/labels.json` and the `Public surface` required check; set `ARKS_DENY_TERMS` | O | owner-blocked | owner applies the declared GitHub settings |
| Q12 | Fleet `Project` and `Session` for arks in the private fleet repository | O | owner-blocked | owner merges the prepared fleet patch |
| Q13 | Commit signing or hook change for the Unverified badge | O | owner-blocked | owner picks signing or a hook change |

After Q1–Q8 merge, the next session re-runs the Docker A_final receipt and checks §11 of the
autonomous-operation SoT.
