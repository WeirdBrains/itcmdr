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

begin {
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Copy-Agent($c, $Source) {
  $dest = "\\$c\ADMIN$\itcmdr\agent"
  New-Item -ItemType Directory -Path $dest -Force | Out-Null
  Copy-Item -LiteralPath (Join-Path $Source 'Invoke-ItcmdrAgent.ps1') -Destination $dest -Force
  Copy-Item -LiteralPath (Join-Path $Source 'Install-ItcmdrAgent.ps1') -Destination $dest -Force
  Copy-Item -LiteralPath (Join-Path $Source 'Poll-ItcmdrRequests.ps1') -Destination $dest -Force
  foreach ($f in @('Itcmdr.Checks.ps1', 'Itcmdr.PluginRunner.ps1', 'Itcmdr.Plugin.psm1', 'Set-ItcmdrCutover.ps1')) {
    Copy-Item -LiteralPath (Join-Path $Source $f) -Destination $dest -Force
  }
  # Built-in check plugins: the only plugin copies agents run (installed, never downloaded).
  $plugins = Join-Path $Source 'plugins'
  if (-not (Test-Path -LiteralPath $plugins)) { $plugins = Join-Path (Join-Path (Join-Path (Join-Path $Source '..') 'third_party') 'itcmdr-plugins') 'plugins' }
  foreach ($d in @(Get-ChildItem -LiteralPath $plugins -Directory -ErrorAction SilentlyContinue)) {
    $to = Join-Path (Join-Path $dest 'plugins') $d.Name
    New-Item -ItemType Directory -Path $to -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $d.FullName 'main.ps1') -Destination $to -Force
  }
}
}

process {
foreach ($c in $ComputerName) {
  if ($PSCmdlet.ShouldProcess($c, 'Push itcmdr agent')) {
    Copy-Agent -c $c -Source $SourceDir
    $taskName = 'itcmdr Bootstrap'
    $tr = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\Windows\itcmdr\agent\Install-ItcmdrAgent.ps1'
    schtasks /S $c /Create /RU SYSTEM /SC ONCE /ST 00:00 /TN $taskName /TR "$tr" /F | Out-Null
    schtasks /S $c /Run /TN $taskName | Out-Null
    Write-Output "Triggered install on $c"
  }
}
}
