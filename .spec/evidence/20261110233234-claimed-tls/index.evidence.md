# Evidence: B9 Product in-process TLS terminate

- **Gate:** B9 ([[sot:root/arks-claimed-residual-gates]])
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-tls`
- **Image:** `arks-verify-m1:0.1.0`
- **Path:** `arks serve --tls-cert/--tls-key` (rustls) + stock `curl --cacert` info/refs 200
- **Honesty:** ops reverse-proxy remains a valid deploy pattern and does **not** by itself satisfy B9

Artifacts: `receipt.json`, `curl-info-refs.body`, `server.err`
