---
id: sot:root/arks-agent-cli-surface
title: arks agent CLI surface
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-10T19:23:00Z
tags: [arks-agent, arks-agent-cmd, cli, review, cev, provider, bot, serve, openai-compat, single-release]
verify: manual
rel:
  depends_on:
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-llm-providers
    - sot:root/arks-agent-cev-review
    - sot:root/product-naming
    - sot:root/cli-surface
    - sot:root/arks-hub-cli-surface
    - sot:root/public-surface
    - plan:root/20261111042300
  evidence: []
---

# arks agent CLI surface

Version **0.1.0**. Normative **CLI surface** for the **`arks agent`** command group on the **single** published binary **`arks`**. Intent: [[sot:root/arks-agent-product-intent]]. Providers: [[sot:root/arks-agent-llm-providers]]. CEV: [[sot:root/arks-agent-cev-review]].

Until implemented: refuse with non-zero exit; **no Claimed**.

## Rule

### Identity (single CLI)

1. **Published binary is `arks` only.** There is no separate `arks-agent` release binary.
2. Agent UX is the command group **`arks agent …`**. Internal crate MAY be named `arks-agent`.
3. **Not part of git alias DoD.** `alias git=arks` MUST NOT require these commands.
4. **Global flags (intent):** `--help`, `--version`, `--provider <id>`, `--model <name>`, `--base-url <url>`, `--config <path>`, `--json` (machine output when claimed). Repo inference via **in-process** `.git` / hub config — **never** `git`/`gh` spawn.
5. **Auth material:** env + agent config only ([[sot:root/arks-agent-llm-providers]]) — **never** secrets in git.
6. **Exit codes:** 0 success; non-zero on failure/refuse/findings-above-threshold when a check mode is claimed.

### Command tree (v1 intent)

Invoked as `arks agent <group|command> …`.

| Group / command | Role | Notes |
|-----------------|------|-------|
| `review` | Automatic code review | Local diff and/or hub PR; posts comments via hub API when asked |
| `cev-check` | CEV inspection | Consistency / Evidence / Verify against SoT + matrix ([[sot:root/arks-agent-cev-review]]) |
| `provider` | List / show / test / set endpoint | Includes OpenAI-compat base URL + optional key |
| `config` | Show / edit agent config paths | No secrets printed by default |
| `run` | One-shot bot job | Named profile: review, cev, or combined |
| `serve` / `bot` | Long-running bot listener | Webhook / poll against hub when claimed |
| `doctor` | Sanity-check provider reachability + config | No Claimed until evidenced |

### Subcommand sketches

```text
arks agent --help
arks agent chat --message <text> [--message <text> …] [--base-url …] [--model …]
arks agent review [--pr <n>] [--diff <range>] [--files …] [--post] [--provider …] [--model …]
arks agent cev-check [--path .spec] [--pr <n>] [--strict] [--json]
arks agent provider list
arks agent provider show <id>
arks agent provider test <id> [--base-url …] [--model …]
arks agent provider set openai-compat --base-url <url> [--api-key-env <VAR>]
arks agent config show|path
arks agent run <profile> […]
arks agent serve [--bind …] [--hub …]     # later
arks agent bot run|status                 # alias-shaped later; may fold into serve/run
```

### `provider` group (normative intent)

7. **`arks agent provider …`** owns discoverability of Cloudflare / OpenAI / Claude / Grok / Gemini / **openai-compat**.
8. OpenAI-compat endpoints (Ollama, LocalAI, vLLM, LM Studio, custom) are configured with **`--base-url`** (default local Ollama documented in [[sot:root/arks-agent-llm-providers]]).
9. `provider test` MUST use native HTTP — never spawn vendor CLIs.
9a. `chat` is a bounded multi-turn native-HTTP surface. Repeated `--message`
values are sent in order with prior assistant responses retained; it does not
enable tool execution or shell invocation.

### Hub / review posting

10. When `--post` (or bot serve) comments on a PR/review, use **hub native API** ([[sot:root/arks-hub-cli-surface]] / `arks-hub` client) — **never** `gh`.
11. Local-only review (no `--post`) MAY print findings to stdout/stderr only.

### Name hygiene

| User types | Agent (`arks agent`) | Hub (`arks hub`) | Core Git (`arks`) |
|------------|----------------------|------------------|-------------------|
| LLM review of a PR | `arks agent review --pr N` | — | — |
| Post comment on PR | agent → hub API | `arks hub pr comment` (human) | — |
| `arks merge` | — | — | Git merge |

### Cross-OS

12. Windows / macOS / Linux same argv. Config under XDG or `%APPDATA%\arks\` agent section (documented). UTF-8 messages. Local loopback providers (Ollama) first-class.

## Why

One published CLI keeps release honest. A dedicated `arks agent` group isolates bot UX from Git porcelain and from hub’s `gh`-shaped surface while still composing hub APIs for PR comments.

## Verification

1. `arks agent --help` lists implemented groups only until Claimed; refuse others.
2. No product-path `gh`/`git` spawn.
3. No docs claiming a separate `arks-agent` release binary.
4. Provider config docs never commit secrets.
