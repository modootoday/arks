---
id: adr:root/20261110151000
title: G8 live-forge default refuse-close without operator secrets
status: superseded
created: 2026-11-10T15:10:00Z
updated: 2026-11-10T22:00:00Z
tags: [g8, live-forge, refuse-close, clean-room, network-policy, m22, superseded]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/arks-remote
    - sot:root/remote-host-compatibility-surface
    - sot:root/production-residual-register
    - plan:root/20261110150000
  evidence: [evidence:root/20261110152000]
---

# G8 live-forge default refuse-close without operator secrets

## Context

Gate **G8** (`remote-live-smoke`) is the optional live clone/fetch/push against a throwaway GitHub-class peer ([[sot:root/interop-test-matrix]]). Offline **G7** loopback/recorded smart-HTTP remains the default authoritative remote-client proof. The clean-room SoT previously left “Network policy for live GitHub smoke” as an **Open** decision, and the residual register tracked G8 as **Closed-deferred (policy)** — which M22 forbids as a lasting work item ([[plan:root/20261110150000]]).

Live forge smoke requires operator-supplied network access and a secret (PAT / deploy token) for a throwaway repository. Shipping those secrets into default CI or inventing a fake live green is forbidden.

## Decision

1. **Policy unlock (Closed):** Default authoritative remote-client path is **offline G7**. Live G8 is **opt-in** only when all of the following are set in the clean-room environment: `ARKS_G8_LIVE=1`, a non-empty forge token (`ARKS_G8_TOKEN` or `GITHUB_TOKEN`), and a throwaway HTTPS clone URL (`ARKS_G8_REPO_URL`).
2. **Default product residual — Refuse-closed:** When opt-in secrets are absent, G8 is **Refuse-closed** for product residual honesty. The compose service `g8` MUST record a refuse receipt and MUST NOT pretend live forge verification passed.
3. **Opt-in verify:** When opt-in vars are present, `g8` MUST perform live clone → push branch → fetch tip against the throwaway peer and scrub token material from logs; success yields **Closed (implement+verify)** evidence superseding the default refuse for that run only.
4. **No fake greens:** Absence of secrets MUST NOT be labeled “G8 PASS (live)”. Catalog CRED/NET Docker-local honesty (M21 Dz-3) remains distinct from product G8 live-forge.
5. **G7 unchanged:** G7 continues as CI-default remote proof; this ADR does not redefine M6/M9 exits.

## Consequences

- [[sot:root/product-verification-clean-room]] Open “Network policy for live GitHub smoke” becomes **Closed** under this ADR.
- Residual register G8 row becomes **Refuse-closed (default)** citing this ADR; opt-in live evidence MAY later amend to Closed (implement) without reopening Closed-deferred.
- Public copy MUST NOT claim live GitHub-class verified without a live G8 receipt.
- Operators who need live smoke enable opt-in vars locally or in a secrets-aware CI job.

## Supersedes / amends

- Closes the Open network-policy decision in [[sot:root/product-verification-clean-room]].
- Replaces residual **Closed-deferred (policy)** for G8 with **Refuse-closed (default)** / opt-in Closed.
- Does **not** amend G7 carrying receipts or M21 catalog Claimed rows.

## Superseded by

- [[adr:root/20261110220000]] — G8 live default-on when operator secrets present (`ARKS_G8_LIVE=1` no longer required).
