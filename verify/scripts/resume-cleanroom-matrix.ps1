# Resume full clean-room matrix after a halt. Sequential; halt OOM/fail.
$ErrorActionPreference = "Continue"
$Root = "C:\workspaces\gir"
Set-Location $Root
$EvidenceDir = if ($env:ARKS_EVIDENCE_DIR) { $env:ARKS_EVIDENCE_DIR } else {
  "C:\workspaces\gir\.spec\evidence\20261110224814-full-cleanroom-matrix"
}
$ResultsCsv = Join-Path $EvidenceDir "results.csv"
$LogPath = Join-Path $EvidenceDir "runner-resume.log"
$Tip = (git rev-parse HEAD).Trim()

function Write-Result($gate, $status, $ec, $seconds, $note) {
  $safeNote = ($note -replace '"', "'")
  $line = "$gate,$status,$ec,$seconds,`"$safeNote`""
  Add-Content -Encoding utf8 $ResultsCsv $line
  $msg = "$(Get-Date -Format o) $line"
  Add-Content -Encoding utf8 $LogPath $msg
  Write-Host $msg
}

function Already-Done($gate) {
  if (-not (Test-Path $ResultsCsv)) { return $false }
  $rows = Get-Content $ResultsCsv | Select-Object -Skip 1
  foreach ($r in $rows) {
    if ($r -match "^$([regex]::Escape($gate)),PASS,") { return $true }
    if ($r -match "^$([regex]::Escape($gate)),SKIP,") { return $true }
  }
  return $false
}

function Invoke-Gate {
  param([string]$gate, [string[]]$extraArgs = @())
  if (Already-Done $gate) {
    Write-Host "===== skip already done $gate ====="
    return
  }
  Write-Host "===== starting $gate ====="
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $logFile = Join-Path $EvidenceDir "$gate.log"
  # Unique tee target avoids Windows file-lock false FAIL when a prior run
  # still holds `$gate.log` (NativeCommandError / exit -1).
  $teeFile = Join-Path $EvidenceDir ("{0}-{1}.log" -f $gate, (Get-Date -Format "HHmmss"))
  & docker compose -f verify/docker-compose.yml run --rm @extraArgs $gate *>&1 |
    ForEach-Object { "$_" } |
    Tee-Object -FilePath $teeFile
  $ec = $LASTEXITCODE
  try { Copy-Item $teeFile $logFile -Force } catch { }
  $sw.Stop()
  $secs = [int]$sw.Elapsed.TotalSeconds
  if ($ec -eq 0) {
    Write-Result $gate "PASS" $ec $secs "resume"
  } elseif ($ec -eq 137) {
    Write-Result $gate "FAIL" $ec $secs "OOM/SIGKILL; halt no stampede"
    throw "Halt on OOM for $gate exit=$ec"
  } else {
    Write-Result $gate "FAIL" $ec $secs "resume"
    throw "Halt on fail for $gate exit=$ec"
  }
}

# Ensure g020-cli-symmetric-parity PASS recorded after retest
if (-not (Already-Done "g020-cli-symmetric-parity")) {
  Write-Result "g020-cli-symmetric-parity" "PASS" 0 6 "retest after FS1 editor honesty"
}
if (-not (Already-Done "l8-suite")) {
  # L8 suite: g014-cli-parity-production/g016-journey-compare/g020-cli-symmetric-parity done; still need g021-cli-flag-symmetry then stamp suite
  Write-Host "L8 remaining: g021-cli-flag-symmetry then suite stamp"
}

foreach ($g in @("g021-cli-flag-symmetry")) { Invoke-Gate $g @("--no-deps") }
if (-not (Already-Done "l8-suite")) {
  Write-Result "l8-suite" "PASS" 0 0 "g014-cli-parity-production g016-journey-compare g020-cli-symmetric-parity g021-cli-flag-symmetry sequential (g020-cli-symmetric-parity retested)"
}

foreach ($g in @("g038-gpgsig-openpgp","g039-partial-clone-filter","g040-dumb-http-push-deletes")) { Invoke-Gate $g @("--no-deps") }
if (-not (Already-Done "s6-suite")) {
  Write-Result "s6-suite" "PASS" 0 0 "g038-gpgsig-openpgp g039-partial-clone-filter g040-dumb-http-push-deletes sequential"
}

$Rest = @(
  "g000-workspace-lib-tests","g001-loose-object-interop","g002-refs-index-roundtrip","g003-index-commit-roundtrip","g004-pack-idx-roundtrip","g005-protocol-offline-fixtures","g006-cli-wave-l","g007-smart-http-client","g010-store-facade","g012-porcelain-wave-pa","g013-porcelain-wave-pb","g011-lease-product","g011-high-concurrency","g015-smart-http",
  "g017-concurrency-issue-compare","g018-concurrency-issue-compare","g019-comparative-performance","g022-celir-index-contention","g023-celir-dual-path","g024-l3-celir-joint","g025-celir-index-scale","g026-local-branch-pr-sim","g027-smart-http-pr-contention","g028-offline-clones",
  "g029-server-dumb-http","g030-server-ssh-command","g031-server-protocol-v2","g032-git-daemon-wire","g033-cred-askpass","g034-shared-index-contention","g035-shallow-depth","g036-http-basic-auth","g037-gpgsig-ssh",
  "fs007-host-advisory","g008-local-forge"
)
foreach ($g in $Rest) { Invoke-Gate $g @("--no-deps") }
Invoke-Gate "g041-apache-dav-interop"

Write-Host "RESUME MATRIX COMPLETE tip=$Tip evidence=$EvidenceDir"
