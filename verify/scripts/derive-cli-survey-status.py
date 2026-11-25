#!/usr/bin/env python3
"""Derive `survey_status` of verify/matrices/cli-git-help-matrix.csv from an argv receipt.

Owner: queue unit Q20 (.spec/plan/20261124000000-autonomous-operation-queue.plan.md);
rule: .spec/sot/cli-git-help-matrix.sot.md (rule 7, `Verified`).
This is a non-compose helper: it reads a scrubbed clean-room receipt and never runs git or arks.

Usage:
  derive-cli-survey-status.py --receipt <evidence-dir> --out <derived.tsv>
                              [--supplement <tsv>] [--apply]

<evidence-dir> holds summary.json (per-command argv inventory), corpora/*.results.tsv
(line, SAME|DIFF, argv, why) and reasoned-diffs.tsv (corpus, line, command, class, reason).
--supplement is a TSV (command, cite) whose cite is added to the reason of a row that stays
Partial, for evidence other than the receipt (for example Docker crate-test runs).
--apply rewrites survey_status and the leading `[Q20 ...]` note of Partial / Verified rows.

A row is `Verified` only when all of these hold for its command in the receipt:
  1. the argv inventory is FULL with every option ACCEPTED, or the command has no inventory;
  2. at least one corpus line runs the command, and at least one of them is SAME;
  3. every DIFF line is reasoned with class post-pin (the pin rejects a later git feature)
     or timing (wall-clock output). A `design` DIFF means the pin image has no oracle for the
     command, so the row stays Partial; an unreasoned DIFF also keeps it Partial.
Rows in any other state (Parity-candidate, Synonym-wired, arks-only) are reported, not changed.
"""
import argparse
import csv
import glob
import json
import os
import re
import shlex
import sys

PASS_CLASSES = ("post-pin", "timing")
CANDIDATE = ("Partial", "Verified")
GLOBAL_OPTS = ("-C", "-c")
TAG = re.compile(r"^\[Q20 [^\]]*\] ?")


def command_of(argv):
    toks = shlex.split(argv)
    i = 0
    while i < len(toks) and toks[i] in GLOBAL_OPTS:
        i += 2
    return toks[i] if i < len(toks) else None


def load_reasons(path):
    reasons = {}
    with open(path, newline="") as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            reasons[(row["corpus"], row["line"])] = row
    return reasons


def load_corpora(receipt, reasons):
    lines = {}
    seen = set()
    for path in sorted(glob.glob(os.path.join(receipt, "corpora", "*.results.tsv"))):
        corpus = os.path.basename(path)[: -len(".results.tsv")]
        with open(path, newline="") as fh:
            for raw in fh:
                lid, verdict, argv, why = raw.rstrip("\n").split("\t")
                cmd = command_of(argv)
                cls = "SAME"
                if verdict != "SAME":
                    r = reasons.get((corpus, lid))
                    if r is None:
                        cls = "unreasoned"
                    else:
                        if r["command"] != argv:
                            sys.exit(f"reasoned-diffs {corpus} {lid}: argv mismatch")
                        cls = r["class"]
                    seen.add((corpus, lid))
                lines.setdefault(cmd, []).append((corpus, lid, cls, "exit" in why))
    stale = sorted(set(reasons) - seen)
    if stale:
        sys.exit(f"reasoned-diffs rows without a DIFF line: {stale[:5]}")
    return lines


def derive(cmd, inv, lines):
    status = inv.get("status", "ABSENT")
    opts, acc = inv.get("options", 0), inv.get("accepted", 0)
    counts = {"SAME": 0, "post-pin": 0, "timing": 0, "design": 0, "unreasoned": 0}
    exit_same = 0
    corpora = set()
    for corpus, _lid, cls, exit_diff in lines:
        counts[cls if cls in counts else "unreasoned"] += 1
        exit_same += 0 if exit_diff else 1
        corpora.add(corpus)
    n = len(lines)
    inv_txt = f"inventory {acc}/{opts} ACCEPTED" if status == "FULL" else "no option inventory"
    reasoned = counts["post-pin"] + counts["timing"]
    corpus_txt = f"corpus {counts['SAME']}/{n} SAME" + (f" + {reasoned} reasoned" if reasoned else "")
    if status not in ("FULL", "NO_INVENTORY"):
        return "Partial", f"not in the receipt inventory ({status})", counts, exit_same, corpora
    if status == "FULL" and acc != opts:
        return "Partial", f"{inv_txt}", counts, exit_same, corpora
    if n == 0:
        why = f"{inv_txt}; no corpus line compares its behaviour with the pin"
        return "Partial", why, counts, exit_same, corpora
    if counts["unreasoned"]:
        return "Partial", f"{inv_txt}; {counts['unreasoned']} unreasoned corpus DIFF", counts, exit_same, corpora
    if counts["design"]:
        why = f"{inv_txt}; {corpus_txt}; {counts['design']} design DIFF (pin image has no oracle)"
        return "Partial", why, counts, exit_same, corpora
    if counts["SAME"] == 0:
        return "Partial", f"{inv_txt}; no SAME corpus line", counts, exit_same, corpora
    return "Verified", f"{inv_txt}; {corpus_txt}", counts, exit_same, corpora


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--receipt", required=True)
    ap.add_argument("--matrix", default="verify/matrices/cli-git-help-matrix.csv")
    ap.add_argument("--out", required=True)
    ap.add_argument("--supplement")
    ap.add_argument("--apply", action="store_true")
    a = ap.parse_args()

    receipt_id = os.path.basename(os.path.normpath(a.receipt)).split("-")[0]
    summary = json.load(open(os.path.join(a.receipt, "summary.json")))
    per_cmd = summary["per_command"]
    lines = load_corpora(a.receipt, load_reasons(os.path.join(a.receipt, "reasoned-diffs.tsv")))
    supplement = {}
    if a.supplement:
        with open(a.supplement, newline="") as fh:
            for row in csv.reader(fh, delimiter="\t"):
                if row and not row[0].startswith("#"):
                    supplement[row[0]] = row[1]

    with open(a.matrix, newline="") as fh:
        rows = list(csv.reader(fh))
    header, body = rows[0], rows[1:]
    out_rows = []
    for row in body:
        cmd, before = row[0], row[2]
        derived, why, counts, exit_same, corpora = derive(cmd, per_cmd.get(cmd, {}), lines.get(cmd, []))
        n = len(lines.get(cmd, []))
        if before not in CANDIDATE:
            derived, why = before, "not a Partial row; unchanged"
        elif derived == "Partial" and cmd in supplement:
            why = f"{why}; {supplement[cmd]}"
        inv = per_cmd.get(cmd, {})
        out_rows.append([
            cmd, before, derived, inv.get("status", "ABSENT"), str(inv.get("options", 0)),
            str(inv.get("accepted", 0)), str(n), str(counts["SAME"]), str(counts["post-pin"]),
            str(counts["timing"]), str(counts["design"]), str(counts["unreasoned"]),
            str(exit_same), ",".join(sorted(corpora)), why,
        ])
        if a.apply and before in CANDIDATE:
            row[2] = derived
            rest = TAG.sub("", row[4]).replace("A3 host-tested; ", "A3; ")
            row[4] = f"[Q20 {derived} per receipt {receipt_id}: {why}] {rest}".rstrip()
            if "]" in why:
                sys.exit(f"{cmd}: reason must not contain ']'")

    with open(a.out, "w", newline="") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(["command", "survey_before", "survey_derived", "inventory", "options", "accepted",
                    "corpus_lines", "same", "diff_post_pin", "diff_timing", "diff_design",
                    "diff_unreasoned", "exit_code_same", "corpora", "reason"])
        w.writerows(out_rows)
    if a.apply:
        with open(a.matrix, "w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow(header)
            w.writerows(body)
    tally = {}
    for r in out_rows:
        tally[r[2]] = tally.get(r[2], 0) + 1
    print(json.dumps({"receipt": receipt_id, "rows": len(out_rows), "derived": tally}, sort_keys=True))


if __name__ == "__main__":
    main()
