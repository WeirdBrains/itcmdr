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
$trigger = New-ScheduledTaskTrigger -Daily -At ((Get-Date).Date.AddHours(1).AddMinutes(30).AddSeconds($StaggerSeconds))
$principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest -LogonType ServiceAccount
$task = New-ScheduledTask -Action $action -Trigger $trigger -Principal $principal -Settings (New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable)

if ($PSCmdlet.ShouldProcess($taskName, 'Register-ScheduledTask')) {
  try {
    Register-ScheduledTask -TaskName $taskName -InputObject $task -Force | Out-Null
    Write-Output "Installed scheduled task '$taskName'"
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

