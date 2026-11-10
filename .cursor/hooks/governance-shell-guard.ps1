# Governance shell guard — deny/ask dangerous git/shell; commit honesty scans.
# Rules: 05, 28, 29, 30, 35, 91. Affinity: humanymous pre-tool-guard deny ideas (cite, do not fork).
$ErrorActionPreference = 'Stop'

function Write-Permission {
  param(
    [Parameter(Mandatory = $true)][ValidateSet('allow', 'ask', 'deny')][string]$Permission,
    [string]$UserMessage = '',
    [string]$AgentMessage = ''
  )
  $payload = @{ permission = $Permission }
  if ($UserMessage) { $payload.user_message = $UserMessage }
  if ($AgentMessage) { $payload.agent_message = $AgentMessage }
  Write-Output ($payload | ConvertTo-Json -Compress)
}

function Get-RepoRoot {
  try {
    $root = (git rev-parse --show-toplevel 2>$null)
    if ($root) { return $root.Trim() }
  }
  catch { }
  return (Get-Location).Path
}

function Test-Hangul([string]$text) {
  if ([string]::IsNullOrEmpty($text)) { return $false }
  return [regex]::IsMatch($text, '[\uAC00-\uD7A3\u1100-\u11FF\u3130-\u318F]')
}

function Test-ClaimedInvent([string]$text) {
  if ([string]::IsNullOrEmpty($text)) { return $false }
  # Accidental product success language in commit subjects/bodies.
  return [regex]::IsMatch($text, '(?i)\b(Claimed|Verified|PASS)\b') -and
    -not [regex]::IsMatch($text, '(?i)\b(no invent|not Claimed|unClaimed|non-Claimed|do not invent|without inventing)\b')
}

try {
  $raw = [Console]::In.ReadToEnd()
  $inputObj = $raw | ConvertFrom-Json
  $cmd = [string]($inputObj.command)
  if ([string]::IsNullOrWhiteSpace($cmd)) {
    Write-Permission -Permission allow
    exit 0
  }

  $norm = ($cmd -replace '\s+', ' ').Trim()

  # --- Deny: force-push to main/master ---
  if ($norm -match '(?i)\bgit\s+push\b' -and $norm -match '(?i)(?:^|\s)(--force(?:-with-lease)?|-f)(?=$|\s)' -and $norm -match '(?i)(?:^|[^\w/-])(main|master)(?![\w/-])') {
    Write-Permission -Permission deny `
      -UserMessage 'Force-push to main/master is blocked by project governance hooks.' `
      -AgentMessage 'Rule 91-commit-hygiene: never force-push to main/master.'
    exit 0
  }

  # --- Deny: stage gitignored local / secrets ---
  if ($norm -match '(?i)\bgit\s+add\b' -and $norm -match '(?i)(\.spec\.local(/|\\|\s|$)|(?<![.\w])\.env(?![.\w])|\.env\.|/secrets(/|\\|\s|$))') {
    Write-Permission -Permission deny `
      -UserMessage 'Refusing to stage .spec.local / .env / secrets paths.' `
      -AgentMessage 'Rules 35/80/91: .spec.local and secrets stay untracked.'
    exit 0
  }

  # --- Deny: recursive delete of filesystem/home roots (humanymous affinity) ---
  if ($norm -match '(?i)\brm\b' -and $norm -match '(?i)(?:^|\s)(-[a-z]*r[a-z]*|--recursive)(?=$|\s)' -and $norm -match '(?i)(?:^|\s)(-[a-z]*f[a-z]*|--force)(?=$|\s)' -and $norm -match '(?i)(?:^|\s)(["'']?(?:/|~|\$home|[a-z]:[\\/]+)["'']?(?=$|\s))') {
    Write-Permission -Permission deny `
      -UserMessage 'Blocked recursive force-delete targeting filesystem/home root.' `
      -AgentMessage 'Safety: refuse rm -rf of / or home roots.'
    exit 0
  }
  if ($norm -match '(?i)\bRemove-Item\b' -and $norm -match '(?i)-(?:Recurse|r)\b' -and $norm -match '(?i)(?:^|\s)(["'']?(?:/|~|\$home|\$env:(?:UserProfile|HomeDrive)|[a-z]:[\\/]+)["'']?(?=$|\s))') {
    Write-Permission -Permission deny `
      -UserMessage 'Blocked PowerShell recursive delete targeting filesystem/home root.' `
      -AgentMessage 'Safety: refuse Remove-Item -Recurse of / or home roots.'
    exit 0
  }

  # --- Ask: any git push (push-deferred) ---
  if ($norm -match '(?i)\bgit\s+push\b') {
    Write-Permission -Permission ask `
      -UserMessage 'arks push is deferred unless you explicitly authorize a clean history push.' `
      -AgentMessage 'Rule 91-commit-hygiene: do not git push while push-deferred unless the owner explicitly authorizes.'
    exit 0
  }

  # --- Ask: blind add ---
  if ($norm -match '(?i)\bgit\s+add\s+(-A|\.|--all)\b') {
    Write-Permission -Permission ask `
      -UserMessage 'Blind git add may stage sibling-owned crate dirt. Prefer explicit paths.' `
      -AgentMessage 'Rule 91-commit-hygiene: stage only owned paths; leave sibling CLI/crate WIP unstaged.'
    exit 0
  }

  # --- Ask: destructive git ---
  if ($norm -match '(?i)\bgit\s+reset\s+--hard\b' -or $norm -match '(?i)\bgit\s+clean\s+-[a-z]*f' -or $norm -match '(?i)\bgit\s+push\b.*(?i)(--force(?:-with-lease)?|-f)') {
    Write-Permission -Permission ask `
      -UserMessage 'Destructive git command — confirm before continuing.' `
      -AgentMessage 'Rule 91: no force-push / hard reset / clean -f unless the owner explicitly orders it.'
    exit 0
  }

  # --- Ask: gh CLI (operator vs product) ---
  if ($norm -match '(?i)\bgh\s+') {
    Write-Permission -Permission ask `
      -UserMessage 'Confirm this gh invocation is operator tooling, not product-path behavior.' `
      -AgentMessage 'Rule 30-no-git-gh-subprocess: product crates must not spawn gh; verify harnesses/operator tools may ask.'
    exit 0
  }

  # --- git commit: Hangul + Claimed invent + staged Hangul docs ---
  if ($norm -match '(?i)\bgit\s+commit\b') {
    $msg = ''
    if ($norm -match '(?i)-m\s+"([^"]*)"') { $msg = $Matches[1] }
    elseif ($norm -match "(?i)-m\s+'([^']*)'") { $msg = $Matches[1] }
    elseif ($norm -match '(?i)-m\s+(\S+)') { $msg = $Matches[1] }

    if (Test-Hangul $msg) {
      Write-Permission -Permission deny `
        -UserMessage 'Hangul detected in commit message. Convert to English (rule 05).' `
        -AgentMessage 'Rule 05-english-only: commit subjects/bodies must be English.'
      exit 0
    }

    if (Test-ClaimedInvent $msg) {
      Write-Permission -Permission ask `
        -UserMessage 'Commit message mentions Claimed/Verified/PASS. Confirm evidence exists (no invent).' `
        -AgentMessage 'Rules 28/29: do not invent Claimed/Verified/PASS in commit prose without citing evidence.'
      exit 0
    }

    # Staged Hangul in watched paths
    $root = Get-RepoRoot
    $watched = '(?i)(^|/)\.spec/|(^|/)docs/|(^|/)AGENTS\.md$|(^|/)\.agents/|(^|/)README\.md$|(^|/)CONTRIBUTING\.md$|(^|/)SECURITY\.md$|(^|/)LICENSE|(^|/)verify/|(^|/)\.cursor/'
    try {
      $staged = @(git -C $root diff --cached --name-only --diff-filter=ACMR 2>$null)
      foreach ($rel in $staged) {
        $normPath = ($rel -replace '\\', '/')
        if ($normPath -notmatch $watched) { continue }
        $full = Join-Path $root $rel
        if (-not (Test-Path -LiteralPath $full)) { continue }
        $text = Get-Content -LiteralPath $full -Raw -ErrorAction SilentlyContinue
        if (Test-Hangul $text) {
          Write-Permission -Permission deny `
            -UserMessage "Hangul detected in staged path '$normPath'. Convert to English before commit." `
            -AgentMessage 'Rule 05-english-only: staged Spec/docs/agent paths must be English.'
          exit 0
        }
      }
    }
    catch { }
  }

  Write-Permission -Permission allow
  exit 0
}
catch {
  Write-Permission -Permission allow
  exit 0
}
