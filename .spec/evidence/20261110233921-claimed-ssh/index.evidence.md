# Evidence: B1 OpenSSH ChaCha wire (B2 not Claimed)

- **Gates:** B1 Claimed; **B2 Missing** ([[sot:root/arks-claimed-residual-gates]] v0.1.2 audit)
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-ssh`
- **Image:** `arks-verify-m1:0.1.0`
- **Tip on receipt:** `4a99f8e`
- **B1:** stock OpenSSH ChaCha KEX / NEWKEYS / SERVICE_ACCEPT (see `logs/ssh-client.err`, `logs/server.err`)
- **B2 honesty:** `logs/git-clone.err` records `Connection to 127.0.0.1 closed by remote host.`; `git-clone.out` empty; no archived clone README. Prior receipt `b2: Claimed` was **invent** and corrected to Missing.

Artifacts: `receipt.json`, `logs/`
