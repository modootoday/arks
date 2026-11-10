# After-edit Hangul guard for tracked docs / Spec / agent trees.
# Rule 05-english-only — chat Korean OK; committed Hangul is not.
$ErrorActionPreference = 'Stop'
try {
  $raw = [Console]::In.ReadToEnd()
  $inputObj = $raw | ConvertFrom-Json

  $path = $null
  foreach ($key in @('file_path', 'path', 'filePath', 'uri')) {
    if ($inputObj.PSObject.Properties.Name -contains $key -and $inputObj.$key) {
      $path = [string]$inputObj.$key
      break
    }
  }
  if (-not $path -and $inputObj.file -and $inputObj.file.path) {
    $path = [string]$inputObj.file.path
  }

  if ([string]::IsNullOrWhiteSpace($path)) {
    exit 0
  }

  $norm = $path -replace '\\', '/'
  $watched = $norm -match '(?i)(^|/)\.spec/|(^|/)docs/|(^|/)AGENTS\.md$|(^|/)\.agents/|(^|/)README\.md$|(^|/)CONTRIBUTING\.md$|(^|/)SECURITY\.md$'
  if (-not $watched) {
    exit 0
  }

  if (-not (Test-Path -LiteralPath $path)) {
    exit 0
  }

  # Hangul syllables + compatibility jamo (authored prose ban).
  $hangul = [regex]'[\uAC00-\uD7A3\u1100-\u11FF\u3130-\u318F]'
  $text = Get-Content -LiteralPath $path -Raw -ErrorAction SilentlyContinue
  if ($null -eq $text) {
    exit 0
  }

  if ($hangul.IsMatch($text)) {
    $msg = "Hangul detected in tracked docs path '$norm'. Rule 05-english-only: convert to English before commit (chat Korean OK)."
    $payload = @{
      additional_context = $msg
    } | ConvertTo-Json -Compress
    Write-Output $payload
  }

  exit 0
}
catch {
  exit 0
}
