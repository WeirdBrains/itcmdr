<#
.SYNOPSIS
  Install or update the daily log rotation task (SYSTEM, server only). Run by the installer.
  Time: site.json rotation.timeLocal, else 02:10.
#>
[CmdletBinding(SupportsShouldProcess)]
param()
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$time = '02:10'
try {
  $site = Get-Content -Raw -LiteralPath (Join-Path $env:ProgramData 'itcmdr\site.json') | ConvertFrom-Json
  if ($site.PSObject.Properties['rotation'] -and $site.rotation.PSObject.Properties['timeLocal'] -and $site.rotation.timeLocal) { $time = [string]$site.rotation.timeLocal }
} catch { Write-Verbose "No site.json rotation time: $_" }
$script = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'Rotate-ItcmdrLogs.ps1'
$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$script`""
$t = New-ScheduledTaskTrigger -Daily -At $time
$p = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest -LogonType ServiceAccount
$s = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
if ($PSCmdlet.ShouldProcess('itcmdr Log Rotation', 'Register-ScheduledTask')) {
  Register-ScheduledTask -TaskName 'itcmdr Log Rotation' -Action $a -Trigger $t -Principal $p -Settings $s -Force | Out-Null
  Write-Output "Installed scheduled task 'itcmdr Log Rotation' at $time"
}
