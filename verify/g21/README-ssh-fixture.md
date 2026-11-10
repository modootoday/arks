# G21 Sy-4 — OpenSSH loopback fixture

Clean-room image `arks-verify-m1` includes `openssh-client` and `openssh-server`.
Sy-4 tests start a **local sshd** on `127.0.0.1` with an ephemeral host key and
authorized client key (no passwords; no secrets committed to the repo).

## Harness shape

1. Generate ed25519 host + client keys under the test temp dir.
2. Write `sshd_config` (`Port`, `ListenAddress 127.0.0.1`, `StrictModes no`,
   `PasswordAuthentication no`, `PubkeyAuthentication yes`).
3. Seed a **stock Git** bare repo; expose it as `ssh://test@127.0.0.1:<port>/<abs-path>`.
4. Set `GIT_SSH_COMMAND` for arks (and stock Git peer checks) to:
   `ssh -i <client_key> -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null`

arks client invokes OpenSSH only (no product-path `git` subprocess). The peer
commands are stock `git-upload-pack` / `git-receive-pack` over that SSH session.

## Claims

| ID | Focus |
|----|-------|
| M16-SY4-01 | `clone` / `fetch` / `push` over `ssh://` fixture |
| M16-SY4-02 | `submodule add` over SSH URL fixture |

Server SSH **command-mode** remains N/A (not claimed).
