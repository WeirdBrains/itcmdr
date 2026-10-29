<#
.SYNOPSIS
  Install or update the itcmdr agent scheduled task (SYSTEM) with nightly run and lightweight poller.

.PARAMETER OnlyAgent
  If set, only (re)install the agent components (no server/app).

.PARAMETER StaggerSeconds
  Optional stagger offset to avoid all hosts running at the same time.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
  [switch]$OnlyAgent,
  [int]$StaggerSeconds = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$taskName = 'itcmdr Agent Nightly'
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$agentPath = Join-Path $scriptDir 'Invoke-ItcmdrAgent.ps1'

if (-not (Test-Path -LiteralPath $agentPath)) {
  throw \"Agent script not found at $agentPath\"
}

$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument \"-NoProfile -ExecutionPolicy Bypass -File `\"$agentPath`\" -ReportOnly:\$false\"
$trigger = New-ScheduledTaskTrigger -Daily -At 1:30am
if ($StaggerSeconds -gt 0) {
  $trigger = New-ScheduledTaskTrigger -Once -At ((Get-Date).Date.AddHours(1).AddMinutes(30).AddSeconds($StaggerSeconds))
  $trigger.Repetition = New-ScheduledTaskTrigger -Once -At ((Get-Date).Date.AddHours(1).AddMinutes(30).AddSeconds($StaggerSeconds))
  $trigger.Repetition.Interval = 'P1D'
}
$principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest -LogonType ServiceAccount
$task = New-ScheduledTask -Action $action -Trigger $trigger -Principal $principal -Settings (New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable)

if ($PSCmdlet.ShouldProcess($taskName, 'Register-ScheduledTask')) {
  try {
    Register-ScheduledTask -TaskName $taskName -InputObject $task -Force | Out-Null
    Write-Host \"Installed scheduled task '$taskName'\"
  } catch {
    throw \"Failed to register scheduled task: $_\"
  }
}

