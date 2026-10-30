<#
.SYNOPSIS
  Install or update the itcmdr agent scheduled task (SYSTEM) with nightly run and lightweight poller.

.PARAMETER StaggerSeconds
  Optional stagger offset to avoid all hosts running at the same time.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
  [int]$StaggerSeconds = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$taskName = 'itcmdr Agent Nightly'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$agentPath = Join-Path $scriptDir 'Invoke-ItcmdrAgent.ps1'

if (-not (Test-Path -LiteralPath $agentPath)) {
  throw "Agent script not found at $agentPath"
}

# Nightly run: repairs on (SYSTEM default); the clinical-activity guard still applies.
$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$agentPath`""
# Time: setting agent.nightlyTime (site.json agent.nightlyTime, else central, else 01:30).
$nightly = '01:30'
$site = $null
try {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . $agentPath
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  $site = $null
  try { $site = Get-ItcmdrSiteConfig } catch { $site = @{} }
  $nightly = [string](Get-ItcmdrSetting -Site $site -Key 'agent.nightlyTime')
} catch { Write-Warning "Using the default nightly time 01:30: $_" }
$at = ConvertTo-ItcmdrNightlyTime -Time $nightly
$nightlyTrigger = New-ScheduledTaskTrigger -Daily -At ($at.AddSeconds($StaggerSeconds))
$principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest -LogonType ServiceAccount
$task = New-ScheduledTask -Action $action -Trigger $nightlyTrigger -Principal $principal -Settings (New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable)

if ($PSCmdlet.ShouldProcess($taskName, 'Register-ScheduledTask')) {
  try {
    Register-ScheduledTask -TaskName $taskName -InputObject $task -Force | Out-Null
    Write-Output "Installed scheduled task '$taskName'"
    # v0.11.0: after the cutover the task is a disabled fallback; an upgrade must keep it disabled.
    $cutState = 'task'
    try { if ($site) { $cutState = (Get-ItcmdrCutoverState -Site $site).State } } catch { $cutState = 'task' }
    if ($cutState -eq 'schedule') {
      Disable-ScheduledTask -TaskName $taskName | Out-Null
      Write-Output "Kept '$taskName' disabled (cutover: the default schedule runs the nightly)"
    }
  } catch {
    throw "Failed to register scheduled task: $_"
  }
}

# Poller task (every 1 minute)
$pollName = 'itcmdr Agent Poller'
$pollAction = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $scriptDir 'Poll-ItcmdrRequests.ps1')`""
# Repeats every minute indefinitely; a still-running poll makes the next one skip.
$pollTrigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Minutes 1)
$pollSettings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 2)
$pollTask = New-ScheduledTask -Action $pollAction -Trigger $pollTrigger -Principal $principal -Settings $pollSettings
if ($PSCmdlet.ShouldProcess($pollName, 'Register-ScheduledTask')) {
  try {
    Register-ScheduledTask -TaskName $pollName -InputObject $pollTask -Force | Out-Null
    Write-Output "Installed scheduled task '$pollName'"
  } catch {
    throw "Failed to register poller task: $_"
  }
}

