<#
.SYNOPSIS
  itcmdr nightly health check and optional repair agent.

.DESCRIPTION
  Runs as SYSTEM via Scheduled Task. Detects role (Server, RCU, Workstation) heuristically,
  loads site config from C:\ProgramData\itcmdr\site.json or ./config/site.json, executes
  checks, optionally performs bounded, idempotent repairs, and writes JSON Lines to
  C:\maintenanceRuns\<COMPUTER>_<yyyyMMdd-HHmmss>_<runId>.log.

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
  Optional subset of categories to run: network, connectivity, services.
#>
[CmdletBinding()]
param(
  [switch]$ReportOnly,
  [switch]$Repair,
  [switch]$Force,
  [string[]]$Categories
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:AgentVersion = '0.2.0'

function ConvertTo-ItcmdrHashtable {
  param($InputObject)
  if ($null -eq $InputObject) { return $null }
  if ($InputObject -is [System.Collections.IDictionary]) {
    $h = @{}
    foreach ($k in $InputObject.Keys) { $h[$k] = ConvertTo-ItcmdrHashtable $InputObject[$k] }
    return $h
  }
  if ($InputObject -is [System.Management.Automation.PSCustomObject]) {
    $h = @{}
    foreach ($p in $InputObject.PSObject.Properties) { $h[$p.Name] = ConvertTo-ItcmdrHashtable $p.Value }
    return $h
  }
  if ($InputObject -is [System.Collections.IEnumerable] -and $InputObject -isnot [string]) {
    return , @($InputObject | ForEach-Object { ConvertTo-ItcmdrHashtable $_ })
  }
  return $InputObject
}

function ConvertFrom-ItcmdrJson {
  # ConvertFrom-Json -AsHashtable does not exist in Windows PowerShell 5.1.
  param([string]$Text)
  return ConvertTo-ItcmdrHashtable ($Text.TrimStart([char]0xFEFF) | ConvertFrom-Json)
}

function Get-ItcmdrValue {
  <# Safe nested lookup (StrictMode throws on missing keys/properties). #>
  param($Obj, [string[]]$Path)
  $cur = $Obj
  foreach ($n in $Path) {
    if ($null -eq $cur) { return $null }
    if ($cur -is [System.Collections.IDictionary]) {
      if (-not $cur.Contains($n)) { return $null }
      $cur = $cur[$n]
    } else {
      $prop = $cur.PSObject.Properties[$n]
      if (-not $prop) { return $null }
      $cur = $prop.Value
    }
  }
  return $cur
}

function Get-ItcmdrSiteConfig {
  $candidates = @(
    (Join-Path -Path (Get-Location) -ChildPath 'config/site.json'),
    'C:\ProgramData\itcmdr\site.json'
  )
  foreach ($p in $candidates) {
    if (Test-Path -LiteralPath $p) {
      try {
        return ConvertFrom-ItcmdrJson (Get-Content -LiteralPath $p -Raw)
      } catch {
        throw "Failed to parse site config at $($p): $($_)"
      }
    }
  }
  throw 'Site config not found. Provide config/site.json or C:\ProgramData\itcmdr\site.json.'
}

function Get-ItcmdrRole {
  $hostname = $env:COMPUTERNAME
  if ($hostname -match 'SERVER|DC|SRV') { return 'Server' }
  if ($hostname -match 'RCU') { return 'RCU' }
  return 'Workstation'
}

function Get-ItcmdrException {
  <#
    Exceptions from site.json plus the server export (exceptions.json in the share).
    The share copy is cached to CacheDir so offline runs still honour it.
  #>
  param([hashtable]$Site, [string]$CacheDir = 'C:\ProgramData\itcmdr')
  $rules = @()
  if ((Get-ItcmdrValue $Site @('exceptions'))) { $rules += @((Get-ItcmdrValue $Site @('exceptions'))) }
  $cache = Join-Path $CacheDir 'exceptions.json'
  $text = $null
  $share = if ((Get-ItcmdrValue $Site @('shares'))) { (Get-ItcmdrValue $Site @('shares','maintenanceRuns')) } else { $null }
  if ($share) {
    $remote = Join-Path $share 'exceptions.json'
    try {
      if (Test-Path -LiteralPath $remote) {
        $text = Get-Content -LiteralPath $remote -Raw
        if (-not (Test-Path -LiteralPath $CacheDir)) { New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null }
        Set-Content -LiteralPath $cache -Value $text -Encoding UTF8
      }
    } catch { $text = $null }
  }
  if (-not $text -and (Test-Path -LiteralPath $cache)) { $text = Get-Content -LiteralPath $cache -Raw }
  if ($text) {
    try {
      $doc = ConvertFrom-ItcmdrJson $text
      if ($doc -is [hashtable] -and $doc.ContainsKey('rules')) { $rules += @($doc.rules) } else { $rules += @($doc) }
    } catch { Write-Warning "Ignoring unreadable exceptions.json: $_" }
  }
  return , @($rules | Where-Object { $_ })
}

function Find-ItcmdrException {
  <#
    First rule matching this machine/role and check id that hasn't expired, or $null.
    A rule with no machines and no role applies to every machine.
  #>
  param([object[]]$Rules, [string]$Id, [string]$Computer, [string]$Role, [datetime]$Now = (Get-Date))
  foreach ($ex in @($Rules)) {
    if (-not $ex) { continue }
    $scope = (Get-ItcmdrValue $ex @('scope'))
    $machines = @()
    $scopeRole = $null
    if ($scope) {
      $machines = @((Get-ItcmdrValue $scope @('machines')) | Where-Object { $_ })
      $scopeRole = (Get-ItcmdrValue $scope @('role'))
    }
    $applies = ($machines.Count -eq 0 -and -not $scopeRole)
    if ($machines -contains $Computer) { $applies = $true }
    if ($scopeRole -and $scopeRole -eq $Role) { $applies = $true }
    if (-not $applies) { continue }
    $checkId = (Get-ItcmdrValue $ex @('checkId'))
    $pattern = (Get-ItcmdrValue $ex @('checkPattern'))
    if (-not $checkId -and -not $pattern) { continue }
    if ($checkId -and $checkId -ne $Id) { continue }
    if ($pattern -and ($Id -notmatch $pattern)) { continue }
    $expires = (Get-ItcmdrValue $ex @('expires'))
    if ($expires) {
      $exp = [datetime]::Parse([string]$expires, [Globalization.CultureInfo]::InvariantCulture)
      if ($exp.TimeOfDay -eq [TimeSpan]::Zero -and ([string]$expires).Length -le 10) { $exp = $exp.AddDays(1) }
      if ($exp -le $Now) { continue }
    }
    return $ex
  }
  return $null
}

function New-ItcmdrRunContext {
  [CmdletBinding(SupportsShouldProcess)]
  param([hashtable]$Site, [string[]]$Categories)
  $runId = [guid]::NewGuid().ToString()
  $ts = Get-Date
  $computer = $env:COMPUTERNAME
  $logRoot = 'C:\maintenanceRuns'
  if (-not (Test-Path -LiteralPath $logRoot) -and $PSCmdlet.ShouldProcess($logRoot, 'Create log directory')) {
    New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
  }
  $stamp = $ts.ToString('yyyyMMdd-HHmmss')
  $role = Get-ItcmdrRole
  return @{
    Site = $Site
    RunId = $runId
    Started = $ts
    Computer = $computer
    LogFile = (Join-Path $logRoot "$computer`_$stamp`_$runId.log")
    Role = $role
    Categories = @($Categories | Where-Object { $_ })
    Exceptions = (Get-ItcmdrException -Site $Site)
  }
}

function Push-ItcmdrLog {
  param([hashtable]$Ctx)
  $share = (Get-ItcmdrValue $Ctx.Site @('shares','maintenanceRuns'))
  if (-not $share) { return }
  $spool = 'C:\ProgramData\itcmdr\spool\outbox'
  if (-not (Test-Path $spool)) { New-Item -ItemType Directory -Path $spool -Force | Out-Null }
  $toPush = @($Ctx.LogFile) + @(Get-ChildItem -LiteralPath $spool -Filter '*.log' -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
  foreach ($f in $toPush) {
    $dest = Join-Path $share ([IO.Path]::GetFileName($f))
    try {
      Copy-Item -LiteralPath $f -Destination $dest -Force
      if ($f -ne $Ctx.LogFile) { Remove-Item -LiteralPath $f -Force -ErrorAction SilentlyContinue }
    } catch {
      $queued = Join-Path $spool ([IO.Path]::GetFileName($f))
      if ($f -ne $queued) { Copy-Item -LiteralPath $f -Destination $queued -Force }
    }
  }
}

function Write-ItcmdrJsonLine {
  param([object]$Record, [string]$Path)
  $json = $Record | ConvertTo-Json -Compress -Depth 6
  Add-Content -LiteralPath $Path -Value $json -Encoding UTF8
}

function Invoke-ItcmdrCheck {
  param(
    [string]$Id,
    [string]$Category,
    [scriptblock]$Test,
    [scriptblock]$Repair,  # omit when there is no safe automatic repair
    [switch]$ReportOnlyLocal,
    [hashtable]$Ctx
  )
  $cats = @()
  if ($Ctx.ContainsKey('Categories')) { $cats = @($Ctx.Categories | Where-Object { $_ }) }
  if ($cats.Count -gt 0 -and $cats -notcontains $Category) { return }

  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $status = 'ok'
  $severity = $null
  $detail = ''
  $attempted = $false
  $action = ''
  $result = 'skipped'
  $rules = @()
  if ($Ctx.ContainsKey('Exceptions') -and $Ctx.Exceptions) { $rules += @($Ctx.Exceptions) }
  elseif ($Ctx.Site -and (Get-ItcmdrValue $Ctx.Site @('exceptions'))) { $rules += @((Get-ItcmdrValue $Ctx.Site @('exceptions'))) }
  $rule = Find-ItcmdrException -Rules $rules -Id $Id -Computer $Ctx.Computer -Role $Ctx.Role
  $excepted = [bool]$rule
  $exceptionReason = if ($rule) { (Get-ItcmdrValue $rule @('reason')) } else { $null }
  $actionOverride = if ($rule) { (Get-ItcmdrValue $rule @('action')) } else { $null }

  try {
    $passed = & $Test
    if (-not $passed) {
      $status = 'failed'
      if ($excepted) {
        switch -regex ($actionOverride) {
          '^ignore$' { $status = 'ok' }
          '^downgrade:warn$' { $status = 'failed'; $severity = 'warning' }
          '^downgrade:info$' { $status = 'ok' }
          default {}
        }
      } elseif (-not $ReportOnlyLocal -and $Repair) {
        $attempted = $true
        $action = 'repair'
        $ok = & $Repair
        if ($ok) {
          $status = 'ok'
          $severity = $null
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
    severity = if ($severity) { $severity } elseif ($status -eq 'failed') { 'warning' } else { 'info' }
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

# Clinical activity guard: recent writes to the RCU data path mean imaging may be in use.
function Test-ItcmdrClinicalQuiet {
  param([hashtable]$Site, [int]$QuietMinutes = 10)
  $p = $null
  if ((Get-ItcmdrValue $Site @('services')) -and (Get-ItcmdrValue $Site @('services','rcu'))) { $p = (Get-ItcmdrValue $Site @('services','rcu','dataPath')) }
  if (-not $p -or -not (Test-Path $p)) { return $true }
  $since = (Get-Date).AddMinutes(-[int]$QuietMinutes)
  $recent = Get-ChildItem -LiteralPath $p -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -gt $since } | Select-Object -First 1
  return -not [bool]$recent
}

function Resolve-ItcmdrRepairMode {
  <# Returns $true when repairs must be suppressed (report-only). #>
  param([bool]$ReportOnlyGiven, [bool]$ReportOnly, [bool]$Repair, [bool]$IsSystem, [bool]$Quiet, [bool]$Force)
  $reportOnly = if ($ReportOnlyGiven) { $ReportOnly } elseif ($Repair) { $false } else { -not $IsSystem }
  if (-not $reportOnly -and -not $Quiet -and -not $Force) { $reportOnly = $true }
  return $reportOnly
}

# Network checks and repairs
function Invoke-ItcmdrNetworkCheck {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  $site = $Ctx.Site
  $expectedDns = @((Get-ItcmdrValue $site @('network','dnsServers')))
  $expectedGw = (Get-ItcmdrValue $site @('network','defaultGateway'))
  $lanCidr = (Get-ItcmdrValue $site @('network','lanCidr'))
  $server = (Get-ItcmdrValue $site @('network','serverHostname'))
  $domainSuffix = (Get-ItcmdrValue $site @('network','domainSuffix'))

  Invoke-ItcmdrCheck -Id 'NET-IP-CORRECT' -Category 'network' -Test {
    $ok = $true
    $ipv4 = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.ValidLifetime -ne ([TimeSpan]::Zero) -and $_.IPAddress -ne '127.0.0.1' })
    if (@($ipv4 | Where-Object { $_.IPAddress -match '^169\.254\.' }).Count -gt 0) { $ok = $false }
    if ($lanCidr) {
      $prefix = $lanCidr.Split('/')[0]
      $net = $prefix.Substring(0, $prefix.LastIndexOf('.') + 1)
      if (@($ipv4 | Where-Object { $_.IPAddress -like "$net*" }).Count -eq 0) { $ok = $false }
    }
    if ($expectedDns) {
      $cfgDns = (Get-DnsClientServerAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Select-Object -ExpandProperty ServerAddresses)
      foreach ($d in $expectedDns) { if ($cfgDns -notcontains $d) { $ok = $false } }
    }
    if ($expectedGw) {
      $gws = @(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty NextHop)
      if ($gws -notcontains $expectedGw) { $ok = $false }
    }
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

  $ports = @()
  if ((Get-ItcmdrValue $site @('services')) -and (Get-ItcmdrValue $site @('services','sidexisRestService'))) { $ports += @((Get-ItcmdrValue $site @('services','sidexisRestService','ports'))) }
  $ports += @(445, 53)
  foreach ($port in $ports) {
    Invoke-ItcmdrCheck -Id "NET-SERVER-TCP-$port" -Category 'connectivity' -Test {
      Test-NetConnection -ComputerName $server -Port $port -WarningAction SilentlyContinue | Select-Object -ExpandProperty TcpTestSucceeded
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  if ((Get-ItcmdrValue $site @('shares')) -and (Get-ItcmdrValue $site @('shares','pdata'))) {
    Invoke-ItcmdrCheck -Id 'SHARE-PDATA' -Category 'connectivity' -Test {
      Test-Path -LiteralPath (Get-ItcmdrValue $site @('shares','pdata'))
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  foreach ($p in @((Get-ItcmdrValue $site @('printers')) | Where-Object { $_ })) {
    $addr = (Get-ItcmdrValue $p @('address'))
    Invoke-ItcmdrCheck -Id "PRN-$(Get-ItcmdrValue $p @('name'))-9100" -Category 'connectivity' -Test {
      Test-NetConnection -ComputerName $addr -Port 9100 -WarningAction SilentlyContinue | Select-Object -ExpandProperty TcpTestSucceeded
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }
}

# Services
function Invoke-ItcmdrServiceCheck {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  $site = $Ctx.Site
  if ($Ctx.Role -eq 'Server') {
    Invoke-ItcmdrCheck -Id 'SVC-SidexisRestService' -Category 'services' -Test {
      (Get-Service -Name (Get-ItcmdrValue $site @('services','sidexisRestService','name')) -ErrorAction SilentlyContinue).Status -eq 'Running'
    } -Repair {
      try { Start-Service -Name (Get-ItcmdrValue $site @('services','sidexisRestService','name')) -ErrorAction Stop; $true } catch { $false }
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  } else {
    Invoke-ItcmdrCheck -Id 'SIRONA-PLUGIN-HEALTH' -Category 'services' -Test {
      [bool](Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue)
    } -Repair {
      try {
        Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Service -Name (Get-ItcmdrValue $site @('services','sironaUsbService','name')) -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 3
        [bool](Get-Process -Name 'Sirona.Schick.SidexisPlugin.ServiceHost' -ErrorAction SilentlyContinue)
      } catch { $false }
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }
}

if (-not $env:ITCMDR_IMPORT_ONLY) {
  $site = Get-ItcmdrSiteConfig
  $ctx = New-ItcmdrRunContext -Site $site -Categories $Categories
  $isSystem = [bool]([Security.Principal.WindowsIdentity]::GetCurrent().IsSystem)
  $quietOk = Test-ItcmdrClinicalQuiet -Site $site -QuietMinutes 10
  $reportOnlyEffective = Resolve-ItcmdrRepairMode -ReportOnlyGiven:$PSBoundParameters.ContainsKey('ReportOnly') -ReportOnly:$ReportOnly -Repair:$Repair -IsSystem:$isSystem -Quiet:$quietOk -Force:$Force

  Write-ItcmdrJsonLine -Path $ctx.LogFile -Record ([pscustomobject]@{
    type = 'run-start'
    runId = $ctx.RunId
    computer = $ctx.Computer
    role = $ctx.Role
    reportOnly = $reportOnlyEffective
    categories = $ctx.Categories
    started = (Get-Date).ToString('o')
    agentVersion = $script:AgentVersion
  })

  Invoke-ItcmdrNetworkCheck -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective
  Invoke-ItcmdrServiceCheck -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective

  $retentionDays = if ((Get-ItcmdrValue $site @('retention')) -and (Get-ItcmdrValue $site @('retention','agentLocalDays'))) { [int](Get-ItcmdrValue $site @('retention','agentLocalDays')) } else { 400 }
  Get-ChildItem -LiteralPath 'C:\maintenanceRuns' -File -Filter '*.log' -ErrorAction SilentlyContinue |
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
