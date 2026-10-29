<#
.SYNOPSIS
  Report coverage of expected backup sources and optionally stage a fallback archive.

.PARAMETER StageTo
  Optional folder to stage a compressed fallback of maintenanceRuns after nightly runs.
#>
[CmdletBinding()]
param(
  [string]$StageTo
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ItcmdrSite {
  $candidates = @('config/site.json','C:\ProgramData\itcmdr\site.json')
  foreach ($p in $candidates) { if (Test-Path $p) { return Get-Content $p -Raw | ConvertFrom-Json -AsHashtable } }
  return @{} 
}

$site = Get-ItcmdrSite
$expected = @($site.backup.expectedBackupSources)

$tasks = @('Sidexis SQL Backup','EndoVision DB Backup')
$found = @()
foreach ($t in $tasks) {
  $task = Get-ScheduledTask -TaskName $t -ErrorAction SilentlyContinue
  if ($task) {
    $found += [pscustomobject]@{ name=$t; exists=$true }
  } else {
    $found += [pscustomobject]@{ name=$t; exists=$false }
  }
}

$report = [pscustomobject]@{
  timestamp = (Get-Date).ToString('o')
  expected = $expected
  tasks = $found
}

$out = $report | ConvertTo-Json -Depth 6
$dest = 'C:\maintenanceRuns\backup-coverage.json'
$out | Out-File -LiteralPath $dest -Encoding utf8

if ($StageTo) {
  if (-not (Test-Path $StageTo)) { New-Item -ItemType Directory -Path $StageTo -Force | Out-Null }
  $zipPath = Join-Path $StageTo ('maintenanceRuns_{0:yyyyMMddHHmmss}.zip' -f (Get-Date))
  Compress-Archive -LiteralPath 'C:\maintenanceRuns' -DestinationPath $zipPath -CompressionLevel Optimal -Force
}

Write-Output "Wrote coverage report to $dest"

