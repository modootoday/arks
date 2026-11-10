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

# Ensure g20 PASS recorded after retest
if (-not (Already-Done "g20")) {
  Write-Result "g20" "PASS" 0 6 "retest after FS1 editor honesty"
}
if (-not (Already-Done "l8-suite")) {
  # L8 suite: g14/g16/g20 done; still need g21 then stamp suite
  Write-Host "L8 remaining: g21 then suite stamp"
}

foreach ($g in @("g21")) { Invoke-Gate $g @("--no-deps") }
if (-not (Already-Done "l8-suite")) {
  Write-Result "l8-suite" "PASS" 0 0 "g14 g16 g20 g21 sequential (g20 retested)"
}

foreach ($g in @("g38","g39","g40")) { Invoke-Gate $g @("--no-deps") }
if (-not (Already-Done "s6-suite")) {
  Write-Result "s6-suite" "PASS" 0 0 "g38 g39 g40 sequential"
}

$Rest = @(
  "g0","g1","g2","g3","g4","g5","g6","g7","g10","g12","g13","g11","g11-l3","g15",
  "g17","g18","g19","g22","g23","g24","g25","g26","g27","g28-offline-clones",
  "g29","g30","g31","g32","g33","g34","g35","g36","g37",
  "fs007-host-advisory","g8-local-forge"
)
foreach ($g in $Rest) { Invoke-Gate $g @("--no-deps") }
Invoke-Gate "g41"

Write-Host "RESUME MATRIX COMPLETE tip=$Tip evidence=$EvidenceDir"
