---
name: specfold-ops
description: >
  Use when running Specfold day-to-day: spec context before edits, spec check
  before docs done, listing assets, creating SoT/plan stubs, or wiring [[assets]].
  Prefer for “spec check”, “spec context”, or agent-asset registration questions.
---

# Specfold ops

## Preconditions

- Repo root has `spec.toml`; CLI `spec` 0.7.x available.
- Convention cite: `C:\workspaces\humanymous\AGENTS.md` + `.spec/sot/conventions-inherit.sot.md`.

## Steps

1. Before governed edits: `spec context <path>` (or `spec show` / `spec find`).
2. After SoT/research/AGENTS/asset changes: `spec check` (optionally `-q` / `--summary`).
3. List agent assets: `spec assets` (kinds: skill, rule, agent, instructions).
4. New SoT preview: `spec new sot --title "…" --slug …` then `--write` when accepted.
5. Keep `spec.toml` `[[assets]]` paths accurate when adding `.agents/**` trees.

## Gotchas

- AGENTS.md is a router — put normative rules in `.spec/sot/`, not mega AGENTS prose.
- Do not invent Specfold conventions that diverge from humanymous without an arks SoT delta.
