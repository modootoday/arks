---
id: sot:root/arks-agent-cev-review
title: arks agent CEV review
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-10T19:23:00Z
tags: [arks-agent, cev, consistency, evidence, verify, specfold, review, matrix, honesty]
verify: manual
rel:
  depends_on:
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/agent-governance
    - sot:root/product-verification-clean-room
    - sot:root/public-surface
    - sot:root/interop-test-matrix
    - plan:root/20261111042300
  evidence: []
---

# arks agent CEV review

Version **0.1.0**. Normative definition of **CEV** for **`arks agent cev-check`** / CEV bots, and how that relates to Specfold, evidence, and clean-room verify.

Product intent: [[sot:root/arks-agent-product-intent]]. CLI: [[sot:root/arks-agent-cli-surface]]. Clean-room authority: [[sot:root/product-verification-clean-room]]. Classification: [[sot:root/agent-governance]].

## Rule

### Chosen definition (normative)

**CEV** in this repository means **Consistency / Evidence / Verify** inspection:

| Letter | Meaning | Agent checks (intent) |
|--------|---------|------------------------|
| **C — Consistency** | Change set vs Specfold SoT / plan / ADR / matrix wording | Do claims, command names, and single-`arks` shipping language stay consistent? Do new surfaces contradict active non-goals? |
| **E — Evidence** | Honesty of evidence pointers and residual language | Are `[[evidence:…]]` / receipt paths cited only when appropriate? Are secrets absent? Is “Design ≠ Claimed” respected? |
| **V — Verify** | Respect for verify / clean-room authority | Does the change invent **PASS** / **Verified** without Docker clean-room receipts? Does it treat host-only greens as authoritative? |

This is **not** an acronym collision with unrelated industry “CEV” terms. If prose needs disambiguation, expand once: **Consistency/Evidence/Verify**.

### What CEV bots do

1. Read **SoT + plan + matrix** (and optional PR diff) as inputs.
2. Emit structured findings: inconsistency, missing evidence cite, over-claim, secret risk, shipping-shape violation (e.g. advertising a second release binary).
3. Prefer **fail-closed**: when unsure whether a Claimed/PASS flip is evidenced, report a finding — do **not** auto-write PASS into matrices.
4. MAY suggest patch text; MUST NOT silently mutate verify receipts.

### What CEV bots MUST NOT do

5. Replace [[sot:root/product-verification-clean-room]] Docker authority with LLM judgment.
6. Mark matrix rows **PASS** / flip SoT **Claimed** without human + evidence process.
7. Act as a general chatbot or unrestricted repo rewriter.
8. Spawn `git`/`gh` to gather context — use arks libs / hub API / filesystem reads of tracked trees.

### Inputs (typical)

| Input | Source |
|-------|--------|
| Active SoTs / plans | `.spec/sot/`, `.spec/plan/` |
| Matrices | `verify/matrices/`, [[sot:root/interop-test-matrix]] |
| Evidence catalogs | `.spec/evidence/**` (scrubbed) |
| Diff / PR | Local patch via arks engine **or** hub PR API |
| Governance rules | [[sot:root/agent-governance]], `.spec/README.md` |

### Outputs (intent)

9. Human-readable report + optional `--json` for CI.
10. Optional hub PR review comments via native hub client (same rules as `arks agent review --post`).
11. Exit non-zero when `--strict` and any high-severity finding exists (when claimed).

### Relationship to automatic code review

| Mode | Focus |
|------|-------|
| `arks agent review` | Code quality / diff risk / style / likely bugs (LLM review) |
| `arks agent cev-check` | Specfold honesty: Consistency / Evidence / Verify |

They MAY share provider config and hub posting helpers; they MUST remain distinct commands/profiles.

### Clean-room / matrix pillar (cross-link only)

12. A future **agent e2e** clean-room pillar (bot comments on hub PR in Docker, etc.) MAY extend the verify-matrix stream under [[sot:root/verify-cleanroom-symmetry-matrix]] (today: three pillars CLI/daemon/hub). This SoT **does not** invent PASS rows or gate IDs for that pillar.
13. Until such a gate exists with evidence, agent CEV remains **design / advisory** only.

## Why

arks already treats Specfold SoT + evidence + clean-room verify as the honesty spine. Naming CEV explicitly as Consistency/Evidence/Verify gives bots a crisp job that complements (not replaces) Docker verification and keeps LLM output from laundering false Claimed/PASS language.

## Verification

1. This SoT is the sole normative CEV definition for `arks agent`.
2. No PASS/Claimed invented from design-only agent runs.
3. `spec check` clean for this document.
