#!/usr/bin/env python3
"""argv symmetry inventory: which stock-git options does `arks` accept?

HOST ADVISORY unless run in the clean-room image (pin git 2.39.5).
Plan: .spec/plan/20261120000000-argv-symmetry-ladder.plan.md (A-ladder, rung A0).

Usage: run-argv-inventory.py <arks-bin> <out-dir> [command ...]

For every git command in verify/matrices/cli-git-help-matrix.csv whose target is
`must-implement` (or the commands given), read the option list from stock
`git <cmd> -h` (parse-options usage; oracle only), then run `arks <cmd> <option>`
once per option in a fresh copy of the corpus fixture repository. An option is
REFUSED when arks reports it as unsupported/unknown/not implemented, otherwise
ACCEPTED (parse-level only; behaviour equality is the flag corpus' job).
Commands whose `-h` lists no options (for example the rev-list family) are
reported as NO_INVENTORY and need a documentation-derived list.

Writes <out-dir>/argv-inventory.tsv and <out-dir>/summary.json.
"""
import csv
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
MATRIX = os.path.join(ROOT, "verify/matrices/cli-git-help-matrix.csv")
FIXTURE_SH = os.path.join(ROOT, "verify/fixtures/cli-flag-corpus/build-fixture.sh")

REFUSED_RE = re.compile(
    r"unsupported option|unknown option|unknown switch|unrecognized (argument|option)"
    r"|command not implemented|not implemented|not a git command",
    re.IGNORECASE,
)
# "    -n, --dry-run         dry run" / "    --depth <depth>" / "    --foo[=<bar>]"
OPT_RE = re.compile(
    r"^\s{2,}(?:(-[A-Za-z0-9#]),?\s*)?(--(?:\[no-\])?[A-Za-z0-9][A-Za-z0-9-]*)?"
    r"(\[=<[^>]*>\]|\[=[^\]]*\]|[ =]<[^>]*>|\s\(\S[^)]*\))?"
)

ENV = dict(os.environ)
ENV.update(
    GIT_CONFIG_NOSYSTEM="1",
    GIT_CONFIG_GLOBAL="/dev/null",
    GIT_PAGER="cat",
    PAGER="cat",
    TZ="UTC",
    GIT_EDITOR="true",
    GIT_AUTHOR_NAME="A U Thor",
    GIT_AUTHOR_EMAIL="author@example.com",
    GIT_COMMITTER_NAME="C O Mitter",
    GIT_COMMITTER_EMAIL="committer@example.com",
    ARKS_PERSONALITY="git",
)


def placeholder(meta):
    m = meta.lower()
    if any(k in m for k in ("commit", "rev", "tree", "object", "ref", "branch", "upstream", "base", "tip")):
        return "HEAD"
    if any(k in m for k in ("n>", "num", "depth", "count", "level", "limit", "jobs", "size", "width", "<n")):
        return "1"
    if "file" in m or "path" in m:
        return "README.md"
    if "when" in m:
        return "never"
    if "date" in m or "time" in m:
        return "2023-11-15"
    return "x"


def parse_options(help_text):
    opts = []
    for line in help_text.splitlines():
        if not line.startswith("  ") or line.strip().startswith(("usage:", "or:")):
            continue
        if line.strip().startswith("-NUM "):
            # "-NUM" is a digit option (archive compression level, grep -C NUM),
            # not the letter N: probe it with one digit.
            opts.append(("-1", False, ""))
            continue
        m = OPT_RE.match(line)
        if not m or not (m.group(1) or m.group(2)):
            continue
        short, long_, meta = m.group(1), m.group(2), (m.group(3) or "").strip()
        valued = bool(meta) and not meta.startswith("[")
        for spelling in (short, long_):
            if not spelling:
                continue
            if spelling.startswith("--[no-]"):
                opts.append(("--" + spelling[7:], valued, meta))
                # A negated option never takes the value.
                opts.append(("--no-" + spelling[7:], False, ""))
            else:
                opts.append((spelling, valued, meta))
    seen, out = set(), []
    for o in opts:
        if o[0] not in seen:
            seen.add(o[0])
            out.append(o)
    return out


def run(argv, cwd, timeout=8):
    try:
        p = subprocess.run(argv, cwd=cwd, env=ENV, stdin=subprocess.DEVNULL,
                           capture_output=True, timeout=timeout)
        return p.returncode, p.stdout.decode(errors="replace"), p.stderr.decode(errors="replace")
    except subprocess.TimeoutExpired:
        return 124, "", "timeout"


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    arks = os.path.abspath(sys.argv[1])
    out = os.path.abspath(sys.argv[2])
    os.makedirs(out, exist_ok=True)
    if len(sys.argv) > 3:
        commands = sys.argv[3:]
    else:
        with open(MATRIX, encoding="utf-8-sig") as f:
            commands = [r["git_command"] for r in csv.DictReader(f)
                        if r["arks_status_target"] == "must-implement"]
    work = tempfile.mkdtemp()
    try:
        fixture = os.path.join(work, "fixture")
        subprocess.run(["bash", FIXTURE_SH, fixture], check=True, capture_output=True)
        git_version = run(["git", "--version"], work)[1].strip()
        rows, per_cmd = [], {}
        for cmd in commands:
            _, so, se = run(["git", cmd, "-h"], fixture)
            opts = parse_options(so + se)
            if not opts:
                per_cmd[cmd] = {"options": 0, "accepted": 0, "status": "NO_INVENTORY"}
                rows.append((cmd, "-", "", "NO_INVENTORY", ""))
                continue
            acc = 0
            for spelling, valued, meta in opts:
                if spelling in ("-h", "--help"):
                    continue
                # A fresh copy per probe: commands that start background
                # servers (instaweb) may keep files busy in the previous copy.
                r = os.path.join(tempfile.mkdtemp(dir=work), "r")
                shutil.copytree(fixture, r, symlinks=True)
                argv = [arks, cmd, spelling] + ([placeholder(meta)] if valued else [])
                code, so2, se2 = run(argv, r)
                if cmd == "instaweb":
                    run([arks, "instaweb", "--stop"], r)
                shutil.rmtree(os.path.dirname(r), ignore_errors=True)
                refused = bool(REFUSED_RE.search(se2 + so2))
                verdict = "REFUSED" if refused else "ACCEPTED"
                acc += 0 if refused else 1
                note = (se2.strip().splitlines() or [""])[0][:160] if refused else ""
                rows.append((cmd, spelling, meta, verdict, note))
            total = len([o for o in opts if o[0] not in ("-h", "--help")])
            per_cmd[cmd] = {"options": total, "accepted": acc,
                            "status": "FULL" if acc == total else "PARTIAL"}
        with open(os.path.join(out, "argv-inventory.tsv"), "w", encoding="utf-8") as f:
            f.write("command\toption\tmeta\tverdict\tnote\n")
            for r in rows:
                f.write("\t".join(r) + "\n")
        opts_total = sum(v["options"] for v in per_cmd.values())
        acc_total = sum(v["accepted"] for v in per_cmd.values())
        summary = {
            "git": git_version,
            "commands": len(per_cmd),
            "commands_full": sum(1 for v in per_cmd.values() if v["status"] == "FULL"),
            "commands_no_inventory": sum(1 for v in per_cmd.values() if v["status"] == "NO_INVENTORY"),
            "options": opts_total,
            "accepted": acc_total,
            "per_command": per_cmd,
        }
        with open(os.path.join(out, "summary.json"), "w", encoding="utf-8") as f:
            json.dump(summary, f, indent=1, sort_keys=True)
        print(json.dumps({k: v for k, v in summary.items() if k != "per_command"}))
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
