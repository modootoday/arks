---
title: arks agent
description: Optional LLM review and CEV bots on the same arks binary — B4 Claimed e2e; smokes Claimed as diagnostics
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
arks agent provider smoke ollama   # one chat against local Ollama (http://); ≠ B4 proof
arks agent provider smoke --mock   # same, against the in-process openai-compat mock
arks agent review --smoke --mock   # review fixture through the mock; ≠ B4 proof
```

## Honesty (tip)

| Leaf / area | Class |
|-------------|-------|
| provider / config / doctor | **Partial** (TCP/connect retry + error class) |
| `provider smoke [ollama\|openai-compat\|--mock]` | **Claimed (diagnostic)** over plain-HTTP openai-compat endpoints: exit 0 only on a 2xx chat answer, 1 when unreachable ([20261125031425-claimed-smoke](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261125031425-claimed-smoke)); never spawns `ollama` CLI. `https://` endpoints use the same TLS stack as the git remote client: built-in web roots plus `GIT_SSL_CAINFO` / `http.sslCAInfo` / `SSL_CERT_FILE`, and `HTTPS_PROXY` / `NO_PROXY`; an untrusted certificate fails with `error_class=err_class=tls` and exit 1. Host-tested against a test CA, and run live against an OpenAI-compatible vendor with the owner's key on 2026-11-25 ([20261125063733-agent-live-https-smoke](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261125063733-agent-live-https-smoke), host-advisory: a live third-party endpoint has no clean-room receipt) |
| `review --smoke [--mock]` | **Claimed (diagnostic)** built-in fixture (+ optional mock server); exit 1 when the provider cannot be reached ([20261125031425-claimed-smoke](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261125031425-claimed-smoke)) |
| review / cev-check / run / serve / bot | **Partial** (health + openai-compat chat proxy; JSON + stream SSE; connect retry Partial; `--once` OK) |
| Claimed LLM chatbot / review e2e (**B4**) | **Claimed** — Docker openai-compat fixture e2e ([20261110231959-claimed-agent-llm](https://github.com/modootoday/arks/tree/main/.spec/evidence/20261110231959-claimed-agent-llm)); smoke ≠ Claimed |

Owning residual gate: [arks-claimed-residual-gates](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-claimed-residual-gates.sot.md) **B4**. See [Completeness](../completion/).

Owning SoTs: [agent product intent](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-product-intent.sot.md), [agent CLI surface](https://github.com/modootoday/arks/blob/main/.spec/sot/arks-agent-cli-surface.sot.md).

## Next

[Completeness](../completion/) [Forge client](../arks-hub/) [Limits](../limits/) [Contribute](../contribute/)
