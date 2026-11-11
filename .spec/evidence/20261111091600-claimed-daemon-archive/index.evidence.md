---
id: evidence:root/20261111091600-claimed-daemon-archive
title: B5 daemon upload-archive ustar byte-identical
status: active
created: 2026-11-11T00:16:00Z
updated: 2026-11-11T00:16:00Z
tags: [b5, daemon, upload-archive, ustar, docker, clean-room, iw-b5]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-daemon-product-intent
    - research:root/20261111084400
    - research:root/20261111090827
---

# Evidence: B5 byte-identical ustar

- **Gate:** B5 Claimed (IW-B5)
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-daemon-archive`
- **Image:** `arks-verify-m1:0.1.0`
- **Oracle:** stock `git version 2.39.5` `git-daemon` + `git archive --remote`; product `arks daemon --enable=upload-archive` (no product `git archive` spawn)
- **Claimed check:** `cmp -s stock.tar arks.tar`
- **Encoder:** git v2.39.5 `archive-tar.c` `prepare_header` (`%07o` / `%011o`, device fields written, chksum sum-with-spaces)
- **B5+:** Refuse-terminal (openlog/LOG_DAEMON Partial only)

Artifacts: `receipt.json`, `first-header.hex`
