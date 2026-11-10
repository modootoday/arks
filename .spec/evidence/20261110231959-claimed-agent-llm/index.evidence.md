# Evidence: B4 Claimed LLM chatbot / review e2e

- **Gate:** B4 ([[sot:root/arks-claimed-residual-gates]])
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-agent-llm`
- **Image:** `arks-verify-m1:0.1.0`
- **Tip at run:** workspace tip was `0f25c27`+ (uncommitted TLS CLI wiring present); receipt is product-path observe, not invent PASS from host.
- **Checks observed:**
  1. `arks agent chat` two-turn history against openai-compat fixture (`assistant_1` / `assistant_2-history-ok`)
  2. `arks agent review` native HTTP with fixture token; output contains honesty string (no invented Claimed/PASS)
- **Product never spawned:** `ollama` / `git` / `gh`
- **Artifacts:** `receipt.json`

Disposition: **verified-product-path** suitable to graduate B4 to Claimed when owning agent SoTs + residual register are amended together.
