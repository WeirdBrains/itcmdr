<#
.SYNOPSIS
  itcmdr nightly health check and optional repair agent.

.DESCRIPTION
  Runs as SYSTEM via Scheduled Task. Detects role (Server, RCU, Workstation) heuristically,
  loads site config from C:\ProgramData\itcmdr\site.json or ./config/site.json, executes
  checks, optionally performs bounded, idempotent repairs, and writes JSON Lines to
  <paths.localLogs>\<COMPUTER>_<yyyyMMdd-HHmmss>_<runId>.log (default C:\maintenanceRuns).

  Exceptions (allowlist) come from site.json "exceptions" plus the server-managed
  exceptions.json in the maintenanceRuns share (cached locally for offline runs).

  No site-specific values are hardcoded; everything comes from site.json.
  Compatible with Windows PowerShell 5.1 and PowerShell 7.

.PARAMETER ReportOnly
  Do not attempt any repairs. Default: report-only when not running as SYSTEM.

.PARAMETER Repair
  Attempt repairs even when not running as SYSTEM (ignored if -ReportOnly is given).

.PARAMETER Force
  Override the guard that skips repairs when clinical activity may be in progress.

.PARAMETER Categories
  Optional subset of categories to run: network, connectivity, configuration, backup, services.

.PARAMETER Trigger
  What started this run: task (the SYSTEM nightly task, default), schedule (the server's default
  nightly schedule, via the poller), fallback (poller, server unreachable after the switch),
  on-demand. Recorded in run-start; task/schedule/fallback count as tonight's nightly.

.NOTES
  Built-in checks as plugins (v0.11.0, agent.pluginMode): before the cutover the built-in checks
  are the run and the plugin versions run first, report-only, for a parity line; after it the
  plugins are the run and a plugin that can't run falls back to its built-in check function.
#>
[CmdletBinding()]
param(
  [switch]$ReportOnly,
  [switch]$Repair,
  [switch]$Force,
  [string[]]$Categories,
  [ValidateSet('task', 'schedule', 'fallback', 'on-demand')][string]$Trigger = 'task'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:AgentVersion = '0.14.0'

# The check library (shared with the built-in check plugins).
. (Join-Path $PSScriptRoot 'Itcmdr.Checks.ps1')
. (Join-Path $PSScriptRoot 'Itcmdr.PluginRunner.ps1')

if (-not $env:ITCMDR_IMPORT_ONLY) {
  $site = Get-ItcmdrSiteConfig
  $script:ItcmdrSite = $site
  $ctx = New-ItcmdrRunContext -Site $site -Categories $Categories
  # Not Windows (tests on Linux): never SYSTEM.
  $isSystem = $false
  try { $isSystem = [bool]([Security.Principal.WindowsIdentity]::GetCurrent().IsSystem) } catch { $isSystem = $false }
  $quietOk = Test-ItcmdrClinicalQuiet -Site $site -QuietMinutes ([int](Get-ItcmdrSetting -Site $site -Key 'agent.quietMinutes'))
  $reportOnlyEffective = Resolve-ItcmdrRepairMode -ReportOnlyGiven:$PSBoundParameters.ContainsKey('ReportOnly') -ReportOnly:$ReportOnly -Repair:$Repair -IsSystem:$isSystem -Quiet:$quietOk -Force:$Force

  $schedState = $null; $cut = $null
  try { $schedState = Get-ItcmdrScheduleState -Site $site } catch { Write-Verbose "schedule state: $_" }
  try { $cut = Sync-ItcmdrCutover -Site $site -ScheduleState $schedState } catch { $cut = Get-ItcmdrCutoverState -Site $site }
  $pluginMode = Resolve-ItcmdrPluginMode -Site $site -CutoverState $cut.State
  $ssTs = if ($schedState -and $schedState.Ts) { $schedState.Ts.ToString('o') } else { $null }

  Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
    type = 'run-start'
    runId = $ctx.RunId
    computer = $ctx.Computer
    role = $ctx.Role
    reportOnly = $reportOnlyEffective
    categories = $ctx.Categories
    started = (Get-Date).ToString('o')
    agentVersion = $script:AgentVersion
    trigger = $Trigger
    cutover = $cut.State
    scheduleStateTs = $ssTs
    pluginMode = $pluginMode
  })

  $builtinFns = @($script:ItcmdrBuiltinPlugins.Values)
  if ($pluginMode -eq 'plugins') {
    # After the switch: the plugin versions are the run (their records go into this log).
    try {
      $pr = Invoke-ItcmdrBuiltinPluginRun -Site $site -ReportOnly:$reportOnlyEffective -AgentLog $ctx.LogFile -RunId $ctx.RunId -Categories $ctx.Categories -Force:$Force -Quiet $quietOk -Role $ctx.Role -Computer $ctx.Computer
    } catch {
      $err = "runner: $($_.ToString())"
      $pr = @{ Results = @(); Ran = @(); Errors = @($script:ItcmdrBuiltinPlugins.Keys | ForEach-Object { [pscustomobject]@{ Plugin = $_; Error = $err } }) }
    }
    $failedPlugins = @($pr.Errors | ForEach-Object { $_.Plugin })
    $builtinFns = @($failedPlugins | ForEach-Object { $script:ItcmdrBuiltinPlugins[$_] })
    foreach ($e in @($pr.Errors)) {
      Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{ type = 'plugin-fallback'; runId = $ctx.RunId; computer = $ctx.Computer; plugin = $e.Plugin; error = $e.Error; ts = (Get-Date).ToString('o') })
    }
  } elseif ($pluginMode -eq 'parallel') {
    # Before the switch: plugin versions first, report-only, so they see the pre-repair state.
    $pr = $null
    try { $pr = Invoke-ItcmdrBuiltinPluginRun -Site $site -ReportOnly -RunId $ctx.RunId -Categories $ctx.Categories -Role $ctx.Role -Computer $ctx.Computer } catch { $pr = @{ Results = @(); Ran = @(); Errors = @([pscustomobject]@{ Plugin = '*'; Error = $_.ToString() }) } }
  }

  foreach ($fn in $builtinFns) { & $fn -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective }

  if ($pluginMode -eq 'parallel' -and $pr) {
    $builtin = @(Get-Content -LiteralPath $ctx.LogFile | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.type -eq 'check' -and $_.runId -eq $ctx.RunId })
    $cmp = Compare-ItcmdrParity -BuiltinRecords $builtin -PluginResults $pr.Results
    Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
      type = 'parity'; runId = $ctx.RunId; computer = $ctx.Computer; ts = (Get-Date).ToString('o')
      matched = $cmp.Matched; mismatched = @($cmp.Mismatched); errors = @($pr.Errors); plugins = @($pr.Ran)
    })
  }

  if ($Trigger -in @('task', 'schedule', 'fallback')) {
    try { Set-ItcmdrNightlyMarker -Site $site -Trigger $Trigger -RunId $ctx.RunId } catch { Write-Verbose "marker: $_" }
  }

  $retentionDays = [int](Get-ItcmdrSetting -Site $site -Key 'retention.agentLocalDays')
  Get-ChildItem -LiteralPath (Split-Path -Parent $ctx.LogFile) -File -Filter '*.log' -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-$retentionDays) } |
    Remove-Item -Force -ErrorAction SilentlyContinue

  Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
    type = 'run-complete'
    runId = $ctx.RunId
    computer = $ctx.Computer
    finished = (Get-Date).ToString('o')
    exitCode = 0
  })

  Push-ItcmdrLog -Ctx $ctx
}
