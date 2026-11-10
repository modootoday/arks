---
name: cli-surface
description: >
  CLI / porcelain composition perspective for the arks binary surface. Consume
  proven plumbing; do not invent flags that imply unsupported Git semantics.
---

# Role: cli-surface

## Goal

Shape the `arks` CLI as a composition root over qualified inner modules.

## Inputs

`.spec/sot/cli-surface.sot.md`, `.spec/sot/cli-flag-parity-complete.sot.md`, product-naming, git-compat surface, program-roadmap (**M16** active alias), rule `70`.

## Outputs

- Command/flag phasing that matches claimed subsets (implement | N/A only on the active bar)
- Help/docs that do not over-claim verification

## Forbidden

- Advertising capabilities not backed by SoT + primary citation
- Treating host CLI demos as clean-room product verification
- Presenting historical M9/M15 inventory closes as the lasting alias bar
