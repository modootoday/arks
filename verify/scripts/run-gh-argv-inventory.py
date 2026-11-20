#!/usr/bin/env python3
"""gh argv symmetry inventory: which stock-gh flags does `arks` accept?

HOST ADVISORY. Plan: .spec/plan/20261120000000-argv-symmetry-ladder.plan.md (rung A4).

Usage: run-gh-argv-inventory.py <arks-bin> <out-dir> [gh-command ...]

For every in-scope row of .spec/sot/arks-hub-gh-command-matrix.csv (phase `v1` or
`later`, a real gh peer, not a GitHub-platform-only group per
adr:root/20261119230000) the flag list is read from the installed stock
`gh <cmd> --help` (FLAGS + INHERITED FLAGS, without --help). Stock gh is an oracle
only: it is never called by product code. Leaves that the installed gh does not
know yet (newer than its version) come from
verify/matrices/gh-argv-supplement.csv, transcribed from the online gh manual (pin
2.103.3 or later).

Each flag is then run as `arks <cmd> <flag> [placeholder]` with the gh
personality, against a closed API port (ARKS_HUB_API=http://127.0.0.1:9) in a
throw-away HOME and working directory, so no forge call can succeed and no state
leaks. Verdicts:

  ACCEPTED     the flag parsed (any later failure is not a parse refusal)
  REFUSED      arks reported the flag (or the whole leaf) as unknown/unsupported
  UNVALIDATED  the leaf also accepts a bogus flag, so acceptance means nothing
               (the leaf does not validate its argv); not counted as accepted

Parse level only; behaviour is covered by the hub e2e script
(ARKS_HUB_IW_GH_ARGV=1) and crate tests.

Writes <out-dir>/gh-argv-inventory.tsv and <out-dir>/summary.json.
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
MATRIX = os.path.join(ROOT, ".spec/sot/arks-hub-gh-command-matrix.csv")
SUPPLEMENT = os.path.join(ROOT, "verify/matrices/gh-argv-supplement.csv")

# GitHub-platform-only groups stay out (adr:root/20261119230000 decision 3).
OUT_PREFIXES = ("attestation", "codespace", "run", "workflow", "cache", "secret",
                "variable", "extension", "copilot", "agent-task", "skill",
                "release verify")

REFUSED_RE = re.compile(
    r"unknown flag|unknown shorthand flag|unknown command|unknown subcommand"
    r"|flag provided but not defined|not yet implemented|later — in scope"
    r"|Partial stub|terminal Partial|not supported by arks|unsupported flag"
    r"|Deferred —|refuse exit 2|not claimed",
    re.IGNORECASE,
)
BOGUS = "--arks-argv-probe-bogus"
# "  -a, --assignee login   Assign …" / "      --dry-run   Print …"
FLAG_RE = re.compile(
    r"^\s+(?:(-[A-Za-z0-9]), )?(--[A-Za-z0-9][A-Za-z0-9-]*)(?: ([^\s]+))?\s{2,}(.*)$"
)


def gh_help(cmd):
    try:
        p = subprocess.run(["gh"] + cmd.split() + ["--help"], capture_output=True,
                           timeout=20, env=dict(os.environ, GH_PROMPT_DISABLED="1",
                                                NO_COLOR="1", GH_NO_UPDATE_NOTIFIER="1"))
    except (OSError, subprocess.TimeoutExpired):
        return None
    text = p.stdout.decode(errors="replace")
    if p.returncode != 0 or "USAGE" not in text:
        return None
    # `gh <unknown>` falls back to the parent help: require the leaf in USAGE.
    usage = text.split("USAGE", 1)[1].split("\n\n", 1)[0]
    if ("gh " + cmd) not in usage and not cmd in ("co", "rs"):
        return None
    return text


def parse_flags(text):
    flags, json_fields, section = [], [], None
    for line in text.splitlines():
        if line and not line.startswith(" "):
            section = line.strip()
            continue
        if section in ("FLAGS", "INHERITED FLAGS"):
            m = FLAG_RE.match(line)
            if not m:
                continue
            short, long_, meta, desc = m.groups()
            if long_ == "--help":
                continue
            flags.append({"short": short, "long": long_, "meta": meta or "", "desc": desc})
        elif section == "JSON FIELDS":
            json_fields += [f.strip() for f in line.split(",") if f.strip()]
    return flags, json_fields


def supplement():
    rows = {}
    if not os.path.exists(SUPPLEMENT):
        return rows
    with open(SUPPLEMENT, encoding="utf-8") as f:
        for r in csv.DictReader(l for l in f if not l.startswith("#")):
            rows.setdefault(r["gh_command"], ([], []))
            if r["long"] == "(json-fields)":
                rows[r["gh_command"]][1].extend(r["meta"].split(","))
            elif r["long"]:
                rows[r["gh_command"]][0].append({
                    "short": r["short"] or None, "long": r["long"],
                    "meta": r["meta"], "desc": r["desc"]})
    return rows


def placeholder(flag, json_fields, work):
    meta, desc, long_ = flag["meta"].lower(), flag["desc"], flag["long"]
    if not meta:
        return None
    enum = re.search(r"\{([^}|]+)\|", desc)
    if enum:
        return enum.group(1)
    if long_ == "--json":
        return json_fields[0] if json_fields else "id"
    if long_ in ("--jq",) or meta == "expression":
        return "."
    if long_ == "--template":
        return "{{.}}"
    if meta in ("int", "uint", "number", "float"):
        return "1"
    if meta in ("file", "path") or "file" in long_:
        p = os.path.join(work, "placeholder.txt")
        with open(p, "w") as f:
            f.write("placeholder\n")
        return p
    if meta in ("directory", "dir"):
        return work
    if meta == "duration":
        return "1s"
    if meta == "date":
        return "2026-01-01"
    if meta in ("branch", "ref"):
        return "main"
    if "owner/repo" in meta:
        return "local/inventory"
    return "x"


def run(argv, cwd, env, timeout=10):
    try:
        p = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                           capture_output=True, timeout=timeout)
        return p.returncode, p.stdout.decode(errors="replace") + p.stderr.decode(errors="replace")
    except subprocess.TimeoutExpired:
        return 124, "timeout"


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
            commands = [r["gh_command"] for r in csv.DictReader(f)
                        if r["phase"] in ("v1", "later") and r["gh_command"] != "(none)"
                        and not r["gh_command"].startswith(OUT_PREFIXES)]
    gh_version = subprocess.run(["gh", "--version"], capture_output=True).stdout.decode().splitlines()[0]
    supp = supplement()
    work = tempfile.mkdtemp(prefix="arks-gh-argv-")
    rows, per_cmd = [], {}
    try:
        for cmd in commands:
            text = gh_help(cmd)
            source = "gh-help"
            if text is not None:
                flags, fields = parse_flags(text)
            elif cmd in supp:
                flags, fields = supp[cmd]
                source = "supplement"
            else:
                per_cmd[cmd] = {"flags": 0, "accepted": 0, "status": "NO_ORACLE", "source": "-"}
                rows.append((cmd, "-", "", "NO_ORACLE", "installed gh lacks this leaf"))
                continue

            def probe(spelling, value):
                cwd = os.path.join(work, "cwd")
                home = os.path.join(work, "userhome")
                for d in (cwd, home):
                    shutil.rmtree(d, ignore_errors=True)
                    os.makedirs(d)
                env = dict(os.environ, HOME=home, ARKS_PERSONALITY="gh",
                           ARKS_HUB_API="http://127.0.0.1:9",
                           ARKS_HUB_TOKEN_FILE=os.path.join(home, "hub-token"),
                           ARKS_HUB_CLIENT_STATE=os.path.join(home, "hub-client.json"),
                           ARKS_HUB_REPO="local/inventory", BROWSER="false",
                           ARKS_HUB_TOKEN="inventory-token", EDITOR="true", VISUAL="true",
                           GH_EDITOR="true")
                with open(os.path.join(home, "hub-token"), "w") as f:
                    f.write("inventory-token\n")
                argv = [arks] + cmd.split() + [spelling] + ([value] if value is not None else [])
                return run(argv, cwd, env)

            _, bogus_out = probe(BOGUS, None)
            validates = bool(REFUSED_RE.search(bogus_out))
            acc = 0
            total = 0
            for fl in flags:
                for spelling in (fl["long"], fl["short"]):
                    if not spelling:
                        continue
                    total += 1
                    value = placeholder(fl, fields, work)
                    code, text_out = probe(spelling, value)
                    if REFUSED_RE.search(text_out):
                        verdict = "REFUSED"
                    elif not validates:
                        verdict = "UNVALIDATED"
                    else:
                        verdict = "ACCEPTED"
                        acc += 1
                    note = (text_out.strip().splitlines() or [""])[-1][:160] if verdict != "ACCEPTED" else ""
                    rows.append((cmd, spelling, fl["meta"], verdict, note.replace("\t", " ")))
            status = "FULL" if acc == total else ("NONE" if acc == 0 else "PARTIAL")
            if total == 0:
                status = "NO_FLAGS"
            per_cmd[cmd] = {"flags": total, "accepted": acc, "status": status,
                            "source": source, "validates_argv": validates}
        with open(os.path.join(out, "gh-argv-inventory.tsv"), "w", encoding="utf-8") as f:
            f.write("gh_command\tflag\tmeta\tverdict\tnote\n")
            for r in rows:
                f.write("\t".join(r) + "\n")
        flags_total = sum(v["flags"] for v in per_cmd.values())
        acc_total = sum(v["accepted"] for v in per_cmd.values())
        summary = {
            "oracle": gh_version,
            "pin": "2.103.3",
            "note": "installed gh is the flag oracle; leaves missing from it come from "
                    "verify/matrices/gh-argv-supplement.csv (online gh manual)",
            "commands": len(per_cmd),
            "commands_full": sum(1 for v in per_cmd.values() if v["status"] in ("FULL", "NO_FLAGS")),
            "commands_no_oracle": sum(1 for v in per_cmd.values() if v["status"] == "NO_ORACLE"),
            "flags": flags_total,
            "accepted": acc_total,
            "refused": sum(1 for r in rows if r[3] == "REFUSED"),
            "unvalidated": sum(1 for r in rows if r[3] == "UNVALIDATED"),
            "per_command": per_cmd,
        }
        with open(os.path.join(out, "summary.json"), "w", encoding="utf-8") as f:
            json.dump(summary, f, indent=1, sort_keys=True)
            f.write("\n")
        print(json.dumps({k: v for k, v in summary.items() if k != "per_command"}))
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
