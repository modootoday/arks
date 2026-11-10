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
  "g0","g1","g2","g3","g4","g5","g6","g7","g8","g8-live-scale","g8-local-forge",
  "fs007-host-advisory","g10","g11","g11l3","g12","g13","g14","g15","g16","g17","g18",
  "g19","g20","g21","g22","g23","g24","g25","g26","g27","g28","g28-offline-clones",
  "g29","g30","g31","g32","g33","g34","g35","g36","g37","g38","g39","g40","g41"
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
  Write-Result "g8" "SKIP" 0 0 "secrets-absent (token/URL); ADR 20261110220000; G7 offline remains"
  Write-Result "g8-live-scale" "SKIP" 0 0 "secrets-absent; ADR 20261110221000 harness Claimed; no live latency PASS"
} else {
  Invoke-Gate "g8"
  Invoke-Gate "g8-live-scale"
}

if (-not $hasG28) {
  Write-Result "g28" "SKIP" 0 0 "secrets/opt-in absent (ARKS_G28_LIVE=1 + token + URL); offline clones still run"
} else {
  Invoke-Gate "g28"
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

# --- L8 suite (g14 g16 g20 g21) — PowerShell host; scripts run inside containers ---
Write-Host "===== L8 suite ====="
$l8Sw = [Diagnostics.Stopwatch]::StartNew()
foreach ($g in @("g14","g16","g20","g21")) {
  Invoke-Gate $g @("--no-deps")
}
$l8Sw.Stop()
Write-Result "l8-suite" "PASS" 0 ([int]$l8Sw.Elapsed.TotalSeconds) "g14 g16 g20 g21 sequential"

# --- S6 suite (g38 g39 g40) ---
Write-Host "===== S6 suite ====="
$s6Sw = [Diagnostics.Stopwatch]::StartNew()
foreach ($g in @("g38","g39","g40")) {
  Invoke-Gate $g @("--no-deps")
}
$s6Sw.Stop()
Write-Result "s6-suite" "PASS" 0 ([int]$s6Sw.Elapsed.TotalSeconds) "g38 g39 g40 sequential"

# --- Remaining gates (not covered by L8/S6/live SKIP) ---
$Rest = @(
  "g0","g1","g2","g3","g4","g5","g6","g7","g10","g12","g13","g11","g11l3","g15",
  "g17","g18","g19","g22","g23","g24","g25","g26","g27","g28-offline-clones",
  "g29","g30","g31","g32","g33","g34","g35","g36","g37",
  "fs007-host-advisory","g8-local-forge"
)
foreach ($g in $Rest) { Invoke-Gate $g @("--no-deps") }

# G41 needs seed+dav deps
Invoke-Gate "g41"

Write-Host "FULL MATRIX COMPLETE tip=$Tip evidence=$EvidenceDir"
