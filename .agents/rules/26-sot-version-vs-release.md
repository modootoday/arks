# SoT Version ≠ product / crate release

Body `Version **0.x.y**` on a Specfold SoT is a **document revision**, not crates.io / git-tag product release. Workspace crates remain `0.0.0` until an intentional product tag.

Owning index: `.spec/README.md` §SoT `Version` ≠ product release. Classification: rule `25-spec-classification`.

## Must

- Bump SoT `Version` **only** when MUST/MAY/inventory/advice **meaning** changes.
- Prefer frontmatter `updated:` for link fixes, typo fixes, honesty refreshes, and tip-table currency without meaning change.
- Prefer **+0.0.1** additive clarification, **+0.1.0** superseding advice; stay in **0.x** until a deliberate post-release SoT policy says otherwise.
- Mark new design SoTs **design / Partial** until Claimed + evidence (rule `28-evidence-honesty`).

## Must not

- Treat SoT `0.2 → 0.5` (or roadmap `0.15.x`) as “five product releases.”
- Bump SoT `Version` because code landed, workflows landed, or residual rows refreshed.
- Advertise crate/`v*` release from Spec document revision alone.

## Cite

- `.spec/README.md`
- skill `design-sot`
