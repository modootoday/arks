# Ports — normative vs demo / alternate

Product docs and `--help` MUST distinguish **IANA / Git-documented normative ports** from **unprivileged demo / alternate** binds. Owning product SoT: `.spec/sot/arks-self-host-server.sot.md`. Survey: `.spec/research/20261111055609-git-transport-standard-ports.research.md`.

## Normative targets (prefer in operator story)

| Surface | Port |
|---------|------|
| Git SSH | **22** |
| Git HTTP | **80** |
| Git HTTPS | **443** |
| Native `git://` (`arks daemon`) | **9418** |
| Forge API (arks-local) | **3920** (product convention — not IANA Git) |

## Demo / alternate (MUST label)

| Bind | Role |
|------|------|
| **2222** | Unprivileged / host-sshd conflict SSH alternate — **not** “more standard” than 22 |
| **8080** | Unprivileged HTTP demo — **not** more standard than 80 |

## Must

- Teach normative ports as the product story when privileged bind / ops front is available.
- Label 8080 / 2222 as **alternate / demo** in help, `docs/self-host.md`, and Spec prose.
- Keep clap demo defaults honest: safe loopback demos MAY remain until a coordinated flip — never market them as the Git standard.

## Must not

- Call 2222 or 8080 the “standard Git port.”
- Silently flip privileged defaults without tests + help + docs together.
- Invent Claimed listen behavior from port tables alone (rule `28`).

## Cite

- skill `self-host-serve`
- rule `33-serve-vs-hub`
