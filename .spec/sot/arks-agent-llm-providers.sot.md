---
id: sot:root/arks-agent-llm-providers
title: arks agent LLM providers
status: active
created: 2026-11-10T19:23:00Z
updated: 2026-11-25T18:00:00Z
tags: [arks-agent, llm, providers, openai-compat, ollama, cloudflare, anthropic, xai, gemini, auth, config]
verify: manual
rel:
  depends_on:
    - sot:root/arks-agent-product-intent
    - sot:root/arks-agent-cli-surface
    - sot:root/public-surface
    - sot:root/agent-governance
    - sot:root/cloud-agent-dropin
    - plan:root/20261111042300
  evidence:
    - evidence:root/20261110231959-claimed-agent-llm
---

# arks agent LLM providers

Version **0.2.0**. Normative **provider abstraction and auth/config** for **`arks agent`**. Product intent: [[sot:root/arks-agent-product-intent]]. CLI: [[sot:root/arks-agent-cli-surface]].

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
3a. The OpenAI-compatible path carries complete prior user/assistant history for multi-turn requests. B4 Docker evidence covers two turns and a separate review request; it does not claim provider or model quality ([[evidence:root/20261110231959-claimed-agent-llm]]).
3b. `arks agent provider smoke` and `arks agent review --smoke` are **diagnostics** over plain-HTTP openai-compat endpoints (local Ollama, a fixture, or the in-process `--mock`): exit 0 only on a 2xx chat answer, exit 1 when the provider cannot be reached. Claimed as diagnostics by [[evidence:root/20261125031425-claimed-smoke]] (queue Q21); they never stand in for B4. Over `https://` they use the transport in 3c; a live vendor run still needs an owner-held API key (owner queue, Q23).

3c. **Transport.** `http://` bases use the agent's plain HTTP/1.1 client. `https://` bases use the one HTTPS stack the git remote client and `arks hub` use: `ureq` + `rustls`, configured by `arks_net::HttpsPolicy` ([[sot:root/cloud-agent-dropin]] §T). No second TLS stack is added. That means:
    - Trust anchors: the built-in web roots (`webpki-roots`, the Mozilla set) **plus** the PEM bundle named by `GIT_SSL_CAINFO` (env, wins) or `http.sslCAInfo` (config discovered from the current directory), and `SSL_CERT_FILE`. The OS-native certificate store is not read; a session CA is supplied through those variables. An anchor file that holds no certificate is an error, not a silent fallback.
    - Proxy: `http.proxy`, else `https_proxy` / `HTTPS_PROXY` / `all_proxy` / `ALL_PROXY`, minus `no_proxy` / `NO_PROXY` host-suffix exclusions; HTTPS goes through HTTP `CONNECT`. With a proxy in effect, the reachability probe (`tcp=`) connects to the proxy, not the origin.
    - Redirects are not followed (a 3xx is reported as the status). The API key header is sent only to the configured base URL.
    - Covered paths: `/models` probe, single-shot chat (`provider smoke`, `provider test`, `review`, `chat`) and the `serve` / `bot` proxy (buffered and `stream=true`).
    - A certificate the anchors do not trust fails the request with `error_class=err_class=tls` and a `reason=` line naming the anchor variables; `provider smoke` and `review --smoke` then exit 1. TLS is never downgraded or skipped.
    - Proof: host crate tests against a loopback TLS server whose leaf is signed by a test CA generated at run time (`rcgen`, no PEM fixture tracked): the smoke succeeds with the CA as anchor, succeeds through a loopback `CONNECT` proxy, and fails with the TLS class and exit 1 without it. Host runs are advisory; no Docker receipt covers HTTPS yet.

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
