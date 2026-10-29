<#
.SYNOPSIS
  itcmdr nightly health check and optional repair agent.

.DESCRIPTION
  Runs as SYSTEM via Scheduled Task. Detects role (Server, RCU, Workstation) heuristically,
  loads site config from C:\\ProgramData\\itcmdr\\site.json or ./config/site.json, executes
  checks, optionally performs bounded, idempotent repairs, and writes JSON Lines to
  C:\\maintenanceRuns\\<COMPUTER>_<yyyyMMdd-HHmmss>_<runId>.log.

  No site-specific values are hardcoded; everything comes from site.json.

.PARAMETER ReportOnly
  If set, do not attempt any repairs. Default: $true when not running as SYSTEM.

.PARAMETER Force
  Override guardrails that skip disruptive actions when clinical activity may be active.

.PARAMETER Categories
  Optional subset of categories to run (e.g., connectivity, configuration).
#>
[CmdletBinding()]
param(
  [switch]$ReportOnly,
  [switch]$Force,
  [string[]]$Categories
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ItcmdrSiteConfig {
  $candidates = @(
    (Join-Path -Path (Get-Location) -ChildPath 'config/site.json'),
    'C:\ProgramData\itcmdr\site.json'
  )
  foreach ($p in $candidates) {
    if (Test-Path -LiteralPath $p) {
      try {
        return Get-Content -LiteralPath $p -Raw | ConvertFrom-Json -AsHashtable
      } catch {
        throw \"Failed to parse site config at $p: $_\"
      }
    }
  }
  throw 'Site config not found. Provide config/site.json or C:\ProgramData\itcmdr\site.json.'
}

function Get-ItcmdrRole {
  # Heuristic role detection (override via site config later if needed)
  $hostname = $env:COMPUTERNAME
  if ($hostname -match 'SERVER|DC|SRV') { return 'Server' }
  if ($hostname -match 'RCU') { return 'RCU' }
  return 'Workstation'
}

function New-ItcmdrRunContext {
  param([hashtable]$Site)
  $runId = [guid]::NewGuid().ToString()
  $ts = Get-Date
  $computer = $env:COMPUTERNAME
  $logRoot = 'C:\maintenanceRuns'
  if (-not (Test-Path -LiteralPath $logRoot)) {
    New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
  }
  $stamp = $ts.ToString('yyyyMMdd-HHmmss')
  $logFile = Join-Path $logRoot \"$computer`_$stamp`_$runId.log\"
  return [pscustomobject]@{
    Site = $Site
    RunId = $runId
    Started = $ts
    Computer = $computer
    LogFile = $logFile
    Role = Get-ItcmdrRole
  }
}

function Write-ItcmdrJsonLine {
  param([object]$Record, [string]$Path)
  $json = $Record | ConvertTo-Json -Compress -Depth 6
  Add-Content -LiteralPath $Path -Value $json
}

function Invoke-ItcmdrCheck {
  param(
    [string]$Id,
    [string]$Category,
    [scriptblock]$Test,
    [scriptblock]$Repair,
    [switch]$ReportOnlyLocal,
    [hashtable]$Ctx
  )
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $status = 'ok'
  $detail = ''
  $attempted = $false
  $action = ''
  $result = 'skipped'
  # Determine if an exception applies
  $excepted = $false
  $exceptionReason = $null
  $actionOverride = $null
  if ($Ctx.Site.exceptions) {
    foreach ($ex in $Ctx.Site.exceptions) {
      $applies = $false
      if ($ex.scope) {
        if ($ex.scope.machines) {
          if ($ex.scope.machines -contains $Ctx.Computer) { $applies = $true }
        }
        if ($ex.scope.role) {
          if ($ex.scope.role -eq $Ctx.Role) { $applies = $true }
        }
      }
      if (-not $applies) { continue }
      if ($ex.checkId -and $ex.checkId -ne $Id) { continue }
      if ($ex.checkPattern -and ($Id -notmatch $ex.checkPattern)) { continue }
      if ($ex.expires) {
        $exp = [datetime]::Parse($ex.expires)
        if ($exp -lt (Get-Date)) { continue }
      }
      $excepted = $true
      $exceptionReason = $ex.reason
      $actionOverride = $ex.action
      break
    }
  }

  try {
    $passed = & $Test
    if (-not $passed) {
      $status = 'failed'
      if ($excepted) {
        # downgrade or ignore
        switch -regex ($actionOverride) {
          '^ignore$' { $status = 'ok' }
          '^downgrade:warn$' { $status = 'failed'; $severity = 'warning' }
          '^downgrade:info$' { $status = 'ok' }
          default {}
        }
      } elseif (-not $ReportOnlyLocal) {
        $attempted = $true
        $action = 'repair'
        $ok = & $Repair
        if ($ok) {
          $status = 'ok'
          $result = 'success'
        } else {
          $result = 'failed'
        }
      }
    }
  } catch {
    $status = 'failed'
    $detail = $_.ToString()
  } finally {
    $sw.Stop()
  }
  $rec = [pscustomobject]@{
    type = 'check'
    runId = $Ctx.RunId
    computer = $Ctx.Computer
    category = $Category
    checkId = $Id
    severity = if ($status -eq 'failed') { 'warning' } else { 'info' }
    status = $status
    detail = $detail
    attemptedRepair = $attempted
    action = $action
    result = $result
    durationMs = [int]$sw.ElapsedMilliseconds
    ts = (Get-Date).ToString('o')
    excepted = $excepted
    exceptionReason = $exceptionReason
  }
  Write-ItcmdrJsonLine -Record $rec -Path $Ctx.LogFile
}

# Main
$site = Get-ItcmdrSiteConfig
$ctx = New-ItcmdrRunContext -Site $site
$isSystem = [bool]([Security.Principal.WindowsIdentity]::GetCurrent().IsSystem)
$reportOnlyEffective = if ($PSBoundParameters.ContainsKey('ReportOnly')) { $ReportOnly } else { -not $isSystem }

# Write run-start
Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
  type = 'run-start'
  runId = $ctx.RunId
  computer = $ctx.Computer
  role = $ctx.Role
  reportOnly = $reportOnlyEffective
  started = (Get-Date).ToString('o')
  agentVersion = '0.1.0'
})

# Example connectivity check (placeholder; config-driven in future commits)
Invoke-ItcmdrCheck -Id 'NET-SELF-LOOPBACK' -Category 'connectivity' -Test {
  Test-Connection -TargetName '127.0.0.1' -Count 1 -Quiet
} -Repair { $true } -ReportOnlyLocal:$reportOnlyEffective -Ctx $ctx

# Retention: prune local logs older than site.retention.agentLocalDays (default 400)
$retentionDays = if ($site.retention.agentLocalDays) { [int]$site.retention.agentLocalDays } else { 400 }
Get-ChildItem -LiteralPath 'C:\maintenanceRuns' -File -Filter '*.log' -ErrorAction SilentlyContinue |
  Where-Object { $_.LastWriteTime -lt (Get-Date).AddDays(-$retentionDays) } |
  Remove-Item -Force -ErrorAction SilentlyContinue

# Run-complete
Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
  type = 'run-complete'
  runId = $ctx.RunId
  computer = $ctx.Computer
  finished = (Get-Date).ToString('o')
  exitCode = 0
})

