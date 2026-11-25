---
id: plan:root/20261124000000
title: Autonomous operation queue — units after the argv-symmetry ladder
status: active
created: 2026-11-24T00:00:00Z
updated: 2026-11-24T00:00:00Z
tags: [autonomy, queue, residual, plan]
verify: manual
rel:
  depends_on:
    - sot:root/autonomous-operation
    - plan:root/20261120000000
    - sot:root/production-residual-register
---

# Autonomous operation queue

The work queue unattended sessions consume (tier 3 of [[sot:root/autonomous-operation]] §3).
Each unit is one PR. Status values: `open`, `in-progress`, `done` (with the PR), `owner-blocked`
(with the owner action that unblocks it; tracked as a `needs-owner` issue).

| # | Unit | Class | Status | Exit |
|---|------|-------|--------|------|
| Q1 | `log -s --raw` prints nothing and `log --name-only -s` exits 128, as pin 2.39.5 | A | done | new corpus lines SAME in Docker |
| Q2 | `fsck --unreachable` output order (family-plumbing L428): derive the order from observed pin output or record Impossible with the reason | A | done (arks replays git's object-table lookups; L428 SAME in Docker) | line SAME or reasoned Impossible in the register |
| Q3 | gh personality: unknown flag exits 1 like gh (arks exits 2) | A | done | gh argv inventory exit-code column matches |
| Q4 | Rulesets enforced on push to an `arks serve` forge (today stored, not enforced) | A | done (end-to-end test `serve_rulesets_e2e` in CI on three OSes; not a Docker receipt, so the hub rows stay Partial) | hub e2e journey refuses a push that a ruleset forbids |
| Q5 | `instaweb` serves gitweb-equivalent pages | A | done (reasoned: no stock gitweb oracle, its Perl CGI module is missing in the image; HTTP-level checks instead) | corpus lines for page fetches SAME or reasoned (`.spec/evidence/20261125015603-instaweb-pages`, 5/5 page tests in Docker) |
| Q6 | `send-email` output against a captured SMTP session (no git-email oracle in the image) | A | done | receipt from a fixture SMTP server (`.spec/evidence/20261125005021-send-email-smtp-fixture`, 16/16 in Docker) |
| Q7 | Daemon on Windows: syslog, inetd, detach, user and drive paths | A | done (daemon matrix SoT §Windows; `--inetd` and drive-letter paths implemented, syslog / `--detach` / `--user` refused; `cfg(windows)` tests run on the `windows-2025` CI job, locally only cross-checked) | Windows CI test plus honest refuse rows |
| Q8 | Real OS credential stores for `credential-manager` / `credential-wincred` on each OS | A | done (in-process Win32 `Cred*` with GCM targets, Security.framework, Secret Service over a native D-Bus client; host: Linux private-bus round trip and no-bus refuse pass; Windows / macOS round trips run in the PR CI jobs) | per-OS CI test where a store exists, honest refuse elsewhere |
| Q9 | TLS / CRAM-MD5 for `send-email` / `imap-send` against real servers | O | owner-blocked | owner supplies a test mail account (secret) |
| Q10 | Live G8 latency and L9 publish timing | O | owner-blocked | owner supplies forge secrets and publish policy |
| Q11 | Apply the `Public surface` required check from `.github/rulesets/main-protected.json`; set `ARKS_DENY_TERMS` | O | done (applied from the owner's node with gh, issue #45 closed; the ruleset already required `Public surface`) | owner applies the ruleset and enters the secret value (rulesets are owner-only in the playpilot permission canon; labels are created when first applied) |
| Q12 | Fleet `Project` and `Session` for arks in the private fleet repository | O | done | resources already on the fleet `main`; scheduled task created on the owner's instruction and recorded by fleet PR #46 |
| Q13 | Commit signing or hook change for the Unverified badge | O | done | decided on the owner's delegated recommendation (2026-11-24): keep unsigned commits under the owner noreply identity; the History check is the integrity gate, and a signing key in unattended sessions would be a secret to protect for a cosmetic badge |

After Q1–Q8 merge, the next session re-runs the Docker A_final receipt and checks §11 of the
autonomous-operation SoT.

## Q5 design: instaweb pages

Primary: <https://git-scm.com/docs/gitweb> (ACTIONS, AND URLS; path_info form
`<repo>/<action>/<revision>:/<path>`) and <https://git-scm.com/docs/git-instaweb>.

1. **Oracle.** The clean-room image (pin 2.39.5) ships `/usr/share/gitweb/gitweb.cgi`, but stock
   gitweb cannot run there: the Perl `CGI` module is missing (the script fails to compile), and
   none of the httpds instaweb supports is installed except `python3`. There is no stock page
   oracle, so the pages follow the documentation and are verified by HTTP-level tests (status
   codes, content types, commit ids, subjects, file lists, raw blob bytes). The HTML structure is
   arks's own; it is not compared with, or copied from, gitweb.
2. **Serving.** `arks instaweb` keeps its argv and its `--start` / `--stop` / `--restart` /
   `--local` / `--port` / `--httpd` / `--browser` behaviour, and starts `arks serve-http --gitweb`.
   The pages answer `/`, `/gitweb.cgi`, `/index.cgi`, `/static/` and path_info URLs under the
   project name; every other path stays git HTTP, as before.
3. **Project.** One project: the served git directory, named by its directory name (`.git` for a
   repository with a work tree, `<name>.git` for a bare one). Description from
   `$GIT_DIR/description` or `gitweb.description`; owner from `gitweb.owner`; clone URLs from
   `$GIT_DIR/cloneurl` or `gitweb.url`.
4. **Actions.** `project_list`, `project_index`, `summary`, `shortlog`, `log`, `commit`,
   `commitdiff`, `commitdiff_plain`, `patch`, `tree`, `blob`, `blob_plain`, `heads`, `tags`, `tag`,
   `rss` and `atom`, with the parameters `p` (project), `a` (action), `h` (object or revision),
   `hb` (base revision), `f` (path) and `pg` (page), separated by `;` or `&`. Defaults follow the
   documentation: no project lists projects, a project without an action shows its summary.
   `blame` and `blame_incremental` answer 403 (the documentation says blame is off by default);
   other documented actions (`blobdiff`, `history`, `remotes`, `search`, `snapshot`) answer
   501 until implemented; an unknown action answers 400; an unknown project or object answers 404.
