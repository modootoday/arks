---
id: adr:root/20261110220000
title: G8 live-forge default-on when operator secrets present
status: active
created: 2026-11-10T22:00:00Z
updated: 2026-11-10T22:00:00Z
tags: [g8, live-forge, default-on, secrets, clean-room, residual, claimed]
rel:
  depends_on:
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/arks-remote
    - sot:root/remote-host-compatibility-surface
    - sot:root/production-residual-register
    - plan:root/20261110150000
  supersedes: [adr:root/20261110151000]
  evidence: [evidence:root/20261110222000]
---

# G8 live-forge default-on when operator secrets present

## Context

[[adr:root/20261110151000]] Refuse-closed default live forge unless operators set `ARKS_G8_LIVE=1` **and** a forge token **and** a throwaway HTTPS URL. Residual **FW-G8-DEFAULT** tracked that refuse as terminal until a policy ADR amend for default-on live.

Operators who already supply throwaway secrets expect live G8 to run without a redundant `ARKS_G8_LIVE=1` flag. Inventing live PASS without secrets remains forbidden.

## Decision

1. **Default-on when secrets present (Claimed):** When a non-empty forge token (`ARKS_G8_TOKEN` or `GITHUB_TOKEN`) **and** a throwaway HTTPS clone URL (`ARKS_G8_REPO_URL`) are present in the clean-room environment, gate **G8** MUST run live clone → push branch → fetch tip by default. `ARKS_G8_LIVE=1` is **not** required.
2. **Honest SKIP / refuse when secrets absent:** When token or URL is missing, G8 MUST record an honest **SKIP / refuse-closed** receipt and MUST NOT pretend live forge verification passed. Offline **G7** remains the default authoritative remote-client proof for CI without secrets.
3. **Explicit force-off:** `ARKS_G8_LIVE=0` MAY force SKIP even when secrets are present (operator escape hatch). Unset or any other value does not block default-on live when secrets exist.
4. **No fake greens:** Absence of secrets MUST NOT be labeled “G8 PASS (live)”. Catalog CRED/NET Docker-local honesty (M21 Dz-3) remains distinct from product G8 live-forge.
5. **Scrub:** Live runs MUST scrub token material from logs (same honesty as prior ADR §3).
6. **G7 unchanged:** This ADR does not redefine M6/M9 exits or G7 offline carrying receipts.

## Consequences

- [[sot:root/production-residual-register]] **FW-G8-DEFAULT** → **Claimed (default-on when secrets)** citing this ADR.
- [[sot:root/product-verification-clean-room]] network-policy row cites default-on-when-secrets + honest SKIP when absent.
- Compose service `g8` and `verify/scripts/run-g8.sh` implement the enablement rule above.
- Public copy MUST NOT claim live GitHub-class verified without a live G8 receipt.

## Supersedes / amends

- **Supersedes** [[adr:root/20261110151000]] for G8 enablement policy (opt-in `ARKS_G8_LIVE=1` triple → secrets-default-on).
- Does **not** amend G7 carrying receipts, G28 journey secrets policy, or invent latency SLOs.
