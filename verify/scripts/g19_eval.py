#!/usr/bin/env python3
"""G19 allow-list evaluator. Invoked by verify/scripts/run-g19.sh.

M17 / Pf3 regression bar: arks_median <= 0.8 * git_median (hard fail).
M18 / Pf4 aggressive observability: also record <= 0.5 claim hits (does not
redefine M17 exit). Override via ARKS_G19_WAVE / ARKS_G19_RATIO_BAR.
"""
import json
import os
import sys
from pathlib import Path

# M17 / Pf3: full G16 catalog (verify/matrices/matrix-journey.md required IDs).
allow = [
    "J-LOCAL-01",
    "J-BRANCH-01",
    "J-MERGE-FF-01",
    "J-REBASE-01",
    "J-BUNDLE-01",
    "J-CLONE-LOCAL-01",
    "J-CLONE-PATH-01",
    "J-ALIAS-01",
    "J-WORKTREE-01",
    "J-TAG-01",
    "J-STASH-01",
    "J-CHERRY-01",
    "J-DIFF-01",
    "J-RM-MV-01",
    "J-ARCHIVE-01",
    "J-REFLOG-01",
    "J-REVERT-01",
    "J-REPACK-01",
    "J-SUBMODULE-01",
]
# SoT N/A for M17 ≤0.8× regression: empty — every current G16 journey must
# measure and meet ≤0.8× (plan 20261109043916).
na = {}

# Pf5 aggressive ≤0.5× candidates (does NOT waive M17 ≤0.8×). After
# sot:root/pf5-journey-seed-amortization reopen + evidence 6/8→7/8, leftovers stay
# aggressive_na ONLY when measured ratio > 0.5; measured ≤0.5× flips to claimed.
# CLONE-LOCAL: further HTTP seed inflation worsened ratio (~0.78); amortization
# path exhausted without product HTTP-clone work — keep NA when >0.5.
# CHERRY kept as short-wall flicker guard (claim when ≤0.5×, else NA).
# Silent keep-at-0.8 for ≤0.5× leftovers is forbidden.
aggressive_na = {
    "J-RM-MV-01": "quantum-timer-floor: ≤0.5× N/A unless seed amortization clears bar (sot:root/pf5-journey-seed-amortization)",
    "J-STASH-01": "quantum-timer-floor: ≤0.5× N/A unless seed amortization clears bar (sot:root/pf5-journey-seed-amortization)",
    "J-CLONE-LOCAL-01": "mid-residual: HTTP seed amortization exhausted (larger seeds worsened ratio); ≤0.5× needs product smart-HTTP clone work (sot:root/pf5-journey-seed-amortization)",
    "J-WORKTREE-01": "mid-residual: ≤0.5× N/A unless seed amortization clears bar (sot:root/pf5-journey-seed-amortization)",
    "J-CHERRY-01": "quantum-timer-floor: short-wall flicker; ≤0.5× N/A when measured >0.5 (sot:root/pf5-journey-seed-amortization)",
}

# Hard regression bar (M17). Pf4/Pf5 aggressive claim is reported separately.
wave = os.environ.get("ARKS_G19_WAVE", "Pf5")
ratio_bar = float(os.environ.get("ARKS_G19_RATIO_BAR", "0.8"))
aggressive_bar = float(os.environ.get("ARKS_G19_AGGRESSIVE_BAR", "0.5"))

metrics_path = Path(os.environ["ARKS_G19_METRICS"])
data = json.loads(metrics_path.read_text(encoding="utf-8"))
by_id = {j["id"]: j for j in data.get("journeys", [])}
rows = []
fails = []
aggressive_hits = []
aggressive_miss = []
aggressive_na_hits = []
for jid in allow:
    if jid in na:
        rows.append(
            {
                "journey_id": jid,
                "git_median_ms": None,
                "arks_median_ms": None,
                "ratio": None,
                "pass": True,
                "na": True,
                "na_reason": na[jid],
            }
        )
        print(f"G19 {jid}: N/A ({na[jid]})")
        continue
    j = by_id.get(jid)
    if not j:
        fails.append(f"missing journey {jid}")
        continue
    git = arks = None
    for lane in j.get("lanes", []):
        if lane.get("peer") == "git":
            git = lane.get("wall_ms_total")
        if lane.get("peer") in ("arks", "alias"):
            arks = lane.get("wall_ms_total")
    if jid == "J-ALIAS-01" and git is None:
        local = by_id.get("J-LOCAL-01") or {}
        for lane in local.get("lanes", []):
            if lane.get("peer") == "git":
                git = lane.get("wall_ms_total")
    if git is None or arks is None or git <= 0:
        fails.append(f"{jid}: missing medians git={git} arks={arks}")
        continue
    ratio = arks / git
    ok = arks <= ratio_bar * git
    aggressive_ok = arks <= aggressive_bar * git
    used_aggressive_na = False
    if aggressive_ok:
        aggressive_hits.append(jid)
        aggressive_status = f"aggressive(<={aggressive_bar})=True"
    elif jid in aggressive_na:
        # Measured miss but SoT-cited Pf5 leftover — keep terminal, do not fail.
        used_aggressive_na = True
        aggressive_ok = True
        aggressive_na_hits.append(jid)
        aggressive_status = f"aggressive_na ({aggressive_na[jid]})"
    else:
        aggressive_miss.append(jid)
        fails.append(
            f"{jid}: aggressive ratio {ratio:.4f} > {aggressive_bar} "
            f"(not in aggressive_na)"
        )
        aggressive_status = f"aggressive(<={aggressive_bar})=False"
    rows.append(
        {
            "journey_id": jid,
            "git_median_ms": git,
            "arks_median_ms": arks,
            "ratio": round(ratio, 4),
            "pass": ok,
            "aggressive_pass": aggressive_ok,
            "aggressive_na": used_aggressive_na,
            "aggressive_na_reason": aggressive_na.get(jid) if used_aggressive_na else None,
            "na": False,
        }
    )
    print(
        f"G19 {jid}: git={git} arks={arks} ratio={ratio:.4f} "
        f"pass(<={ratio_bar})={ok} {aggressive_status}"
    )
    if not ok:
        fails.append(f"{jid}: ratio {ratio:.4f} > {ratio_bar}")

run = data.get("run") or {}
receipt = {
    "schema": "arks-g19-perf-comparative/v1",
    "venue": run.get("venue") or "clean-room",
    "wave": wave,
    "bar": f"arks_median <= {ratio_bar} * git_median",
    "aggressive_bar": f"arks_median <= {aggressive_bar} * git_median",
    "aggressive_hits": aggressive_hits,
    "aggressive_miss": aggressive_miss,
    "aggressive_na": aggressive_na,
    "aggressive_na_hits": aggressive_na_hits,
    "aggregation": run.get("aggregation") or "median_p95",
    "iterations": run.get("iterations"),
    "image_tag": run.get("image_tag") or os.environ.get("ARKS_G19_IMAGE_TAG", ""),
    "image_digest": run.get("image_digest") or os.environ.get("ARKS_G19_IMAGE_DIGEST", ""),
    "stock_git": run.get("stock_git_version") or os.environ.get("ARKS_G19_STOCK_GIT", ""),
    "revision": run.get("arks_revision") or os.environ.get("ARKS_G19_REVISION", ""),
    "allow_list": allow,
    "na": na,
    "journeys": rows,
}
out = Path(os.environ["ARKS_G19_RECEIPT_OUT"])
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
print(f"G19: receipt written to {out}")
claimed = len(aggressive_hits)
na_n = len(aggressive_na_hits)
terminal = claimed + na_n
print(
    f"G19: {wave} aggressive ≤{aggressive_bar}× claimed={claimed} "
    f"aggressive_na={na_n} terminal={terminal}/{len([r for r in rows if not r.get('na')])} "
    f"miss={aggressive_miss}"
)
if fails:
    print("G19: FAIL", "; ".join(fails), file=sys.stderr)
    sys.exit(1)
print(f"G19: {wave} allow-list PASS (regression bar ≤{ratio_bar}×)")
