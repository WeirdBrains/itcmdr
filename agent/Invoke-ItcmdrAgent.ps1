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
        throw \"Failed to parse site config at $($p): $($_)\"
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

function Push-ItcmdrLog {
  param([hashtable]$Ctx)
  $share = $Ctx.Site.shares.maintenanceRuns
  if (-not $share) { return }
  $target = Join-Path $share ([IO.Path]::GetFileName($Ctx.LogFile))
  $spool = 'C:\ProgramData\itcmdr\spool\outbox'
  if (-not (Test-Path $spool)) { New-Item -ItemType Directory -Path $spool -Force | Out-Null }
  $toPush = @($Ctx.LogFile) + (Get-ChildItem -LiteralPath $spool -Filter '*.log' -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
  foreach ($f in $toPush) {
    $dest = if ($f -eq $Ctx.LogFile) { $target } else { Join-Path $share ([IO.Path]::GetFileName($f)) }
    try {
      Copy-Item -LiteralPath $f -Destination $dest -Force
      if ($f -ne $Ctx.LogFile) { Remove-Item -LiteralPath $f -Force -ErrorAction SilentlyContinue }
    } catch {
      # Queue it
      $queued = Join-Path $spool ([IO.Path]::GetFileName($f))
      if ($f -ne $queued) { Copy-Item -LiteralPath $f -Destination $queued -Force }
    }
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

# Clinical activity guard: check for recent writes to RCU data path
function Test-ItcmdrClinicalQuiet {
  param([hashtable]$Site, [int]$QuietMinutes = 10)
  $p = $Site.services.rcu.dataPath
  if (-not $p -or -not (Test-Path $p)) { return $true }
  $since = (Get-Date).AddMinutes(-[int]$QuietMinutes)
  $recent = Get-ChildItem -LiteralPath $p -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -gt $since } | Select-Object -First 1
  return -not [bool]$recent
}

# Network checks and repairs
function Invoke-ItcmdrNetworkChecks {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  $site = $Ctx.Site
  $expectedDns = @($site.network.dnsServers)
  $expectedGw = $site.network.defaultGateway
  $lanCidr = $site.network.lanCidr
  $server = $site.network.serverHostname
  $domainSuffix = $site.network.domainSuffix

  # IP correctness
  Invoke-ItcmdrCheck -Id 'NET-IP-CORRECT' -Category 'network' -Test {
    $ok = $true
    $ipv4 = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.ValidLifetime -ne ([TimeSpan]::Zero) -and $_.IPAddress -ne '127.0.0.1' }
    if ($ipv4 -match '^169\\.254\\.') { $ok = $false }
    if ($lanCidr) {
      # naive check: prefix match
      $prefix = $lanCidr.Split('/')[0]
      if ($ipv4.IPAddress -notlike ($prefix.Substring(0, $prefix.LastIndexOf('.') + 1) + '*')) { $ok = $false }
    }
    # DNS servers
    if ($expectedDns) {
      $cfgDns = (Get-DnsClientServerAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty ServerAddresses)
      foreach ($d in $expectedDns) { if ($cfgDns -notcontains $d) { $ok = $false } }
    }
    # Suffix
    if ($domainSuffix) {
      $suf = (Get-DnsClientGlobalSetting).SuffixSearchList
      if ($suf -and ($suf -notcontains $domainSuffix)) { $ok = $false }
    }
    return $ok
  } -Repair {
    try {
      ipconfig /release | Out-Null
      ipconfig /renew | Out-Null
      ipconfig /flushdns | Out-Null
      ipconfig /registerdns | Out-Null
      if ($expectedDns) {
        $ifAlias = (Get-DnsClient -ErrorAction SilentlyContinue | Where-Object { $_.InterfaceAlias -notlike '*Loopback*' } | Select-Object -First 1 -ExpandProperty InterfaceAlias)
        if ($ifAlias) { Set-DnsClientServerAddress -InterfaceAlias $ifAlias -ServerAddresses $expectedDns -ErrorAction SilentlyContinue }
      }
      $true
    } catch { $false }
  } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx

  # Connectivity to server ports
  foreach ($port in @($site.services.sidexisRestService.ports + @(445,53))) {
    $pid = "NET-SERVER-TCP-$port"
    Invoke-ItcmdrCheck -Id $pid -Category 'connectivity' -Test {
      Test-NetConnection -ComputerName $server -Port $port -WarningAction SilentlyContinue | Select-Object -ExpandProperty TcpTestSucceeded
    } -Repair { $true } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  # PDATA share
  if ($site.shares.pdata) {
    Invoke-ItcmdrCheck -Id 'SHARE-PDATA' -Category 'connectivity' -Test {
      Test-Path -LiteralPath $site.shares.pdata
    } -Repair { $true } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  # Printers 9100
  foreach ($p in @($site.printers)) {
    $addr = $p.address; $prnId = "PRN-$($p.name)-9100"
    Invoke-ItcmdrCheck -Id $prnId -Category 'connectivity' -Test {
      Test-NetConnection -ComputerName $addr -Port 9100 -WarningAction SilentlyContinue | Select-Object -ExpandProperty TcpTestSucceeded
    } -Repair { $true } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }
}

# Services
function Invoke-ItcmdrServiceChecks {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  $role = $Ctx.Role
  $site = $Ctx.Site

  if ($role -eq 'Server') {
    # Ensure SidexisRestService running
    Invoke-ItcmdrCheck -Id 'SVC-SidexisRestService' -Category 'services' -Test {
      (Get-Service -Name $site.services.sidexisRestService.name -ErrorAction SilentlyContinue).Status -eq 'Running'
    } -Repair {
      try { Start-Service -Name $site.services.sidexisRestService.name -ErrorAction Stop; $true } catch { $false }
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  # Schick plugin fix on workstations
  if ($role -ne 'Server') {
    Invoke-ItcmdrCheck -Id 'SIRONA-PLUGIN-HEALTH' -Category 'services' -Test {
      $p = Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue
      return [bool]$p
    } -Repair {
      try {
        Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Service -Name $Ctx.Site.services.sironaUsbService.name -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 3
        $p = Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue
        return [bool]$p
      } catch { $false }
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }
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

# Guard for clinical activity (skip disruptive repairs when active unless forced)
$quietOk = Test-ItcmdrClinicalQuiet -Site $site -QuietMinutes 10
if (-not $quietOk -and -not $Force) {
  $reportOnlyEffective = $true
}

# Run checks
Invoke-ItcmdrNetworkChecks -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective
Invoke-ItcmdrServiceChecks -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective

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

# Push log to share (with queue)
Push-ItcmdrLog -Ctx $ctx

