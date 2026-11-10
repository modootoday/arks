---
id: evidence:root/20261110232045-claimed-rebind
title: B7 HTTP dual-bind drain
status: active
created: 2026-11-10T23:20:45Z
updated: 2026-11-10T23:20:45Z
tags: [b7, rebind, dual-bind, connection-drain, docker]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-self-host-server
---

# B7 HTTP dual-bind drain

The pinned Docker gate held an HTTP request open across SIGHUP rebind, then
completed that request and fresh requests through both the draining old
listener and the new listener.

Disposition is limited to the HTTP scenario. It makes no SSH crypto claim.

Artifact: `receipt.json`.
