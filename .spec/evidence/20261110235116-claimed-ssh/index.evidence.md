---
id: evidence:root/20261110235116-claimed-ssh
title: B1 OpenSSH ChaCha wire + B2 SSH git clone
status: active
created: 2026-11-10T23:51:17Z
updated: 2026-11-10T23:51:17Z
tags: [b1, b2, ssh, openssh, git-clone, docker, clean-room]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-self-host-server
---

# Evidence: B1 OpenSSH ChaCha + B2 git clone ssh://

- **Gates:** B1 Claimed; B2 Claimed
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-ssh`
- **Image:** `arks-verify-m1:0.1.0`
- **Tip on receipt:** `f6c5780`
- **B1:** stock OpenSSH ChaCha KEX / NEWKEYS / SERVICE_ACCEPT (`logs/ssh-client.err`, `logs/server.err`)
- **B2:** stock `git clone ssh://` clone_rc=0; archived `clone-README`

Artifacts: `receipt.json`, `logs/`, `clone-README`
