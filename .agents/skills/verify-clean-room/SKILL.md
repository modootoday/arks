---
name: verify-clean-room
description: >
  Use when finishing product/interop work or before claiming verified: run or
  record Docker clean-room gates per the verification SoT. Prefer for acceptance,
  stock-Git round-trip, or GitHub-class remote verification claims. Gates G0–G27 (+G28 when present).
---

# Verify clean-room

## Policy

**Product verification completion authority is Docker clean-room.** Host/loopback alone is **not** done.
SoT: `.spec/sot/product-verification-clean-room.sot.md`. Rule: `60-clean-room-verify`.
Gate ladder: `.spec/sot/interop-test-matrix.sot.md` (**G0–G27**; +G28 when present).

## Steps

1. Read the verification SoT (authority, forbidden-as-verified, open image/lock decisions).
2. Name the Gate ID for the claim (G14/G11/G15/G16–G27 / additive live-forge when present). Prefer `verify/matrix-*.md` + compose services.
3. If harness/images exist: run the declared compose/entrypoint; record image pins and outcomes.
4. If harness is not yet present: report **manual / not yet harnessed** — do not invent digests or fake green.
5. Keep host unit results clearly labeled advisory.
6. **Sole-runner honesty:** before starting heavy journey / live forge / contention compose, check for an already-running sibling Docker agent. Do not interrupt or invent a competing sole-runner.
7. On Windows engine questions, cite humanymous `96-docker-engine-affinity` / `60-e2e-docker-only` — do not fork.

## Cite

- Humanymous completion-authority pattern: `C:\workspaces\humanymous\.agents\skills\red-blue-validate\SKILL.md` (Docker-only e2e) — affinity of *idea*, not a mandate to copy lab topology.
- Live forge journeys: residual register + any landed `*g28*` SoT/matrix (opt-in secrets; default Refuse).
