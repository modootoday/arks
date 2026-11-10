# Evidence: B3 Claimed hub HTTPS forge remote e2e

- **Gate:** B3 ([[sot:root/arks-claimed-residual-gates]])
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-hub-https`
- **Image:** `arks-verify-m1:0.1.0`
- **Path:** forge module HTTPS terminate (rustls PEM) + `arks hub` client with `ARKS_HUB_CA_FILE`; stock curl `/health` oracle
- **Checks:** HTTPS banner; curl health 200; hub smoke GET `/user`; authenticated `arks hub repo list`
- **Refuse:** hub `--local-tls` smoke alone ≠ this Claimed bar

Artifacts: `receipt.json`, `hub-smoke.out`, `hub-repo-list.out`, `curl-health.body`, `server.err`
