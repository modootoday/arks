---
id: adr:root/20261109224236
title: Optional HTTP Basic auth product path
status: active
created: 2026-11-09T22:42:36Z
updated: 2026-11-09T22:42:36Z
tags: [fw-auth-proxy, http-basic, smart-http, arks-server, g36, auth, refuse-oauth]
rel:
  depends_on:
    - sot:root/arks-server
    - sot:root/production-residual-register
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/cli-surface
    - research:root/20261108071916
    - evidence:root/20261108231300
  evidence: [evidence:root/20261109224717]
---

# Optional HTTP Basic auth product path

## Context

**FW-AUTH-PROXY** was residual-closed as **Refuse-closed (product auth)** + **Closed (ops pattern)** — TLS + HTTP Basic / reverse-proxy only in front of unauthenticated `arks serve-http`, with forge OAuth / GitHub App as server refused ([[sot:root/arks-server]] Refuse matrix; [[sot:root/production-residual-register]] §F).

[gitprotocol-http](https://git-scm.com/docs/gitprotocol-http) treats authentication as a **transport** concern (clients SHOULD support Basic; cookies not required for correctness). RFC 7617 defines HTTP Basic (Base64 of `userid:password`, `WWW-Authenticate: Basic realm="…"` challenge). Catalog: [[research:root/20261108071916]].

Operator intent now claims **optional in-process HTTP Basic** on the smart-HTTP product path so stock `git` can clone/fetch/push with URL userinfo or a credential helper against `arks-server`, without inventing a forge auth product.

## Decision

1. **Claim optional HTTP Basic** on `arks serve-http` / `arks http-backend` when configured via `--basic-auth user:pass`, `ARKS_HTTP_BASIC=user:pass`, and/or a plaintext `user:pass` credential file (`--basic-auth-file` / `ARKS_HTTP_BASIC_FILE`). Unset remains the default: **unauthenticated loopback** (G15 clean-room unchanged).
2. When configured, smart HTTP paths (`info/refs` with `service=`, `POST …/git-upload-pack`, `POST …/git-receive-pack`) and other HTTP Git paths served by the same listener MUST require a valid `Authorization: Basic …` credential. Missing/invalid credentials → **401** with `WWW-Authenticate: Basic realm="arks"`.
3. **Still refuse:** forge OAuth, GitHub App as server, session cookies as the auth product, product credential/OAuth server UX. Ops reverse-proxy / external TLS termination remain valid *additional* patterns; they are no longer the sole claimed auth story.
4. **Not claimed:** Apache `htpasswd` crypt/bcrypt hash verification (plaintext `user:pass` lines only — “htpasswd-like” file shape, not Apache algorithm parity); TLS termination inside arks; multi-realm / Digest / Bearer.
5. **Verification:** additive clean-room gate **G36** `server-http-basic` (do not reuse G33–G35). Host unit tests are advisory. Product Claimed requires Docker sole-runner PASS + evidence. G15 unauthenticated smart HTTP receipt ([[evidence:root/20261108231300]]) remains frozen.

## Consequences

- [[sot:root/arks-server]], [[sot:root/cli-surface]] Wave Sv, [[sot:root/production-residual-register]] §C/§F (**FW-AUTH-PROXY** → Claimed product Basic + OAuth refuse), and [[sot:root/interop-test-matrix]] MUST amend.
- New clean-room gate: **G36** `server-http-basic` — stock `git` clone (URL userinfo and/or credential) against Basic-protected arks-server; 401 challenge honesty.
- Public copy MAY mention optional Basic only after G36 Claimed evidence exists.
- Does **not** reopen forge OAuth, redefine M11/G15 unauthenticated loopback, or invent wire capabilities.

## Supersedes / amends

- Supersedes **Refuse-closed (product auth)** + ops-only Basic disposition for **FW-AUTH-PROXY** product Basic path in [[sot:root/arks-server]] and [[sot:root/production-residual-register]].
- Retains **Refuse-closed** for forge OAuth / GitHub App as server.
- Does **not** supersede [[evidence:root/20261108231300]] (G15 unauthenticated smart HTTP remains Claimed).
