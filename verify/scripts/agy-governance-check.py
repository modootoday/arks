#!/usr/bin/env python3
"""Portable governance sweep, pre-push gate, and PR hygiene checker for agy sessions.

Implements the executable checks defined in:
- .spec/sot/agy-automated-development.sot.md
- .spec/sot/autonomous-operation.sot.md
- .spec/adr/20261119220000-commit-history-playpilot-h1-h7.adr.md
- .spec/adr/20261128035000-agy-parallel-governance-and-branch-taxonomy.adr.md

Subcommands:
  sweep      Run Step 0 pre-flight sweep against origin/main and GitHub open PRs.
  gate       Run portable local pre-push gate on <base>..HEAD.
  pr-check   Validate agy PR Parallel Lease header, lane/hot-file isolation, and PR cap.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
from typing import Any

OWNER_IDENTITY = "Choi Won <1237913+myso-kr@users.noreply.github.com>"
BRANCH_RE = re.compile(
    r"^agy/(feat|fix|refactor|docs|test|verify|gov|int|revert)/[a-z0-9][a-z0-9-]{2,47}$"
)
CC_RE = re.compile(
    r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?!?: .+$"
)
FORBIDDEN_PREFIX_RE = re.compile(r"^(WIP\b|fixup!|squash!|amend!)", re.IGNORECASE)
HANGUL_RE = re.compile(r"[\uAC00-\uD7A3\u1100-\u11FF\u3130-\u318F]")
FORBIDDEN_TRAILER_RE = re.compile(
    r"^(Signed-off-by:|Co-authored-by:(?!.*@users\.noreply\.github\.com)|https?://claude\.ai/|Session:)",
    re.IGNORECASE | re.MULTILINE,
)
SPEC_ID_RE = re.compile(r"^id:\s*(\S+)\s*$", re.MULTILINE)
WIKI_LINK_RE = re.compile(r"\[\[((?:sot|plan|adr|research):root/[^\]|]+)\]\]")
REL_ID_RE = re.compile(r"^\s+-\s+((?:sot|plan|adr|research):root/\S+)\s*$", re.MULTILINE)
HOME_PATH_RE = re.compile(r"(?:/home/[a-z0-9._-]+/modootoday|/(?:Users)/[A-Za-z0-9._-]+/)")

ALLOWED_LANES = {
    "L-core",
    "L-cli",
    "L-remote",
    "L-hub",
    "L-serve",
    "L-agent",
    "L-verify",
    "L-gov",
}

HOT_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "crates/arks/src/cmd/mod.rs",
    "crates/arks/src/cmd/git_dispatch.rs",
    "crates/arks/src/cmd/porcelain_opts.rs",
    "verify/fixtures/cli-flag-corpus/build-fixture.sh",
    ".spec/sot/program-roadmap.sot.md",
    ".spec/sot/production-residual-register.sot.md",
    ".spec/plan/20261124000000-autonomous-operation-queue.plan.md",
    "docs/completion.md",
    "AGENTS.md",
    ".spec/README.md",
}

DENY_EDIT_PREFIXES = (
    ".claude/settings",
    ".cursor/hooks",
    ".spec.local/",
    ".env",
)

MAX_OPEN_AGENT_PRS = 3


def run(cmd: list[str], *, check: bool = True) -> str:
    proc = subprocess.run(cmd, text=True, capture_output=True)
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"command failed ({proc.returncode}): {' '.join(cmd)}\n{proc.stderr.strip()}"
        )
    return proc.stdout.strip()


def parse_lease_field(body: str, label: str) -> str | None:
    pattern = re.compile(
        rf"^\s*-\s*\*\*{re.escape(label)}:\*\*\s*(.+?)\s*$",
        re.IGNORECASE | re.MULTILINE,
    )
    match = pattern.search(body or "")
    return match.group(1).strip() if match else None


def gh_json(args: list[str]) -> Any:
    return json.loads(run(["gh", *args]))


def list_remote_branches(repo: str | None) -> list[str]:
    if repo:
        data = gh_json(["api", f"repos/{repo}/branches", "--paginate"])
        return sorted(b["name"] for b in data)
    proc = subprocess.run(
        ["git", "ls-remote", "--heads", "origin"], text=True, capture_output=True
    )
    if proc.returncode == 0 and proc.stdout.strip():
        out: list[str] = []
        for line in proc.stdout.splitlines():
            parts = line.split("\t")
            if len(parts) == 2 and parts[1].startswith("refs/heads/"):
                out.append(parts[1].removeprefix("refs/heads/"))
        return sorted(out)
    data = gh_json(["api", "repos/{owner}/{repo}/branches", "--paginate"])
    return sorted(b["name"] for b in data)


def cmd_sweep(args: argparse.Namespace) -> int:
    repo = args.repo
    repo_flags = ["-R", repo] if repo else []

    if not args.no_fetch:
        run(["git", "fetch", "origin", "main"])

    remote_branches = list_remote_branches(repo)

    open_prs = gh_json(
        [
            "pr",
            "list",
            *repo_flags,
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,headRefName,isDraft,mergeable,mergeStateStatus,labels,body,files",
        ]
    )

    main_runs = gh_json(
        [
            "run",
            "list",
            *repo_flags,
            "--branch",
            "main",
            "--limit",
            "15",
            "--json",
            "workflowName,headSha,status,conclusion,createdAt",
        ]
    )

    open_heads = {pr["headRefName"] for pr in open_prs}
    orphans = [b for b in sorted(remote_branches) if b != "main" and b not in open_heads]
    agent_prs = [
        pr
        for pr in open_prs
        if pr["headRefName"].startswith(("agy/", "claude/"))
    ]

    lanes_in_use: dict[str, list[int]] = {}
    hot_files_in_use: dict[str, list[int]] = {}
    conflicting: list[int] = []

    for pr in open_prs:
        num = pr["number"]
        if pr.get("mergeable") == "CONFLICTING":
            conflicting.append(num)
        lane = parse_lease_field(pr.get("body", ""), "Lane")
        if lane:
            lanes_in_use.setdefault(lane, []).append(num)
        for fobj in pr.get("files", []):
            path = fobj.get("path", "")
            if path in HOT_FILES:
                hot_files_in_use.setdefault(path, []).append(num)

    # Check only the latest non-cancelled run per workflow on main so a
    # superseded run cancelled by back-to-back merges does not trigger a false red.
    latest_by_workflow: dict[str, dict[str, Any]] = {}
    for r in main_runs:
        wf = r.get("workflowName", "")
        if wf in latest_by_workflow:
            continue
        if r.get("conclusion") == "cancelled":
            continue
        latest_by_workflow[wf] = r

    red_main = [
        r
        for r in latest_by_workflow.values()
        if r.get("conclusion") not in (None, "", "success", "skipped", "neutral")
    ]

    report = {
        "remote_branches": sorted(remote_branches),
        "orphan_branches": orphans,
        "open_prs_total": len(open_prs),
        "open_agent_prs": len(agent_prs),
        "max_open_agent_prs": MAX_OPEN_AGENT_PRS,
        "conflicting_prs": conflicting,
        "lanes_in_use": lanes_in_use,
        "hot_files_in_use": hot_files_in_use,
        "red_main_runs": red_main,
    }

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print("== agy Step 0 Pre-Flight Sweep ==")
        print(f"Remote branches ({len(remote_branches)}): {', '.join(sorted(remote_branches))}")
        print(f"Orphan branches ({len(orphans)}): {', '.join(orphans) if orphans else 'none'}")
        print(
            f"Open PRs: {len(open_prs)} total / {len(agent_prs)} agent "
            f"(cap: {MAX_OPEN_AGENT_PRS})"
        )
        print(f"Conflicting PRs: {conflicting if conflicting else 'none'}")
        print(f"Lanes in use: {lanes_in_use if lanes_in_use else 'none'}")
        print(f"Hot files in use: {hot_files_in_use if hot_files_in_use else 'none'}")
        print(f"Red main runs (last 5): {len(red_main)}")

    errors: list[str] = []
    if red_main:
        errors.append(f"main has {len(red_main)} failing recent workflow run(s)")
    if orphans:
        errors.append(f"orphan remote branch(es) without open PR: {', '.join(orphans)}")
    if conflicting:
        errors.append(f"conflicting open PR(s): {conflicting}")
    if len(agent_prs) > MAX_OPEN_AGENT_PRS:
        errors.append(
            f"open agent PR count ({len(agent_prs)}) exceeds cap ({MAX_OPEN_AGENT_PRS})"
        )
    for hf, nums in hot_files_in_use.items():
        if len(nums) > 1:
            errors.append(f"concurrent hot-file edit on {hf} across PRs {nums}")
    for lane, nums in lanes_in_use.items():
        if len(nums) > 1:
            errors.append(f"concurrent ownership lane {lane} across PRs {nums}")

    if errors and args.strict:
        for err in errors:
            print(f"FAIL: {err}", file=sys.stderr)
        return 1
    if errors:
        for err in errors:
            print(f"WARN: {err}", file=sys.stderr)
    else:
        print("OK: clean pre-flight state")
    return 0


def check_specfold_links(changed_files: list[str], errors: list[str]) -> None:
    spec_root = pathlib.Path(".spec")
    if not spec_root.is_dir():
        return
    all_ids: set[str] = set()
    for md in spec_root.rglob("*.md"):
        text = md.read_text(encoding="utf-8", errors="ignore")
        m = SPEC_ID_RE.search(text)
        if m:
            all_ids.add(m.group(1))

    for fpath in changed_files:
        if not fpath.startswith(".spec/") or not fpath.endswith(".md"):
            continue
        p = pathlib.Path(fpath)
        if not p.exists():
            continue
        text = p.read_text(encoding="utf-8")
        for ref in WIKI_LINK_RE.findall(text):
            if ref not in all_ids:
                errors.append(f"broken Specfold wiki link [[{ref}]] in {fpath}")
        for ref in REL_ID_RE.findall(text):
            if ref not in all_ids:
                errors.append(f"broken Specfold rel reference {ref} in {fpath}")
        if text.startswith("---\n") and "governs:" in text:
            parts = text.split("---\n", 2)
            if len(parts) >= 2:
                in_gov = False
                for line in parts[1].splitlines():
                    if line.strip() == "governs:":
                        in_gov = True
                        continue
                    if in_gov:
                        if line.startswith("    - "):
                            target = line.strip()[2:].strip()
                            if not pathlib.Path(target).exists():
                                errors.append(f"missing governed path {target} in {fpath}")
                        else:
                            in_gov = False


def cmd_gate(args: argparse.Namespace) -> int:
    errors: list[str] = []
    base = args.base

    branch = run(["git", "rev-parse", "--abbrev-ref", "HEAD"])
    if branch == "main":
        errors.append("HEAD is on main; agy work must run on agy/<lane>/<slug>")
    elif not args.allow_any_branch and not BRANCH_RE.match(branch):
        errors.append(
            f"branch '{branch}' does not match required pattern agy/<lane>/<slug>"
        )

    if not args.allow_dirty:
        status = run(["git", "status", "--porcelain"])
        if status:
            errors.append("working tree is not clean; commit or stash changes before pushing")

    rev_range = f"{base}..HEAD"
    merges = run(["git", "rev-list", "--merges", rev_range])
    if merges:
        errors.append(f"merge commit(s) found in {rev_range}: {merges.splitlines()}")

    commits = run(["git", "rev-list", rev_range]).splitlines()
    if not commits:
        errors.append(f"no commits in range {rev_range}")

    for sha in commits:
        short = sha[:7]
        author = run(["git", "show", "-s", "--format=%an <%ae>", sha])
        committer = run(["git", "show", "-s", "--format=%cn <%ce>", sha])
        subj = run(["git", "show", "-s", "--format=%s", sha])
        body = run(["git", "show", "-s", "--format=%b", sha])

        if author != OWNER_IDENTITY:
            errors.append(f"{short}: invalid author '{author}' (expected '{OWNER_IDENTITY}')")
        if committer != OWNER_IDENTITY:
            errors.append(
                f"{short}: invalid committer '{committer}' (expected '{OWNER_IDENTITY}')"
            )
        if len(subj) > 72:
            errors.append(f"{short}: subject exceeds 72 chars ({len(subj)}): {subj}")
        if not CC_RE.match(subj):
            errors.append(f"{short}: subject is not Conventional Commits: {subj}")
        if FORBIDDEN_PREFIX_RE.match(subj):
            errors.append(f"{short}: forbidden commit prefix in subject: {subj}")
        if HANGUL_RE.search(subj + "\n" + body):
            errors.append(f"{short}: Hangul found in commit message")
        if FORBIDDEN_TRAILER_RE.search(body):
            errors.append(f"{short}: forbidden trailer found in commit body")
        diff_tree = run(["git", "diff-tree", "--no-commit-id", "--name-only", "-r", sha])
        if not diff_tree:
            errors.append(f"{short}: empty commit is forbidden (H3)")

    changed = run(["git", "diff", "--name-only", rev_range]).splitlines()
    for fpath in changed:
        for prefix in DENY_EDIT_PREFIXES:
            if fpath.startswith(prefix):
                errors.append(f"forbidden path modified: {fpath}")
        p = pathlib.Path(fpath)
        if not p.exists() or p.is_dir():
            continue
        raw = p.read_bytes()
        if b"\x00" in raw:
            continue
        text = raw.decode("utf-8", errors="ignore")
        if HANGUL_RE.search(text):
            errors.append(f"Hangul found in tracked file: {fpath}")
        if HOME_PATH_RE.search(text):
            errors.append(f"local user home path found in tracked file: {fpath}")

    check_specfold_links(changed, errors)

    touches_rust = any(
        f.startswith("crates/") or f in ("Cargo.toml", "Cargo.lock") for f in changed
    )
    if args.rust or (touches_rust and not args.skip_rust):
        for rust_cmd in (
            ["cargo", "fmt", "--all", "--", "--check"],
            ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
            ["cargo", "test", "--workspace"],
        ):
            print(f"Running: {' '.join(rust_cmd)}")
            proc = subprocess.run(rust_cmd)
            if proc.returncode != 0:
                errors.append(f"Rust check failed: {' '.join(rust_cmd)}")
                break

    if errors:
        print("FAIL: portable pre-push gate found issues:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(
        f"PASS: portable pre-push gate green "
        f"(branch={branch}, commits={len(commits)}, files={len(changed)})"
    )
    return 0


def cmd_pr_check(args: argparse.Namespace) -> int:
    errors: list[str] = []
    repo_flags = ["-R", args.repo] if args.repo else []

    body = args.body
    head_ref = args.head_ref
    pr_number = args.pr_number

    if pr_number is not None and (body is None or head_ref is None):
        pr_data = gh_json(
            [
                "pr",
                "view",
                str(pr_number),
                *repo_flags,
                "--json",
                "headRefName,body,files",
            ]
        )
        head_ref = head_ref or pr_data.get("headRefName", "")
        body = body if body is not None else pr_data.get("body", "")
        pr_files = [f.get("path", "") for f in pr_data.get("files", [])]
    else:
        pr_files = args.files or []

    if head_ref and head_ref.startswith("agy/"):
        if not BRANCH_RE.match(head_ref):
            errors.append(
                f"PR head branch '{head_ref}' does not match agy/<lane>/<slug>"
            )
        for field in ("Runtime", "Tier", "Lane", "Hot files touched", "Base tip"):
            val = parse_lease_field(body or "", field)
            if not val:
                errors.append(f"PR body missing Parallel Lease field '**{field}:**'")
        lane = parse_lease_field(body or "", "Lane")
        if lane and lane not in ALLOWED_LANES:
            errors.append(
                f"PR body Lane '{lane}' not in allowed set {sorted(ALLOWED_LANES)}"
            )
        if "## Verification" not in (body or ""):
            errors.append("PR body missing '## Verification' section")
        if "## Class" not in (body or "") and "## Level" not in (body or ""):
            errors.append("PR body missing '## Class' section")
        if HANGUL_RE.search(body or ""):
            errors.append("PR body contains Hangul (English-only rule 05)")

    if args.check_open_prs:
        open_prs = gh_json(
            [
                "pr",
                "list",
                *repo_flags,
                "--state",
                "open",
                "--limit",
                "100",
                "--json",
                "number,headRefName,body,files",
            ]
        )
        agent_prs = [
            p for p in open_prs if p["headRefName"].startswith(("agy/", "claude/"))
        ]
        if len(agent_prs) > MAX_OPEN_AGENT_PRS:
            errors.append(
                f"open agent PR count ({len(agent_prs)}) exceeds cap ({MAX_OPEN_AGENT_PRS})"
            )

        my_lane = parse_lease_field(body or "", "Lane")
        my_hot = {f for f in pr_files if f in HOT_FILES}
        for other in open_prs:
            if pr_number is not None and other["number"] == pr_number:
                continue
            other_lane = parse_lease_field(other.get("body", ""), "Lane")
            if my_lane and other_lane and my_lane == other_lane:
                errors.append(
                    f"lane collision: {my_lane} is also claimed by open PR #{other['number']}"
                )
            other_hot = {
                f.get("path", "")
                for f in other.get("files", [])
                if f.get("path", "") in HOT_FILES
            }
            overlap = sorted(my_hot & other_hot)
            if overlap:
                errors.append(
                    f"hot-file collision with open PR #{other['number']}: {', '.join(overlap)}"
                )

    if errors:
        print("FAIL: agy PR hygiene check found issues:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(f"PASS: agy PR hygiene check green (head={head_ref or 'n/a'})")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="agy automated-development governance checker")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_sweep = sub.add_parser("sweep", help="Step 0 pre-flight sweep")
    p_sweep.add_argument("--repo", default=None, help="GitHub owner/repo override")
    p_sweep.add_argument("--no-fetch", action="store_true", help="Skip git fetch origin main")
    p_sweep.add_argument("--json", action="store_true", help="Emit JSON output")
    p_sweep.add_argument("--strict", action="store_true", help="Exit 1 on any sweep finding")
    p_sweep.set_defaults(func=cmd_sweep)

    p_gate = sub.add_parser("gate", help="Portable local pre-push gate")
    p_gate.add_argument("--base", default="origin/main", help="Base ref for commit range")
    p_gate.add_argument("--rust", action="store_true", help="Force Rust fmt/clippy/test run")
    p_gate.add_argument("--skip-rust", action="store_true", help="Skip Rust checks")
    p_gate.add_argument("--allow-dirty", action="store_true", help="Allow dirty working tree")
    p_gate.add_argument("--allow-any-branch", action="store_true", help="Skip agy/* branch check")
    p_gate.set_defaults(func=cmd_gate)

    p_pr = sub.add_parser("pr-check", help="Validate agy PR lease and overlap")
    p_pr.add_argument("--repo", default=None, help="GitHub owner/repo override")
    p_pr.add_argument("--pr-number", type=int, default=None, help="Pull request number")
    p_pr.add_argument("--head-ref", default=None, help="Head branch name")
    p_pr.add_argument("--body", default=None, help="Pull request body text")
    p_pr.add_argument("--files", nargs="*", default=None, help="Changed file paths")
    p_pr.add_argument(
        "--check-open-prs",
        action="store_true",
        help="Check open PR cap, lane collisions, and hot-file collisions via gh",
    )
    p_pr.set_defaults(func=cmd_pr_check)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
