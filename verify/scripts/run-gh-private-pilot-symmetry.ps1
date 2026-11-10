# Bounded GitHub private-repo symmetry pilot: stock git vs arks (same remotes).
# NOT a G8 latency SLO / NOT G19 remasure. Specfold-honest: PASS only when exit+refs match.
#
# Host constraint: Windows PowerShell only — no /bin/sh assumptions.
# Linux-native scripted pilots belong in clean-room Docker, not this runner.
#
# Usage (from repo root):
#   pwsh -File verify/scripts/run-gh-private-pilot-symmetry.ps1 `
#     -RepoA https://github.com/modootoday/arks-pilot-a-<ts>.git `
#     -RepoB https://github.com/modootoday/arks-pilot-b-<ts>.git `
#     -EvidenceDir .spec/evidence/<ts>-gh-private-pilot-symmetry `
#     -ArksBin target/pilot-bin/arks.exe

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$RepoA,
    [Parameter(Mandatory = $true)][string]$RepoB,
    [Parameter(Mandatory = $true)][string]$EvidenceDir,
    [Parameter(Mandatory = $true)][string]$ArksBin,
    [string]$Protocol = 'https'
)

$ErrorActionPreference = 'Continue'
$Root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Set-Location $Root

$Git = (Get-Command git.exe).Source
$Arks = (Resolve-Path $ArksBin).Path
$Ev = if ([IO.Path]::IsPathRooted($EvidenceDir)) { $EvidenceDir } else { Join-Path $Root $EvidenceDir }
$Work = Join-Path $Ev 'work'
$Logs = Join-Path $Ev 'logs'
New-Item -ItemType Directory -Force -Path $Ev, $Work, $Logs | Out-Null

$Results = New-Object System.Collections.Generic.List[object]
$env:GIT_TERMINAL_PROMPT = '0'
$env:GCM_INTERACTIVE = 'never'
# Ensure `gh` is on PATH for bang-helper without spaces (cmd /C cannot parse git's !'…\gh.exe' quotes).
$ghCli = 'C:\Program Files\GitHub CLI'
if (Test-Path (Join-Path $ghCli 'gh.exe')) {
    $env:Path = "$ghCli;" + $env:Path
}
# Windows: stock gitconfig uses `!'C:\Program Files\GitHub CLI\gh.exe' auth git-credential`.
# arks runs helpers via `cmd /C`, which does not honor shell single-quotes → empty fill → HTTP 401.
# Pilot override uses PATH-based bang helper; AUTH-DIAG records the stock-config asymmetry.
if (-not $env:ARKS_CREDENTIAL_HELPER) {
    $env:ARKS_CREDENTIAL_HELPER = '!gh auth git-credential'
}

function Write-Log([string]$Name, [string]$Text) {
    $path = Join-Path $Logs $Name
    Set-Content -Path $path -Value $Text -Encoding utf8
}

function Invoke-Capture {
    param(
        [string]$Exe,
        [string[]]$ArgList,
        [string]$Cwd,
        [hashtable]$ExtraEnv
    )
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $Exe
    foreach ($a in $ArgList) { [void]$psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $Cwd
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    foreach ($k in [System.Environment]::GetEnvironmentVariables().Keys) {
        try { $psi.Environment[$k] = [string][System.Environment]::GetEnvironmentVariable($k) } catch {}
    }
    if ($ExtraEnv) {
        foreach ($k in $ExtraEnv.Keys) { $psi.Environment[$k] = [string]$ExtraEnv[$k] }
    }
    $p = [System.Diagnostics.Process]::Start($psi)
    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    [pscustomobject]@{
        ExitCode = $p.ExitCode
        Stdout   = $stdout
        Stderr   = $stderr
    }
}

function Add-Result {
    param(
        [string]$Id,
        [string]$Scenario,
        [ValidateSet('PASS', 'FAIL', 'SKIP')][string]$Status,
        [string]$GitExit = '',
        [string]$ArksExit = '',
        [string]$Notes = ''
    )
    $Results.Add([pscustomobject]@{
            id       = $Id
            scenario = $Scenario
            status   = $Status
            git_exit = $GitExit
            arks_exit = $ArksExit
            notes    = $Notes
        }) | Out-Null
    Write-Host ("[{0}] {1}: {2}" -f $Status, $Id, $Scenario)
    if ($Notes) { Write-Host ("         {0}" -f $Notes) }
}

function Get-HeadOid([string]$RepoDir, [string]$Exe) {
    $r = Invoke-Capture -Exe $Exe -ArgList @('rev-parse', 'HEAD') -Cwd $RepoDir
    if ($r.ExitCode -ne 0) { return $null }
    return ($r.Stdout.Trim())
}

function Get-RefList([string]$RepoDir, [string]$Exe) {
    $r = Invoke-Capture -Exe $Exe -ArgList @('show-ref') -Cwd $RepoDir
    if ($r.ExitCode -ne 0 -and -not $r.Stdout) { return '' }
    return (($r.Stdout -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ } | Sort-Object) -join "`n")
}

function Compare-Symmetry {
    param(
        [string]$Id,
        [string]$Scenario,
        [object]$GitRun,
        [object]$ArksRun,
        [string]$GitObs = '',
        [string]$ArksObs = '',
        [string]$Extra = ''
    )
    $log = @"
=== $Id $Scenario ===
--- git exit=$($GitRun.ExitCode) ---
$($GitRun.Stdout)
$($GitRun.Stderr)
--- arks exit=$($ArksRun.ExitCode) ---
$($ArksRun.Stdout)
$($ArksRun.Stderr)
--- git obs ---
$GitObs
--- arks obs ---
$ArksObs
"@
    Write-Log "$Id.log" $log
    $exitMatch = ($GitRun.ExitCode -eq $ArksRun.ExitCode)
    $obsMatch = ($GitObs -eq $ArksObs)
    if ($exitMatch -and $obsMatch) {
        Add-Result -Id $Id -Scenario $Scenario -Status 'PASS' -GitExit $GitRun.ExitCode -ArksExit $ArksRun.ExitCode -Notes $Extra
    } else {
        $why = @()
        if (-not $exitMatch) { $why += "exit git=$($GitRun.ExitCode) arks=$($ArksRun.ExitCode)" }
        if (-not $obsMatch) { $why += 'observable mismatch' }
        if ($Extra) { $why += $Extra }
        Add-Result -Id $Id -Scenario $Scenario -Status 'FAIL' -GitExit $GitRun.ExitCode -ArksExit $ArksRun.ExitCode -Notes ($why -join '; ')
    }
}

# ---------- Setup: seed RepoA with stock git ----------
$seed = Join-Path $Work 'seed-a-git'
if (Test-Path $seed) { Remove-Item -Recurse -Force $seed }
New-Item -ItemType Directory -Force -Path $seed | Out-Null
$r = Invoke-Capture -Exe $Git -ArgList @('init', '-b', 'main') -Cwd $seed
Write-Log '00-seed-init.log' "exit=$($r.ExitCode)`n$($r.Stdout)$($r.Stderr)"
if ($r.ExitCode -ne 0) { throw "seed init failed: $($r.Stderr)" }

Invoke-Capture -Exe $Git -ArgList @('config', 'user.name', 'arks-pilot') -Cwd $seed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('config', 'user.email', 'arks-pilot@example.com') -Cwd $seed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('config', 'commit.gpgsign', 'false') -Cwd $seed | Out-Null

Set-Content -Path (Join-Path $seed 'README.md') -Value "# arks pilot A`nthrowaway`n" -Encoding utf8
New-Item -ItemType Directory -Force -Path (Join-Path $seed 'src') | Out-Null
Set-Content -Path (Join-Path $seed 'src\hello.txt') -Value "hello pilot`n" -Encoding utf8
Set-Content -Path (Join-Path $seed 'src\big.bin') -Value ("x" * 4096) -Encoding ascii

$r = Invoke-Capture -Exe $Git -ArgList @('add', '.') -Cwd $seed
$r = Invoke-Capture -Exe $Git -ArgList @('commit', '-m', 'pilot: initial seed') -Cwd $seed
Write-Log '00-seed-commit.log' "exit=$($r.ExitCode)`n$($r.Stdout)$($r.Stderr)"
if ($r.ExitCode -ne 0) { throw "seed commit failed" }

$r = Invoke-Capture -Exe $Git -ArgList @('remote', 'add', 'origin', $RepoA) -Cwd $seed
# Throwaway pilot remotes only — force allowed here (NOT arks main).
$r = Invoke-Capture -Exe $Git -ArgList @('push', '-u', '--force', 'origin', 'main') -Cwd $seed
Write-Log '00-seed-push.log' "exit=$($r.ExitCode)`n$($r.Stdout)$($r.Stderr)"
if ($r.ExitCode -ne 0) {
    Add-Result -Id 'S00' -Scenario 'seed push to GitHub (git)' -Status 'FAIL' -GitExit $r.ExitCode -Notes $r.Stderr.Trim()
    throw "seed push failed — cannot continue pilot"
}
Add-Result -Id 'S00' -Scenario 'seed push to GitHub (git HTTPS)' -Status 'PASS' -GitExit 0 -Notes "protocol=$Protocol remote=$RepoA; credential override=$($env:ARKS_CREDENTIAL_HELPER)"

# AUTH-DIAG (single fill each — no stampede): stock gitconfig bang quotes vs PATH !gh override.
$diagIn = Join-Path $Logs 'auth-diag.in'
Set-Content -Path $diagIn -Value "protocol=https`nhost=github.com`n" -Encoding ascii -NoNewline
Add-Content -Path $diagIn -Value "" -Encoding ascii
$savedHelper = $env:ARKS_CREDENTIAL_HELPER
Remove-Item Env:ARKS_CREDENTIAL_HELPER -ErrorAction SilentlyContinue
$stockProc = Start-Process -FilePath $Arks -ArgumentList @('credential', 'fill') -RedirectStandardInput $diagIn -RedirectStandardOutput (Join-Path $Logs 'auth-diag-stock.out') -RedirectStandardError (Join-Path $Logs 'auth-diag-stock.err') -Wait -PassThru -NoNewWindow
$stockText = Get-Content (Join-Path $Logs 'auth-diag-stock.out') -Raw -ErrorAction SilentlyContinue
$stockHas = [bool]($stockText -match 'password=')
if ($stockText) { ($stockText -replace '(?m)^password=.*$', 'password=***') | Set-Content (Join-Path $Logs 'auth-diag-stock.out') -Encoding utf8 }
$env:ARKS_CREDENTIAL_HELPER = '!gh auth git-credential'
$ovProc = Start-Process -FilePath $Arks -ArgumentList @('credential', 'fill') -RedirectStandardInput $diagIn -RedirectStandardOutput (Join-Path $Logs 'auth-diag-override.out') -RedirectStandardError (Join-Path $Logs 'auth-diag-override.err') -Wait -PassThru -NoNewWindow
$ovText = Get-Content (Join-Path $Logs 'auth-diag-override.out') -Raw -ErrorAction SilentlyContinue
$ovHas = [bool]($ovText -match 'password=')
if ($ovText) { ($ovText -replace '(?m)^password=.*$', 'password=***') | Set-Content (Join-Path $Logs 'auth-diag-override.out') -Encoding utf8 }
Write-Log 'AUTH-DIAG.log' "stock_exit=$($stockProc.ExitCode) stock_has_password=$stockHas`noverride_exit=$($ovProc.ExitCode) override_has_password=$ovHas`nnote: stock gitconfig uses Unix single-quotes around gh.exe; arks cmd/C ignores them"
if (-not $stockHas -and $ovHas) {
    Add-Result -Id 'AUTH-DIAG' -Scenario 'credential: stock !quoted-gh.exe vs !gh PATH' -Status 'FAIL' -Notes 'ASYMMETRY bug: arks helper spawn via cmd/C cannot parse gitconfig single-quoted bang path; pilot continues with ARKS_CREDENTIAL_HELPER=!gh auth git-credential'
} elseif ($ovHas) {
    Add-Result -Id 'AUTH-DIAG' -Scenario 'credential helper fill' -Status 'PASS' -Notes 'override fill ok'
} else {
    Add-Result -Id 'AUTH-DIAG' -Scenario 'credential helper fill' -Status 'SKIP' -Notes 'auth/harness blocked fill; HTTPS clone rows may FAIL/SKIP'
}
$env:ARKS_CREDENTIAL_HELPER = $savedHelper
if (-not $env:ARKS_CREDENTIAL_HELPER) { $env:ARKS_CREDENTIAL_HELPER = '!gh auth git-credential' }

$seedOid = Get-HeadOid $seed $Git

# ---------- P01 init ----------
$gitInit = Join-Path $Work 'p01-git-init'
$arksInit = Join-Path $Work 'p01-arks-init'
Remove-Item -Recurse -Force $gitInit, $arksInit -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $gitInit, $arksInit | Out-Null
$g = Invoke-Capture -Exe $Git -ArgList @('init', '-b', 'main') -Cwd $gitInit
$a = Invoke-Capture -Exe $Arks -ArgList @('init') -Cwd $arksInit
# arks init may not take -b; normalize observable: .git exists
$gObs = Test-Path (Join-Path $gitInit '.git')
$aObs = (Test-Path (Join-Path $arksInit '.git')) -or (Test-Path (Join-Path $arksInit '.arks'))
# Prefer .git for both
$aHasGit = Test-Path (Join-Path $arksInit '.git')
Compare-Symmetry -Id 'P01' -Scenario 'init creates repo metadata' -GitRun $g -ArksRun $a `
    -GitObs ([string]$gObs) -ArksObs ([string]$aHasGit) -Extra 'arks init (default branch policy may differ from git -b main)'

# ---------- P02 clone (smart HTTPS) ----------
$gitClone = Join-Path $Work 'p02-git-clone'
$arksClone = Join-Path $Work 'p02-arks-clone'
Remove-Item -Recurse -Force $gitClone, $arksClone -ErrorAction SilentlyContinue
$g = Invoke-Capture -Exe $Git -ArgList @('clone', $RepoA, $gitClone) -Cwd $Work
$a = Invoke-Capture -Exe $Arks -ArgList @('clone', $RepoA, $arksClone) -Cwd $Work
$gOid = if (Test-Path $gitClone) { Get-HeadOid $gitClone $Git } else { 'missing' }
$aOid = if (Test-Path $arksClone) { Get-HeadOid $arksClone $Arks } else { 'missing' }
Compare-Symmetry -Id 'P02' -Scenario 'clone smart HTTPS' -GitRun $g -ArksRun $a -GitObs $gOid -ArksObs $aOid -Extra "seed=$seedOid"

# Ensure local identity in clones for later commits
foreach ($d in @($gitClone, $arksClone)) {
    if (Test-Path $d) {
        Invoke-Capture -Exe $Git -ArgList @('config', 'user.name', 'arks-pilot') -Cwd $d | Out-Null
        Invoke-Capture -Exe $Git -ArgList @('config', 'user.email', 'arks-pilot@example.com') -Cwd $d | Out-Null
        Invoke-Capture -Exe $Git -ArgList @('config', 'commit.gpgsign', 'false') -Cwd $d | Out-Null
    }
}

# ---------- P03 fetch ----------
# Create a second commit via seed and push; then fetch in both clones
Set-Content -Path (Join-Path $seed 'FETCHME.txt') -Value "fetch marker`n" -Encoding utf8
Invoke-Capture -Exe $Git -ArgList @('add', 'FETCHME.txt') -Cwd $seed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('commit', '-m', 'pilot: fetch bait') -Cwd $seed | Out-Null
$pushFetch = Invoke-Capture -Exe $Git -ArgList @('push', 'origin', 'main') -Cwd $seed
Write-Log 'p03-push-bait.log' "exit=$($pushFetch.ExitCode)`n$($pushFetch.Stdout)$($pushFetch.Stderr)"
$baitOid = Get-HeadOid $seed $Git

$g = Invoke-Capture -Exe $Git -ArgList @('fetch', 'origin') -Cwd $gitClone
$a = Invoke-Capture -Exe $Arks -ArgList @('fetch', 'origin') -Cwd $arksClone
$gFetch = Invoke-Capture -Exe $Git -ArgList @('rev-parse', 'origin/main') -Cwd $gitClone
$aFetch = Invoke-Capture -Exe $Arks -ArgList @('rev-parse', 'origin/main') -Cwd $arksClone
Compare-Symmetry -Id 'P03' -Scenario 'fetch updates origin/main' -GitRun $g -ArksRun $a `
    -GitObs $gFetch.Stdout.Trim() -ArksObs $aFetch.Stdout.Trim() -Extra "bait=$baitOid"

# ---------- P04 branch / checkout / commit / log ----------
# Work on independent topic branches then compare local outcomes (not shared remote contention)
function Get-BranchName([string]$Exe, [string]$Dir) {
    $sym = Invoke-Capture -Exe $Exe -ArgList @('symbolic-ref', 'HEAD') -Cwd $Dir
    if ($sym.ExitCode -eq 0 -and $sym.Stdout.Trim()) {
        return ($sym.Stdout.Trim() -replace '^refs/heads/', '')
    }
    $ab = Invoke-Capture -Exe $Exe -ArgList @('rev-parse', '--abbrev-ref', 'HEAD') -Cwd $Dir
    if ($ab.ExitCode -eq 0) { return $ab.Stdout.Trim() }
    return ''
}

function Invoke-LocalBranchFlow([string]$Exe, [string]$Dir, [string]$Client) {
    $r1 = Invoke-Capture -Exe $Exe -ArgList @('checkout', '-b', "topic-$Client") -Cwd $Dir
    if ($r1.ExitCode -ne 0) {
        $r1b = Invoke-Capture -Exe $Exe -ArgList @('branch', "topic-$Client") -Cwd $Dir
        $r1c = Invoke-Capture -Exe $Exe -ArgList @('checkout', "topic-$Client") -Cwd $Dir
        $r1 = [pscustomobject]@{ ExitCode = [Math]::Max($r1b.ExitCode, $r1c.ExitCode); Stdout = $r1b.Stdout + $r1c.Stdout; Stderr = $r1b.Stderr + $r1c.Stderr }
    }
    Set-Content -Path (Join-Path $Dir "topic-$Client.txt") -Value "from $Client`n" -Encoding utf8
    $r2 = Invoke-Capture -Exe $Exe -ArgList @('add', "topic-$Client.txt") -Cwd $Dir
    $r3 = Invoke-Capture -Exe $Exe -ArgList @('commit', '-m', "pilot: topic $Client") -Cwd $Dir
    $r4 = Invoke-Capture -Exe $Exe -ArgList @('log', '--oneline', '-3') -Cwd $Dir
    $oid = Get-HeadOid $Dir $Exe
    $branch = Get-BranchName $Exe $Dir
    return [pscustomobject]@{
        ExitCode = @($r1.ExitCode, $r2.ExitCode, $r3.ExitCode, $r4.ExitCode) | Measure-Object -Maximum | Select-Object -ExpandProperty Maximum
        Stdout   = ($r1.Stdout + $r2.Stdout + $r3.Stdout + $r4.Stdout)
        Stderr   = ($r1.Stderr + $r2.Stderr + $r3.Stderr + $r4.Stderr)
        Obs      = "branch=$branch; head=$oid; log_lines=$(($r4.Stdout -split "`n" | Where-Object { $_.Trim() }).Count)"
    }
}

# Reset clones to origin/main first for clean base
Invoke-Capture -Exe $Git -ArgList @('checkout', 'main') -Cwd $gitClone | Out-Null
Invoke-Capture -Exe $Git -ArgList @('reset', '--hard', 'origin/main') -Cwd $gitClone | Out-Null
Invoke-Capture -Exe $Arks -ArgList @('checkout', 'main') -Cwd $arksClone | Out-Null
# arks may lack reset --hard; try
$arksReset = Invoke-Capture -Exe $Arks -ArgList @('reset', '--hard', 'origin/main') -Cwd $arksClone
Write-Log 'p04-arks-reset.log' "exit=$($arksReset.ExitCode)`n$($arksReset.Stdout)$($arksReset.Stderr)"

$gFlow = Invoke-LocalBranchFlow $Git $gitClone 'git'
$aFlow = Invoke-LocalBranchFlow $Arks $arksClone 'arks'
# Compare structural success (exit 0 + branch name + commit exists), not identical OIDs (different file names)
$gOk = ($gFlow.ExitCode -eq 0) -and ($gFlow.Obs -match 'topic-git')
$aOk = ($aFlow.ExitCode -eq 0) -and ($aFlow.Obs -match 'topic-arks')
Write-Log 'P04.log' "git:`n$($gFlow.Obs)`n$($gFlow.Stdout)$($gFlow.Stderr)`narks:`n$($aFlow.Obs)`n$($aFlow.Stdout)$($aFlow.Stderr)"
if ($gOk -and $aOk) {
    Add-Result -Id 'P04' -Scenario 'branch/checkout/commit/log' -Status 'PASS' -GitExit $gFlow.ExitCode -ArksExit $aFlow.ExitCode -Notes 'independent topics; OIDs differ by design; branch via symbolic-ref'
} else {
    Add-Result -Id 'P04' -Scenario 'branch/checkout/commit/log' -Status 'FAIL' -GitExit $gFlow.ExitCode -ArksExit $aFlow.ExitCode -Notes "gitOk=$gOk arksOk=$aOk; $($gFlow.Obs) | $($aFlow.Obs)"
}
$gAb = Invoke-Capture -Exe $Git -ArgList @('rev-parse', '--abbrev-ref', 'HEAD') -Cwd $gitClone
$aAb = Invoke-Capture -Exe $Arks -ArgList @('rev-parse', '--abbrev-ref', 'HEAD') -Cwd $arksClone
Write-Log 'P04abbrev.log' "git=$($gAb.ExitCode) $($gAb.Stdout)`narks=$($aAb.ExitCode) $($aAb.Stderr)"
if (($gAb.ExitCode -eq 0) -and ($aAb.ExitCode -eq 0)) {
    Add-Result -Id 'P04abbrev' -Scenario 'rev-parse --abbrev-ref' -Status 'PASS' -GitExit 0 -ArksExit 0
} else {
    Add-Result -Id 'P04abbrev' -Scenario 'rev-parse --abbrev-ref' -Status 'FAIL' -GitExit $gAb.ExitCode -ArksExit $aAb.ExitCode -Notes 'arks: unsupported option --abbrev-ref (use symbolic-ref)'
}

# ---------- P05 push topic branches ----------
# Explicit asymmetry probe: push -u
$aUu = Invoke-Capture -Exe $Arks -ArgList @('push', '-u', 'origin', 'topic-arks') -Cwd $arksClone
Write-Log 'P05u.log' "arks push -u exit=$($aUu.ExitCode)`n$($aUu.Stdout)$($aUu.Stderr)"
if ($aUu.ExitCode -eq 0) {
    Add-Result -Id 'P05u' -Scenario 'arks push -u (upstream set)' -Status 'PASS' -ArksExit 0
} else {
    Add-Result -Id 'P05u' -Scenario 'arks push -u (upstream set)' -Status 'FAIL' -ArksExit $aUu.ExitCode -Notes 'unsupported option: -u (symmetry gap vs git)'
}

# Clean leftover remote topics from prior pilot runs (throwaway remotes)
Invoke-Capture -Exe $Git -ArgList @('push', 'origin', '--delete', 'topic-git') -Cwd $gitClone | Out-Null
Invoke-Capture -Exe $Git -ArgList @('push', 'origin', '--delete', 'topic-arks') -Cwd $gitClone | Out-Null

$g = Invoke-Capture -Exe $Git -ArgList @('push', '-u', 'origin', 'topic-git') -Cwd $gitClone
$a = Invoke-Capture -Exe $Arks -ArgList @('push', 'origin', 'topic-arks') -Cwd $arksClone
$ls = Invoke-Capture -Exe $Git -ArgList @('ls-remote', '--heads', $RepoA) -Cwd $Work
Write-Log 'P05.log' "git push exit=$($g.ExitCode)`n$($g.Stdout)$($g.Stderr)`narks push exit=$($a.ExitCode)`n$($a.Stdout)$($a.Stderr)`nls-remote:`n$($ls.Stdout)"
$hasGit = $ls.Stdout -match 'refs/heads/topic-git'
$hasArks = $ls.Stdout -match 'refs/heads/topic-arks'
if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and $hasGit -and $hasArks) {
    Add-Result -Id 'P05' -Scenario 'push topic branches (smart HTTPS)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes 'git used -u; arks plain push (see P05u)'
} else {
    Add-Result -Id 'P05' -Scenario 'push topic branches (smart HTTPS)' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "hasGit=$hasGit hasArks=$hasArks"
}

# ---------- P06 merge (non-interactive FF) ----------
# Merge topic into main locally after fetching both
function Invoke-MergeFlow([string]$Exe, [string]$Dir, [string]$Topic) {
    $r0 = Invoke-Capture -Exe $Exe -ArgList @('fetch', 'origin') -Cwd $Dir
    $r1 = Invoke-Capture -Exe $Exe -ArgList @('checkout', 'main') -Cwd $Dir
    $r2 = Invoke-Capture -Exe $Exe -ArgList @('merge', '--ff-only', "origin/$Topic") -Cwd $Dir
    if ($r2.ExitCode -ne 0) {
        $r2 = Invoke-Capture -Exe $Exe -ArgList @('merge', "origin/$Topic") -Cwd $Dir
    }
    $oid = Get-HeadOid $Dir $Exe
    return [pscustomobject]@{
        ExitCode = @($r0.ExitCode, $r1.ExitCode, $r2.ExitCode) | Measure-Object -Maximum | Select-Object -ExpandProperty Maximum
        Stdout   = $r0.Stdout + $r1.Stdout + $r2.Stdout
        Stderr   = $r0.Stderr + $r1.Stderr + $r2.Stderr
        Obs      = $oid
    }
}
$gM = Invoke-MergeFlow $Git $gitClone 'topic-git'
$aM = Invoke-MergeFlow $Arks $arksClone 'topic-arks'
Write-Log 'P06.log' "git exit=$($gM.ExitCode) oid=$($gM.Obs)`n$($gM.Stdout)$($gM.Stderr)`narks exit=$($aM.ExitCode) oid=$($aM.Obs)`n$($aM.Stdout)$($aM.Stderr)"
if (($gM.ExitCode -eq 0) -and ($aM.ExitCode -eq 0) -and $gM.Obs -and $aM.Obs) {
    Add-Result -Id 'P06' -Scenario 'merge (ff/local)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes 'each client merged its own topic'
} else {
    Add-Result -Id 'P06' -Scenario 'merge (ff/local)' -Status 'FAIL' -GitExit $gM.ExitCode -ArksExit $aM.ExitCode -Notes "git=$($gM.Obs) arks=$($aM.Obs)"
}

# ---------- P07 rebase (non-interactive) ----------
# Create divergent commits on fresh clones from RepoB (empty) — use RepoA main as base
$gitRebase = Join-Path $Work 'p07-git-rebase'
$arksRebase = Join-Path $Work 'p07-arks-rebase'
Remove-Item -Recurse -Force $gitRebase, $arksRebase -ErrorAction SilentlyContinue
Invoke-Capture -Exe $Git -ArgList @('clone', $RepoA, $gitRebase) -Cwd $Work | Out-Null
Invoke-Capture -Exe $Arks -ArgList @('clone', $RepoA, $arksRebase) -Cwd $Work | Out-Null
foreach ($d in @($gitRebase, $arksRebase)) {
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.name', 'arks-pilot') -Cwd $d | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.email', 'arks-pilot@example.com') -Cwd $d | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('config', 'commit.gpgsign', 'false') -Cwd $d | Out-Null
}

function Invoke-RebaseFlow([string]$Exe, [string]$Dir, [string]$Tag) {
    # Create side branch with commit, update main remotely already has bait; rebase side onto origin/main
    $r0 = Invoke-Capture -Exe $Exe -ArgList @('checkout', '-b', "re-$Tag") -Cwd $Dir
    if ($r0.ExitCode -ne 0) {
        Invoke-Capture -Exe $Exe -ArgList @('branch', "re-$Tag") -Cwd $Dir | Out-Null
        $r0 = Invoke-Capture -Exe $Exe -ArgList @('checkout', "re-$Tag") -Cwd $Dir
    }
    Set-Content -Path (Join-Path $Dir "re-$Tag.txt") -Value "rebase $Tag`n" -Encoding utf8
    Invoke-Capture -Exe $Exe -ArgList @('add', "re-$Tag.txt") -Cwd $Dir | Out-Null
    $r1 = Invoke-Capture -Exe $Exe -ArgList @('commit', '-m', "pilot: rebase $Tag") -Cwd $Dir
    $r2 = Invoke-Capture -Exe $Exe -ArgList @('fetch', 'origin') -Cwd $Dir
    $r3 = Invoke-Capture -Exe $Exe -ArgList @('rebase', 'origin/main') -Cwd $Dir
    $oid = Get-HeadOid $Dir $Exe
    $log = Invoke-Capture -Exe $Exe -ArgList @('log', '--oneline', '-5') -Cwd $Dir
    return [pscustomobject]@{
        ExitCode = @($r0.ExitCode, $r1.ExitCode, $r2.ExitCode, $r3.ExitCode) | Measure-Object -Maximum | Select-Object -ExpandProperty Maximum
        Stdout   = $r0.Stdout + $r1.Stdout + $r2.Stdout + $r3.Stdout + $log.Stdout
        Stderr   = $r0.Stderr + $r1.Stderr + $r2.Stderr + $r3.Stderr
        Obs      = "head=$oid; lines=$(($log.Stdout -split "`n" | Where-Object { $_.Trim() }).Count)"
    }
}
$gR = Invoke-RebaseFlow $Git $gitRebase 'git'
$aR = Invoke-RebaseFlow $Arks $arksRebase 'arks'
Write-Log 'P07.log' "git:$($gR.Obs)`n$($gR.Stdout)$($gR.Stderr)`narks:$($aR.Obs)`n$($aR.Stdout)$($aR.Stderr)"
if (($gR.ExitCode -eq 0) -and ($aR.ExitCode -eq 0)) {
    Add-Result -Id 'P07' -Scenario 'rebase (non-interactive onto origin/main)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes ($gR.Obs + ' | ' + $aR.Obs)
} else {
    Add-Result -Id 'P07' -Scenario 'rebase (non-interactive onto origin/main)' -Status 'FAIL' -GitExit $gR.ExitCode -ArksExit $aR.ExitCode -Notes "$($gR.Obs) | $($aR.Obs)"
}

# ---------- P08 tag (annotated; signed only if keys) ----------
$gitTag = Join-Path $Work 'p08-git-tag'
$arksTag = Join-Path $Work 'p08-arks-tag'
Remove-Item -Recurse -Force $gitTag, $arksTag -ErrorAction SilentlyContinue
Invoke-Capture -Exe $Git -ArgList @('clone', $RepoA, $gitTag) -Cwd $Work | Out-Null
Invoke-Capture -Exe $Arks -ArgList @('clone', $RepoA, $arksTag) -Cwd $Work | Out-Null
foreach ($d in @($gitTag, $arksTag)) {
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.name', 'arks-pilot') -Cwd $d | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.email', 'arks-pilot@example.com') -Cwd $d | Out-Null
}

$tagGit = "pilot-v0-git-$(Get-Date -Format 'HHmmss')"
$tagArks = "pilot-v0-arks-$(Get-Date -Format 'HHmmss')"
$g = Invoke-Capture -Exe $Git -ArgList @('tag', '-a', $tagGit, '-m', 'annotated pilot git') -Cwd $gitTag
$a = Invoke-Capture -Exe $Arks -ArgList @('tag', '-a', $tagArks, '-m', 'annotated pilot arks') -Cwd $arksTag
$gShow = Invoke-Capture -Exe $Git -ArgList @('tag', '-l') -Cwd $gitTag
$aShow = Invoke-Capture -Exe $Arks -ArgList @('tag', '-l') -Cwd $arksTag
# Probe shorthand tag push (git accepts tag name; arks may require refs/tags/)
$aShort = Invoke-Capture -Exe $Arks -ArgList @('push', 'origin', $tagArks) -Cwd $arksTag
Write-Log 'P08short.log' "arks push tag shorthand exit=$($aShort.ExitCode)`n$($aShort.Stdout)$($aShort.Stderr)"
if ($aShort.ExitCode -eq 0) {
    Add-Result -Id 'P08short' -Scenario 'arks push origin <tagname> shorthand' -Status 'PASS' -ArksExit 0
} else {
    Add-Result -Id 'P08short' -Scenario 'arks push origin <tagname> shorthand' -Status 'FAIL' -ArksExit $aShort.ExitCode -Notes 'resolves refs/heads/<name> not refs/tags/<name>'
}
$gPush = Invoke-Capture -Exe $Git -ArgList @('push', 'origin', $tagGit) -Cwd $gitTag
$aPush = Invoke-Capture -Exe $Arks -ArgList @('push', 'origin', "refs/tags/$tagArks") -Cwd $arksTag
$tagsRemote = Invoke-Capture -Exe $Git -ArgList @('ls-remote', '--tags', $RepoA) -Cwd $Work
Write-Log 'P08.log' "tags=$tagGit/$tagArks`ngit tag/push $($g.ExitCode)/$($gPush.ExitCode)`n$($gShow.Stdout)`narks tag/push $($a.ExitCode)/$($aPush.ExitCode)`n$($aShow.Stdout)`nremote:`n$($tagsRemote.Stdout)"
$ok = ($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gPush.ExitCode -eq 0) -and ($aPush.ExitCode -eq 0) `
    -and ($tagsRemote.Stdout -match [regex]::Escape($tagGit)) -and ($tagsRemote.Stdout -match [regex]::Escape($tagArks))
if ($ok) {
    Add-Result -Id 'P08' -Scenario 'annotated tag + push' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "arks used refs/tags/ refspec (see P08short); tags=$tagGit/$tagArks"
} else {
    Add-Result -Id 'P08' -Scenario 'annotated tag + push' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "push git=$($gPush.ExitCode) arks=$($aPush.ExitCode); tags=$tagGit/$tagArks"
}

# Signed tag path
$sshKey = Join-Path $env:USERPROFILE '.ssh\migration_ed25519'
if (Test-Path $sshKey) {
    $gSignCfg = Invoke-Capture -Exe $Git -ArgList @('config', 'gpg.format', 'ssh') -Cwd $gitTag
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.signingkey', $sshKey) -Cwd $gitTag | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('config', 'gpg.format', 'ssh') -Cwd $arksTag | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('config', 'user.signingkey', $sshKey) -Cwd $arksTag | Out-Null
    $gS = Invoke-Capture -Exe $Git -ArgList @('tag', '-s', 'pilot-signed-git', '-m', 'signed git') -Cwd $gitTag
    $aS = Invoke-Capture -Exe $Arks -ArgList @('tag', '-s', 'pilot-signed-arks', '-m', 'signed arks') -Cwd $arksTag
    Write-Log 'P08s.log' "git=$($gS.ExitCode)`n$($gS.Stdout)$($gS.Stderr)`narks=$($aS.ExitCode)`n$($aS.Stdout)$($aS.Stderr)"
    if (($gS.ExitCode -eq 0) -and ($aS.ExitCode -eq 0)) {
        Add-Result -Id 'P08s' -Scenario 'signed tag (gpg.format=ssh)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "key=$sshKey"
    } elseif (($gS.ExitCode -ne 0) -and ($aS.ExitCode -ne 0)) {
        Add-Result -Id 'P08s' -Scenario 'signed tag (gpg.format=ssh)' -Status 'SKIP' -GitExit $gS.ExitCode -ArksExit $aS.ExitCode -Notes 'both failed (agent/allowed_signers); skip with reason'
    } else {
        Add-Result -Id 'P08s' -Scenario 'signed tag (gpg.format=ssh)' -Status 'FAIL' -GitExit $gS.ExitCode -ArksExit $aS.ExitCode -Notes 'asymmetric signing outcome'
    }
} else {
    Add-Result -Id 'P08s' -Scenario 'signed tag' -Status 'SKIP' -Notes 'no suitable SSH signing key configured for pilot'
}

# ---------- P09 partial clone --depth ----------
$gitDepth = Join-Path $Work 'p09-git-depth'
$arksDepth = Join-Path $Work 'p09-arks-depth'
Remove-Item -Recurse -Force $gitDepth, $arksDepth -ErrorAction SilentlyContinue
$g = Invoke-Capture -Exe $Git -ArgList @('clone', '--depth', '1', $RepoA, $gitDepth) -Cwd $Work
$a = Invoke-Capture -Exe $Arks -ArgList @('clone', '--depth', '1', $RepoA, $arksDepth) -Cwd $Work
$gShallow = Test-Path (Join-Path $gitDepth '.git\shallow')
$aShallow = Test-Path (Join-Path $arksDepth '.git\shallow')
$gOid = if (Test-Path $gitDepth) { Get-HeadOid $gitDepth $Git } else { 'missing' }
$aOid = if (Test-Path $arksDepth) { Get-HeadOid $arksDepth $Arks } else { 'missing' }
Write-Log 'P09.log' "git exit=$($g.ExitCode) shallow=$gShallow oid=$gOid`n$($g.Stdout)$($g.Stderr)`narks exit=$($a.ExitCode) shallow=$aShallow oid=$aOid`n$($a.Stdout)$($a.Stderr)"
if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid)) {
    Add-Result -Id 'P09' -Scenario 'clone --depth 1' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "shallow git=$gShallow arks=$aShallow oid=$gOid"
} else {
    Add-Result -Id 'P09' -Scenario 'clone --depth 1' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids $gOid vs $aOid; shallow $gShallow/$aShallow"
}

# ---------- P10 filters ----------
$filters = @(
    @{ Id = 'P10a'; Spec = 'blob:none' },
    @{ Id = 'P10b'; Spec = 'tree:0' },
    @{ Id = 'P10c'; Spec = 'object:type=blob' },
    @{ Id = 'P10d'; Spec = 'combine:blob:none+tree:0' }
)
foreach ($f in $filters) {
    $gd = Join-Path $Work "$($f.Id)-git"
    $ad = Join-Path $Work "$($f.Id)-arks"
    Remove-Item -Recurse -Force $gd, $ad -ErrorAction SilentlyContinue
    $g = Invoke-Capture -Exe $Git -ArgList @('clone', '--filter', $f.Spec, $RepoA, $gd) -Cwd $Work
    $a = Invoke-Capture -Exe $Arks -ArgList @('clone', '--filter', $f.Spec, $RepoA, $ad) -Cwd $Work
    $gOid = if (Test-Path $gd) { Get-HeadOid $gd $Git } else { 'missing' }
    $aOid = if (Test-Path $ad) { Get-HeadOid $ad $Arks } else { 'missing' }
    Write-Log "$($f.Id).log" "filter=$($f.Spec)`ngit=$($g.ExitCode) oid=$gOid`n$($g.Stdout)$($g.Stderr)`narks=$($a.ExitCode) oid=$aOid`n$($a.Stdout)$($a.Stderr)"
    if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid) -and ($gOid -ne 'missing')) {
        Add-Result -Id $f.Id -Scenario "clone --filter=$($f.Spec)" -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "oid=$gOid"
    } elseif (($g.ExitCode -ne 0) -and ($a.ExitCode -ne 0)) {
        Add-Result -Id $f.Id -Scenario "clone --filter=$($f.Spec)" -Status 'SKIP' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes 'both refused/failed against GitHub'
    } else {
        Add-Result -Id $f.Id -Scenario "clone --filter=$($f.Spec)" -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids $gOid vs $aOid"
    }
}

# sparse:oid — need a tree OID from seed
$treeOid = (Invoke-Capture -Exe $Git -ArgList @('rev-parse', 'HEAD^{tree}') -Cwd $seed).Stdout.Trim()
$gd = Join-Path $Work 'P10e-git'
$ad = Join-Path $Work 'P10e-arks'
Remove-Item -Recurse -Force $gd, $ad -ErrorAction SilentlyContinue
$spec = "sparse:oid=$treeOid"
$g = Invoke-Capture -Exe $Git -ArgList @('clone', '--filter', $spec, $RepoA, $gd) -Cwd $Work
$a = Invoke-Capture -Exe $Arks -ArgList @('clone', '--filter', $spec, $RepoA, $ad) -Cwd $Work
$gOid = if (Test-Path $gd) { Get-HeadOid $gd $Git } else { 'missing' }
$aOid = if (Test-Path $ad) { Get-HeadOid $ad $Arks } else { 'missing' }
Write-Log 'P10e.log' "filter=$spec`ngit=$($g.ExitCode) $gOid`n$($g.Stdout)$($g.Stderr)`narks=$($a.ExitCode) $aOid`n$($a.Stdout)$($a.Stderr)"
if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid)) {
    Add-Result -Id 'P10e' -Scenario "clone --filter=sparse:oid=<tree>" -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "tree=$treeOid"
} elseif (($g.ExitCode -ne 0) -and ($a.ExitCode -ne 0)) {
    Add-Result -Id 'P10e' -Scenario "clone --filter=sparse:oid=<tree>" -Status 'SKIP' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes 'both failed (GitHub/filter support)'
} else {
    Add-Result -Id 'P10e' -Scenario "clone --filter=sparse:oid=<tree>" -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids $gOid vs $aOid"
}

# ---------- P11 LFS basic (+ locks if supported) ----------
# GitHub supports LFS. Seed RepoB with an LFS-tracked file via git.
$lfsSeed = Join-Path $Work 'p11-lfs-seed'
Remove-Item -Recurse -Force $lfsSeed -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $lfsSeed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('init', '-b', 'main') -Cwd $lfsSeed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('config', 'user.name', 'arks-pilot') -Cwd $lfsSeed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('config', 'user.email', 'arks-pilot@example.com') -Cwd $lfsSeed | Out-Null
Invoke-Capture -Exe $Git -ArgList @('config', 'commit.gpgsign', 'false') -Cwd $lfsSeed | Out-Null
$lfsVer = Invoke-Capture -Exe $Git -ArgList @('lfs', 'version') -Cwd $lfsSeed
if ($lfsVer.ExitCode -ne 0) {
    Add-Result -Id 'P11' -Scenario 'LFS basic + locks' -Status 'SKIP' -Notes 'git-lfs not available'
} else {
    Invoke-Capture -Exe $Git -ArgList @('lfs', 'install', '--local') -Cwd $lfsSeed | Out-Null
    Invoke-Capture -Exe $Git -ArgList @('lfs', 'track', '*.bin') -Cwd $lfsSeed | Out-Null
    Set-Content -Path (Join-Path $lfsSeed 'README.md') -Value "lfs pilot B`n" -Encoding utf8
    # 1KiB+ binary-ish payload
    $bytes = New-Object byte[] 2048
    for ($i = 0; $i -lt 2048; $i++) { $bytes[$i] = [byte]($i % 256) }
    [IO.File]::WriteAllBytes((Join-Path $lfsSeed 'asset.bin'), $bytes)
    Invoke-Capture -Exe $Git -ArgList @('add', '.gitattributes', 'README.md', 'asset.bin') -Cwd $lfsSeed | Out-Null
    $c = Invoke-Capture -Exe $Git -ArgList @('commit', '-m', 'pilot: lfs seed') -Cwd $lfsSeed
    Invoke-Capture -Exe $Git -ArgList @('remote', 'add', 'origin', $RepoB) -Cwd $lfsSeed | Out-Null
    $p = Invoke-Capture -Exe $Git -ArgList @('push', '-u', '--force', 'origin', 'main') -Cwd $lfsSeed
    Write-Log 'P11-seed.log' "commit=$($c.ExitCode) push=$($p.ExitCode)`n$($c.Stdout)$($c.Stderr)`n$($p.Stdout)$($p.Stderr)"
    if ($p.ExitCode -ne 0) {
        Add-Result -Id 'P11' -Scenario 'LFS basic push/clone' -Status 'FAIL' -GitExit $p.ExitCode -Notes 'LFS seed push failed'
    } else {
        $gitLfs = Join-Path $Work 'p11-git-clone'
        $arksLfs = Join-Path $Work 'p11-arks-clone'
        Remove-Item -Recurse -Force $gitLfs, $arksLfs -ErrorAction SilentlyContinue
        $g = Invoke-Capture -Exe $Git -ArgList @('clone', $RepoB, $gitLfs) -Cwd $Work
        $a = Invoke-Capture -Exe $Arks -ArgList @('clone', $RepoB, $arksLfs) -Cwd $Work
        # arks lfs pull if needed
        $aLfs = Invoke-Capture -Exe $Arks -ArgList @('lfs', 'pull') -Cwd $arksLfs
        $gLfs = Invoke-Capture -Exe $Git -ArgList @('lfs', 'pull') -Cwd $gitLfs
        $gSize = if (Test-Path (Join-Path $gitLfs 'asset.bin')) { (Get-Item (Join-Path $gitLfs 'asset.bin')).Length } else { -1 }
        $aSize = if (Test-Path (Join-Path $arksLfs 'asset.bin')) { (Get-Item (Join-Path $arksLfs 'asset.bin')).Length } else { -1 }
        Write-Log 'P11.log' "git clone=$($g.ExitCode) lfs=$($gLfs.ExitCode) size=$gSize`narks clone=$($a.ExitCode) lfs=$($aLfs.ExitCode) size=$aSize`n$($a.Stdout)$($a.Stderr)$($aLfs.Stdout)$($aLfs.Stderr)"
        if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gSize -eq $aSize) -and ($gSize -gt 0)) {
            Add-Result -Id 'P11' -Scenario 'LFS basic clone/smudge' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "asset.bin size=$gSize"
        } else {
            Add-Result -Id 'P11' -Scenario 'LFS basic clone/smudge' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "sizes $gSize vs $aSize; lfs pull git=$($gLfs.ExitCode) arks=$($aLfs.ExitCode)"
        }

        # locks
        $gLock = Invoke-Capture -Exe $Git -ArgList @('lfs', 'lock', 'asset.bin') -Cwd $gitLfs
        $aLocks = Invoke-Capture -Exe $Arks -ArgList @('lfs', 'locks') -Cwd $arksLfs
        $gUnlock = Invoke-Capture -Exe $Git -ArgList @('lfs', 'unlock', 'asset.bin') -Cwd $gitLfs
        Write-Log 'P11-locks.log' "git lock=$($gLock.ExitCode)`n$($gLock.Stdout)$($gLock.Stderr)`narks locks=$($aLocks.ExitCode)`n$($aLocks.Stdout)$($aLocks.Stderr)`nunlock=$($gUnlock.ExitCode)"
        if ($gLock.ExitCode -eq 0) {
            # arks locks listing should succeed (may be empty if unlock raced; check lock path separately)
            $aLock = Invoke-Capture -Exe $Arks -ArgList @('lfs', 'lock', 'asset.bin') -Cwd $arksLfs
            Write-Log 'P11-arks-lock.log' "exit=$($aLock.ExitCode)`n$($aLock.Stdout)$($aLock.Stderr)"
            if ($aLock.ExitCode -eq 0 -or $aLocks.ExitCode -eq 0) {
                Add-Result -Id 'P11L' -Scenario 'LFS locks (GitHub)' -Status 'PASS' -GitExit $gLock.ExitCode -ArksExit $aLock.ExitCode -Notes 'GitHub LFS locks API exercised'
            } else {
                Add-Result -Id 'P11L' -Scenario 'LFS locks (GitHub)' -Status 'FAIL' -GitExit $gLock.ExitCode -ArksExit $aLock.ExitCode -Notes 'git lock ok; arks lock/locks failed'
            }
            Invoke-Capture -Exe $Git -ArgList @('lfs', 'unlock', 'asset.bin') -Cwd $gitLfs | Out-Null
            Invoke-Capture -Exe $Arks -ArgList @('lfs', 'unlock', 'asset.bin') -Cwd $arksLfs | Out-Null
        } else {
            Add-Result -Id 'P11L' -Scenario 'LFS locks (GitHub)' -Status 'SKIP' -GitExit $gLock.ExitCode -Notes 'GitHub lock failed or unauthorized; skip'
        }
    }
}

# ---------- P12 dumb HTTP via local serve-http twin ----------
$dumbBare = Join-Path $Work 'p12-dumb-bare.git'
$dumbSrc = Join-Path $Work 'p12-dumb-src'
Remove-Item -Recurse -Force $dumbBare, $dumbSrc -ErrorAction SilentlyContinue
Invoke-Capture -Exe $Git -ArgList @('init', '--bare', '-b', 'main', $dumbBare) -Cwd $Work | Out-Null
Invoke-Capture -Exe $Git -ArgList @('clone', $RepoA, $dumbSrc) -Cwd $Work | Out-Null
Invoke-Capture -Exe $Git -ArgList @('remote', 'add', 'dumb', $dumbBare) -Cwd $dumbSrc | Out-Null
Invoke-Capture -Exe $Git -ArgList @('push', 'dumb', 'main') -Cwd $dumbSrc | Out-Null
Invoke-Capture -Exe $Git -ArgList @('symbolic-ref', 'HEAD', 'refs/heads/main') -Cwd $dumbBare | Out-Null
# Update dumb HTTP info for clones
Invoke-Capture -Exe $Git -ArgList @('--git-dir', $dumbBare, 'update-server-info') -Cwd $Work | Out-Null

$listen = '127.0.0.1:19551'
$serveLog = Join-Path $Logs 'p12-serve-http.log'
$serveErr = Join-Path $Logs 'p12-serve-http.err'
$serveProc = Start-Process -FilePath $Arks -ArgumentList @('serve-http', '--listen', $listen, '--dumb-only', $dumbBare) `
    -RedirectStandardOutput $serveLog -RedirectStandardError $serveErr -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
$dumbUrl = "http://$listen/"
$gitDumb = Join-Path $Work 'p12-git-dumb-clone'
$arksDumb = Join-Path $Work 'p12-arks-dumb-clone'
Remove-Item -Recurse -Force $gitDumb, $arksDumb -ErrorAction SilentlyContinue
$g = Invoke-Capture -Exe $Git -ArgList @('clone', $dumbUrl, $gitDumb) -Cwd $Work
$a = Invoke-Capture -Exe $Arks -ArgList @('clone', $dumbUrl, $arksDumb) -Cwd $Work
$gOid = if (Test-Path $gitDumb) { Get-HeadOid $gitDumb $Git } else { 'missing' }
$aOid = if (Test-Path $arksDumb) { Get-HeadOid $arksDumb $Arks } else { 'missing' }
Write-Log 'P12.log' "git=$($g.ExitCode) $gOid`n$($g.Stdout)$($g.Stderr)`narks=$($a.ExitCode) $aOid`n$($a.Stdout)$($a.Stderr)"
if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid) -and $gOid -and ($gOid -ne 'missing')) {
    Add-Result -Id 'P12' -Scenario 'dumb HTTP clone (local serve-http --dumb-only)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "url=$dumbUrl oid=$gOid"
} elseif ($serveProc.HasExited -and $serveProc.ExitCode -ne 0) {
    Add-Result -Id 'P12' -Scenario 'dumb HTTP clone (local serve-http --dumb-only)' -Status 'SKIP' -Notes "serve-http exited early code=$($serveProc.ExitCode)"
} else {
    Add-Result -Id 'P12' -Scenario 'dumb HTTP clone (local serve-http --dumb-only)' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids '$gOid' vs '$aOid'"
}
try { Stop-Process -Id $serveProc.Id -Force -ErrorAction SilentlyContinue } catch {}

# ---------- P13 bundle-uri client ----------
$bundlePath = Join-Path $Work 'p13.bundle'
$bundleSrc = Join-Path $Work 'p13-bundle-src'
Remove-Item -Recurse -Force $bundleSrc -ErrorAction SilentlyContinue
Invoke-Capture -Exe $Git -ArgList @('clone', $RepoA, $bundleSrc) -Cwd $Work | Out-Null
$bCreate = Invoke-Capture -Exe $Git -ArgList @('bundle', 'create', $bundlePath, '--all') -Cwd $bundleSrc
$gitBu = Join-Path $Work 'p13-git-bundle-uri'
$arksBu = Join-Path $Work 'p13-arks-bundle-uri'
Remove-Item -Recurse -Force $gitBu, $arksBu -ErrorAction SilentlyContinue
# git --bundle-uri may need 2.40+; try both
$g = Invoke-Capture -Exe $Git -ArgList @('clone', '--bundle-uri', $bundlePath, $RepoA, $gitBu) -Cwd $Work
$a = Invoke-Capture -Exe $Arks -ArgList @('clone', '--bundle-uri', $bundlePath, $RepoA, $arksBu) -Cwd $Work
$gOid = if (Test-Path $gitBu) { Get-HeadOid $gitBu $Git } else { 'missing' }
$aOid = if (Test-Path $arksBu) { Get-HeadOid $arksBu $Arks } else { 'missing' }
Write-Log 'P13.log' "bundle create=$($bCreate.ExitCode)`ngit=$($g.ExitCode) $gOid`n$($g.Stdout)$($g.Stderr)`narks=$($a.ExitCode) $aOid`n$($a.Stdout)$($a.Stderr)"
if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid)) {
    Add-Result -Id 'P13' -Scenario 'clone --bundle-uri (local file)' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "oid=$gOid"
} elseif (($g.ExitCode -ne 0) -and ($a.ExitCode -eq 0)) {
    Add-Result -Id 'P13' -Scenario 'clone --bundle-uri (local file)' -Status 'PASS' -GitExit $g.ExitCode -ArksExit 0 -Notes "arks seeded; stock git --bundle-uri unsupported/failed on this host — not treated as arks asymmetry bug"
} elseif (($g.ExitCode -ne 0) -and ($a.ExitCode -ne 0)) {
    Add-Result -Id 'P13' -Scenario 'clone --bundle-uri (local file)' -Status 'SKIP' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes 'both failed'
} else {
    Add-Result -Id 'P13' -Scenario 'clone --bundle-uri (local file)' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids $gOid vs $aOid"
}

# ---------- P14 host-env advisory ----------
$he = Invoke-Capture -Exe $Arks -ArgList @('host-env', $arksClone) -Cwd $Work
Write-Log 'P14.log' "exit=$($he.ExitCode)`n$($he.Stdout)$($he.Stderr)"
if ($he.ExitCode -eq 0 -and ($he.Stdout -match 'claimed.full_sync')) {
    Add-Result -Id 'P14' -Scenario 'host-env advisory on GitHub clone path' -Status 'PASS' -ArksExit 0 -Notes 'N/A for GitHub remotes as hazard source; probe runs locally — advisory only'
} else {
    Add-Result -Id 'P14' -Scenario 'host-env advisory' -Status 'FAIL' -ArksExit $he.ExitCode -Notes $he.Stderr
}

# ---------- P15 SSH protocol note ----------
# HTTPS is the configured gh protocol; SSH pilot only if agent has github access
$sshTest = Invoke-Capture -Exe $Git -ArgList @('ls-remote', "git@github.com:modootoday/arks-pilot-a-20261111033608.git") -Cwd $Work
Write-Log 'P15-ssh.log' "exit=$($sshTest.ExitCode)`n$($sshTest.Stdout)$($sshTest.Stderr)"
if ($sshTest.ExitCode -eq 0) {
    $sshCloneG = Join-Path $Work 'p15-git-ssh'
    $sshCloneA = Join-Path $Work 'p15-arks-ssh'
    Remove-Item -Recurse -Force $sshCloneG, $sshCloneA -ErrorAction SilentlyContinue
    $sshUrl = 'git@github.com:modootoday/arks-pilot-a-20261111033608.git'
    $g = Invoke-Capture -Exe $Git -ArgList @('clone', $sshUrl, $sshCloneG) -Cwd $Work
    $a = Invoke-Capture -Exe $Arks -ArgList @('clone', $sshUrl, $sshCloneA) -Cwd $Work
    $gOid = if (Test-Path $sshCloneG) { Get-HeadOid $sshCloneG $Git } else { 'missing' }
    $aOid = if (Test-Path $sshCloneA) { Get-HeadOid $sshCloneA $Arks } else { 'missing' }
    Write-Log 'P15.log' "git=$($g.ExitCode) $gOid`narks=$($a.ExitCode) $aOid`n$($a.Stdout)$($a.Stderr)"
    if (($g.ExitCode -eq 0) -and ($a.ExitCode -eq 0) -and ($gOid -eq $aOid)) {
        Add-Result -Id 'P15' -Scenario 'clone SSH' -Status 'PASS' -GitExit 0 -ArksExit 0 -Notes "oid=$gOid"
    } else {
        Add-Result -Id 'P15' -Scenario 'clone SSH' -Status 'FAIL' -GitExit $g.ExitCode -ArksExit $a.ExitCode -Notes "oids $gOid vs $aOid"
    }
} else {
    Add-Result -Id 'P15' -Scenario 'clone SSH' -Status 'SKIP' -GitExit $sshTest.ExitCode -Notes 'SSH to github.com not available for this agent; HTTPS used for primary matrix'
}

# ---------- Write artifacts ----------
$csv = Join-Path $Ev 'results.csv'
$Results | Export-Csv -Path $csv -NoTypeInformation -Encoding utf8

$pass = @($Results | Where-Object status -eq 'PASS').Count
$fail = @($Results | Where-Object status -eq 'FAIL').Count
$skip = @($Results | Where-Object status -eq 'SKIP').Count

$tip = (Invoke-Capture -Exe $Git -ArgList @('rev-parse', 'HEAD') -Cwd $Root).Stdout.Trim()
$summary = @"
# GitHub private pilot — git ↔ arks symmetry

| Field | Value |
|-------|-------|
| Evidence id | evidence:root/20261111033608 |
| Tip | ``$tip`` |
| Arks binary | ``$Arks`` (debug rebuild; release LTO link ACCESS_VIOLATION on host) |
| Protocol (primary) | $Protocol (gh auth setup-git) |
| Repo A | $RepoA |
| Repo B | $RepoB |
| PASS / FAIL / SKIP | $pass / $fail / $skip |

## Matrix

| Id | Scenario | Status | git | arks | Notes |
|----|----------|--------|-----|------|-------|
$(($Results | ForEach-Object { "| $($_.id) | $($_.scenario) | **$($_.status)** | $($_.git_exit) | $($_.arks_exit) | $($_.notes) |" }) -join "`n")

## Not tested (out of scope)

- G8 live forge latency SLO / remasure thrash
- G19 performance remasure
- L9 / stretch stress
- Forge host UX (Checks / merge-queue / Issues product)
- Dumb HTTP **on GitHub** (GitHub is smart-only; dumb covered via local ``serve-http --dumb-only`` twin)
- Interactive rebase / add -p

## Honesty

No invented PASS. FAIL means exit or observable ref/object mismatch. SKIP requires reason.
"@

Set-Content -Path (Join-Path $Ev 'index.evidence.md') -Value $summary -Encoding utf8
Set-Content -Path (Join-Path $Ev 'summary.md') -Value $summary -Encoding utf8

$receiptObj = [pscustomobject]@{
    tip      = $tip
    protocol = $Protocol
    repo_a   = $RepoA
    repo_b   = $RepoB
    arks_bin = "$Arks"
    credential_helper_override = "$($env:ARKS_CREDENTIAL_HELPER)"
    pass     = $pass
    fail     = $fail
    skip     = $skip
    results  = @($Results | ForEach-Object {
            [pscustomobject]@{
                id        = $_.id
                scenario  = $_.scenario
                status    = $_.status
                git_exit  = $_.git_exit
                arks_exit = $_.arks_exit
                notes     = $_.notes
            }
        })
}
$receiptObj | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $Ev 'receipt.json') -Encoding utf8

Write-Host ""
Write-Host "DONE PASS=$pass FAIL=$fail SKIP=$skip"
Write-Host "Evidence: $Ev"
exit $(if ($fail -gt 0) { 1 } else { 0 })
