# Full clean-room matrix — sequential Docker only. Halt on OOM/stampede.
# Never invent PASS. Secrets-absent live gates recorded as SKIP.
# Usage (from repo root): powershell -File verify/scripts/run-full-cleanroom-matrix.ps1
# Continue: docker compose writes progress to stderr; Stop would abort falsely.
$ErrorActionPreference = "Continue"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Set-Location $Root

# Reuse ARKS_EVIDENCE_DIR when parent pre-created the pack; else stamp a new one.
if ($env:ARKS_EVIDENCE_DIR -and (Test-Path $env:ARKS_EVIDENCE_DIR)) {
  $EvidenceDir = $env:ARKS_EVIDENCE_DIR
  $Stamp = Split-Path $EvidenceDir -Leaf
} else {
  $Stamp = Get-Date -Format "yyyyMMddHHmmss"
  $EvidenceDir = Join-Path $Root ".spec/evidence/${Stamp}-full-cleanroom-matrix"
}
New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$ResultsCsv = Join-Path $EvidenceDir "results.csv"
$LogPath = Join-Path $EvidenceDir "runner.log"
"gate,status,exit_code,seconds,note" | Set-Content -Encoding utf8 $ResultsCsv

function Compose { param([Parameter(ValueFromRemainingArguments=$true)]$Args) & docker compose -f verify/docker-compose.yml @Args }
$Tip = (git rev-parse HEAD).Trim()

$OutGates = @(
  "g000-workspace-lib-tests","g001-loose-object-interop","g002-refs-index-roundtrip","g003-index-commit-roundtrip","g004-pack-idx-roundtrip","g005-protocol-offline-fixtures","g006-cli-wave-l","g007-smart-http-client","g008-live-forge-smoke","g008-live-scale","g008-local-forge",
  "fs007-host-advisory","g010-store-facade","g011-lease-product","g011-high-concurrency","g012-porcelain-wave-pa","g013-porcelain-wave-pb","g014-cli-parity-production","g015-smart-http","g016-journey-compare","g017-concurrency-issue-compare","g018-concurrency-issue-compare",
  "g019-comparative-performance","g020-cli-symmetric-parity","g021-cli-flag-symmetry","g022-celir-index-contention","g023-celir-dual-path","g024-l3-celir-joint","g025-celir-index-scale","g026-local-branch-pr-sim","g027-smart-http-pr-contention","g028-live-private-arks","g028-offline-clones",
  "g029-server-dumb-http","g030-server-ssh-command","g031-server-protocol-v2","g032-git-daemon-wire","g033-cred-askpass","g034-shared-index-contention","g035-shallow-depth","g036-http-basic-auth","g037-gpgsig-ssh","g038-gpgsig-openpgp","g039-partial-clone-filter","g040-dumb-http-push-deletes","g041-apache-dav-interop"
)
foreach ($g in $OutGates) {
  New-Item -ItemType Directory -Force -Path (Join-Path $Root "verify/gates/$g/out") | Out-Null
}

function Write-Result($gate, $status, $ec, $seconds, $note) {
  $safeNote = ($note -replace '"', "'")
  $line = "$gate,$status,$ec,$seconds,`"$safeNote`""
  Add-Content -Encoding utf8 $ResultsCsv $line
  $msg = "$(Get-Date -Format o) $line"
  Add-Content -Encoding utf8 $LogPath $msg
  Write-Host $msg
}

function Invoke-Gate {
  param([string]$gate, [string[]]$extraArgs = @())
  Write-Host "===== starting $gate ====="
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $logFile = Join-Path $EvidenceDir "$gate.log"
  $prev = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  & docker compose -f verify/docker-compose.yml run --rm @extraArgs $gate *>&1 |
    ForEach-Object { "$_" } |
    Tee-Object -FilePath $logFile
  $ec = $LASTEXITCODE
  $ErrorActionPreference = $prev
  $sw.Stop()
  $secs = [int]$sw.Elapsed.TotalSeconds
  if ($ec -eq 0) {
    Write-Result $gate "PASS" $ec $secs ""
  } elseif ($ec -eq 137) {
    Write-Result $gate "FAIL" $ec $secs "OOM/SIGKILL; halt no stampede"
    throw "Halt on OOM for $gate exit=$ec"
  } else {
    Write-Result $gate "FAIL" $ec $secs ""
    throw "Halt on fail for $gate exit=$ec"
  }
}

@"
tip=$Tip
stamp=$Stamp
evidence=$EvidenceDir
policy=sequential Docker only; halt OOM/fail; no invent PASS
"@ | Set-Content -Encoding utf8 (Join-Path $EvidenceDir "meta.txt")

# --- Secrets-blocked live (honest SKIP) ---
$hasG8 = [bool]($env:ARKS_G8_TOKEN -or $env:GITHUB_TOKEN)
$hasG8Url = [bool]$env:ARKS_G8_REPO_URL
$hasG28 = ($env:ARKS_G28_LIVE -eq "1") -and ($env:ARKS_G28_TOKEN -or $env:GITHUB_TOKEN) -and [bool]$env:ARKS_G28_REPO_URL

if (-not ($hasG8 -and $hasG8Url)) {
  Write-Result "g008-live-forge-smoke" "SKIP" 0 0 "secrets-absent (token/URL); ADR 20261110220000; G7 offline remains"
  Write-Result "g008-live-scale" "SKIP" 0 0 "secrets-absent; ADR 20261110221000 harness Claimed; no live latency PASS"
} else {
  Invoke-Gate "g008-live-forge-smoke"
  Invoke-Gate "g008-live-scale"
}

if (-not $hasG28) {
  Write-Result "g028-live-private-arks" "SKIP" 0 0 "secrets/opt-in absent (ARKS_G28_LIVE=1 + token + URL); offline clones still run"
} else {
  Invoke-Gate "g028-live-private-arks"
}

Write-Host "===== ensure toolchain image ====="
$prev = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& docker compose -f verify/docker-compose.yml build toolchain *>&1 |
  ForEach-Object { "$_" } |
  Tee-Object -FilePath (Join-Path $EvidenceDir "toolchain-build.log")
$buildEc = $LASTEXITCODE
$ErrorActionPreference = $prev
if ($buildEc -ne 0) { throw "toolchain build failed exit=$buildEc" }

# --- L8 suite (g014-cli-parity-production g016-journey-compare g020-cli-symmetric-parity g021-cli-flag-symmetry) — PowerShell host; scripts run inside containers ---
Write-Host "===== L8 suite ====="
$l8Sw = [Diagnostics.Stopwatch]::StartNew()
foreach ($g in @("g014-cli-parity-production","g016-journey-compare","g020-cli-symmetric-parity","g021-cli-flag-symmetry")) {
  Invoke-Gate $g @("--no-deps")
}
$l8Sw.Stop()
Write-Result "l8-suite" "PASS" 0 ([int]$l8Sw.Elapsed.TotalSeconds) "g014-cli-parity-production g016-journey-compare g020-cli-symmetric-parity g021-cli-flag-symmetry sequential"

# --- S6 suite (g038-gpgsig-openpgp g039-partial-clone-filter g040-dumb-http-push-deletes) ---
Write-Host "===== S6 suite ====="
$s6Sw = [Diagnostics.Stopwatch]::StartNew()
foreach ($g in @("g038-gpgsig-openpgp","g039-partial-clone-filter","g040-dumb-http-push-deletes")) {
  Invoke-Gate $g @("--no-deps")
}
$s6Sw.Stop()
Write-Result "s6-suite" "PASS" 0 ([int]$s6Sw.Elapsed.TotalSeconds) "g038-gpgsig-openpgp g039-partial-clone-filter g040-dumb-http-push-deletes sequential"

# --- Remaining gates (not covered by L8/S6/live SKIP) ---
$Rest = @(
  "g000-workspace-lib-tests","g001-loose-object-interop","g002-refs-index-roundtrip","g003-index-commit-roundtrip","g004-pack-idx-roundtrip","g005-protocol-offline-fixtures","g006-cli-wave-l","g007-smart-http-client","g010-store-facade","g012-porcelain-wave-pa","g013-porcelain-wave-pb","g011-lease-product","g011-high-concurrency","g015-smart-http",
  "g017-concurrency-issue-compare","g018-concurrency-issue-compare","g019-comparative-performance","g022-celir-index-contention","g023-celir-dual-path","g024-l3-celir-joint","g025-celir-index-scale","g026-local-branch-pr-sim","g027-smart-http-pr-contention","g028-offline-clones",
  "g029-server-dumb-http","g030-server-ssh-command","g031-server-protocol-v2","g032-git-daemon-wire","g033-cred-askpass","g034-shared-index-contention","g035-shallow-depth","g036-http-basic-auth","g037-gpgsig-ssh",
  "fs007-host-advisory","g008-local-forge"
)
foreach ($g in $Rest) { Invoke-Gate $g @("--no-deps") }

# G41 needs seed+dav deps
Invoke-Gate "g041-apache-dav-interop"

Write-Host "FULL MATRIX COMPLETE tip=$Tip evidence=$EvidenceDir"
