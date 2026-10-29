<#
.SYNOPSIS
  Push-install the itcmdr agent to remote Windows machines via ADMIN$ and schtasks (no WinRM).

.PARAMETER ComputerName
  Target computer names.

.PARAMETER SourceDir
  Local source dir (defaults to path of this script's parent).
#>
[CmdletBinding(SupportsShouldProcess)]
param(
  [Parameter(Mandatory, ValueFromPipeline, ValueFromPipelineByPropertyName)]
  [string[]]$ComputerName,
  [string]$SourceDir = (Split-Path -Parent $MyInvocation.MyCommand.Path)
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Copy-Agent($c) {
  $dest = \"\\\\$c\\ADMIN$\\itcmdr\\agent\"
  New-Item -ItemType Directory -Path $dest -Force | Out-Null
  Copy-Item -LiteralPath (Join-Path $SourceDir 'Invoke-ItcmdrAgent.ps1') -Destination $dest -Force
  Copy-Item -LiteralPath (Join-Path $SourceDir 'Install-ItcmdrAgent.ps1') -Destination $dest -Force
  Copy-Item -LiteralPath (Join-Path $SourceDir 'Poll-ItcmdrRequests.ps1') -Destination $dest -Force
}

foreach ($c in $ComputerName) {
  if ($PSCmdlet.ShouldProcess($c, 'Push itcmdr agent')) {
    Copy-Agent -c $c
    $taskName = 'itcmdr Bootstrap'
    $tr = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\Windows\itcmdr\agent\Install-ItcmdrAgent.ps1'
    schtasks /S $c /Create /RU SYSTEM /SC ONCE /ST 00:00 /TN $taskName /TR "$tr" /F | Out-Null
    schtasks /S $c /Run /TN $taskName | Out-Null
    Write-Host "Triggered install on $c"
  }
}

