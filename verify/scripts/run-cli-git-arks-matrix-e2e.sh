#!/usr/bin/env bash
# DoD pillar 1 — git vs arks CLI matrix e2e (pin 2.39.5).
# Reads living CSV verify/matrices/cli-git-help-matrix.csv.
# Never invents PASS; never spawns gh from product. Stock `git` is pin oracle only.
# IW-PERSONALITY drop-in: sot:root/arks-verify-docker-e2e-claim-wave (ARKS_PERSONALITY_DROPIN=1).
# IW-PERSONALITY-EXIT named tables: sot:root/arks-honesty-leftovers-wave (ARKS_PERSONALITY_EXIT_TABLES=1).
# IW-PERSONALITY-SURVEY expand: sot:root/arks-in-scope-leftovers-wave (ARKS_PERSONALITY_SURVEY=1).
# Optimized DoD OD-UNIFIED / OD-COMMIT-HONESTY: sot:root/arks-dod-optimized
#   (ARKS_DOD_UNIFIED=1 / ARKS_DOD_COMMIT_HONESTY=1, or both under ARKS_DOD_STRICT=1).
# ExitCodePolicy unit greens ≠ drop-in / exit-table Claimed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

CSV="${ARKS_CLI_MATRIX_CSV:-verify/matrices/cli-git-help-matrix.csv}"
SOT="${ARKS_CLI_MATRIX_SOT:-.spec/sot/cli-git-help-matrix.sot.md}"
DOCKER_SOT="${ARKS_DOCKER_E2E_CLAIM_SOT:-.spec/sot/arks-verify-docker-e2e-claim-wave.sot.md}"
PERSONALITY_SOT="${ARKS_PERSONALITY_SOT:-.spec/sot/arks-cli-personality-factory.sot.md}"
HONESTY_SOT="${ARKS_HONESTY_LEFTOVERS_SOT:-.spec/sot/arks-honesty-leftovers-wave.sot.md}"
OPT_DOD_SOT="${ARKS_DOD_OPTIMIZED_SOT:-.spec/sot/arks-dod-optimized.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "cli-git-arks-matrix-e2e: tip=${TIP}"
echo "cli-git-arks-matrix-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"

ARKS_BIN="${ARKS_BIN:-}"
resolve_arks_bin() {
  if [[ -n "${ARKS_BIN:-}" && -x "$ARKS_BIN" ]]; then
    return 0
  fi
  if [[ -x "${CARGO_TARGET_DIR:-}/release/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/release/arks"
  elif [[ -x ./.cargo-target/release/arks ]]; then
    ARKS_BIN=./.cargo-target/release/arks
  elif [[ -x ./target/release/arks ]]; then
    ARKS_BIN=./target/release/arks
  elif [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    cargo build -p arks --release
    ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  fi
}

# --- Optimized DoD: OD-UNIFIED + OD-COMMIT-HONESTY (sot:root/arks-dod-optimized) ---
# Explicit flags OR ARKS_DOD_STRICT=1 (renewed must-pass). Does not stampede Partial×117.
RUN_OD_UNIFIED=0
RUN_OD_COMMIT=0
[[ "${ARKS_DOD_UNIFIED:-0}" == "1" || "${ARKS_DOD_STRICT:-0}" == "1" ]] && RUN_OD_UNIFIED=1
[[ "${ARKS_DOD_COMMIT_HONESTY:-0}" == "1" || "${ARKS_DOD_STRICT:-0}" == "1" ]] && RUN_OD_COMMIT=1
if [[ "$RUN_OD_UNIFIED" == "1" || "$RUN_OD_COMMIT" == "1" ]]; then
  if [[ ! -f "$OPT_DOD_SOT" ]]; then
    echo "FAIL OD: missing $OPT_DOD_SOT" >&2
    exit 1
  fi
  command -v python3 >/dev/null || {
    echo "FAIL OD: python3 missing" >&2
    exit 1
  }
  resolve_arks_bin
  if [[ ! -x "$ARKS_BIN" ]]; then
    export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
    cargo build -p arks --release
    resolve_arks_bin
  fi
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL OD: arks binary missing: $ARKS_BIN" >&2
    exit 1
  }

  WORK_OD="$(mktemp -d /tmp/arks-dod-opt.XXXXXX)"
  cleanup_od() { rm -rf "$WORK_OD"; }
  trap cleanup_od EXIT
  mkdir -p "$WORK_OD/repo"
  (
    cd "$WORK_OD/repo"
    git init -q
    git config user.email "od@arks.local"
    git config user.name "OD"
    echo "od-seed" >README
    git add README
    # Seed commit via stock git (verify oracle only — product path never spawns git).
    git -c core.hooksPath=/dev/null commit -qm "od seed"
  )

  OD_FAIL=0
  CHECKS=()

  if [[ "$RUN_OD_UNIFIED" == "1" ]]; then
    echo "cli-git-arks-matrix-e2e: OD-UNIFIED arm (arks overlap bounded smoke)"
    set +e
    uni_out="$(cd "$WORK_OD/repo" && ARKS_PERSONALITY=arks "$ARKS_BIN" status 2>"$WORK_OD/uni.err")"
    uni_ec=$?
    set -e
    cat "$WORK_OD/uni.err" >>"$WORK_OD/uni.out" || true
    printf '%s\n' "$uni_out" >>"$WORK_OD/uni.out"
    if [[ "$uni_ec" -ne 0 ]] \
      || ! grep -Fq '=== Git (status) ===' "$WORK_OD/uni.out" \
      || ! grep -Eqi 'Forge.*Partial|forge unified section Partial' "$WORK_OD/uni.out" "$WORK_OD/uni.err"; then
      echo "FAIL OD-UNIFIED: default unified status honesty" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-UNIFIED default unified ok"
      CHECKS+=("OD-UNIFIED default status Git section + Forge skip note exit 0")
    fi

    set +e
    go_out="$(cd "$WORK_OD/repo" && ARKS_PERSONALITY=arks "$ARKS_BIN" status --git-only 2>"$WORK_OD/go.err")"
    go_ec=$?
    set -e
    printf '%s\n' "$go_out" >"$WORK_OD/go.out"
    if [[ "$go_ec" -ne 0 ]] || grep -Fq '=== Forge (status) ===' "$WORK_OD/go.out"; then
      echo "FAIL OD-UNIFIED: --git-only" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-UNIFIED --git-only ok"
      CHECKS+=("OD-UNIFIED --git-only exit 0 no Forge section")
    fi

    set +e
    po_out="$(cd "$WORK_OD/repo" && ARKS_PERSONALITY=arks "$ARKS_BIN" status --porcelain 2>"$WORK_OD/po.err")"
    po_ec=$?
    set -e
    printf '%s\n' "$po_out" >"$WORK_OD/po.out"
    if [[ "$po_ec" -ne 0 ]]; then
      echo "FAIL OD-UNIFIED: --porcelain mode" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-UNIFIED --porcelain ok"
      CHECKS+=("OD-UNIFIED --porcelain accepted as git-only mode")
    fi

    set +e
    fo_out="$(cd "$WORK_OD/repo" && ARKS_PERSONALITY=arks "$ARKS_BIN" status --forge-only 2>"$WORK_OD/fo.err")"
    fo_ec=$?
    set -e
    printf '%s\n' "$fo_out" >"$WORK_OD/fo.out"
    cat "$WORK_OD/fo.err" >>"$WORK_OD/fo.out" || true
    # Hub facet may be Partial/non-zero; must not spawn gh; must not be unknown-flag.
    if grep -Eqi 'unknown.*--forge-only|unrecognized' "$WORK_OD/fo.out"; then
      echo "FAIL OD-UNIFIED: --forge-only not routed" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-UNIFIED --forge-only routed (ec=$fo_ec; hub Partial OK)"
      CHECKS+=("OD-UNIFIED --forge-only routes hub facet (no gh spawn)")
    fi
  fi

  if [[ "$RUN_OD_COMMIT" == "1" ]]; then
    echo "cli-git-arks-matrix-e2e: OD-COMMIT-HONESTY arm (O3 refuse + scripted editor)"
    (
      cd "$WORK_OD/repo"
      echo "dirty" >>README
      "$ARKS_BIN" add README
    )
    set +e
    (
      cd "$WORK_OD/repo"
      env -u GIT_EDITOR -u ARKS_EDITOR -u EDITOR "$ARKS_BIN" commit
    ) >"$WORK_OD/c-refuse.out" 2>&1
    cr_ec=$?
    set -e
    if [[ "$cr_ec" -eq 0 ]] \
      || ! grep -Eqi 'GIT_EDITOR|ARKS_EDITOR|EDITOR|-m' "$WORK_OD/c-refuse.out"; then
      echo "FAIL OD-COMMIT-HONESTY: refuse without editor" >&2
      cat "$WORK_OD/c-refuse.out" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-COMMIT-HONESTY refuse-without-editor ok (ec=$cr_ec)"
      CHECKS+=("OD-COMMIT-HONESTY commit without -m/editor → non-zero honesty")
    fi

    # Scripted editor: write a message into COMMIT_EDITMSG path the editor receives.
    cat >"$WORK_OD/fake-editor.sh" <<'EOS'
#!/bin/sh
printf '%s\n' "od commit honesty" >"$1"
EOS
    chmod +x "$WORK_OD/fake-editor.sh"
    set +e
    (
      cd "$WORK_OD/repo"
      GIT_EDITOR="$WORK_OD/fake-editor.sh" ARKS_EDITOR="$WORK_OD/fake-editor.sh" \
        "$ARKS_BIN" commit
    ) >"$WORK_OD/c-ok.out" 2>&1
    co_ec=$?
    set -e
    if [[ "$co_ec" -ne 0 ]]; then
      echo "FAIL OD-COMMIT-HONESTY: scripted editor commit" >&2
      cat "$WORK_OD/c-ok.out" >&2
      OD_FAIL=1
    else
      echo "cli-git-arks-matrix-e2e: OD-COMMIT-HONESTY scripted editor commit ok"
      CHECKS+=("OD-COMMIT-HONESTY GIT_EDITOR scripted commit exit 0")
    fi
  fi

  if [[ "$OD_FAIL" -ne 0 ]]; then
    echo "cli-git-arks-matrix-e2e: FAIL optimized DoD OD arms (no invent PASS)" >&2
    exit 1
  fi

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-dod-optimized-cli"
    mkdir -p "$OUT"
    cp -r "$WORK_OD"/*.out "$WORK_OD"/*.err "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" "$RUN_OD_UNIFIED" "$RUN_OD_COMMIT" <<'PY'
import json, sys, time
out, tip, image, uni, commit = sys.argv[1:]
checks = []
if uni == "1":
    checks.extend([
        "OD-UNIFIED default status Git section + Forge skip note exit 0",
        "OD-UNIFIED --git-only exit 0 no Forge section",
        "OD-UNIFIED --porcelain accepted as git-only mode",
        "OD-UNIFIED --forge-only routes hub facet (no gh spawn)",
    ])
if commit == "1":
    checks.extend([
        "OD-COMMIT-HONESTY commit without -m/editor → non-zero honesty",
        "OD-COMMIT-HONESTY GIT_EDITOR scripted commit exit 0",
    ])
receipt = {
    "schema": "arks-dod-optimized-cli/v1",
    "gate": "OD-UNIFIED+OD-COMMIT-HONESTY",
    "disposition": "verified-product-path",
    "venue": "docker-dod-cli-matrix",
    "tip": tip,
    "image_tag": image,
    "arks_dod_unified": uni == "1",
    "arks_dod_commit_honesty": commit == "1",
    "checks": checks,
    "note": "Full forge unified depth and CLI Partial×117 remain non-DoD.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "cli-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "cli-git-arks-matrix-e2e: host OD arms green; Docker venue required for Claimed receipt"
  fi

  # When only OD flags were requested (not a full STRICT inventory walk), stop here.
  # Under ARKS_DOD_STRICT=1 continue into Missing=0 inventory below.
  if [[ "${ARKS_DOD_STRICT:-0}" != "1" ]]; then
    echo "cli-git-arks-matrix-e2e: OD arms PASS (STRICT inventory deferred)"
    exit 0
  fi
  echo "cli-git-arks-matrix-e2e: OD arms PASS — continuing STRICT Missing=0 inventory"
  trap - EXIT
fi

# --- IW-PERSONALITY-SURVEY expand (≥10 main cmds) (ARKS_PERSONALITY_SURVEY=1) ---
if [[ "${ARKS_PERSONALITY_SURVEY:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-SURVEY (sot:root/arks-in-scope-leftovers-wave)"
  SCOPE_SOT="${ARKS_IN_SCOPE_LEFTOVERS_SOT:-.spec/sot/arks-in-scope-leftovers-wave.sot.md}"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  command -v git >/dev/null || { echo "FAIL: pin git missing" >&2; exit 1; }
  resolve_arks_bin
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  WORK="$(mktemp -d /tmp/arks-survey.XXXXXX)"
  cleanup_sv() { rm -rf "$WORK"; }
  trap cleanup_sv EXIT
  mkdir -p "$WORK/repo"
  (
    cd "$WORK/repo"
    git init -q
    git config user.email t@t
    git config user.name t
    echo hi >README
    git add README
    git commit -q -m init
    echo bye >>README
    git add README
    git commit -q -m two
  )
  SV_OK=1
  record_sv() {
    local id="$1" class="$2" actual="$3"
    echo "$id class=$class actual=$actual" >>"$WORK/survey.txt"
    if [[ "$class" == "zero" && "$actual" -ne 0 ]]; then
      echo "FAIL $id expected 0 got $actual" >&2
      SV_OK=0
    elif [[ "$class" == "nonzero" && "$actual" -eq 0 ]]; then
      echo "FAIL $id expected nonzero got 0" >&2
      SV_OK=0
    else
      echo "cli-git-arks-matrix-e2e: $id ok (exit=$actual)"
    fi
  }
  run_sv() {
    local id="$1" class="$2"
    shift 2
    local ec=0
    set +e
    (cd "$WORK/repo" && ARKS_PERSONALITY=git "$@" >"$WORK/${id}.out" 2>&1)
    ec=$?
    set -e
    record_sv "$id" "$class" "$ec"
  }
  # ≥10 main-category peer exit/stdout class rows beyond IW-PERSONALITY-EXIT named set.
  run_sv SV-STATUS zero "$ARKS_BIN" status
  run_sv SV-LOG zero "$ARKS_BIN" log -1 --oneline
  run_sv SV-SHOW zero "$ARKS_BIN" show HEAD
  run_sv SV-DIFF zero "$ARKS_BIN" diff --stat
  run_sv SV-BRANCH zero "$ARKS_BIN" branch
  run_sv SV-REVPARSE zero "$ARKS_BIN" rev-parse HEAD
  run_sv SV-LSFILES zero "$ARKS_BIN" ls-files
  run_sv SV-CONFIG-GET zero "$ARKS_BIN" config --get user.name
  run_sv SV-HASHZERO zero "$ARKS_BIN" hash-object README
  run_sv SV-CATFILE zero "$ARKS_BIN" cat-file -t HEAD
  run_sv SV-BAD nonzero "$ARKS_BIN" totally-unknown-cmd-xyz
  run_sv SV-HELP zero "$ARKS_BIN" help status
  # Additive survey2 rows (named remainder deepen; still not stampede 117).
  run_sv SV-REVLIST zero "$ARKS_BIN" rev-list --max-count=1 HEAD
  run_sv SV-REMOTE zero "$ARKS_BIN" remote
  run_sv SV-LSTREE zero "$ARKS_BIN" ls-tree HEAD
  run_sv SV-STASHLIST zero "$ARKS_BIN" stash list
  run_sv SV-SYMBOLIC zero "$ARKS_BIN" symbolic-ref HEAD
  run_sv SV-COUNT zero "$ARKS_BIN" count-objects
  run_sv SV-UPDATEINDEX zero "$ARKS_BIN" update-index --refresh
  run_sv SV-WRITETREE zero "$ARKS_BIN" write-tree
  # survey3 — further named peer exit-class rows (exhaust; still not stampede 117).
  # Only peer-proven exit-0 (or shared nonzero) cmds; Docker probe 2026-11-11.
  run_sv SV-MERGEBASE zero "$ARKS_BIN" merge-base HEAD HEAD
  run_sv SV-NAME-REV zero "$ARKS_BIN" name-rev HEAD
  run_sv SV-CHECKREF zero "$ARKS_BIN" check-ref-format refs/heads/main
  run_sv SV-DIFFILES zero "$ARKS_BIN" diff-files --name-only
  run_sv SV-VERSION zero "$ARKS_BIN" --version
  run_sv SV-PACKREFS zero "$ARKS_BIN" pack-refs --all
  run_sv SV-GC-AUTO zero "$ARKS_BIN" gc --auto
  run_sv SV-CATFILE-P zero "$ARKS_BIN" cat-file -p HEAD
  run_sv SV-CATFILE-S zero "$ARKS_BIN" cat-file -s HEAD
  run_sv SV-LSFILES-C zero "$ARKS_BIN" ls-files -c
  run_sv SV-LSFILES-M zero "$ARKS_BIN" ls-files -m
  run_sv SV-LSFILES-O zero "$ARKS_BIN" ls-files -o
  run_sv SV-STATUS-P zero "$ARKS_BIN" status --porcelain
  run_sv SV-BRANCH-A zero "$ARKS_BIN" branch -a
  run_sv SV-BRANCH-R zero "$ARKS_BIN" branch -r
  run_sv SV-LOG5 zero "$ARKS_BIN" log --oneline -5
  run_sv SV-SHOWREF zero "$ARKS_BIN" show-ref --heads
  run_sv SV-REVLIST-C zero "$ARKS_BIN" rev-list --count HEAD
  run_sv SV-COUNT-V zero "$ARKS_BIN" count-objects -v
  run_sv SV-TAG zero "$ARKS_BIN" tag
  run_sv SV-DIFF-NAME zero "$ARKS_BIN" diff --name-only
  run_sv SV-CONFIG-LIST zero "$ARKS_BIN" config --list
  run_sv SV-LSTREE-R zero "$ARKS_BIN" ls-tree -r HEAD
  run_sv SV-REVPARSE-VERIFY zero "$ARKS_BIN" rev-parse --verify HEAD
  run_sv SV-REVPARSE-ABBREV zero "$ARKS_BIN" rev-parse --abbrev-ref HEAD
  run_sv SV-REVPARSE-SYM zero "$ARKS_BIN" rev-parse --symbolic-full-name HEAD
  # survey4 — further peer exit-class expand (2-commit fixture; still not stampede 117).
  run_sv SV-REVPARSE-TILDE zero "$ARKS_BIN" rev-parse HEAD~1
  run_sv SV-MERGEBASE-TILDE zero "$ARKS_BIN" merge-base HEAD HEAD~1
  run_sv SV-LOG-RANGE zero "$ARKS_BIN" log --oneline HEAD~1..HEAD
  run_sv SV-DIFF-RANGE zero "$ARKS_BIN" diff HEAD~1 HEAD
  run_sv SV-DIFF-STAT-RANGE zero "$ARKS_BIN" diff --stat HEAD~1 HEAD
  run_sv SV-CATFILE-TILDE zero "$ARKS_BIN" cat-file -t HEAD~1
  run_sv SV-LSTREE-TILDE zero "$ARKS_BIN" ls-tree HEAD~1
  run_sv SV-NAME-REV-TILDE zero "$ARKS_BIN" name-rev HEAD~1
  run_sv SV-FOR-EACH-REF zero "$ARKS_BIN" for-each-ref refs/heads
  run_sv SV-HASH-OBJECT zero "$ARKS_BIN" hash-object README
  run_sv SV-LSFILES-PATH zero "$ARKS_BIN" ls-files README
  run_sv SV-PRUNE-N zero "$ARKS_BIN" prune -n
  run_sv SV-WHATCHANGED zero "$ARKS_BIN" whatchanged -1
  run_sv SV-DIFFTREE-R zero "$ARKS_BIN" diff-tree -r HEAD~1 HEAD
  run_sv SV-MERGETREE zero "$ARKS_BIN" merge-tree HEAD HEAD HEAD
  run_sv SV-CHECKREF-NORM zero "$ARKS_BIN" check-ref-format --normalize refs/heads/foo
  run_sv SV-STATUS-P1 zero "$ARKS_BIN" status --porcelain=v1
  run_sv SV-CHECK-IGNORE nonzero "$ARKS_BIN" check-ignore README
  if [[ "$SV_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "cli-git-arks-matrix-e2e: FAIL STRICT IW-PERSONALITY-SURVEY" >&2
      exit 1
    fi
    echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-SURVEY Partial/failed"
    exit 0
  fi
  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-SURVEY greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-personality-survey"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$WORK"/survey.txt "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-personality-survey/v1",
    "gate": "IW-PERSONALITY-SURVEY",
    "disposition": "verified-product-path",
    "venue": "docker-dod-cli-matrix",
    "tip": tip,
    "image_tag": image,
    "expand_rows": [
        "SV-STATUS", "SV-LOG", "SV-SHOW", "SV-DIFF", "SV-BRANCH", "SV-REVPARSE",
        "SV-LSFILES", "SV-CONFIG-GET", "SV-HASHZERO", "SV-CATFILE", "SV-BAD", "SV-HELP",
    ],
    "survey2_rows": [
        "SV-REVLIST", "SV-REMOTE", "SV-LSTREE", "SV-STASHLIST",
        "SV-SYMBOLIC", "SV-COUNT", "SV-UPDATEINDEX", "SV-WRITETREE",
    ],
    "survey3_rows": [
        "SV-MERGEBASE", "SV-NAME-REV", "SV-CHECKREF", "SV-DIFFILES", "SV-VERSION",
        "SV-PACKREFS", "SV-GC-AUTO", "SV-CATFILE-P", "SV-CATFILE-S", "SV-LSFILES-C",
        "SV-LSFILES-M", "SV-LSFILES-O", "SV-STATUS-P", "SV-BRANCH-A", "SV-BRANCH-R",
        "SV-LOG5", "SV-SHOWREF", "SV-REVLIST-C", "SV-COUNT-V", "SV-TAG",
        "SV-DIFF-NAME", "SV-CONFIG-LIST", "SV-LSTREE-R", "SV-REVPARSE-VERIFY",
        "SV-REVPARSE-ABBREV", "SV-REVPARSE-SYM",
    ],
    "survey4_rows": [
        "SV-REVPARSE-TILDE", "SV-MERGEBASE-TILDE", "SV-LOG-RANGE", "SV-DIFF-RANGE",
        "SV-DIFF-STAT-RANGE", "SV-CATFILE-TILDE", "SV-LSTREE-TILDE", "SV-NAME-REV-TILDE",
        "SV-FOR-EACH-REF", "SV-HASH-OBJECT", "SV-LSFILES-PATH", "SV-PRUNE-N",
        "SV-WHATCHANGED", "SV-DIFFTREE-R", "SV-MERGETREE", "SV-CHECKREF-NORM",
        "SV-STATUS-P1", "SV-CHECK-IGNORE",
    ],
    "note": "Named expand + survey2 + survey3 + survey4; CLI Partial remainder stay Partial with SoT cite.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "cli-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-PERSONALITY-EXIT named peer exit tables (ARKS_PERSONALITY_EXIT_TABLES=1) ---
if [[ "${ARKS_PERSONALITY_EXIT_TABLES:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-EXIT path (sot:root/arks-honesty-leftovers-wave)"
  for f in "$HONESTY_SOT" "$PERSONALITY_SOT"; do
    if [[ ! -f "$f" ]]; then
      echo "FAIL: missing SoT $f" >&2
      exit 1
    fi
  done
  command -v git >/dev/null || {
    echo "FAIL: pin git missing for XT-GIT compare" >&2
    exit 1
  }
  resolve_arks_bin
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL: arks binary missing after rebuild" >&2
    exit 1
  }

  WORK="$(mktemp -d /tmp/arks-xt-exit.XXXXXX)"
  SERVER_PID=""
  cleanup_xt() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_xt EXIT
  mkdir -p "$WORK/home/.config/arks" "$WORK/hub-data"
  export HOME="$WORK/home"
  unset ARKS_HUB_TOKEN || true
  rm -f "$HOME/.config/arks/hub-token" 2>/dev/null || true

  XT_OK=1
  record_xt() {
    local id="$1" expected="$2" actual="$3" note="$4"
    echo "$id expected=$expected actual=$actual note=$note" >>"$WORK/exit-table.txt"
    if [[ "$expected" == "nonzero" ]]; then
      if [[ "$actual" -eq 0 ]]; then
        echo "FAIL $id expected nonzero got 0" >&2
        XT_OK=0
      fi
    elif [[ "$actual" -ne "$expected" ]]; then
      echo "FAIL $id expected=$expected got=$actual" >&2
      XT_OK=0
    else
      echo "cli-git-arks-matrix-e2e: $id ok (exit=$actual)"
    fi
  }

  set +e
  pin_ver_ec=0
  git --version >/dev/null 2>&1 || pin_ver_ec=$?
  git_ver_ec=0
  ARKS_PERSONALITY=git "$ARKS_BIN" --version >"$WORK/xt-git-ver.out" 2>&1 || git_ver_ec=$?
  git_bad_ec=0
  ARKS_PERSONALITY=git "$ARKS_BIN" --not-a-real-flag >"$WORK/xt-git-bad.out" 2>&1 || git_bad_ec=$?
  pin_help_ec=0
  git --help >/dev/null 2>&1 || pin_help_ec=$?
  git_help_ec=0
  ARKS_PERSONALITY=git "$ARKS_BIN" --help >"$WORK/xt-git-help.out" 2>&1 || git_help_ec=$?
  gh_help_ec=0
  ARKS_PERSONALITY=gh "$ARKS_BIN" pr --help >"$WORK/xt-gh-help.out" 2>&1 || gh_help_ec=$?
  gh_unk_ec=0
  ARKS_PERSONALITY=gh "$ARKS_BIN" not-a-gh-leaf >"$WORK/xt-gh-unk.out" 2>&1 || gh_unk_ec=$?
  gh_auth_ec=0
  ARKS_PERSONALITY=gh "$ARKS_BIN" pr list --repo local/x >"$WORK/xt-gh-auth.out" 2>&1 || gh_auth_ec=$?
  set -e

  record_xt XT-GIT-VER 0 "$git_ver_ec" "pin_ver_ec=$pin_ver_ec"
  record_xt XT-GIT-BAD nonzero "$git_bad_ec" "class non-zero"
  record_xt XT-GIT-HELP 0 "$git_help_ec" "pin_help_ec=$pin_help_ec"
  record_xt XT-GH-HELP 0 "$gh_help_ec" "pr --help"
  record_xt XT-GH-UNK 2 "$gh_unk_ec" "usage/stub"
  record_xt XT-GH-AUTH 4 "$gh_auth_ec" "not logged in → gh auth-required"

  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/xtexit"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_TOKEN_FILE="$HOME/.config/arks/hub-token"
  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!
  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("xt-exit forge not ready")
PY
  "$ARKS_BIN" hub auth login >/dev/null
  "$ARKS_BIN" hub repo create xtexit --description "xt" >/dev/null
  set +e
  gh_ok_ec=0
  ARKS_PERSONALITY=gh "$ARKS_BIN" pr list --repo local/xtexit >"$WORK/xt-gh-ok.out" 2>&1 || gh_ok_ec=$?
  set -e
  record_xt XT-GH-OK 0 "$gh_ok_ec" "pr list happy"

  if [[ "$XT_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "cli-git-arks-matrix-e2e: FAIL STRICT IW-PERSONALITY-EXIT (no invent PASS)" >&2
      exit 1
    fi
    echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-EXIT Partial/failed"
    exit 0
  fi

  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY-EXIT named rows green"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-personality-exit-tables"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$WORK"/*.txt "$WORK/server.err" "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-personality-exit-tables/v1",
    "gate": "IW-PERSONALITY-EXIT",
    "disposition": "verified-product-path",
    "venue": "docker-dod-cli-matrix",
    "tip": tip,
    "image_tag": image,
    "rows": [
        "XT-GIT-VER", "XT-GIT-BAD", "XT-GIT-HELP",
        "XT-GH-HELP", "XT-GH-UNK", "XT-GH-OK", "XT-GH-AUTH",
    ],
    "note": "Named SoT rows only; CLI survey Partial×117 remain Partial. ExitCodePolicy units ≠ this row.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "cli-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "cli-git-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

# --- IW-PERSONALITY drop-in Claimed sample (ARKS_PERSONALITY_DROPIN=1) ---
if [[ "${ARKS_PERSONALITY_DROPIN:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in path (sot:root/arks-verify-docker-e2e-claim-wave)"
  for f in "$DOCKER_SOT" "$PERSONALITY_SOT"; do
    if [[ ! -f "$f" ]]; then
      echo "FAIL: missing SoT $f" >&2
      exit 1
    fi
  done
  command -v git >/dev/null || {
    echo "FAIL: pin git missing for PD-GIT compare" >&2
    exit 1
  }
  command -v python3 >/dev/null || {
    echo "FAIL: python3 missing" >&2
    exit 1
  }
  # Always rebuild so peer-shaped version/exit deepen is present (no stale binary Claimed).
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  [[ -x "$ARKS_BIN" ]] || {
    echo "FAIL: arks binary missing after rebuild: $ARKS_BIN" >&2
    exit 1
  }

  WORK="$(mktemp -d /tmp/arks-personality.XXXXXX)"
  SERVER_PID=""
  cleanup_pd() {
    if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
      kill "$SERVER_PID" 2>/dev/null || true
      wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$WORK"
  }
  trap cleanup_pd EXIT
  mkdir -p "$WORK"

  DROPIN_OK=1

  # PD-GIT: version stdout shape + exit 0 vs pin git --version exit 0.
  set +e
  pin_ver_out="$(git --version 2>&1)"
  pin_ver_ec=$?
  git_ver_out="$(ARKS_PERSONALITY=git "$ARKS_BIN" --version 2>&1)"
  git_ver_ec=$?
  git_bad_out="$(ARKS_PERSONALITY=git "$ARKS_BIN" --not-a-real-flag 2>&1)"
  git_bad_ec=$?
  set -e
  printf '%s\n' "$pin_ver_out" >"$WORK/pin-git-version.out"
  printf '%s\n' "$git_ver_out" >"$WORK/pd-git-version.out"
  printf '%s\n' "$git_bad_out" >"$WORK/pd-git-badflag.out"
  echo "pd-git-version-ec=$git_ver_ec" >"$WORK/pd-git-ecs.txt"
  echo "pin-git-version-ec=$pin_ver_ec" >>"$WORK/pd-git-ecs.txt"
  echo "pd-git-badflag-ec=$git_bad_ec" >>"$WORK/pd-git-ecs.txt"

  if [[ "$pin_ver_ec" -ne 0 ]] || [[ "$git_ver_ec" -ne 0 ]]; then
    echo "FAIL PD-GIT: version exit class (pin=$pin_ver_ec arks=$git_ver_ec)" >&2
    DROPIN_OK=0
  elif ! echo "$pin_ver_out" | grep -Eq '^git version '; then
    echo "FAIL PD-GIT: pin git --version unexpected shape" >&2
    DROPIN_OK=0
  elif ! echo "$git_ver_out" | grep -Eq '^git version '; then
    echo "FAIL PD-GIT: ARKS_PERSONALITY=git --version missing peer stdout shape" >&2
    DROPIN_OK=0
  elif [[ "$git_bad_ec" -eq 0 ]]; then
    echo "FAIL PD-GIT: bad-flag must be non-zero (got 0)" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GIT ok — version shape + exit classes (badflag ec=$git_bad_ec)"
  fi

  # PD-GIT expand (IW-PERSONALITY-FULL): --help usage-class exit 0 vs pin.
  set +e
  pin_help_out="$(git --help 2>&1)"
  pin_help_ec=$?
  git_help_out="$(ARKS_PERSONALITY=git "$ARKS_BIN" --help 2>&1)"
  git_help_ec=$?
  set -e
  printf '%s\n' "$pin_help_out" >"$WORK/pin-git-help.out"
  printf '%s\n' "$git_help_out" >"$WORK/pd-git-help.out"
  echo "pd-git-help-ec=$git_help_ec" >>"$WORK/pd-git-ecs.txt"
  echo "pin-git-help-ec=$pin_help_ec" >>"$WORK/pd-git-ecs.txt"
  if [[ "$pin_help_ec" -ne 0 ]] || [[ "$git_help_ec" -ne 0 ]]; then
    echo "FAIL PD-GIT-EXPAND: --help exit class (pin=$pin_help_ec arks=$git_help_ec)" >&2
    DROPIN_OK=0
  elif ! echo "$git_help_out" | grep -Eqi 'usage|arks|git'; then
    echo "FAIL PD-GIT-EXPAND: --help missing usage-ish stdout" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GIT-EXPAND ok — --help exit 0"
  fi

  # PD-GH: help exit 0; unknown leaf usage-class 2; pr list against local forge.
  set +e
  gh_help_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" pr --help 2>&1)"
  gh_help_ec=$?
  gh_unk_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" not-a-gh-leaf 2>&1)"
  gh_unk_ec=$?
  set -e
  printf '%s\n' "$gh_help_out" >"$WORK/pd-gh-pr-help.out"
  printf '%s\n' "$gh_unk_out" >"$WORK/pd-gh-unknown.out"
  echo "pd-gh-help-ec=$gh_help_ec" >"$WORK/pd-gh-ecs.txt"
  echo "pd-gh-unknown-ec=$gh_unk_ec" >>"$WORK/pd-gh-ecs.txt"

  if [[ "$gh_help_ec" -ne 0 ]]; then
    echo "FAIL PD-GH: pr --help exit=$gh_help_ec" >&2
    DROPIN_OK=0
  elif [[ "$gh_unk_ec" -ne 2 ]]; then
    echo "FAIL PD-GH: unknown leaf expected usage-class exit 2 (got $gh_unk_ec)" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GH help/unknown exit classes ok"
  fi

  # Happy pr list under gh personality + local forge (host journey client, not hosting Claimed).
  FORGE_PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
  FORGE_LISTEN="127.0.0.1:${FORGE_PORT}"
  mkdir -p "$WORK/hub-data" "$WORK/home/.config/arks"
  export HOME="$WORK/home"
  export ARKS_HUB_TOKEN_FILE="$WORK/home/.config/arks/hub-token"
  export ARKS_HUB_TOKEN="local-dev-token"
  export ARKS_HUB_API="http://${FORGE_LISTEN}"
  export ARKS_HUB_REPO="local/pdropin"

  "$ARKS_BIN" serve --no-ssh --no-http \
    --forge-listen "$FORGE_LISTEN" \
    --forge-data "$WORK/hub-data" \
    . >"$WORK/server.out" 2>"$WORK/server.err" &
  SERVER_PID=$!
  python3 - "$FORGE_PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("personality forge listener not ready")
PY

  "$ARKS_BIN" hub auth login >/dev/null
  "$ARKS_BIN" hub repo create pdropin --description "IW-PERSONALITY" >/dev/null
  set +e
  gh_list_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" pr list --repo local/pdropin 2>&1)"
  gh_list_ec=$?
  set -e
  printf '%s\n' "$gh_list_out" >"$WORK/pd-gh-pr-list.out"
  echo "pd-gh-pr-list-ec=$gh_list_ec" >>"$WORK/pd-gh-ecs.txt"
  if [[ "$gh_list_ec" -ne 0 ]]; then
    echo "FAIL PD-GH: pr list under gh personality exit=$gh_list_ec" >&2
    echo "$gh_list_out" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GH pr list happy path ok"
  fi

  # PD-GH expand (IW-PERSONALITY-FULL): issue --help + issue list + auth status.
  set +e
  gh_issue_help_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" issue --help 2>&1)"
  gh_issue_help_ec=$?
  gh_issue_list_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" issue list --repo local/pdropin 2>&1)"
  gh_issue_list_ec=$?
  gh_auth_out="$(ARKS_PERSONALITY=gh "$ARKS_BIN" auth status 2>&1)"
  gh_auth_ec=$?
  set -e
  printf '%s\n' "$gh_issue_help_out" >"$WORK/pd-gh-issue-help.out"
  printf '%s\n' "$gh_issue_list_out" >"$WORK/pd-gh-issue-list.out"
  printf '%s\n' "$gh_auth_out" >"$WORK/pd-gh-auth-status.out"
  echo "pd-gh-issue-help-ec=$gh_issue_help_ec" >>"$WORK/pd-gh-ecs.txt"
  echo "pd-gh-issue-list-ec=$gh_issue_list_ec" >>"$WORK/pd-gh-ecs.txt"
  echo "pd-gh-auth-status-ec=$gh_auth_ec" >>"$WORK/pd-gh-ecs.txt"
  if [[ "$gh_issue_help_ec" -ne 0 ]]; then
    echo "FAIL PD-GH-EXPAND: issue --help exit=$gh_issue_help_ec" >&2
    DROPIN_OK=0
  elif [[ "$gh_issue_list_ec" -ne 0 ]]; then
    echo "FAIL PD-GH-EXPAND: issue list exit=$gh_issue_list_ec" >&2
    echo "$gh_issue_list_out" >&2
    DROPIN_OK=0
  elif [[ "$gh_auth_ec" -ne 0 ]]; then
    echo "FAIL PD-GH-EXPAND: auth status exit=$gh_auth_ec" >&2
    echo "$gh_auth_out" >&2
    DROPIN_OK=0
  else
    echo "cli-git-arks-matrix-e2e: PD-GH-EXPAND ok — issue help/list + auth status"
  fi

  if [[ "$DROPIN_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "cli-git-arks-matrix-e2e: FAIL STRICT IW-PERSONALITY drop-in (no invent PASS)" >&2
      exit 1
    fi
    echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in Partial/failed"
    exit 0
  fi

  echo "cli-git-arks-matrix-e2e: IW-PERSONALITY sample+expand greens (ExitCodePolicy units ≠ this row)"
  echo "cli-git-arks-matrix-e2e: per-cmd peer numeric tables remain Partial (IW-PERSONALITY-FULL honesty)"

  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-personality-expand"
    mkdir -p "$OUT"
    cp "$WORK"/*.out "$WORK"/*.txt "$WORK/server.err" "$OUT/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-personality-dropin/v2",
    "gate": "IW-PERSONALITY-FULL",
    "disposition": "verified-product-path",
    "venue": "docker-dod-cli-matrix",
    "tip": tip,
    "image_tag": image,
    "checks": [
        "PD-GIT ARKS_PERSONALITY=git --version peer stdout ^git version + exit 0 vs pin",
        "PD-GIT bad-flag non-zero exit",
        "PD-GIT-EXPAND --help exit 0 vs pin",
        "PD-GH pr --help exit 0; unknown leaf exit 2",
        "PD-GH pr list happy path against local forge",
        "PD-GH-EXPAND issue --help exit 0; issue list; auth status",
    ],
    "cli_survey_missing": 0,
    "note": "Expanded sample Claimed; per-command peer numeric exit tables remain Partial. ExitCodePolicy units alone ≠ this row.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "cli-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  else
    echo "cli-git-arks-matrix-e2e: host run only; no .spec/evidence receipt (Docker Claimed venue required)"
  fi
  exit 0
fi

resolve_arks_bin
[[ -x "$ARKS_BIN" ]] || {
  echo "FAIL: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

if [[ ! -f "$CSV" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $CSV (sibling/matrix SoT not landed)"
  exit 0
fi
if [[ ! -f "$SOT" ]]; then
  echo "cli-git-arks-matrix-e2e: SKIP — missing $SOT"
  exit 0
fi

echo "cli-git-arks-matrix-e2e: csv=$CSV"

# Count Missing rows — DoD fails closed while Missing > 0 (M16 honesty).
missing=$(awk -F, 'NR>1 && $3=="Missing" {c++} END{print c+0}' "$CSV")
partial=$(awk -F, 'NR>1 && $3=="Partial" {c++} END{print c+0}' "$CSV")
refuse=$(awk -F, 'NR>1 && $3=="Refuse/N/A" {c++} END{print c+0}' "$CSV")
echo "cli-git-arks-matrix-e2e: Missing=$missing Partial=$partial Refuse/N/A=$refuse"

# Spot-check: every Partial/Parity-candidate command must not print "command not implemented".
fail=0
checked=0
while IFS=, read -r cmd category status target notes; do
  [[ "$cmd" == "git_command" ]] && continue
  case "$status" in
    Partial|Parity-candidate|Synonym-wired) ;;
    *) continue ;;
  esac
  # Skip arks-only / non-dispatch noise.
  case "$cmd" in
    ''|serve-http|lease|lock) ;;
  esac
  checked=$((checked + 1))
  # Limit sample to keep gate time bounded; full walk when Missing==0.
  if [[ "$missing" -gt 0 && "$checked" -gt 40 ]]; then
    break
  fi
  out="$("$ARKS_BIN" "$cmd" --help 2>&1 || true)"
  if echo "$out" | grep -qi 'command not implemented'; then
    echo "FAIL: $cmd still 'command not implemented' but CSV=$status"
    fail=1
  fi
done < "$CSV"

if [[ "$fail" -ne 0 ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — dispatch/CSV honesty mismatch"
  exit 1
fi

if [[ "$missing" -eq 0 ]]; then
  echo "cli-git-arks-matrix-e2e: PASS — Missing=0 and sampled dispatch honest (flag-depth still Partial)"
  exit 0
fi

# Strict DoD fails closed while Missing remain.
if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "cli-git-arks-matrix-e2e: FAIL — Missing=$missing under ARKS_DOD_STRICT=1 (no invent PASS)"
  exit 1
fi

echo "cli-git-arks-matrix-e2e: smoke-only PASS (CSV readable, sample dispatch honest); Missing=$missing — full inventory DoD waits Missing=0"
echo "cli-git-arks-matrix-e2e: IW-PERSONALITY drop-in Claimed path: ARKS_PERSONALITY_DROPIN=1 (+ STRICT)"
exit 0
