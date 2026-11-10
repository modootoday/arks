# M11 clean-room matrix (G15 git-server / production-server)

Roadmap **v0.3.1+** M11 (Git server / Phase Sv). Authoritative venue: Docker clean-room under `verify/` per [[sot:root/product-verification-clean-room]]. Service `g15` (`verify/scripts/run-g15.sh`). Unit SoT: [[sot:root/arks-server]]. Receipt: [[evidence:root/20261108231300]]; githooks honor list [[evidence:root/20261110200000]]. **Claimed transport: smart HTTP**; **M11-C14** Claimed honor list `pre-receive` / `update` / `post-receive`.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| M11-C01 | Stock `git clone` via arks smart HTTP | G15 | yes | objects/refs match |
| M11-C02 | Stock `git fetch` after new commit on server | G15 | yes | tip advances |
| M11-C03 | Stock `git push` to arks receive-pack | G15 | yes | report-status ok; tip matches |
| M11-C04 | Capability honesty | G15 | yes | advertise delete-refs + report-status-v2 when implemented; no push-options / invented caps |
| M11-C05 | Wave Sv CLI help lists upload-pack/receive-pack/serve-http/http-backend | G15 | yes | help smoke |
| M11-C06 | zero gix / zero celir | G15 | yes | metadata scan |
| M11-C07 | G14 + G7 + G5 regression | G15 | yes | embedded prior suites |
| M11-C08 | Lease not required | G15 | yes | no arks-lease dep on arks-server |
| M11-C09 | SSH command-mode transport | — | no | Moved to **M25 / G30** ([[adr:root/20261109220230]]) |
| M11-C10 | `git://` daemon | — | no | Moved to **M25 / G32** |
| M11-C11 | Dumb HTTP as product server path | — | no | Moved to **M25 / G29** (additive; smart HTTP still required) |
| M11-C12 | Protocol v2 product (beyond cap-honesty check) | — | no | Moved to **M25 / G31** |
| M11-C13 | Product auth server beyond loopback | — | no | Not claimed; reverse-proxy / Basic = ops pattern only |
| M11-C14 | Server-side githooks on receive-pack (`pre-receive` / `update` / `post-receive`) | G15 | yes | Honor list Claimed; non-zero pre-receive rejects push; update rejects one ref; post-receive runs after success ([[evidence:root/20261110200000]]) |

Host-only green does **not** carry M11 production-server exit.
