# G15 bug simulation

1. **CRLF in `run-g015-smart-http.sh`** — LF-only; same `set: pipefail` class as G14.
2. **Dumb-HTTP-only** — forbidden as sole production-server path; suite exercises smart HTTP (`info/refs?service=` + POST).
3. **Advertising unimplemented caps** — refuse matrix checks absence of protocol-v2 / delete-refs when not claimed.
4. **Lease required for server** — forbidden; G15 must not depend on `arks-lease`.
