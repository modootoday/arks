---
id: evidence:root/20261125031426-claimed-ssh
title: B1 OpenSSH ChaCha wire + B2 SSH git clone
status: active
created: 2026-11-25T03:14:26Z
updated: 2026-11-25T03:14:26Z
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
- **Tip on receipt:** `unknown` (queue Q21 re-run from a copy of branch tip `da68efb` without `.git`, after the B6 label change; the server now identifies as `SSH-2.0-arks`)
- **B1:** stock OpenSSH ChaCha KEX / NEWKEYS / SERVICE_ACCEPT (`logs/ssh-client.err`, `logs/server.err`)
- **B2:** stock `git clone ssh://` clone_rc=0; archived `clone-README`

Artifacts: `receipt.json`, `logs/`, `clone-README`

Scrub: in `logs/ssh-client.err` and `receipt.json`, SSH algorithm identifiers of the form
`name@openssh.com` / `name@libssh.org` are written `name[at]domain` (receipt field
`cipher_extension_domain`), because the public-surface scan reads them as e-mail addresses. No
other byte of the logs changed.
