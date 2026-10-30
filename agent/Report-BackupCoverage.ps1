<#
.SYNOPSIS
  Report backup coverage on demand (same checks the nightly agent runs in category "backup").

.DESCRIPTION
  Reads site.json "backup": expectedBackupSources (must exist), tasks (scheduled backup jobs:
  present, enabled, ran within maxAgeHours, result 0) and destinations (newest file matching
  pattern is recent and at least minBytes). Report only: nothing is started or changed.
  Prints one line per check and exits 1 when anything failed.

.PARAMETER SitePath
  site.json to use (defaults to the agent's lookup).

.PARAMETER StageTo
  Optional folder to also stage a compressed copy of -Source (the local maintenance log folder).

.PARAMETER Source
  Folder to stage. Default: site.json paths.localLogs, else C:\maintenanceRuns.
#>
[CmdletBinding()]
param(
  [string]$SitePath,
  [string]$StageTo,
  [string]$Source
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$env:ITCMDR_IMPORT_ONLY = '1'
try { . (Join-Path $PSScriptRoot 'Invoke-ItcmdrAgent.ps1') } finally { Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue }

$site = if ($SitePath) { ConvertFrom-ItcmdrJson (Get-Content -LiteralPath $SitePath -Raw) } else { Get-ItcmdrSiteConfig }
$log = Join-Path ([IO.Path]::GetTempPath()) ("itcmdr-backup-$([guid]::NewGuid()).log")
$ctx = @{ RunId = [guid]::NewGuid().ToString(); Computer = $env:COMPUTERNAME; Role = 'Server'; Site = $site; LogFile = $log; Exceptions = @() }
if (-not $Source) { $Source = Get-ItcmdrPath -Site $site -Name localLogs }
Invoke-ItcmdrBackupCheck -Ctx $ctx -ReportOnlyLocal

$failed = 0
if (Test-Path -LiteralPath $log) {
  foreach ($r in @(Get-Content -LiteralPath $log | ForEach-Object { $_ | ConvertFrom-Json })) {
    if ($r.status -eq 'failed') { $failed++ }
    Write-Output ('{0,-7} {1,-32} {2}' -f $r.status, $r.checkId, $r.detail)
  }
  Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
} else {
  Write-Output 'No backup section in site.json; nothing to check.'
}

if ($StageTo) {
  if (-not (Test-Path $StageTo)) { New-Item -ItemType Directory -Path $StageTo -Force | Out-Null }
  $zipPath = Join-Path $StageTo ('maintenanceRuns_{0:yyyyMMddHHmmss}.zip' -f (Get-Date))
  Compress-Archive -LiteralPath $Source -DestinationPath $zipPath -CompressionLevel Optimal -Force
  Write-Output "Staged $zipPath"
}
if ($failed -gt 0) { exit 1 }
