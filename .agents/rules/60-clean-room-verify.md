# Clean-room verification is completion authority

Product acceptance / interop / regression claims that say **verified** MUST be reproducible in the Docker clean-room defined by `.spec/sot/product-verification-clean-room.sot.md`.

## Must

- Treat host/unit/IDE green as **advisory** unless the same gate is mirrored and green in clean-room.
- Prefer pinned images/compose (when they exist); do not invent digests — track open decisions in the verification SoT.
- On Windows, cite humanymous Docker engine affinity rather than forking affinity prose: `C:\workspaces\humanymous\.agents\rules\96-docker-engine-affinity.md` and `60-e2e-docker-only.md`.
- Gate IDs and unlock order: `.spec/sot/interop-test-matrix.sot.md` (**G0–G27**; +G28 when present).
- Heavy sole-runner gates (journey / live forge / large contention): do **not** start a competing Docker compose run that clashes with an already-running sibling live-GH agent.

## Forbidden as “done”

- Host-only stock-Git round-trips as the sole interop gate.
- Calling a stub, mock, or missing clean-room harness a product pass.
- Inventing live-forge journey PASS without opt-in secrets + clean-room receipt.
- Claiming CELIR ObjectStore verified (ObjectStore Refuse; index-only dual-path only).

Skill: `verify-clean-room`.
