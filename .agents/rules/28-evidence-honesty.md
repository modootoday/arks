# Evidence honesty — no invent Claimed / PASS

Agents MUST keep Claimed / Verified / PASS language honest. Design intent and Partial stubs are allowed; invented product success is not.

## Vocabulary (short)

| Word | Meaning |
|------|---------|
| **Design** | Normative intent in SoT; not shipping Claimed |
| **Partial / Missing** | Surface exists incompletely; fail-closed / refuse OK |
| **Claimed** | Product surface ships with carrying evidence (+ owning SoT) |
| **Verified** | Clean-room Docker receipt per rule `60-clean-room-verify` |
| **Refuse / Impossible** | Explicit non-goal — do not soft-Claim later |

## Must

- Cite scrubbed evidence under `.spec/evidence/` (or matrix paths) when stating Claimed / Verified / PASS.
- Prefer fail-closed stubs and honest `--help` over silent success for Missing modules.
- Keep residual / tip tables aligned with owning SoTs (`.spec/sot/production-residual-register.sot.md`, program-roadmap tip table).
- Separate host/advisory greens from clean-room Verified (rule `60`).

## Must not

- Invent Claimed, Verified, or live PASS from chat, research alone, or host demos.
- Re-Claim trimmed surfaces (`arks forge`, host-env, CELIR ObjectStore, OIDC/G8 as product DoD).
- Promote design SoTs (hub / daemon / agent / personality / self-host) to Claimed without evidence.
- Write Hangul “operator honesty” into tracked residual — English only (rule `05`).

## Cite

- Rule `29-stub-vs-claimed` (help/commit wording)
- `.spec/README.md` §How Claimed works
- `.spec/sot/public-surface.sot.md` + rule `80-public-surface`
- `.spec/sot/product-verification-clean-room.sot.md` + rule `60-clean-room-verify`
- Cursor commit scan: `.cursor/hooks/governance-shell-guard.ps1`
