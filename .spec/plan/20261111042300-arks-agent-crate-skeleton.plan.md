---
id: plan:root/20261111042300
title: arks-agent internal crate + arks agent dispatch
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-10T19:23:00Z
tags: [arks-agent, crate, skeleton, workspace, llm, openai-compat, review, cev, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/arks-agent-llm-providers
    - sot:root/arks-agent-cev-review
    - sot:root/crate-architecture
    - sot:root/product-intent
    - sot:root/arks-hub-product-intent
    - sot:root/program-roadmap
  implements:
    - sot:root/arks-agent-product-intent
---

# arks-agent internal crate + `arks agent` dispatch

## Goal

Design-first home for LLM **code review** and **CEV** bots as **`arks agent …`** on the **single** published binary **`arks`**. Internal library crate **`arks-agent`**; native HTTP to LLM providers + hub — **never** wrap `git`/`gh`/vendor CLIs. SoT set is the primary deliverable; `arks agent --help` stub OK.

## Directive (normative)

1. Crate `crates/arks-agent` is a **library** (not a published product bin).
2. Workspace `members` / `default-members` include agent; `arks` (bin package) depends on it and dispatches the **`agent`** command group.
3. **Do not** ship / advertise a separate `arks-agent` binary or crates.io release.
4. LLM HTTP via provider abstraction ([[sot:root/arks-agent-llm-providers]]): **one OpenAI-compat client** for OpenAI + Ollama + LocalAI + vLLM + LM Studio + custom base URL; vendor clients only where needed (Anthropic / Gemini / Cloudflare / xAI).
5. Hub PR comments via **`arks-hub`** (or shared native HTTP types) — **Forbidden:** spawn `gh` or `git`.
6. Repo/diff context via **arks engine crates** / filesystem — not subprocess Git.
7. No Claimed until evidence. No invented clean-room PASS for agent e2e.

## Layout

```text
crates/arks-agent/
  Cargo.toml          # lib only; publish = false
  src/lib.rs          # dispatch + stub help
  src/provider/       # trait + openai_compat + vendor stubs (phased)
  src/review/         # phased
  src/cev/            # phased
crates/arks/src/main.rs   # early dispatch: arks agent → arks_agent::…
.spec/sot/arks-agent-*.sot.md
```

### Deps boundary

| May depend on | Must not depend on |
|---------------|--------------------|
| `arks-hub` (PR comment API), optional thin HTTP | Spawning `git`/`gh`/`ollama` CLI |
| Inner engine crates for diff/OID when needed (`arks-repo`, …) | Core rings depending **back** on `arks-agent` |
| HTTP client (`ureq` / later `reqwest` as chosen) | Secrets in-tree |
| Serde/json for provider payloads | Publishing a second bin |

Ring placement: **Ring 7 internal lib** (sibling to `arks-hub`) — outer UX extension, not porcelain inventory.

## Phasing

### v1 (skeleton → usable local)

1. SoT + plan + governance cross-links (this slice).
2. Stub `arks agent --help` / refuse unimplemented groups.
3. Provider config surface: `provider list|show|set|test` with **openai-compat** + Ollama default `http://127.0.0.1:11434/v1`.
4. `review` local diff → stdout findings (optional `--post` later).
5. `cev-check` filesystem SoT heuristics + LLM assist (advisory); `--strict` exit policy when claimed.

### v2 (bots + hub)

6. `serve` / webhook or poll against local/remote hub.
7. PR review comments + check-runs style summaries via hub API.
8. Cloudflare / Claude / Grok / Gemini vendor paths beyond compat.
9. Optional clean-room **agent e2e** pillar under [[sot:root/verify-cleanroom-symmetry-matrix]] — **link only; do not invent PASS** (umbrella §Honesty law item 6).

## Acceptance

| Check | Done when |
|-------|-----------|
| SoTs | product-intent, cli-surface, llm-providers (incl. OpenAI-compat + Ollama URL), cev-review |
| Stub (optional) | `arks agent --help` works; `cargo build -p arks` |
| Boundary | no git/gh/vendor-CLI spawn; no second published bin |
| Honesty | no Claimed agent; no fake matrix PASS |
| Governance | `.spec/README.md` / agent-governance / crate-architecture mention `arks agent` |

## Non-goals

Full chatbot product; separate `arks-agent` release; `gh` thin backend; requiring agent for M9/M16 or hub DoD; storing API keys in git.

## Follow-on

1. Implement openai-compat `provider test` against local Ollama + evidence (host advisory until clean-room gate exists).
2. Amend [[sot:root/crate-architecture]] when lib lands (this plan expects the amend in the same design slice if stub lands).
3. Wire hub comment posting once `arks hub pr comment` client exists.
4. Coordinate with verify-matrix owners before any agent e2e gate ID is minted.
