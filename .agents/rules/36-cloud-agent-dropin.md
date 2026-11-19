# Cloud agent drop-in

Applies when changing transport, config, signing, or option parsing that hosted coding-agent sessions rely on.

## Must

1. Keep `.spec/sot/cloud-agent-dropin.sot.md` as the normative contract; amend it before changing meaning (rule `27`).
2. Honor proxy + CA (`http.sslCAInfo`, `GIT_SSL_CAINFO`, `http.proxy`, `HTTPS_PROXY`, `NO_PROXY`) on every HTTPS client path — no per-crate TLS stacks that skip them.
3. Read identity and signing from merged config (system → global → repo → `GIT_CONFIG_COUNT` → `-c`); fail with an error when identity is missing.
4. Split bundled short options parser-wide; unsupported spellings fail closed with a named error.
5. Claimed only with a Docker receipt whose fixture simulates the agent sandbox (TLS-intercepting proxy + private CA + SSH signing program).
6. Push only to the session's `claude/*` branch (ADR `20261119180100` D1).
7. Change how hosted sessions install arks only in `modootoday/x.ws.fleet` `infra/claude-env/default/setup.d/20-arks.sh` (regenerate `setup.sh` with `infra/claude-env/compose.sh`, open a PR there); never call the cloud environment API directly — the fleet design session applies merged canon.

## Must not

- Write commits with a placeholder identity.
- Shadow `/usr/bin/git` or alias `git` globally in shared environments (D5).
- Flip `publish = true`, bump crate version, or push tags (D3).
- Treat a live cloud-session probe as clean-room PASS.

## Cite

- `.spec/plan/20261119180000-cloud-dropin-goal-directive.plan.md`
- `.spec/adr/20261119180100-cloud-dropin-decisions-d1-d6.adr.md`
