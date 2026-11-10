---
id: sot:root/arks-agent-llm-providers
title: arks agent LLM providers
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-10T19:23:00Z
tags: [arks-agent, llm, providers, openai-compat, ollama, cloudflare, anthropic, xai, gemini, auth, config]
verify: manual
rel:
  depends_on:
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/public-surface
    - sot:root/agent-governance
    - plan:root/20261111042300
  evidence: []
---

# arks agent LLM providers

Version **0.1.0**. Normative **provider abstraction and auth/config** for **`arks agent`**. Product intent: [[sot:root/arks-agent-product-intent]]. CLI: [[sot:root/arks-agent-cli-surface]].

Design ≠ Claimed. No secrets in tracked trees ([[sot:root/agent-governance]], [[sot:root/public-surface]]).

## Rule

### Provider set (normative scope)

| Provider id | Wire style | Notes |
|-------------|------------|-------|
| `openai-compat` | OpenAI Chat Completions / Responses-compatible HTTP | **Preferred shared path** for OpenAI cloud **and** local/compat forks |
| `openai` | Alias / preset of `openai-compat` | Default base `https://api.openai.com/v1` |
| `ollama` | Preset of `openai-compat` | Default base `http://127.0.0.1:11434/v1` |
| `cloudflare` | Cloudflare Workers AI HTTP | Vendor path as needed |
| `claude` / `anthropic` | Anthropic Messages API | Vendor SDK or native HTTP |
| `grok` / `xai` | xAI Grok HTTP | Vendor path as needed |
| `gemini` / `google` | Google Gemini API | Vendor SDK or native HTTP |

**OpenAI-compatible environments in scope** (non-exhaustive): **Ollama**, **LocalAI**, **vLLM**, **LM Studio**, OpenAI cloud, and **any** service exposing an OpenAI-compatible base URL. Users MAY set an arbitrary `--base-url`.

### Client architecture (normative preference)

1. **One OpenAI-compat client path** covers `openai`, `ollama`, LocalAI, vLLM, LM Studio, and custom compat bases (shared request/response types, retries, streaming later).
2. **Vendor-specific clients** only where the wire is not OpenAI-compat enough: Anthropic, Gemini, Cloudflare Workers AI, xAI if diverged — keep behind the same `Provider` trait / enum in `arks-agent`.
3. Product paths use **native HTTP** (e.g. `ureq` / `reqwest` as chosen in crate plan) — **MUST NOT** spawn `ollama`, `openai`, `gh`, or `git` CLIs.

### Auth and config (no secrets in git)

4. **Secrets live in environment variables or local config files outside tracked git** (user home / `%APPDATA%` / XDG). **Never** commit API keys into `.spec/`, `.agents/`, or source.
5. Documented env vars (intent; names MAY gain `ARKS_AGENT_*` aliases):

| Purpose | Env (intent) | Notes |
|---------|--------------|-------|
| OpenAI / compat API key | `ARKS_AGENT_API_KEY` or `OPENAI_API_KEY` | Optional for many local Ollama setups |
| Anthropic | `ANTHROPIC_API_KEY` / `ARKS_AGENT_ANTHROPIC_API_KEY` | |
| Gemini | `GOOGLE_API_KEY` / `ARKS_AGENT_GEMINI_API_KEY` | |
| xAI Grok | `XAI_API_KEY` / `ARKS_AGENT_XAI_API_KEY` | |
| Cloudflare | `CLOUDFLARE_API_TOKEN` + account/gateway ids as documented | |
| Default provider | `ARKS_AGENT_PROVIDER` | e.g. `ollama`, `openai-compat` |
| Default model | `ARKS_AGENT_MODEL` | |
| Compat base URL | `ARKS_AGENT_BASE_URL` | Overrides preset |

6. **Local Ollama default base URL:** `http://127.0.0.1:11434/v1` when provider preset is `ollama` or when `openai-compat` is selected without an explicit base and local profile says ollama.
7. API key for local Ollama is **optional** (empty/omitted allowed when the server does not require auth).
8. `arks agent config show` / help MUST **redact** secret values.

### CLI configuration surface

```text
arks agent provider list
arks agent provider show ollama
arks agent provider set openai-compat --base-url http://127.0.0.1:11434/v1
arks agent provider set openai-compat --base-url https://api.openai.com/v1 --api-key-env OPENAI_API_KEY
arks agent provider test ollama --model llama3.2
arks agent review --provider openai-compat --base-url http://127.0.0.1:1234/v1 --model local-model
```

9. `--base-url` and `--api-key-env` (name of env var, not the secret itself) are first-class for compat endpoints.

### Abstraction sketch (implementation guidance)

```text
trait LlmProvider {
  fn id(&self) -> &str;
  fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, ProviderError>;
}

OpenAiCompatClient { base_url, api_key: Option<Secret>, … }  // openai, ollama, localai, vllm, lmstudio, custom
AnthropicClient { … }
GeminiClient { … }
CloudflareWorkersAiClient { … }
XaiGrokClient { … }
```

10. Higher layers (`review`, `cev-check`, `serve`) depend only on the trait — not on vendor SDKs.

### Non-goals

11. Embedding vendor API keys in the repo or CI logs.
12. Requiring cloud providers for local-only workflows (Ollama-first MUST remain viable).
13. Claiming model quality / safety certifications from this SoT alone.

## Why

A single OpenAI-compat HTTP client maximizes local and self-hosted coverage (Ollama et al.) while still naming first-class cloud vendors. Env-based auth keeps the public Specfold tree clean.

## Verification

1. Provider docs list OpenAI-compat + Ollama default URL.
2. No example commits secrets.
3. Implementation review rejects `Command::new("ollama"|"openai"|"gh"|"git")` on product paths.
4. Design ≠ Claimed provider readiness.
