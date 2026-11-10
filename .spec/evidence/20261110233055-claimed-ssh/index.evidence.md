# Evidence: B1 OpenSSH-exact cipher / binary-packet wire

- **Gate:** B1 ([[sot:root/arks-claimed-residual-gates]]); B2 still **Missing** on this receipt
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-ssh`
- **Image:** `arks-verify-m1:0.1.0`
- **Oracle:** stock OpenSSH_9.2p1 + `ssh -vvv` (product never spawns `ssh`)
- **Checks:** `curve25519-sha256` KEX; `chacha20-poly1305@openssh.com`; NEWKEYS; server `wire_SERVICE_ACCEPT=ssh-userauth` after AEAD
- **B2:** `git clone ssh://…` still Missing on this tip (auth/channel pack incomplete at receipt time)

Artifacts: `receipt.json`, `logs/ssh-client.err`, `logs/server.err`
