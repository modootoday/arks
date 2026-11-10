# Cross-OS / Docker-only / pin honesty

Product **Verified** claims require the Docker clean-room (rule `60-clean-room-verify`). Host OS greens and “latest” oracle inventories are **advisory** unless the owning SoT says otherwise.

## Must

- Treat **Linux Docker clean-room** as completion authority for interop / journey / symmetry DoD.
- Keep **pin** Git (clean-room image lock, currently **2.39.5** class) separate from host/runner **latest** inventory — do not collapse latest-green into pin-DoD Claimed ([[sot:root/release-symmetry-precheck]], [[sot:root/verify-cleanroom-symmetry-matrix]]).
- On Windows hosts: drive `docker compose`; authority remains **Linux-in-container**, not host `sh` / host cargo alone.
- Before heavy sole-runner gates, check sibling Docker agents (rule `60`).
- Cite humanymous Docker affinity — do not fork ledgers: `C:\workspaces\humanymous\.agents\rules\60-e2e-docker-only.md`, `96-docker-engine-affinity.md`.

## Must not

- Label host Windows/macOS unit greens as product Verified.
- Advance the normative Git pin by vibes — follow [[sot:root/clean-room-git-pin-advance]] (+ evidence).
- Invent PASS from release-symmetry CI advisory rows (Partial / hub v1 Missing).
- Start competing sole-runner compose against a sibling live-GH agent.

## Cite

- skill `verify-clean-room`
- skill `run-symmetry-matrix`
- skill `release-symmetry-gate`
