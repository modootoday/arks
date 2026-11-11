---
title: arks agent
description: Optional LLM review and CEV bots on the same arks binary — B4 Claimed e2e; smokes Partial
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
arks agent provider smoke ollama   # optional local Ollama Partial; ≠ B4 Claimed
arks agent provider smoke --mock   # in-process openai-compat mock Partial
arks agent review --smoke --mock   # fixture + mock chat Partial; ≠ B4 Claimed
```

## Honesty (tip)

| Leaf / area | Class |
|-------------|-------|
| provider / config / doctor | **Partial** (TCP/connect retry + error class) |
| `provider smoke [ollama\|--mock]` | **Partial** optional local chat or in-process openai-compat mock; documents Missing if daemon absent — never spawns `ollama` CLI |
| `review --smoke [--mock]` | **Partial** built-in fixture (+ optional mock server); documents Missing if daemon absent without `--mock` — never invents Claimed |
| review / cev-check / run / serve / bot | **Partial** (health + openai-compat chat proxy; JSON + stream SSE; connect retry Partial; `--once` OK) |
| Claimed LLM chatbot / review e2e (**B4**) | **Claimed** — Docker openai-compat fixture e2e ([[evidence:root/20261110231959-claimed-agent-llm]]); smoke ≠ Claimed |

Owning residual gate: [arks-claimed-residual-gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md) **B4**. See [Completeness](../completion/).

Owning SoTs: [agent product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-product-intent.sot.md), [agent CLI surface](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-cli-surface.sot.md).

## Next

[Completeness](../completion/) [Forge client](../arks-hub/) [Limits](../limits/) [Contribute](../contribute/)
