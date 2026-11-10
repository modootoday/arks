---
title: arks agent
description: Optional LLM review and CEV bots on the same arks binary — Partial
nav_order: 46
sections:
  - what
  - honesty
  - {c: cta, name: next}
---

# arks agent

`arks agent …` is an **explicit extension** for LLM-powered code review and CEV (Consistency / Evidence / Verify) inspection bots. It ships on the **same** `arks` binary — there is **no** separate `arks-agent` product release.

Agent features are **not** required for `git`→`arks` alias DoD, daemon symmetry, or forge-client Claimed bars.

## What it is (design)

- Nested group: `arks agent <subcommand>…`
- Talks to configured LLM providers over HTTPS; may integrate with local forge APIs via native HTTP
- Must **not** spawn `git` or `gh`
- Must **not** invent PASS / Claimed in CEV output

```bash
arks agent --help
arks agent doctor
```

## Honesty (tip)

| Leaf / area | Class |
|-------------|-------|
| provider / config / doctor | **Partial** |
| review / cev-check / run / serve / bot | **Partial** (health + openai-compat chat proxy; JSON pass-through + `stream=true` SSE Partial; `--once` OK) |
| Claimed LLM chatbot / review e2e | **Missing** — no Claimed review quality bar from docs alone |

Treat everything on this page as **design / Partial** unless a clean-room receipt says otherwise. See [Completeness](../completion/).

Owning SoTs: [agent product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-product-intent.sot.md), [agent CLI surface](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-cli-surface.sot.md).

## Next

[Completeness](../completion/) [Forge client](../arks-hub/) [Limits](../limits/) [Contribute](../contribute/)
