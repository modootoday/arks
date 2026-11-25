---
id: evidence:root/20261125031427-claimed-rebind-q21
title: B7 rebind re-run after the Q21 SSH label change
status: active
created: 2026-11-25T03:14:27Z
updated: 2026-11-25T03:14:27Z
tags: [q21, b7, rebind, ssh, docker, clean-room]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - evidence:root/20261111092000-claimed-rebind-ssh
---

# Evidence: B7 re-run on the Q21 tip

- **Why:** queue Q21 removed the B6 Partial label from the durable SSH listener (startup / rebind / session lines and the `SSH-2.0-arks` identification string). This re-runs `verify/scripts/run-claimed-rebind-e2e.sh` so B7 does not rest only on the pre-change tip.
- **Venue:** clean-room image `arks-verify-m1:0.1.0`, same run and source copy (branch tip `da68efb`) as [[evidence:root/20261125031425-claimed-smoke]].
- **Observed:** HTTP, SSH and forge dual-bind drain scenarios green (held connection + fresh old + fresh new); see `run.log`. The script writes fixed receipt IDs, so it ran in a scratch copy and this directory records its run log instead of overwriting [[evidence:root/20261111092000-claimed-rebind-ssh]].
- **Artifacts:** `run.log`
