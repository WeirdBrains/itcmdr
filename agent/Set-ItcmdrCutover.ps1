<#
.SYNOPSIS
  One-command local cutover control for this PC (v0.11.0).

.DESCRIPTION
  -To task      Rollback: re-enable the SYSTEM nightly task and pin this PC to it (the server's
                schedule-state.json is ignored until -To auto). Works with the server down.
  -To auto      Unpin: follow the server again (itcmdr-cli cutover switch / rollback).
  -To schedule  Pin to the default schedule and disable the task (normally the server does this).
  The fleet-wide rollback is `itcmdr-cli cutover rollback` on the server.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File Set-ItcmdrCutover.ps1 -To task
#>
[CmdletBinding(SupportsShouldProcess)]
param([Parameter(Mandatory)][ValidateSet('task', 'schedule', 'auto')][string]$To)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Itcmdr.Checks.ps1')
. (Join-Path $PSScriptRoot 'Itcmdr.PluginRunner.ps1')
$site = Get-ItcmdrSiteConfig
if ($To -eq 'auto') {
  $cur = Get-ItcmdrCutoverState -Site $site
  if ($PSCmdlet.ShouldProcess('cutover.json', 'unpin')) { Set-ItcmdrCutoverState -Site $site -State $cur.State -Reason 'unpinned (Set-ItcmdrCutover -To auto)' }
  $now = Sync-ItcmdrCutover -Site $site
  Write-Output "Following the server; local state: $($now.State)"
  return
}
if ($PSCmdlet.ShouldProcess($script:ItcmdrNightlyTaskName, "cutover -> $To")) {
  $changed = Set-ItcmdrNightlyTaskEnabled -Enabled ($To -eq 'task')
  Set-ItcmdrCutoverState -Site $site -State $To -Pinned -Reason "Set-ItcmdrCutover -To $To"
  $t = if ($changed) { if ($To -eq 'task') { 'enabled' } else { 'disabled' } } else { 'not found here (state recorded only)' }
  Write-Output "Cutover pinned to '$To'; nightly task $t"
}
