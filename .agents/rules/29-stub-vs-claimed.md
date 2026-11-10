# Stub / Partial vs Claimed wording

Help text, commit subjects, residual rows, and agent summaries MUST keep honesty classes distinct. Design and Partial stubs are allowed; inventing Claimed is not (rule `28-evidence-honesty`).

## Vocabulary (use exactly)

| Class | When to use |
|-------|-------------|
| **Design** | SoT intent only; no shipping claim |
| **Partial / Missing / stub** | Surface exists incompletely; fail-closed / refuse OK |
| **Claimed** | Product surface ships **with** citing evidence + owning SoT |
| **Verified** | Clean-room Docker receipt (rule `60`) |
| **Refuse / Impossible** | Explicit non-goal |

## Must

- Label new clap/help stubs **Partial** or **Missing** until evidence lands.
- Prefer fail-closed exits and honest `--help` over silent success.
- In commits: describe the stub (“add fail-closed … stub”) — do **not** say “Claimed” / “Verified” / “PASS” unless evidence paths are cited in the same change set.
- Keep tip / residual tables aligned with owning SoTs when honesty class changes.

## Must not

- Call a clap stub, scaffold, or host demo **Claimed** / **Verified** / **PASS**.
- Soft-promote hub / daemon / agent / personality / self-host **design** SoTs to Claimed in help or commit prose.
- Use Hangul “operator honesty” in tracked wording (rule `05`).

## Cite

- rule `28-evidence-honesty`
- skill `guard-cli-surface`
- `.spec/sot/production-residual-register.sot.md`
