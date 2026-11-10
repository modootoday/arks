#!/usr/bin/env bash
# Claimed residual gates B1/B2 — OpenSSH-exact cipher + SSH git e2e.
# SoT: .spec/sot/arks-claimed-residual-gates.sot.md
# Normative venue: Linux Docker only. Host PowerShell SSH probes are NOT DoD.
# Product MUST NOT spawn ssh/git/gh; stock OpenSSH/git are oracles only.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

SOT="${ARKS_CLAIMED_RESIDUAL_SOT:-.spec/sot/arks-claimed-residual-gates.sot.md}"
TIP="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "claimed-ssh-e2e: tip=${TIP}"
echo "claimed-ssh-e2e: pin git=$(git --version 2>/dev/null || echo unknown)"
echo "claimed-ssh-e2e: openssh=$(ssh -V 2>&1 || echo unknown)"

if [[ ! -f "$SOT" ]]; then
  echo "claimed-ssh-e2e: missing SoT $SOT" >&2
  exit 1
fi
if [[ ! -f /.dockerenv ]]; then
  echo "claimed-ssh-e2e: refuse host venue — run via dod-claimed-ssh only" >&2
  exit 1
fi

command -v ssh >/dev/null
command -v ssh-keygen >/dev/null
command -v python3 >/dev/null

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
cargo build -p arks --release
ARKS_BIN="${ARKS_BIN:-${CARGO_TARGET_DIR:-target}/release/arks}"
[[ -x "$ARKS_BIN" ]] || {
  echo "claimed-ssh-e2e: arks binary missing: $ARKS_BIN" >&2
  exit 1
}

WORK="$(mktemp -d /tmp/arks-b1.XXXXXX)"
SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

# Ephemeral HOME so serve.conf does not touch the host operator profile.
export HOME="$WORK/home"
mkdir -p "$HOME" "$WORK/ssh"

# Host key + client key (oracle ssh-keygen; not spawned by product).
ssh-keygen -t ed25519 -N '' -f "$WORK/ssh/host_key" -C arks-b1-host >/dev/null
ssh-keygen -t ed25519 -N '' -f "$WORK/ssh/client" -C arks-b1-client >/dev/null
cat "$WORK/ssh/client.pub" >"$WORK/ssh/authorized_keys"

SEED="$(mktemp -d /tmp/arks-b1-seed.XXXXXX)"
git -C "$SEED" init -q
git -C "$SEED" config user.name b1
git -C "$SEED" config user.email b1@example.invalid
printf 'b1 ssh\n' >"$SEED/README"
git -C "$SEED" add README
git -C "$SEED" commit -q -m b1
git clone --bare -q "$SEED" "$WORK/repo.git"
rm -rf "$SEED"

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
LISTEN="127.0.0.1:${PORT}"

"$ARKS_BIN" serve --no-http --no-forge \
  --ssh-listen "$LISTEN" \
  --ssh-host-key "$WORK/ssh/host_key" \
  --authorized-keys "$WORK/ssh/authorized_keys" \
  "$WORK/repo.git" >"$WORK/server.out" 2>"$WORK/server.err" &
SERVER_PID=$!

python3 - "$PORT" <<'PY'
import socket, sys, time
port = int(sys.argv[1])
for _ in range(200):
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=.2):
            raise SystemExit(0)
    except OSError:
        time.sleep(.05)
raise SystemExit("B1 SSH listener not ready")
PY

# B1: stock OpenSSH client completes KEX + AEAD (verbose cipher negotiation).
# BatchMode + accept-new host key; publickey auth may still fail closed → not B2 yet.
set +e
ssh -vvv \
  -o BatchMode=yes \
  -o StrictHostKeyChecking=accept-new \
  -o UserKnownHostsFile="$WORK/ssh/known_hosts" \
  -o IdentitiesOnly=yes \
  -i "$WORK/ssh/client" \
  -p "$PORT" \
  "git@127.0.0.1" true >"$WORK/ssh-client.out" 2>"$WORK/ssh-client.err"
SSH_RC=$?
set -e

B1_GREEN=0
if grep -E 'kex: algorithm: curve25519-sha256' "$WORK/ssh-client.err" >/dev/null \
  && grep -E 'cipher: chacha20-poly1305@openssh.com' "$WORK/ssh-client.err" >/dev/null \
  && grep -E 'SSH2_MSG_NEWKEYS|NEWKEYS sent' "$WORK/ssh-client.err" >/dev/null \
  && grep -E 'wire_SERVICE_ACCEPT=ssh-userauth|OpenSSH ChaCha decrypt\+encrypt ok|openssh_chacha=true' "$WORK/server.err" >/dev/null; then
  B1_GREEN=1
  echo "claimed-ssh-e2e: B1 OpenSSH ChaCha KEX/NEWKEYS/SERVICE_ACCEPT observed"
else
  echo "claimed-ssh-e2e: B1 incomplete (ssh_rc=$SSH_RC)" >&2
  tail -n 80 "$WORK/ssh-client.err" >&2 || true
  tail -n 40 "$WORK/server.err" >&2 || true
fi

# B2: stock git over ssh:// — only green with full auth+channel pack path.
B2_GREEN=0
set +e
GIT_SSH_COMMAND="ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=$WORK/ssh/known_hosts -o IdentitiesOnly=yes -i $WORK/ssh/client -p $PORT" \
  git clone "ssh://git@127.0.0.1:${PORT}$WORK/repo.git" "$WORK/clone" \
  >"$WORK/git-clone.out" 2>"$WORK/git-clone.err"
CLONE_RC=$?
set -e
if [[ "$CLONE_RC" -eq 0 ]] && [[ -f "$WORK/clone/README" ]]; then
  B2_GREEN=1
  echo "claimed-ssh-e2e: B2 git clone over ssh:// green"
else
  echo "claimed-ssh-e2e: B2 still Missing (clone_rc=$CLONE_RC; auth/channel pack incomplete)"
fi

if [[ "$B1_GREEN" -ne 1 ]]; then
  exit 1
fi

TS="$(date -u +%Y%m%d%H%M%S)"
OUT=".spec/evidence/${TS}-claimed-ssh"
mkdir -p "$OUT/logs"
cp "$WORK/server.err" "$OUT/logs/server.err"
cp "$WORK/ssh-client.err" "$OUT/logs/ssh-client.err"
[[ -f "$WORK/git-clone.err" ]] && cp "$WORK/git-clone.err" "$OUT/logs/git-clone.err"
[[ -f "$WORK/git-clone.out" ]] && cp "$WORK/git-clone.out" "$OUT/logs/git-clone.out"
if [[ "$B2_GREEN" -eq 1 ]] && [[ -f "$WORK/clone/README" ]]; then
  cp "$WORK/clone/README" "$OUT/clone-README"
fi
python3 - "$OUT/receipt.json" "$TIP" "${ARKS_JOURNEY_IMAGE_TAG:-arks-verify-m1:0.1.0}" "$B1_GREEN" "$B2_GREEN" "$CLONE_RC" <<'PY'
import json, sys, time
out, tip, image, b1, b2, clone_rc = sys.argv[1:]
b2_claimed = b2 == "1"
checks = [
    "stock ssh -vvv negotiates ChaCha",
    "NEWKEYS observed",
    "server SERVICE_ACCEPT after AEAD",
]
if b2_claimed:
    checks.extend([
        "stock git clone ssh:// exit 0",
        "cloned README present",
    ])
receipt = {
    "schema": "arks-claimed-residual/v1",
    "gate": "B1+B2" if b2_claimed else "B1",
    "b2": "Claimed" if b2_claimed else "Missing",
    "disposition": (
        "verified-openssh-chacha-and-git-clone"
        if b2_claimed
        else ("verified-openssh-chacha-wire" if b1 == "1" else "fail")
    ),
    "venue": "docker-dod-claimed-ssh",
    "tip": tip,
    "image_tag": image,
    "cipher": "chacha20-poly1305@openssh.com",
    "kex": "curve25519-sha256",
    "clone_rc": int(clone_rc),
    "checks": checks,
    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "note": "Host PowerShell OpenSSH is not normative DoD. B2 Claimed only when git clone greens with archived README.",
}
with open(out, "w", encoding="utf-8") as f:
    json.dump(receipt, f, indent=2)
    f.write("\n")
PY
cat >"$OUT/index.evidence.md" <<EOF
---
id: evidence:root/${TS}-claimed-ssh
title: B1 OpenSSH ChaCha wire + B2 SSH git clone
status: active
created: $(date -u +%Y-%m-%dT%H:%M:%SZ)
updated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
tags: [b1, b2, ssh, openssh, git-clone, docker, clean-room]
verify: docker
rel:
  depends_on:
    - sot:root/arks-claimed-residual-gates
    - sot:root/arks-self-host-server
---

# Evidence: B1 OpenSSH ChaCha + B2 git clone ssh://

- **Gates:** B1 Claimed; B2 $([ "$B2_GREEN" -eq 1 ] && echo Claimed || echo Missing)
- **Venue:** \`docker compose -f verify/docker-compose.yml run --rm dod-claimed-ssh\`
- **Image:** \`arks-verify-m1:0.1.0\`
- **Tip on receipt:** \`${TIP}\`
- **B1:** stock OpenSSH ChaCha KEX / NEWKEYS / SERVICE_ACCEPT (\`logs/ssh-client.err\`, \`logs/server.err\`)
- **B2:** stock \`git clone ssh://\` clone_rc=${CLONE_RC}; $([ "$B2_GREEN" -eq 1 ] && echo "archived \`clone-README\`" || echo "README absent / clone failed — no invent Claimed")

Artifacts: \`receipt.json\`, \`logs/\`$([ "$B2_GREEN" -eq 1 ] && echo ", \`clone-README\`")
EOF
echo "claimed-ssh-e2e: receipt=$OUT/receipt.json"

# Do not invent B2 Claimed from B1 alone.
if [[ "$B2_GREEN" -ne 1 ]] && [[ "${ARKS_DOD_STRICT:-0}" == "1" ]]; then
  echo "claimed-ssh-e2e: STRICT — B2 SSH git e2e not Claimed" >&2
  exit 1
fi
echo "claimed-ssh-e2e: B1 evidence written; B2=$([ "$B2_GREEN" -eq 1 ] && echo Claimed || echo Missing)"
