---
id: sot:root/arks-agent-product-intent
title: arks agent product intent
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-10T23:19:59Z
tags: [arks-agent, arks-agent-cmd, llm, code-review, cev, bot, internal-crate, single-release, openai-compat, providers]
verify: manual
rel:
  depends_on:
    - sot:root/product-intent
    - sot:root/product-naming
    - sot:root/cli-surface
    - sot:root/crate-architecture
    - sot:root/public-surface
    - sot:root/agent-governance
    - sot:root/arks-hub-product-intent
    - sot:root/arks-daemon-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/arks-agent-llm-providers
    - sot:root/arks-agent-cev-review
    - sot:root/product-verification-clean-room
    - plan:root/20261111042300
  evidence:
    - evidence:root/20261110231959-claimed-agent-llm
---

# arks agent product intent

Version **0.1.0**. Normative **product intent** for LLM-powered **automatic code review** and **CEV inspection** bots as the **`arks agent`** command group inside the **single** published CLI **`arks`**.

Agent bots are an **explicit extension** (same shipping pattern as hub/daemon) — **not** mixed into Git porcelain. Core Git stays ordinary `arks` commands ([[sot:root/product-intent]], [[sot:root/cli-surface]]). Forge collaboration stays **`arks hub …`** ([[sot:root/arks-hub-product-intent]]).

CLI shape: [[sot:root/arks-agent-cli-surface]]. Providers: [[sot:root/arks-agent-llm-providers]]. CEV meaning: [[sot:root/arks-agent-cev-review]]. Crate plan: [[plan:root/20261111042300]].

## Rule

### Shipping (normative)

1. **Single release = `arks` only.** Do **not** publish a separate `arks-agent` binary or crates.io product release.
2. Users invoke **`arks agent <subcommand>…`**.
3. Implementation MAY live in an **internal** workspace crate **`arks-agent`** (library). The `arks` binary depends on it and dispatches the `agent` command group. **No** standalone `arks-agent` bin in the release story.
4. Git alias DoD (`git`→`arks`) MUST NOT require agent features to be Claimed. Agent is additive bot UX on the same binary.
5. Hub/daemon Claimed bars MUST NOT require agent Claimed.

### Mission

6. Run **automatic code review** over local diffs / hub PRs using configured LLM providers.
7. Run **CEV inspection** bots that check Consistency / Evidence / Verify honesty against Specfold SoT + matrices ([[sot:root/arks-agent-cev-review]]) — fail-closed, cite IDs, never invent **PASS** / **Claimed**.
8. Integrate with **local hub** PRs/reviews via **`arks-hub` native HTTP** (and later optional local hub serve) — **never** subprocess `gh` or `git`.
9. Prefer reading repository / Specfold context via **arks engine libs** and filesystem reads of tracked `.spec/` — not shelling out.
10. A bounded multi-turn OpenAI-compatible conversation path MAY retain user/assistant history for review workflows. B4 is Claimed for the Docker fixture journey plus review request ([[evidence:root/20261110231959-claimed-agent-llm]]); this does not claim model quality.

### Conceptual map

```text
Stock peer concept     →  arks surface              →  internal crate
─────────────────────     ────────────────────────     ────────────────
git                    →  ordinary arks cmds        →  engine crates
git-daemon             →  arks daemon               →  arks-daemon
gh                     →  arks hub …                →  arks-hub
agent bots (new)       →  arks agent …              →  arks-agent
```

```text
Published binary:  arks
                     ├── git porcelain/plumbing/remotes/lease…   (peer: git)
                     ├── arks daemon  → internal arks-daemon     (peer: git-daemon)
                     ├── arks hub …   → internal arks-hub        (peer: gh)
                     └── arks agent … → internal arks-agent      (review / CEV bots)
                                        ├── LLM providers (HTTP; OpenAI-compat + vendor)
                                        └── hub API client (via arks-hub / shared HTTP) for PR comments
```

### Hard non-invocation law (ABSOLUTE)

| Surface | MUST NOT invoke | Allowed |
|---------|-----------------|---------|
| `arks` (incl. `arks agent`, `arks hub`, `arks daemon`, porcelain) | Stock `git` / `gh` on product paths | In-engine Git; native HTTPS to hub + LLM providers; clean-room/`verify` MAY use `git` **outside** the product binary |

Implementing review/CEV by shelling to `gh`/`git` **fails** this SoT.

### Goals

| Goal | Normative meaning |
|------|-------------------|
| Single CLI release | Only `arks` is the product binary/release |
| Internal crate | `arks-agent` library behind `arks agent`; not a second published bin |
| Code review bots | Diff/PR review comments via LLM + hub API |
| CEV bots | Specfold Consistency/Evidence/Verify inspection ([[sot:root/arks-agent-cev-review]]) |
| Provider breadth | Cloudflare, OpenAI, Claude, Grok, Gemini, **plus generic OpenAI-Compat** (Ollama, LocalAI, vLLM, LM Studio, any base URL) ([[sot:root/arks-agent-llm-providers]]) |
| Hub integration | Local/remote hub PRs/reviews without `gh` |
| Honest claims | Design ≠ Claimed; design ≠ clean-room PASS |

### Non-goals

1. Separate `arks-agent` release binary or dual-product marketing.
2. General autonomous assistant frameworks, tool-calling OS control, or unrestricted repository rewriting. The bounded native-HTTP `agent chat --message …` conversation used by B4 is in scope.
3. Mixing agent commands into Git porcelain inventory / alias DoD.
4. Subprocess backends (`git`/`gh`/vendor CLIs such as `openai`, `ollama` CLI wrappers as product path).
5. Storing API keys / tokens in tracked git / `.spec/`.
6. Inventing Claimed review quality or inventing matrix **PASS** from LLM output alone.
7. Replacing [[sot:root/product-verification-clean-room]] Docker authority with LLM judgment.
8. Requiring agent for M9/M16 Git DoD or hub/daemon Claimed exits.

### Relationship to Specfold / clean-room

9. Agent **consumes** SoT/plan/matrix text as inspection input; it does **not** authoritatively flip Claimed/Verified.
10. Future agent e2e clean-room pillars (if any) MUST be added by the verify-matrix owners — this SoT only **cross-links intent**. Do **not** invent PASS rows here. Sibling note: `.spec/README.md` (3-way symmetry stream).

### Success criteria (design phase)

1. Product + CLI + providers + CEV SoTs active and cross-linked; wording uses **`arks agent`**, not a separate bin.
2. Crate graph plan: internal `arks-agent` lib; `arks` depends on it for the `agent` group; **no** published second bin ([[plan:root/20261111042300]]).
3. Public copy: agent is a command group on `arks`; no Claimed without evidence.
4. Provider SoT documents OpenAI-compat + local Ollama default endpoint.

## Why

Review and CEV bots need a clear home that does not pollute Git porcelain or force a second release binary. Mirroring hub/daemon keeps distribution honest; forbidding `git`/`gh` spawn keeps the native-engine story intact; excluding general chatbot scope keeps the crate focused.

## Verification

1. `spec check` for this SoT and dependents.
2. Product-path spawn of `gh`/`git` fails review.
3. No Claimed agent features from design-only.
4. Docs/Pages do not advertise a separate `arks-agent` release.
