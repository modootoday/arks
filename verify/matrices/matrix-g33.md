# G33 clean-room matrix — client credential-helper / askpass / TTY / OS wallets (FW-CRED-ASKPASS)

Unlock: [[sot:root/credential-helper-askpass]] v0.2.0. Primary: [gitcredentials](https://git-scm.com/docs/gitcredentials), [git-credential](https://git-scm.com/docs/git-credential). HTTPS Basic; secrets from process env / runtime helpers / wallet dir — never from the tree.

| ID | Scenario | Gate | Automated | Expect |
|----|----------|------|-----------|--------|
| G33-C01 | `arks credential fill` returns username/password from `!` helper | G33 | yes | attribute protocol stdout |
| G33-C02 | Clone auth-required loopback smart HTTP via `credential.helper` | G33 | yes | clone succeeds; tip matches |
| G33-C03 | Clone auth-required peer via `GIT_ASKPASS` program | G33 | yes | clone succeeds |
| G33-C04 | Missing helper/askpass against auth peer fails closed (`GIT_TERMINAL_PROMPT=0` / no TTY hang) | G33 | yes | non-zero; no hang |
| G33-C05 | No secrets in fixture paths / stdout (scrub check) | G33 | yes | password bytes absent from tree logs |
| G33-C06 | TTY terminal prompt via PTY (`GIT_TERMINAL_PROMPT=1`) for `arks credential fill` | G33 | yes (unix/clean-room) | username/password attributes on capture; 15s hard kill |
| G33-C07 | First-party `wincred` store + `libsecret` fill + `osxkeychain` clone via `ARKS_CREDENTIAL_WALLET_DIR` | G33 | yes | store/fill/clone succeed |

Host-only green does **not** carry G33 Claimed.
