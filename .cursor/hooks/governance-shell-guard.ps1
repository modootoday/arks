# Governance shell guard — ask before push / blind add; warn gh CLI usage.
# Always-on rules: 91-commit-hygiene, 30-no-git-gh-subprocess (product paths).
$ErrorActionPreference = 'Stop'
try {
  $raw = [Console]::In.ReadToEnd()
  $inputObj = $raw | ConvertFrom-Json
  $cmd = [string]($inputObj.command)
  if ([string]::IsNullOrWhiteSpace($cmd)) {
    Write-Output '{"permission":"allow"}'
    exit 0
  }

  if ($cmd -match '(?i)\bgit\s+push\b') {
    $payload = @{
      permission     = 'ask'
      user_message   = 'arks push is deferred unless you explicitly authorize a clean history push.'
      agent_message  = 'Rule 91-commit-hygiene: do not git push while push-deferred unless the owner explicitly authorizes.'
    } | ConvertTo-Json -Compress
    Write-Output $payload
    exit 0
  }

  if ($cmd -match '(?i)\bgit\s+add\s+(-A|\.|--all)\b') {
    $payload = @{
      permission     = 'ask'
      user_message   = 'Blind git add may stage sibling-owned crate dirt. Prefer explicit paths.'
      agent_message  = 'Rule 91-commit-hygiene: stage only owned paths; leave sibling CLI/crate WIP unstaged.'
    } | ConvertTo-Json -Compress
    Write-Output $payload
    exit 0
  }

  if ($cmd -match '(?i)\bgh\s+') {
    $payload = @{
      permission     = 'ask'
      user_message   = 'Confirm this gh invocation is operator tooling, not product-path behavior.'
      agent_message  = 'Rule 30-no-git-gh-subprocess: product crates must not spawn gh; verify harnesses/operator tools may ask.'
    } | ConvertTo-Json -Compress
    Write-Output $payload
    exit 0
  }

  Write-Output '{"permission":"allow"}'
  exit 0
}
catch {
  # Fail open so hooks never brick the agent on parse errors.
  Write-Output '{"permission":"allow"}'
  exit 0
}
