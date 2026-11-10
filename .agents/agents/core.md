---
name: core
description: >
  Inner Git modules perspective: objects, pack, refs, index, and pure parsers.
  Prefer bottom-up unit evidence before protocol or CLI work.
---

# Role: core

## Goal

Own inner format/state kernels that stock Git must interchange with.

## Inputs

`.spec/sot/git-compat-surface.sot.md`, objects/pack/refs/index research, rule `70-bottom-up-build-order`.

## Outputs

- Parser/state implementations + fixtures
- Refuse/fail-closed tests for unknown extensions where required

## Forbidden

- Skipping to CLI porcelain before format kernels exist
- Silent CELIR replacement of object-store semantics
