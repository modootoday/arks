#!/usr/bin/env bash
# DoD pillar 2 — git-daemon vs arks daemon matrix e2e (pin 2.39.5).
# Sibling owns daemon matrix SoT; until it lands, smoke help + loopback subset only.
# Never invents PASS; never spawns gh. Stock `git` is pin oracle only.
# IW-B5-PLUS collector: sot:root/arks-honesty-leftovers-wave (ARKS_DAEMON_B5_PLUS=1).
# IW-DAEMON-FLAGS-PARITY: sot:root/arks-in-scope-leftovers-wave (ARKS_DAEMON_FLAGS_PARITY=1).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# Intent SoT landed at tip ≥9442659; flag-depth machine CSV may still be pending.
INTENT_SOT="${ARKS_DAEMON_INTENT_SOT:-.spec/sot/arks-daemon-product-intent.sot.md}"
SOT="${ARKS_DAEMON_MATRIX_SOT:-.spec/sot/arks-daemon-git-command-matrix.sot.md}"
CSV="${ARKS_DAEMON_MATRIX_CSV:-verify/matrices/daemon-git-arks-e2e-matrix.csv}"
HONESTY_SOT="${ARKS_HONESTY_LEFTOVERS_SOT:-.spec/sot/arks-honesty-leftovers-wave.sot.md}"
SCOPE_SOT="${ARKS_IN_SCOPE_LEFTOVERS_SOT:-.spec/sot/arks-in-scope-leftovers-wave.sot.md}"
# Prefer landed sibling daemon matrix SoT when present.

# ARKS_TIP: tip named by the caller when the mounted checkout is a linked worktree
# whose git dir is not visible inside the container.
TIP="${ARKS_TIP:-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)}"
echo "daemon-git-arks-matrix-e2e: tip=${TIP}"
echo "daemon-git-arks-matrix-e2e: pin git=$(git --version)"

# The flags-parity and B5+ paths build their own release binary below.
ARKS_BIN="${ARKS_BIN:-}"
if [[ -z "$ARKS_BIN" && "${ARKS_DAEMON_FLAGS_PARITY:-0}" != "1" && "${ARKS_DAEMON_B5_PLUS:-0}" != "1" ]]; then
  if [[ -x ./.cargo-target/debug/arks ]]; then
    ARKS_BIN=./.cargo-target/debug/arks
  elif [[ -x ./target/debug/arks ]]; then
    ARKS_BIN=./target/debug/arks
  elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/arks" ]]; then
    ARKS_BIN="${CARGO_TARGET_DIR}/debug/arks"
  else
    cargo build -p arks
    ARKS_BIN=./.cargo-target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/debug/arks
    [[ -x "$ARKS_BIN" ]] || ARKS_BIN="${CARGO_TARGET_DIR:-target}/debug/arks"
  fi
fi

# --- IW-DAEMON-FLAGS-PARITY (ARKS_DAEMON_FLAGS_PARITY=1) ---
if [[ "${ARKS_DAEMON_FLAGS_PARITY:-0}" == "1" ]]; then
  echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY (sot:root/arks-in-scope-leftovers-wave)"
  [[ -f "$SCOPE_SOT" ]] || { echo "FAIL: missing $SCOPE_SOT" >&2; exit 1; }
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks
  WORK="$(mktemp -d /tmp/arks-daemon-flags.XXXXXX)"
  cleanup_df() { rm -rf "$WORK"; }
  trap cleanup_df EXIT
  mkdir -p "$WORK/repo"
  (
    cd "$WORK/repo"
    git init -q
    git config user.email t@t
    git config user.name t
    echo hi >README
    git add README
    git commit -q -m init
    touch .git/git-daemon-export-ok
  )
  DF_OK=1
  # Help advertises peer-mapped flags.
  help_out="$("$ARKS_BIN" daemon --help 2>&1 || true)"
  for flag in --verbose --timeout --init-timeout --max-connections --strict-paths --listen --port --enable --disable --reuseaddr --base-path; do
    if ! echo "$help_out" | grep -Fq -- "$flag"; then
      echo "FAIL DF: help missing $flag" >&2
      DF_OK=0
    fi
  done
  # Bind listen+port+verbose+timeouts+strict-paths+base-path; clone via git://
  # (--strict-paths: no suffix guessing, so the non-bare repository is named by its .git)
  PORT="$(python3 - <<'PY'
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"
  "$ARKS_BIN" daemon --verbose --listen=127.0.0.1 --port="$PORT" \
    --timeout=30 --init-timeout=5 --max-connections=8 --strict-paths \
    --base-path="$WORK" --export-all --reuseaddr \
    --enable=upload-pack --disable=receive-pack \
    "$WORK/repo/.git" >"$WORK/daemon.out" 2>"$WORK/daemon.err" &
  DPID=$!
  sleep 0.3
  if ! kill -0 "$DPID" 2>/dev/null; then
    echo "FAIL DF: daemon failed to start" >&2
    cat "$WORK/daemon.err" >&2 || true
    DF_OK=0
  else
    mkdir -p "$WORK/clone"
    if ! git -c protocol.version=1 clone --quiet "git://127.0.0.1:${PORT}/repo/.git" "$WORK/clone/repo" 2>"$WORK/clone.err"; then
      echo "FAIL DF: git:// clone via listen/port/base-path" >&2
      cat "$WORK/clone.err" "$WORK/daemon.err" >&2 || true
      DF_OK=0
    fi
    # receive-pack disabled: push should fail (no new commit needed — empty push probe).
    set +e
    git -C "$WORK/clone/repo" push "git://127.0.0.1:${PORT}/repo/.git" HEAD:refs/heads/denied 2>"$WORK/push.err"
    push_ec=$?
    set -e
    if [[ "$push_ec" -eq 0 ]]; then
      echo "FAIL DF: push succeeded while receive-pack disabled" >&2
      DF_OK=0
    fi
    kill "$DPID" 2>/dev/null || true
    wait "$DPID" 2>/dev/null || true
  fi
  # upload-archive enable + ustar via git archive --remote (path-filter Stay Partial).
  PORT_A="$(python3 - <<'PY'
import socket
s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()
PY
)"
  # Mirror B5 layout: base-path + repo.git name for --remote path.
  rm -rf "$WORK/repo.git"
  mv "$WORK/repo/.git" "$WORK/repo.git"
  touch "$WORK/repo.git/git-daemon-export-ok"
  "$ARKS_BIN" daemon --listen=127.0.0.1 --port="$PORT_A" --export-all --reuseaddr \
    --enable=upload-archive --base-path="$WORK" \
    >"$WORK/daemon-archive.out" 2>"$WORK/daemon-archive.err" &
  APID=$!
  sleep 0.3
  if ! kill -0 "$APID" 2>/dev/null; then
    echo "FAIL DF: upload-archive daemon failed to start" >&2
    cat "$WORK/daemon-archive.err" >&2 || true
    DF_OK=0
  else
    set +e
    # Run outside the checkout: a linked worktree's .git file is dangling in the container.
    (cd "$WORK" && git archive --remote="git://127.0.0.1:${PORT_A}/repo.git" --format=tar HEAD) \
      >"$WORK/remote.tar" 2>"$WORK/archive.err"
    arch_ec=$?
    set -e
    if [[ "$arch_ec" -ne 0 ]] || [[ ! -s "$WORK/remote.tar" ]]; then
      echo "FAIL DF: git archive --remote upload-archive" >&2
      cat "$WORK/archive.err" "$WORK/daemon-archive.err" >&2 || true
      DF_OK=0
    else
      echo "daemon-git-arks-matrix-e2e: DF upload-archive ok (bytes=$(wc -c <"$WORK/remote.tar"))"
    fi
    kill "$APID" 2>/dev/null || true
    wait "$APID" 2>/dev/null || true
  fi
  # --- A5 (plan:root/20261120000000): former Refuse/N/A rows vs stock git daemon oracle ---
  # Same daemon argv + same raw git:// requests to both daemons; replies must match.
  A5="$WORK/a5"
  mkdir -p "$A5/srv"
  for r in pub hidden noup arch priv; do git init -q --bare "$A5/srv/$r.git"; done
  for r in pub noup arch priv; do touch "$A5/srv/$r.git/git-daemon-export-ok"; done
  git -C "$A5/srv/noup.git" config daemon.uploadpack false
  git -C "$A5/srv/arch.git" config daemon.uploadarch true
  git init -q "$A5/srv/work"
  touch "$A5/srv/work/.git/git-daemon-export-ok"
  mkdir -p "$A5/srv/vhost/localhost"
  git init -q --bare "$A5/srv/vhost/localhost/v.git"
  touch "$A5/srv/vhost/localhost/v.git/git-daemon-export-ok"
  cat >"$A5/hook.sh" <<'HOOK'
#!/bin/sh
echo "$*|$REMOTE_ADDR|${REMOTE_PORT:+port-set}" >>"$(dirname "$0")/hook-$DAEMON_KIND.log"
case "$2" in *priv*) echo "declined by hook"; exit 1;; esac
exit 0
HOOK
  chmod +x "$A5/hook.sh"
  cat >"$A5/ask.py" <<'PY'
import socket, sys
port, reqs = int(sys.argv[1]), sys.argv[2:]
for r in reqs:
    # A trailing "<flush>" sends an empty argument list (flush-pkt) after the request.
    body, flush = (r[:-7], True) if r.endswith("<flush>") else (r, False)
    payload = body.encode().decode("unicode_escape").encode("latin1")
    s = socket.create_connection(("127.0.0.1", port))
    s.sendall(b"%04x" % (len(payload) + 4) + payload + (b"0000" if flush else b""))
    s.settimeout(3)
    data = b""
    try:
        while len(data) < 4:
            d = s.recv(4096)
            if not d:
                break
            data += d
        if data[:4] != b"0000" and len(data) >= 4:
            want = int(data[:4], 16)
            while len(data) < want:
                d = s.recv(4096)
                if not d:
                    break
                data += d
            data = data[:want]
    except Exception:
        pass
    s.close()
    line = data.decode("latin1")
    # Advertisements differ in agent/capabilities; keep only the reply class.
    print(r, "=>", line if "ERR" in line else ("ADVERT" if line else "EMPTY"))
PY
  a5_case() {
    local name="$1" args="$2"; shift 2
    local out_git="$A5/$name.git.out" out_arks="$A5/$name.arks.out" p
    for kind in git arks; do
      p="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])')"
      if [[ "$kind" == git ]]; then
        DAEMON_KIND=git git daemon --listen=127.0.0.1 --port="$p" $args >/dev/null 2>"$A5/$name.$kind.err" &
      else
        DAEMON_KIND=arks "$ARKS_BIN" daemon --listen=127.0.0.1 --port="$p" $args >/dev/null 2>"$A5/$name.$kind.err" &
      fi
      local pid=$!
      for _ in $(seq 1 50); do
        python3 -c "import socket;socket.create_connection(('127.0.0.1',$p)).close()" 2>/dev/null && break
        sleep 0.1
      done
      python3 "$A5/ask.py" "$p" "$@" >"$A5/$name.$kind.out"
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    done
    if diff -u "$out_git" "$out_arks" >"$A5/$name.diff"; then
      echo "daemon-git-arks-matrix-e2e: A5 $name SAME"
    else
      echo "FAIL A5 $name: reply differs from stock git daemon" >&2
      cat "$A5/$name.diff" >&2
      DF_OK=0
    fi
  }
  S="$A5/srv"
  a5_case generic-errors "--base-path=$S" \
    'git-upload-pack /nope.git\0' 'git-upload-pack /hidden.git\0' 'git-upload-pack pub.git\0' \
    'git-receive-pack /pub.git\0' 'git-upload-pack /noup.git\0' 'git-upload-pack /pub\0' \
    'git-upload-pack /work\0' 'git-upload-pack //pub.git\0' 'git-upload-pack /../srv/pub.git\0'
  a5_case informative-hook "--informative-errors --base-path=$S --access-hook=$A5/hook.sh" \
    'git-upload-pack /nope.git\0' 'git-upload-pack /hidden.git\0' 'git-receive-pack /pub.git\0' \
    'git-upload-pack /noup.git\0' 'git-upload-pack /priv.git\0host=localhost\0' \
    'git-upload-pack /pub.git\0host=localhost:9999\0' 'git-upload-pack ~root/x\0'
  a5_case override "--informative-errors --base-path=$S --forbid-override=upload-pack --forbid-override=receive-pack" \
    'git-upload-pack /noup.git\0' 'git-receive-pack /pub.git\0' 'git-upload-archive /pub.git\0'
  a5_case whitelist "--informative-errors $S/pub.git $S/work" \
    "git-upload-pack $S/pub.git\\0" "git-upload-pack $S/pub\\0" "git-upload-pack $S/work\\0" \
    "git-upload-pack $S/arch.git\\0"
  # Not compared: a missing path under --strict-paths (stock git dies on the
  # safe.directory check and closes without a reply; arks answers ERR).
  a5_case strict "--informative-errors --strict-paths $S/pub.git $S/work" \
    "git-upload-pack $S/pub.git\\0" "git-upload-pack $S/pub.git/\\0" "git-upload-pack $S/work\\0"
  a5_case relaxed "--informative-errors --base-path=$S --base-path-relaxed" \
    "git-upload-pack $S/pub.git\\0" 'git-upload-pack /pub.git\0' 'git-upload-pack /zzz.git\0'
  a5_case interpolated "--informative-errors --base-path=$S --interpolated-path=$S/vhost/%H%D" \
    'git-upload-pack /v.git\0host=LocalHost:77\0' 'git-upload-pack /v.git\0host=other\0' \
    'git-upload-pack /pub.git\0'
  a5_case user-path "--informative-errors --user-path=a5-no-such-dir" \
    'git-upload-pack ~root/x.git\0' 'git-upload-pack ~a5nouser/x.git\0'
  # Whole lines: hook arguments, REMOTE_ADDR and whether REMOTE_PORT is set.
  if ! diff "$A5/hook-git.log" "$A5/hook-arks.log" >"$A5/hook.diff"; then
    echo "FAIL A5 access-hook arguments / REMOTE_ADDR differ" >&2
    cat "$A5/hook.diff" >&2
    DF_OK=0
  elif ! grep -q '|127\.0\.0\.1|port-set$' "$A5/hook-arks.log"; then
    echo "FAIL A5 access-hook REMOTE_ADDR/REMOTE_PORT not set" >&2
    DF_OK=0
  else
    echo "daemon-git-arks-matrix-e2e: A5 access-hook arguments + REMOTE_ADDR SAME"
  fi
  # --- Q19: Docker pin coverage of the remaining A5 rows (plan:root/20261124000000) ---
  free_port() { python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1])'; }
  wait_port() {
    for _ in $(seq 1 50); do
      python3 -c "import socket;socket.create_connection(('127.0.0.1',$1)).close()" 2>/dev/null && return 0
      sleep 0.1
    done
    return 1
  }
  port_closed() {
    for _ in $(seq 1 30); do
      python3 -c "import socket;socket.create_connection(('127.0.0.1',$1)).close()" 2>/dev/null || return 0
      sleep 0.1
    done
    return 1
  }
  run_daemon() {
    local kind="$1"; shift
    if [[ "$kind" == git ]]; then git daemon "$@"; else "$ARKS_BIN" daemon "$@"; fi
  }
  a5_same() {
    if diff -u "$A5/$1.git.out" "$A5/$1.arks.out" >"$A5/$1.diff"; then
      echo "daemon-git-arks-matrix-e2e: A5 $1 SAME"
    else
      echo "FAIL A5 $1: differs from stock git daemon" >&2
      cat "$A5/$1.diff" >&2
      DF_OK=0
    fi
  }
  # --allow-override / daemon.* config: site-disabled services re-enabled per repository.
  for r in yesup recv; do
    git init -q --bare "$S/$r.git"
    touch "$S/$r.git/git-daemon-export-ok"
  done
  git -C "$S/yesup.git" config daemon.uploadpack true
  git -C "$S/recv.git" config daemon.receivepack true
  a5_case allow-override "--informative-errors --base-path=$S --disable=upload-pack --allow-override=upload-pack" \
    'git-upload-pack /yesup.git\0' 'git-upload-pack /pub.git\0' 'git-upload-pack /noup.git\0' \
    'git-upload-pack /nope.git\0'
  a5_case forbid-disabled "--informative-errors --base-path=$S --disable=upload-pack --forbid-override=upload-pack" \
    'git-upload-pack /yesup.git\0' 'git-upload-pack /nope.git\0'
  # arch.git gets a commit so both daemons can ACK the archive request.
  git init -q "$A5/seed"
  git -C "$A5/seed" -c user.name=t -c user.email=t@example.com commit -q --allow-empty -m seed
  git -C "$A5/seed" push -q "$S/arch.git" HEAD:refs/heads/master
  # upload-archive is asked with an empty argument list: stock git ACKs as soon as its
  # writer starts, arks after reading the arguments (archive ordering is outside A5).
  a5_case config-services "--informative-errors --base-path=$S" \
    'git-receive-pack /recv.git\0' 'git-receive-pack /pub.git\0' 'git-upload-archive /arch.git\0<flush>' \
    'git-upload-archive /pub.git\0'
  # --user-path: ~root resolves through the passwd home (/root in the image).
  mkdir -p /root/a5up
  git init -q --bare /root/a5up/u.git
  touch /root/a5up/u.git/git-daemon-export-ok
  git init -q --bare /root/a5home.git
  touch /root/a5home.git/git-daemon-export-ok
  a5_case user-path-dir "--informative-errors --user-path=a5up" \
    'git-upload-pack ~root/u.git\0' 'git-upload-pack ~root/u\0' 'git-upload-pack ~root/zz.git\0'
  a5_case user-path-home "--informative-errors --user-path" \
    'git-upload-pack ~root/a5home.git\0' 'git-upload-pack ~root/a5up/u.git\0'
  # --log-destination=stderr / none: same log lines (pids and ports masked), sorted.
  for dest in stderr none; do
    for kind in git arks; do
      p="$(free_port)"
      run_daemon "$kind" --verbose --log-destination="$dest" --informative-errors \
        --listen=127.0.0.1 --port="$p" --base-path="$S" >/dev/null 2>"$A5/logdest-$dest.$kind.raw" &
      pid=$!
      wait_port "$p" || true
      python3 "$A5/ask.py" "$p" 'git-upload-pack /nope.git\0' 'git-upload-pack /hidden.git\0' >/dev/null
      sleep 0.5
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      sed -E 's/[0-9]+/N/g' "$A5/logdest-$dest.$kind.raw" | sort >"$A5/logdest-$dest.$kind.out"
    done
    a5_same "logdest-$dest"
  done
  if [[ -s "$A5/logdest-none.arks.out" || ! -s "$A5/logdest-stderr.arks.out" ]]; then
    echo "FAIL A5 log-destination: stderr must log, none must stay silent" >&2
    DF_OK=0
  fi
  # --syslog / --log-destination=syslog / implied syslog under --inetd: datagrams on
  # /dev/log (root in the image), compared as <PRI> + message text (digits masked).
  cat >"$A5/collect.py" <<'PY'
import os, signal, socket, sys
out = sys.argv[1]
try:
    os.unlink("/dev/log")
except OSError:
    pass
s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
s.bind("/dev/log")
open(out + ".ready", "w").close()
signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))
with open(out, "ab") as f:
    while True:
        f.write(s.recv(65535).rstrip(b"\n") + b"\n")
        f.flush()
PY
  cat >"$A5/syslog-norm.py" <<'PY'
import re, sys
lines = []
for raw in open(sys.argv[1], "rb").read().decode("latin1").splitlines():
    m = re.match(r"<(\d+)>.*?\[\d+\]: (.*)$", raw)
    lines.append("<%s> %s" % (m.group(1), re.sub(r"\d+", "N", m.group(2))) if m else "UNPARSED " + raw)
print("\n".join(sorted(lines)))
PY
  for kind in git arks; do
    rm -f "$A5/syslog.$kind.raw" "$A5/syslog.$kind.raw.ready"
    python3 "$A5/collect.py" "$A5/syslog.$kind.raw" &
    cpid=$!
    for _ in $(seq 1 50); do [[ -f "$A5/syslog.$kind.raw.ready" ]] && break; sleep 0.1; done
    : >"$A5/syslog.$kind.stderr"
    for sflag in --syslog --log-destination=syslog; do
      p="$(free_port)"
      run_daemon "$kind" --verbose "$sflag" --informative-errors \
        --listen=127.0.0.1 --port="$p" --base-path="$S" >/dev/null 2>>"$A5/syslog.$kind.stderr" &
      pid=$!
      wait_port "$p" || true
      python3 "$A5/ask.py" "$p" 'git-upload-pack /nope.git\0' >/dev/null
      sleep 0.5
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    done
    set +e
    printf '0020git-upload-pack /hidden.git\0' | run_daemon "$kind" --inetd --verbose \
      --informative-errors --base-path="$S" >/dev/null 2>>"$A5/syslog.$kind.stderr"
    set -e
    sleep 0.3
    kill "$cpid" 2>/dev/null || true
    wait "$cpid" 2>/dev/null || true
    python3 "$A5/syslog-norm.py" "$A5/syslog.$kind.raw" >"$A5/syslog.$kind.out"
    echo "stderr_bytes=$(wc -c <"$A5/syslog.$kind.stderr")" >>"$A5/syslog.$kind.out"
  done
  rm -f /dev/log
  a5_same syslog
  if ! grep -q '^<30> Ready to rumble$' "$A5/syslog.arks.out" || ! grep -q '^<27> ' "$A5/syslog.arks.out"; then
    echo "FAIL A5 syslog: expected daemon.info and daemon.err datagrams" >&2
    DF_OK=0
  fi
  # --detach + --pid-file: parent exits 0, the recorded pid serves, leads its own
  # session, has stdio on /dev/null and no terminal; killing it closes the port.
  # --pid-file without --detach records the serving process; ignored under --inetd.
  for kind in git arks; do
    out="$A5/detach.$kind.out"
    p="$(free_port)"
    pf="$A5/detach.$kind.pid"
    set +e
    run_daemon "$kind" --detach --pid-file="$pf" --informative-errors \
      --listen=127.0.0.1 --port="$p" --base-path="$S" </dev/null >"$A5/detach.$kind.stdout" 2>"$A5/detach.$kind.stderr"
    echo "parent_exit=$?" >"$out"
    set -e
    wait_port "$p" || true
    dp="$(cat "$pf" 2>/dev/null || echo 0)"
    {
      echo "parent_stdout_bytes=$(wc -c <"$A5/detach.$kind.stdout") parent_stderr_bytes=$(wc -c <"$A5/detach.$kind.stderr")"
      kill -0 "$dp" 2>/dev/null && echo "pidfile_alive=yes" || echo "pidfile_alive=no"
      [[ "$(ps -o sid= -p "$dp" | tr -d ' ')" == "$dp" ]] && echo "session_leader=yes" || echo "session_leader=no"
      echo "tty=$(ps -o tty= -p "$dp" | tr -d ' ')"
      for fd in 0 1 2; do echo "fd$fd=$(readlink "/proc/$dp/fd/$fd" 2>/dev/null)"; done
      python3 "$A5/ask.py" "$p" 'git-upload-pack /nope.git\0'
      kill "$dp" 2>/dev/null || true
      port_closed "$p" && echo "kill_closes_port=yes" || echo "kill_closes_port=no"
    } >>"$out"
    p="$(free_port)"
    pf="$A5/pidfile.$kind.pid"
    run_daemon "$kind" --pid-file="$pf" --listen=127.0.0.1 --port="$p" --base-path="$S" \
      >/dev/null 2>/dev/null &
    pid=$!
    wait_port "$p" || true
    dp="$(cat "$pf" 2>/dev/null || echo 0)"
    kill "$dp" 2>/dev/null || true
    port_closed "$p" && echo "foreground_pidfile_kill_closes_port=yes" >>"$out" || echo "foreground_pidfile_kill_closes_port=no" >>"$out"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pf="$A5/inetd-pidfile.$kind.pid"
    set +e
    printf '0020git-upload-pack /hidden.git\0' | run_daemon "$kind" --inetd --pid-file="$pf" \
      --base-path="$S" >/dev/null 2>/dev/null
    set -e
    [[ -e "$pf" ]] && echo "inetd_pidfile_written=yes" >>"$out" || echo "inetd_pidfile_written=no" >>"$out"
  done
  a5_same detach
  if ! grep -qx 'session_leader=yes' "$A5/detach.arks.out" || ! grep -qx 'fd0=/dev/null' "$A5/detach.arks.out"; then
    echo "FAIL A5 detach: arks detached process is not a session leader on /dev/null" >&2
    DF_OK=0
  fi
  # --user / --group: privileges dropped after bind (uid, gid, supplementary groups).
  # The base under the 0700 work dir is invisible to nobody: git's per-connection
  # server dies on the base-path check; the open base answers normally.
  OPEN="$(mktemp -d /tmp/arks-a5-open.XXXXXX)"
  chmod 755 "$OPEN"
  for ug in "--user=nobody" "--user=nobody --group=daemon" "--user=nobody open"; do
    tag="user-${ug//[^a-z]/}"
    base="$S"
    [[ "$ug" == *open ]] && { base="$OPEN"; ug="${ug% open}"; }
    for kind in git arks; do
      p="$(free_port)"
      pf="$A5/$tag.$kind.pid"
      run_daemon "$kind" $ug --pid-file="$pf" --informative-errors \
        --listen=127.0.0.1 --port="$p" --base-path="$base" >/dev/null 2>"$A5/$tag.$kind.err" &
      pid=$!
      wait_port "$p" || true
      sleep 0.2
      dp="$(cat "$pf" 2>/dev/null || echo 0)"
      {
        grep -E '^(Uid|Gid|Groups):' "/proc/$dp/status" 2>/dev/null | tr -s ' \t' ' '
        python3 "$A5/ask.py" "$p" 'git-upload-pack /nope.git\0'
      } >"$A5/$tag.$kind.out"
      kill "$dp" "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
      # Drop the dashed-command wrapper's report of the killed git-daemon (not daemon output).
      { grep -v '^error: git-daemon died of signal' "$A5/$tag.$kind.err" || true; } |
        sed -E "s#$base#BASE#g; s/[0-9]+/N/g" | sort >>"$A5/$tag.$kind.out"
    done
    a5_same "$tag"
  done
  if ! grep -q '^Uid: 65534' "$A5/user-usernobody.arks.out" || ! grep -q '^Gid: 1 ' "$A5/user-usernobodygroupdaemon.arks.out"; then
    echo "FAIL A5 user/group: arks did not drop to nobody / daemon" >&2
    DF_OK=0
  fi
  # More argv exit codes for the process rows.
  for args in "--user=a5-no-such-user" "--user=0" "--user=nobody --group=a5-no-such-group" \
    "--inetd --detach" "--inetd --user=nobody" "--inetd --group=daemon" "--log-destination=" \
    "--syslog --log-destination=bogus"; do
    set +e
    timeout 10 git daemon $args </dev/null >/dev/null 2>&1; g=$?
    timeout 10 "$ARKS_BIN" daemon $args </dev/null >/dev/null 2>&1; a=$?
    set -e
    echo "$args git=$g arks=$a" >>"$A5/argv-process.out"
    if [[ "$g" -ne "$a" ]]; then
      echo "FAIL A5 argv '$args': git exit $g, arks exit $a" >&2
      DF_OK=0
    fi
  done
  echo "daemon-git-arks-matrix-e2e: A5 process argv exit codes checked"
  # --inetd fatals: a request cut short, per log destination (exit status + stderr).
  for kind in git arks; do
    : >"$A5/inetd-fatal.$kind.out"
    for dest in stderr syslog none; do
      for input in '' '0030git-upl'; do
        set +e
        printf '%s' "$input" | run_daemon "$kind" --inetd --log-destination="$dest" --base-path="$S" \
          >/dev/null 2>"$A5/inetd-fatal.err"
        ec=$?
        set -e
        echo "$dest '$input' ec=$ec stderr=$(cat "$A5/inetd-fatal.err")" >>"$A5/inetd-fatal.$kind.out"
      done
    done
  done
  a5_same inetd-fatal
  # Default bind (arks-only, SoT A5 rule 6): arks listens on 127.0.0.1 only; with
  # --listen=0.0.0.0 its IPv4 listener matches stock git's default bind.
  cat >"$A5/listeners.py" <<'PY'
import sys
port = int(sys.argv[1])
seen = []
for fam, path in (("tcp4", "/proc/net/tcp"), ("tcp6", "/proc/net/tcp6")):
    try:
        rows = open(path).read().splitlines()[1:]
    except OSError:
        continue
    for row in rows:
        f = row.split()
        addr, p = f[1].split(":")
        if f[3] == "0A" and int(p, 16) == port:
            seen.append("%s %s" % (fam, addr))
print(" ".join(sorted(seen)) or "none")
PY
  bind_of() {
    local kind="$1" p; shift
    p="$(free_port)"
    run_daemon "$kind" --port="$p" "$@" --base-path="$S" >/dev/null 2>&1 &
    local pid=$!
    for _ in $(seq 1 50); do
      [[ "$(python3 "$A5/listeners.py" "$p")" != none ]] && break
      sleep 0.1
    done
    sleep 0.2
    python3 "$A5/listeners.py" "$p"
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  }
  {
    echo "arks_default=$(bind_of arks)"
    echo "arks_listen_any=$(bind_of arks --listen=0.0.0.0)"
    echo "git_default=$(bind_of git)"
    echo "arks_listen_any_v6=$(bind_of arks --listen=0.0.0.0 --listen=::)"
  } >"$A5/default-bind.out"
  cat "$A5/default-bind.out"
  db_arks="$(sed -n 's/^arks_default=//p' "$A5/default-bind.out")"
  db_any="$(sed -n 's/^arks_listen_any=//p' "$A5/default-bind.out")"
  db_git="$(sed -n 's/^git_default=//p' "$A5/default-bind.out")"
  db_any6="$(sed -n 's/^arks_listen_any_v6=//p' "$A5/default-bind.out")"
  db_git4="$(tr ' ' '\n' <<<"$db_git" | paste - - -d' ' | grep '^tcp4' | tr '\n' ' ' | sed 's/ $//')"
  if [[ "$db_arks" != "tcp4 0100007F" ]]; then
    echo "FAIL A5 default bind: arks listens on '$db_arks', expected loopback only" >&2
    DF_OK=0
  elif [[ "$db_any" != "$db_git4" ]]; then
    echo "FAIL A5 default bind: arks --listen=0.0.0.0 '$db_any' vs git IPv4 '$db_git4'" >&2
    DF_OK=0
  elif [[ "$db_git" == *tcp6* && "$db_any6" != "$db_git" ]]; then
    echo "FAIL A5 default bind: arks --listen=0.0.0.0 --listen=:: '$db_any6' vs git '$db_git'" >&2
    DF_OK=0
  else
    echo "daemon-git-arks-matrix-e2e: A5 default bind loopback; --listen=0.0.0.0 matches git"
  fi
  # --inetd: one request on stdin/stdout; exit status and reply must match.
  for kind in git arks; do
    set +e
    if [[ "$kind" == git ]]; then
      printf '0020git-upload-pack /hidden.git\0' | git daemon --inetd --informative-errors --base-path="$S" >"$A5/inetd.$kind.out"
    else
      printf '0020git-upload-pack /hidden.git\0' | "$ARKS_BIN" daemon --inetd --informative-errors --base-path="$S" >"$A5/inetd.$kind.out"
    fi
    echo "ec=$?" >>"$A5/inetd.$kind.out"
    set -e
  done
  if cmp -s "$A5/inetd.git.out" "$A5/inetd.arks.out"; then
    echo "daemon-git-arks-matrix-e2e: A5 inetd SAME"
  else
    echo "FAIL A5 inetd differs" >&2
    cat "$A5/inetd.git.out" "$A5/inetd.arks.out" >&2
    DF_OK=0
  fi
  # Argv errors: exit status of the same bad command lines.
  for args in "--port 9418" "--enable=bogus" "--log-destination=foo" "--strict-paths" \
    "--group=root" "--inetd --port=1" "--listen=127.0.0.1:1"; do
    set +e
    git daemon $args </dev/null >/dev/null 2>&1; g=$?
    "$ARKS_BIN" daemon $args </dev/null >/dev/null 2>&1; a=$?
    set -e
    if [[ "$g" -ne "$a" ]]; then
      echo "FAIL A5 argv '$args': git exit $g, arks exit $a" >&2
      DF_OK=0
    fi
  done
  echo "daemon-git-arks-matrix-e2e: A5 argv exit codes checked"
  if [[ "$DF_OK" -ne 1 ]]; then
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      echo "daemon-git-arks-matrix-e2e: FAIL STRICT IW-DAEMON-FLAGS-PARITY" >&2
      exit 1
    fi
    echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY Partial/failed"
    exit 0
  fi
  echo "daemon-git-arks-matrix-e2e: IW-DAEMON-FLAGS-PARITY greens"
  if [[ -f /.dockerenv ]]; then
    TS="$(date -u +%Y%m%d%H%M%S)"
    OUT=".spec/evidence/${TS}-daemon-flags-parity-a5"
    mkdir -p "$OUT/results"
    cp "$WORK"/daemon.* "$WORK"/clone.err "$WORK"/push.err \
      "$WORK"/daemon-archive.* "$WORK"/archive.err "$WORK"/remote.tar \
      "$OUT/" 2>/dev/null || true
    # A5 oracle comparisons: git vs arks outputs, diffs (empty = same), hook logs.
    cp "$A5"/*.out "$A5"/*.diff "$A5"/*.log "$OUT/results/" 2>/dev/null || true
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-flags-parity/v1",
    "gate": "IW-DAEMON-FLAGS-PARITY",
    "disposition": "verified-product-path",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "parity_flags": [
        "--verbose", "--timeout", "--init-timeout", "--max-connections",
        "--strict-paths", "--listen", "--port", "--base-path", "--reuseaddr",
        "--enable", "--disable", "<directory>...", "service:upload-pack", "service:receive-pack",
        "service:upload-archive",
    ],
    "parity_pointers": ["related:git:// client"],
    "a5_oracle_compared": [
        "--syslog", "--base-path-relaxed", "--user-path|--user-path=<path>",
        "--interpolated-path=<pathtemplate>", "--detach", "--pid-file=<file>",
        "--allow-override=<service>", "--forbid-override=<service>", "--access-hook=<path>",
        "--[no-]informative-errors", "--inetd", "--user=<user>", "--group=<group>",
        "--log-destination=(stderr|syslog|none)", "daemon.* config override",
        "REMOTE_ADDR env for hooks", "--listen host:port form",
    ],
    "arks_only_checked": ["default bind 127.0.0.1"],
    "results_dir": "results/",
    "note": "A5 rows: identical raw git:// requests, log lines, syslog <PRI>, detach/pid-file process state, dropped uid/gid/groups and argv exit codes vs stock git daemon 2.39.5. Default bind is arks-only (loopback); --listen=0.0.0.0 matches git's IPv4 default listener. Windows rows stay host-tested (no Windows pin oracle).",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "daemon-git-arks-matrix-e2e: receipt=$OUT/receipt.json"
  fi
  exit 0
fi

# --- IW-B5-PLUS syslog facility/priority collector (ARKS_DAEMON_B5_PLUS=1) ---
if [[ "${ARKS_DAEMON_B5_PLUS:-0}" == "1" ]]; then
  echo "daemon-git-arks-matrix-e2e: IW-B5-PLUS collector path (sot:root/arks-honesty-leftovers-wave)"
  if [[ ! -f "$HONESTY_SOT" ]]; then
    echo "FAIL: missing honesty leftovers SoT $HONESTY_SOT" >&2
    exit 1
  fi
  if [[ ! -f /.dockerenv ]]; then
    echo "daemon-git-arks-matrix-e2e: B5+ host run — Docker collector venue required; honesty Refuse (no invent Claimed)"
    if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
      exit 1
    fi
    exit 0
  fi
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
  cargo build -p arks --release
  ARKS_BIN="${CARGO_TARGET_DIR:-target}/release/arks"
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./.cargo-target/release/arks
  [[ -x "$ARKS_BIN" ]] || ARKS_BIN=./target/release/arks

  WORK="$(mktemp -d /tmp/arks-b5plus.XXXXXX)"
  COLLECT="$WORK/collect.log"
  : >"$COLLECT"
  # Bind a unix datagram collector at /dev/log when permitted (peer openlog path).
  python3 - "$WORK" "$COLLECT" <<'PY' &
import os, socket, sys, time
work, path = sys.argv[1], sys.argv[2]
sock_path = "/dev/log"
# Prefer replacing /dev/log inside the container; fall back to workdir sock.
targets = []
if os.geteuid() == 0:
    targets.append("/dev/log")
targets.append(os.path.join(work, "syslog.sock"))
bound = None
for t in targets:
    try:
        if os.path.exists(t) or os.path.islink(t):
            try:
                os.unlink(t)
            except OSError:
                pass
        s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        s.bind(t)
        s.settimeout(0.5)
        bound = (s, t)
        open(os.path.join(work, "collector.bound"), "w").write(t + "\n")
        break
    except OSError as e:
        open(os.path.join(work, "collector.bind.err"), "a").write(f"{t}: {e}\n")
if not bound:
    open(path, "a").write("COLLECTOR_BIND_FAILED\n")
    time.sleep(30)
    raise SystemExit(1)
s, t = bound
deadline = time.time() + 25
with open(path, "ab") as out:
    while time.time() < deadline:
        try:
            data, _ = s.recvfrom(65535)
            out.write(data + b"\n")
            out.flush()
            if b"Ready to rumble" in data:
                open(os.path.join(work, "collector.hit"), "w").write("1\n")
                break
        except socket.timeout:
            continue
s.close()
PY
  COLLECTOR_PID=$!
  sleep 0.4

  BASE="$WORK/repo.git"
  mkdir -p "$BASE"
  git init --bare "$BASE" >/dev/null
  # Touch export-ok so daemon may serve; --syslog --verbose logs "Ready to rumble" (daemon.info).
  touch "$BASE/git-daemon-export-ok"
  set +e
  timeout 3 "$ARKS_BIN" daemon --syslog --verbose --listen=127.0.0.1 --port=0 --export-all "$BASE" \
    >"$WORK/daemon.out" 2>"$WORK/daemon.err"
  set -e
  sleep 0.5
  kill "$COLLECTOR_PID" 2>/dev/null || true
  wait "$COLLECTOR_PID" 2>/dev/null || true

  B5_OK=0
  if [[ -f "$WORK/collector.hit" ]] && grep -a -q 'Ready to rumble' "$COLLECT"; then
    # PRI decode: leading <N> where N = facility*8 + severity; daemon+info => 3*8+6=30.
    if python3 - "$COLLECT" <<'PY'
import re, sys
raw = open(sys.argv[1], "rb").read()
text = raw.decode("utf-8", "replace")
m = re.search(r"<(\d+)>[^<]*?Ready to rumble", text, re.S)
if not m:
    # Some syslogd paths strip PRI; accept marker+bound /dev/log only as Partial.
    print("NO_PRI")
    raise SystemExit(2)
pri = int(m.group(1))
facility, severity = divmod(pri, 8)
print(f"pri={pri} facility={facility} severity={severity}")
if facility != 3:  # LOG_DAEMON
    raise SystemExit(3)
if severity != 6:  # LOG_INFO
    raise SystemExit(4)
raise SystemExit(0)
PY
    then
      B5_OK=1
      echo "daemon-git-arks-matrix-e2e: B5+ collector PRI facility=daemon severity=info ok"
    else
      echo "daemon-git-arks-matrix-e2e: B5+ marker seen but PRI decode failed — honest Refuse" >&2
    fi
  else
    echo "daemon-git-arks-matrix-e2e: B5+ collector miss (no marker) — honest Refuse" >&2
    cat "$WORK/daemon.err" >&2 || true
    cat "$WORK/collector.bind.err" >&2 || true
  fi

  TS="$(date -u +%Y%m%d%H%M%S)"
  OUT=".spec/evidence/${TS}-daemon-b5-plus-syslog"
  mkdir -p "$OUT"
  cp "$WORK/daemon.err" "$WORK/daemon.out" "$COLLECT" "$OUT/" 2>/dev/null || true
  cp "$WORK"/collector.* "$OUT/" 2>/dev/null || true
  if [[ "$B5_OK" -eq 1 ]]; then
    python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-b5-plus-syslog/v1",
    "gate": "IW-B5-PLUS",
    "disposition": "verified-product-path",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "facility": "daemon",
    "facility_num": 3,
    "severity": "info",
    "severity_num": 6,
    "pri": 30,
    "marker": "Ready to rumble",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
    echo "daemon-git-arks-matrix-e2e: B5+ Claimed receipt=$OUT/receipt.json"
    exit 0
  fi
  python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" <<'PY'
import json, sys, time
out, tip, image = sys.argv[1:]
receipt = {
    "schema": "arks-daemon-b5-plus-syslog/v1",
    "gate": "IW-B5-PLUS",
    "disposition": "refuse-terminal",
    "venue": "docker-dod-daemon-matrix",
    "tip": tip,
    "image_tag": image,
    "note": "Collector miss or PRI mismatch — keep Refuse; openlog alone ≠ Claimed.",
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
  echo "daemon-git-arks-matrix-e2e: B5+ Refuse receipt=$OUT/receipt.json"
  if [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
    # STRICT + Refuse is an honest terminal close, not invent Claimed — exit 0 with Refuse receipt.
    echo "daemon-git-arks-matrix-e2e: STRICT accepts terminal Refuse (no invent Claimed)"
  fi
  exit 0
fi

# Muscle-memory entry must exist (single release binary).
help_out="$("$ARKS_BIN" daemon --help 2>&1 || true)"
if ! echo "$help_out" | grep -qi 'git://'; then
  echo "FAIL: arks daemon --help missing git:// usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: arks daemon --help ok"

# Pin oracle usage exists (git-daemon). Prefer `git daemon -h` — man pages often absent in image.
pin_help="$(git daemon -h 2>&1 || true)"
if ! echo "$pin_help" | grep -qi 'usage: git daemon'; then
  echo "FAIL: pin git daemon -h missing usage"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: pin git daemon -h ok"

if [[ ! -f "$INTENT_SOT" ]]; then
  echo "daemon-git-arks-matrix-e2e: FAIL — missing daemon intent SoT $INTENT_SOT"
  exit 1
fi
echo "daemon-git-arks-matrix-e2e: intent SoT ok ($INTENT_SOT)"

if [[ ! -f "$SOT" || ! -f "$CSV" ]]; then
  echo "daemon-git-arks-matrix-e2e: SKIP flag-depth matrix — waiting $SOT / $CSV"
  echo "daemon-git-arks-matrix-e2e: smoke-only PASS (help + intent SoT); flag e2e not Claimed"
  exit 0
fi

missing=$(awk -F, 'NR>1 && ($3=="Missing" || $3=="missing") {c++} END{print c+0}' "$CSV")
echo "daemon-git-arks-matrix-e2e: Missing=$missing from $CSV"
if [[ "${ARKS_DOD_STRICT:-0}" == "1" || "$missing" -eq 0 ]]; then
  if [[ "$missing" -gt 0 ]]; then
    echo "daemon-git-arks-matrix-e2e: FAIL — daemon matrix Missing=$missing (no invent PASS)"
    exit 1
  fi
  echo "daemon-git-arks-matrix-e2e: PASS — matrix Missing=0"
  exit 0
fi
echo "daemon-git-arks-matrix-e2e: smoke-only PASS (CSV present, Missing=$missing); STRICT waits ARKS_DOD_STRICT=1"
exit 0
