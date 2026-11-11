---
id: evidence:root/20261111092000-claimed-rebind-forge
title: B7 forge dual-bind drain rebind
status: active
created: 2026-11-11T00:20:00Z
updated: 2026-11-11T00:20:00Z
tags: [b7, forge, rebind, dual-bind, docker, iw-b7-forge]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-remaining-implement-wave
    - sot:root/arks-self-host-server
    - research:root/20261111090827
---

# Evidence: IW-B7-FORGE dual-bind drain

- **Gate:** B7 forge Claimed (separate from HTTP)
- **Venue:** `docker compose -f verify/docker-compose.yml run --rm dod-claimed-rebind`
- **Checks:** held forge HTTP session completes; fresh old (draining) + fresh new peers succeed; stderr `live rebind dual-bind`
- **Note:** ≠ B3 hub client HTTPS remote Claimed; ≠ HTTP B7 receipt

Artifacts: `receipt.json`
