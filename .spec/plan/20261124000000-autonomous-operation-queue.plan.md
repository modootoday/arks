---
id: plan:root/20261124000000
title: Autonomous operation queue — units after the argv-symmetry ladder
status: active
created: 2026-11-24T00:00:00Z
updated: 2026-11-25T05:15:00Z
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

After Q1–Q8 merged, the Docker receipt was re-run on `main` `61358f4`
([[evidence:root/20261125023636-argv-corpus-receipt-q-final]]): 1,852 / 1,852 options accepted;
4,398 / 4,502 corpus lines SAME; the other 104 all reasoned (77 post-pin, 26 design, 1 timing);
zero open or unreasoned lines.

§11 check of the autonomous-operation SoT at that tip, updated by the Q14 audit (2026-11-25):

| Criterion | State |
|-----------|-------|
| 1. No active plan except maintenance | **Met (2026-11-25):** the only plan with `status: active` is this queue, which §11 exempts as the maintenance queue; Q14 archived 39 plans with cited evidence and Q15/Q16 archived the hub multi-agent plan |
| 2. Residual rows Claimed, refused or Impossible | **Not met; owner-blocked only.** Every agent-workable row is Claimed or carries a cited refusal (Q17–Q21, Q15/Q16). Open: Q9 (test mail account), Q10 (forge secrets and publish policy) and Q23 (live LLM API key); Q22 is the agent-side TLS work that Q23 needs |
| 3. Zero unreasoned DIFFs in the latest receipts | **Met** (this receipt) |
| 4. Required checks green on the `main` tip | **Met** (PR #56 merged with all seven required checks green) |
| 5. `docs/completion.md` matches the receipts | **Met:** the flat-forge row cites the Q17/Q18 hub receipts and the multi-agent receipt; the daemon, serve and agent rows cite the Q19 and Q21 receipts |

Classes for Q14 onwards use the levels of [[sot:root/autonomous-operation]] §6 (A0 auto-merge, A1
`needs-human`, A2 owner queue); earlier rows use A (agent) and O (owner). Plan statuses follow what
Specfold accepts for a plan (`draft`, `active`, `superseded`, `archived`; `superseded` also needs a
superseding document): done, superseded and withdrawn plans all become `archived`, as the withdrawal
ADR keeps them "as history", and the Status line under each front matter names which of the three
it is. The argv ladder's earlier `status: done` is corrected the same way.
Owner question left by Q14: the register keeps one Design-only process row (verify directory
conventions) that, like the process-closed pin row, carries no product surface; §11 criterion 2
does not say whether such a row counts, so it is listed in register §G and not queued.

| # | Unit | Class | Status | Exit |
|---|------|-------|--------|------|
| Q14 | Plan and residual audit: set each `active` plan to `done` / `archival` / `withdrawn` from the roadmap and receipts, and list every remaining Partial residual as a queue unit or an owner-refused / Impossible row with its reason | A0 | done (39 of 41 plans closed with cited evidence; register §G maps every open row; the remainder is Q15–Q21 plus owner-blocked Q9 / Q10) | §11 criteria 1 and 2 met, or every remaining item is a queue unit |
| Q15 | Hub multi-agent P1–P2 ([[plan:root/20261111042400]]): local `arks serve` forge loopback smoke, then runners for HUB-MA-01…05 and HUB-MA-07 in `verify/scripts/run-hub-local-multi-agent.sh` | A0 | done ([[evidence:root/20261125044100-hub-local-multi-agent]] at tip `ec4bf43`, Docker, 6 and 12 agents as separate forge accounts on one loopback `arks serve`, exit 0: SMOKE and HUB-MA-01…05, 07 Claimed; the scenarios exposed three hub bugs, fixed spec-first ([[sot:root/arks-hub-local-multi-agent-pr]] concurrency contract): label / assignee edits were read-modify-write and lost concurrent changes, `pr view --json files|commits` was empty, and user keys, gists and `@me` projects were not scoped to the account) | Docker receipt per scenario ID; those rows of [[sot:root/arks-hub-local-multi-agent-pr]] move from Scaffold to Claimed and the runner no longer SKIPs them |
| Q16 | Hub multi-agent P3: HUB-MA-06, 08 and 10 runners; HUB-MA-09 (Actions) against local fixtures or N/A citing [[adr:root/20261119230000]] (Actions are GitHub-platform-only) | A0 | done (same receipt [[evidence:root/20261125044100-hub-local-multi-agent]]: HUB-MA-06, 08 and 10 Claimed, the HUB-MA-10 soak touching all 171 v1 hub targets per agent at once; HUB-MA-09 Refuse/N/A citing [[adr:root/20261119230000]] item 3, the runner checking each agent's exit-2 refusal; plan [[plan:root/20261111042400]] `archived`. Left outside these rows: the arks smart-HTTP fetch stores no tags, so HUB-MA-06 reads the tag with `ls-remote`) | every HUB-MA-* ID Claimed by a Docker receipt or N/A with a cited reason; plan [[plan:root/20261111042400]] set to `archived` |
| Q17 | Hub Docker re-receipt after the A4 gh-argv rework: the 33 v1 `Partial` rows (including the five ruleset rows Q4 left Partial) and the 95 `Claimed` rows marked "re-receipt pending" in `verify/matrices/hub-gh-arks-e2e-matrix.csv` (`dod-hub-matrix`, `ARKS_HUB_IW_GH_ARGV=1`); sync the `docs/completion.md` flat-forge row | A0 | done (Docker receipt `.spec/evidence/20261125030055-hub-matrix-rereceipt/`: nine hub paths STRICT, all exit 0; 33 rows Partial → Claimed, 95 re-receipted, v1 Partial 0; the gh-argv path gained a ruleset push journey (deletion / force-push refused, fast-forward lands), `auth refresh`, `ruleset view` and `pr update-branch`; the gh argv inventory stays host-advisory because the image has no gh oracle) | receipt under `.spec/evidence/`; no "re-receipt pending" note left; each row Claimed, or Partial with a cited reason sent to the owner queue (A2) |
| Q18 | Hub `later` arks-only rows: 7 `Deferred` and 1 `Partial` (`arks hub serve` listen/bind) | A0 | done ([[evidence:root/20261125035438-hub-later-rows]] at tip `8d2353f`, STRICT exit 0: the six `milestone` rows Claimed v1 with arks-defined gh-style leaves; `serve` Claimed v1 as the transitional alias with a listen/bind contract, the 0.0.0.0 bind probed on the container address; `review` refused as a duplicate of `pr review`, Refuse/N/A; decisions in [[sot:root/arks-hub-gh-command-matrix]] rules 12a–12c; no `Deferred` or `later` row left) | each row implemented with a receipt or refused with a reason in [[sot:root/arks-hub-gh-command-matrix]]; no `Deferred` left in the hub CSV |
| Q19 | Daemon A5 Docker pin receipt for the 17 host-tested `Parity` rows and the 1 host-tested arks-only row (`dod-daemon-matrix`, `ARKS_DAEMON_FLAGS_PARITY=1`) | A0 | done ([[evidence:root/20261125031214-daemon-flags-parity-a5]] at tip `231e884`: all 18 rows same as stock git daemon 2.39.5; the run first found four fatal-path differences, fixed spec-first; Windows clauses stay `windows-ci`) | receipt under `.spec/evidence/`; CSV notes drop "host-tested"; the `docs/completion.md` daemon row drops "Docker receipt pending" |
| Q20 | CLI survey currency: refresh `survey_status` in `verify/matrices/cli-git-help-matrix.csv` (164 `Partial`, 48 of them A3 host-tested) and the personality per-command exit tables ([[sot:root/arks-cli-personality-factory]]) from `.spec/evidence/20261125023636-argv-corpus-receipt-q-final/`; run the A3 rows in Docker | A0 | done (derived by `verify/scripts/derive-cli-survey-status.py`, [[evidence:root/20261125033117-cli-survey-currency-q20]]: 164 `Partial` → 128 `Verified` + 36 `Partial`, every Partial note carries its derived reason; A3 48 → 33 `Verified` + 15 `Partial` after a Docker re-run of the three A3 corpora (line verdicts as the receipt; the timing line SAME) and 60/60 A3 crate tests; personality exit tables reconciled in rule 22a) | no row `Partial` without a cited reason; the counts in the register, the in-scope leftovers ledger and `docs/completion.md` match the CSV |
| Q21 | Serve and agent Partial labels: B6 durable SSH listen label (its drop condition, B1/B2 + IW-B7-SSH Claimed, holds), B7 Windows file-mtime reload, `arks serve ssh-smoke`, `arks agent provider smoke` / `review --smoke` | A0 | done (B6 label removed, B1/B2 + B7 re-run [[evidence:root/20261125031426-claimed-ssh]] [[evidence:root/20261125031427-claimed-rebind-q21]]; Windows reload Claimed by the `windows-2025` CI test; smokes Claimed as diagnostics [[evidence:root/20261125031425-claimed-smoke]]; HTTPS vendor smoke stays Partial for the owner queue: no TLS client, needs an API key) | each label Claimed with a receipt (Windows CI test for the reload), removed with a cite, or kept Partial with a reason sent to the owner queue (A2) |
| Q22 | `arks-agent` HTTPS client for OpenAI-compatible and vendor endpoints (TLS with the system trust store and `ARKS_CA_BUNDLE`, as the git remote client already does), so `provider smoke` / `review --smoke` can reach `https://` | A0 | open | crate tests against a local TLS server with a test CA; the smokes report a TLS failure honestly |
| Q23 | Live LLM check for the agent smokes against a real `https://` vendor endpoint | A2 | owner-blocked | owner stores an LLM API key as an environment network secret for the chosen vendor host (never in chat); depends on Q22 |

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
