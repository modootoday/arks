#!/usr/bin/env python3
"""Pin git list-cmds × arks HELP/dispatch cross-check (no invented Claimed)."""
from __future__ import annotations

import csv
import re
from collections import Counter
from pathlib import Path

ROOT = Path(r"C:\workspaces\gir")
EV = ROOT / ".spec" / "evidence" / "20261111034600-git-arks-cli-help-crosscheck"
EV.mkdir(parents=True, exist_ok=True)

pin = [
    ln.strip()
    for ln in (ROOT / ".spec" / "evidence" / "_tmp-pin-cmd-list.txt").read_text(encoding="utf-8").splitlines()
    if ln.strip()
]
host = [
    ln.strip()
    for ln in (ROOT / ".spec" / "evidence" / "_tmp-git-list-cmds.txt").read_text(encoding="utf-8").splitlines()
    if ln.strip() and " " not in ln.strip()
]

main = (ROOT / "crates" / "arks" / "src" / "main.rs").read_text(encoding="utf-8")
m = re.search(r"match cmd\.as_str\(\) \{([\s\S]*?)\n    \}", main)
assert m, "match block not found"
arm_cmds = set(re.findall(r'"([a-zA-Z][a-zA-Z0-9_-]*)"', m.group(1)))

help_out = (ROOT / ".spec" / "evidence" / "_tmp-arks-help.txt").read_text(encoding="utf-8", errors="replace")
help_cmds: set[str] = set()
for line in help_out.splitlines():
    mm = re.match(r"^  ([a-z][a-z0-9_|-]*)\s", line)
    if not mm:
        continue
    name = mm.group(1)
    if "|" in name:
        parts = name.split("|")
        if name.startswith("credential-"):
            for p in parts:
                help_cmds.add(p if p.startswith("credential-") else "credential-" + p)
        else:
            help_cmds.update(parts)
    else:
        help_cmds.add(name)

# Explicit refuse arms / SoT product N/A (command-level)
explicit_refuse = {
    "backfill",
    "filter-branch",
    "gui",
    "citool",
    "gitk",
    "scalar",
}
sot_refuse_na = explicit_refuse | {
    "difftool",
    "difftool--helper",
    "mergetool",
    "instaweb",
    # companion / GUI / excluded inventory
}

# Known material command-local N/A or honesty gaps from cli-flag-parity-complete + tip docs
partial_known: dict[str, str] = {
    "status": "SoT N/A: porcelain v2 full schema extras",
    "log": "SoT N/A: --pretty=format: full custom; pager identity",
    "show": "SoT N/A: pretty/pager identity (with log)",
    "branch": "SoT N/A: --sort exotic; --format full",
    "add": "SoT N/A: mergetool GUI identity",
    "clean": "SoT N/A: -e exclude pathspec extras beyond pin subset",
    "describe": "SoT N/A: --match globs beyond claimed",
    "config": "SoT N/A: --worktree multi-wt edge beyond claimed",
    "reset": "SoT may still N/A --merge conflicted restore edge",
    "diff": "SoT N/A: --word-diff / external difftool GUI",
    "tag": "command present; signing depth owned by R4/G38 — not full man-page stamp here",
    "grep": "SoT N/A: -P PCRE",
    "init": "SoT N/A: template/shared exotic",
    "hash-object": "SoT N/A/edge: --stdin-paths batch extras",
    "cat-file": "SoT N/A: textconv filters",
    "rev-parse": "SoT N/A: --parseopt shell helper",
    "worktree": "SoT N/A: main-worktree remove",
    "submodule": "SoT N/A: custom !command update; absorbgitdirs exotic",
    "update-index": "SoT N/A: --stdin full + chmod skip-worktree edges",
    "read-tree": "SoT N/A: 3-tree full merge-index",
    "diff-tree": "SoT N/A: recursive combine",
    "for-each-ref": "SoT N/A: color/atom full set",
    "rev-list": "SoT N/A: --objects full graph extras beyond claimed",
    "pack-objects": "SoT N/A: thin-pack delta reuse exotic",
    "index-pack": "SoT N/A: thin-pack exotic",
    "ls-files": "SoT N/A: --debug",
    "merge-base": "SoT N/A: octopus >2 beyond claimed",
    "notes": "SoT N/A: merge/edit interactive",
    "sparse-checkout": "SoT N/A: non-cone full",
    "range-diff": "SoT N/A: merge-commit dual diff full emit",
    "format-patch": "SoT N/A: binary patch exotic",
    "am": "SoT N/A: interactive",
    "blame": "SoT edge: -L range exotic",
    "annotate": "synonym for blame; same edges",
    "fsck": "SoT N/A: full unreachable progress UI",
    "merge": "SoT N/A: octopus; non-ort strategies",
    "bisect": "SoT N/A: visualize GUI",
    "gc": "SoT refuse/N/A: --prune=now (CORR-001); --auto/detach",
    "maintenance": "SoT N/A: task scheduling start/stop",
    "repack": "SoT edge: --window/geometric/cruft beyond -ad",
    "lfs": "not stock git builtin; in-process subset + git-lfs handoff (FS6 closest)",
    "credential": "SoT N/A: forge OAuth / bundled GCM",
    "credential-wincred": "first-party wallet helper; OS-specific",
    "credential-libsecret": "first-party wallet helper; OS-specific",
    "credential-osxkeychain": "first-party wallet helper; OS-specific",
    "push": "tip claims delete/lease/options subsets; exotic refspecs historically refuse",
    "clone": "SoT: not --shared; bundle-uri/filter subsets Claimed elsewhere",
    "archive": "HELP claims --format=tar; other formats not stamped here",
    "pull": "cred/forge OAuth browser UX N/A; rebase/FF/3way claimed subset",
    "fetch": "filter/shallow Claimed subsets; not every man-page option stamped here",
    "cherry-pick": "conflicted sequencer edges documented; clean ranges claimed",
    "revert": "same sequencer class as cherry-pick",
    "rebase": "TTY -i Claimed FS1; exotic sequencer edges remain",
    "stash": "FS3 -p claimed; remaining man-page depth not stamped here",
    "bundle": "HTTP bundle-uri FS5; full man-page not stamped here",
    "replace": "FS5 claimed forms; full man-page not stamped here",
    "daemon": "G32 Claimed server; full git-daemon option set not stamped here",
    "upload-pack": "Wave Sv Claimed; option depth not stamped here",
    "receive-pack": "Wave Sv + proc-receive FS2/FS8; option depth not stamped here",
    "http-backend": "Wave Sv Claimed; CGI edge depth not stamped here",
    "update-server-info": "G29 Claimed; option depth not stamped here",
}

arks_only_named = {"forge", "host-env", "lease", "lock", "serve-http"}


def classify(c: str) -> tuple[str, str]:
    in_help = c in help_cmds
    in_match = c in arm_cmds
    is_internal = "--" in c

    if c in sot_refuse_na or c in explicit_refuse:
        return "Refuse/N/A", "explicit product exclusion / refuse arm or SoT N/A"

    if in_help or in_match:
        if c in partial_known:
            return "Partial", partial_known[c]
        if in_help and in_match:
            # Honest: present on surface; do NOT invent Claimed
            return (
                "Parity-candidate",
                "arks --help + dispatch; SoT M16 implement-or-N/A bar — survey does not invent Claimed",
            )
        if in_match and not in_help:
            return "Partial", "dispatch present but omitted from arks --help (help honesty gap)"
        return "Partial", "listed in HELP but no match arm"

    if is_internal:
        return "Refuse/N/A", "git internal helper (--*); not alias inventory"

    return "Missing", "pin list-cmds; arks no HELP / generic not-implemented"


rows = []
for c in sorted(set(pin)):
    status, note = classify(c)
    rows.append(
        {
            "git_cmd": c,
            "arks_help": c in help_cmds,
            "arks_dispatch": c in arm_cmds,
            "status": status,
            "note": note,
        }
    )

pin_set = set(pin)
host_only = sorted(set(host) - pin_set)
arks_extra = []
for a in sorted(arm_cmds):
    if a in pin_set:
        continue
    if a in {"help", "version"}:
        kind = "meta (git has --help/--version; list-cmds may omit)"
    elif a in arks_only_named:
        kind = "arks-only (beyond stock git builtins)"
    elif a == "lfs":
        kind = "companion surface (git-lfs); not pin builtin"
    elif a in explicit_refuse:
        kind = "refuse arm for exclusion"
    else:
        kind = "dispatch name not in pin list-cmds"
    arks_extra.append({"arks_cmd": a, "kind": kind, "in_help": a in help_cmds})

counts = Counter(r["status"] for r in rows)

with (EV / "matrix-pin-git-vs-arks.csv").open("w", encoding="utf-8", newline="") as f:
    w = csv.DictWriter(f, fieldnames=["git_cmd", "arks_help", "arks_dispatch", "status", "note"])
    w.writeheader()
    w.writerows(rows)

with (EV / "arks-only-or-unlisted.csv").open("w", encoding="utf-8", newline="") as f:
    w = csv.DictWriter(f, fieldnames=["arks_cmd", "kind", "in_help"])
    w.writeheader()
    w.writerows(arks_extra)

(EV / "host-2.54-only-cmds.txt").write_text("\n".join(host_only) + "\n", encoding="utf-8")
(EV / "arks-help-cmds.txt").write_text("\n".join(sorted(help_cmds)) + "\n", encoding="utf-8")
(EV / "arks-dispatch-cmds.txt").write_text("\n".join(sorted(arm_cmds)) + "\n", encoding="utf-8")
(EV / "pin-2.39.5-list-cmds.txt").write_text("\n".join(sorted(set(pin))) + "\n", encoding="utf-8")
(EV / "counts.txt").write_text(
    "\n".join(f"{k}={v}" for k, v in sorted(counts.items()))
    + f"\npin_unique={len(set(pin))}\nhost_only={len(host_only)}\narks_dispatch={len(arm_cmds)}\narks_help={len(help_cmds)}\n",
    encoding="utf-8",
)

print("COUNTS", dict(counts))
print("pin", len(set(pin)), "host_only", len(host_only), "help", len(help_cmds), "dispatch", len(arm_cmds))
print("--- Missing ---")
print(", ".join(r["git_cmd"] for r in rows if r["status"] == "Missing"))
print("--- Refuse/N/A ---")
print(", ".join(r["git_cmd"] for r in rows if r["status"] == "Refuse/N/A"))
print("--- Partial ---")
for r in rows:
    if r["status"] == "Partial":
        print(f"{r['git_cmd']}: {r['note']}")
print("--- Parity-candidate ---")
print(", ".join(r["git_cmd"] for r in rows if r["status"] == "Parity-candidate"))
print("--- arks extra ---")
for r in arks_extra:
    print(f"{r['arks_cmd']}: {r['kind']} help={r['in_help']}")
print("--- host-only ---")
print(", ".join(host_only))
