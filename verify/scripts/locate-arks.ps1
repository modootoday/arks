# Locate arks binaries by filesystem metadata only.
# NEVER Select-String / Get-Content / findstr binary ELFs under target*/ — that dumps
# multi-MB binary blobs into agent terminals and can abort with exit -1 (4294967295).
# Usage (repo root or any cwd): pwsh -File verify/scripts/locate-arks.ps1

$ErrorActionPreference = 'Continue'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$roots = @(
    (Join-Path $root 'target'),
    (Join-Path $root 'target-cleanroom')
) | Where-Object { Test-Path $_ }

$rows = foreach ($r in $roots) {
    Get-ChildItem -Path $r -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Name -eq 'arks' -or $_.Name -eq 'arks.exe'
        } |
        ForEach-Object {
            $kind = 'unknown'
            if ($_.Extension -eq '.exe') {
                $kind = 'pe-windows'
            } elseif ($_.Length -ge 4) {
                $fs = [System.IO.File]::OpenRead($_.FullName)
                try {
                    $buf = New-Object byte[] 4
                    [void]$fs.Read($buf, 0, 4)
                    if ($buf[0] -eq 0x7F -and $buf[1] -eq 0x45 -and $buf[2] -eq 0x4C -and $buf[3] -eq 0x46) {
                        $kind = 'elf-linux'
                    }
                } finally {
                    $fs.Dispose()
                }
            }
            [pscustomobject]@{
                FullName      = $_.FullName
                Length        = $_.Length
                LastWriteTime = $_.LastWriteTime
                Kind          = $kind
            }
        }
}

if (-not $rows) {
    Write-Host 'No arks / arks.exe found under target/ or target-cleanroom/.'
    exit 0
}

$rows | Sort-Object LastWriteTime -Descending | Format-Table -AutoSize
Write-Host ''
Write-Host 'Host advisory: prefer target/release/arks.exe (PE). Do not execute elf-linux on Windows.'
Write-Host 'Clean-room: prefer target-cleanroom/release/arks (ELF) inside Docker only.'
