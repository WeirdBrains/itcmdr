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
  Optional subset of categories to run: network, connectivity, configuration, services.
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
$script:AgentVersion = '0.3.0'

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
    $res = & $Test
    $passed = $res
    $skipped = $false
    if ($res -is [hashtable] -and $res.ContainsKey('itcmdrResult')) {
      $passed = [bool]$res.ok
      $detail = [string]$res.detail
      $skipped = [bool]$res.skipped
    }
    if ($skipped) {
      $status = 'skipped'
    } elseif (-not $passed) {
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
      Test-ItcmdrTcp -Address $server -Port $port
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
      Test-ItcmdrTcp -Address $addr -Port 9100
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

function Test-ItcmdrTcp {
  <# TCP connect with a timeout (cross-platform; replaces Test-NetConnection). #>
  param([string]$Address, [int]$Port, [int]$TimeoutMs = 0)
  if (-not $Address) { return $false }
  if ($TimeoutMs -le 0) { $TimeoutMs = if ($env:ITCMDR_TCP_TIMEOUT_MS) { [int]$env:ITCMDR_TCP_TIMEOUT_MS } else { 3000 } }
  $client = New-Object System.Net.Sockets.TcpClient
  try {
    $ar = $client.BeginConnect($Address, $Port, $null, $null)
    if (-not $ar.AsyncWaitHandle.WaitOne($TimeoutMs)) { return $false }
    $client.EndConnect($ar)
    return $client.Connected
  } catch {
    return $false
  } finally {
    $client.Close()
  }
}

function ConvertTo-ItcmdrResult {
  <# Rich check result: Ok, Detail, Skipped (no decision possible, e.g. nobody logged on). #>
  param([bool]$Ok, [string]$Detail = '', [switch]$Skipped)
  return @{ ok = $Ok; detail = $Detail; skipped = [bool]$Skipped; itcmdrResult = $true }
}

function Test-ItcmdrDnsRecord {
  <#
    Name must resolve to every Expected address and to none of the Forbidden ones
    (e.g. a retired address that keeps re-registering). Resolver returns IPv4 strings.
  #>
  param([string]$Name, [string[]]$Expected = @(), [string[]]$Forbidden = @(), [scriptblock]$Resolver)
  try { $got = @(& $Resolver $Name | Where-Object { $_ } | ForEach-Object { [string]$_ }) } catch { $got = @() }
  if ($got.Count -eq 0) { return (ConvertTo-ItcmdrResult -Ok $false -Detail "$Name does not resolve") }
  $missing = @($Expected | Where-Object { $_ -and ($got -notcontains $_) })
  $bad = @($Forbidden | Where-Object { $_ -and ($got -contains $_) })
  $parts = @()
  if ($missing.Count) { $parts += "missing $($missing -join ', ')" }
  if ($bad.Count) { $parts += "stale $($bad -join ', ')" }
  if ($parts.Count) { return (ConvertTo-ItcmdrResult -Ok $false -Detail "$Name -> $($got -join ', '): $($parts -join '; ')") }
  return (ConvertTo-ItcmdrResult -Ok $true -Detail "$Name -> $($got -join ', ')")
}

function Get-ItcmdrUserSid {
  <# SIDs of loaded user hives (logged-on domain/local users), as seen by SYSTEM. #>
  if (-not (Test-Path 'Registry::HKEY_USERS')) { return @() }
  return @(Get-ChildItem 'Registry::HKEY_USERS' -ErrorAction SilentlyContinue |
      Where-Object { $_.PSChildName -match '^S-1-5-21-[\d-]+$' } | ForEach-Object { $_.PSChildName })
}

function Get-ItcmdrUserDriveMap {
  <# Persistent drive maps per loaded user hive: @{ sid; letter; remotePath }. #>
  $out = @()
  foreach ($sid in Get-ItcmdrUserSid) {
    $root = "Registry::HKEY_USERS\$sid\Network"
    foreach ($k in @(Get-ChildItem $root -ErrorAction SilentlyContinue)) {
      $rp = (Get-ItemProperty -Path $k.PSPath -Name RemotePath -ErrorAction SilentlyContinue).RemotePath
      $out += @{ sid = $sid; letter = $k.PSChildName.ToUpperInvariant(); remotePath = $rp }
    }
  }
  return , $out
}

function Test-ItcmdrDriveMap {
  <# Every logged-on user should have Letter mapped to Path. Skipped when nobody is logged on. #>
  param([string]$Letter, [string]$Path, [string[]]$UserSid = @(), [object[]]$Mapping = @())
  $L = $Letter.TrimEnd(':').ToUpperInvariant()
  if (@($UserSid).Count -eq 0) { return (ConvertTo-ItcmdrResult -Ok $true -Skipped -Detail 'no user logged on') }
  $wrong = @()
  foreach ($sid in $UserSid) {
    $m = @($Mapping | Where-Object { $_.sid -eq $sid -and $_.letter -eq $L }) | Select-Object -First 1
    if (-not $m) { $wrong += "$($sid): $($L): not mapped" }
    elseif ($m.remotePath -ne $Path) { $wrong += "$($sid): $($L): $($m.remotePath)" }
  }
  if ($wrong.Count) { return (ConvertTo-ItcmdrResult -Ok $false -Detail ("expected $($L): -> $Path; " + ($wrong -join '; '))) }
  return (ConvertTo-ItcmdrResult -Ok $true -Detail "$($L): -> $Path for $(@($UserSid).Count) user(s)")
}

function Get-ItcmdrUserPrinterConnection {
  <# Per-user printer connections: @{ sid; server; queue } from HKU\<sid>\Printers\Connections (",,SERVER,QUEUE"). #>
  $out = @()
  foreach ($sid in Get-ItcmdrUserSid) {
    foreach ($k in @(Get-ChildItem "Registry::HKEY_USERS\$sid\Printers\Connections" -ErrorAction SilentlyContinue)) {
      $parts = $k.PSChildName.Split(',', [StringSplitOptions]::RemoveEmptyEntries)
      if ($parts.Count -ge 2) { $out += @{ sid = $sid; server = $parts[0].ToUpperInvariant(); queue = $parts[1].ToUpperInvariant() } }
    }
  }
  return , $out
}

function Test-ItcmdrPrinterConnection {
  <# Workstations: every logged-on user should be connected to \\Server\Queue. #>
  param([string]$Server, [string]$Queue, [string[]]$UserSid = @(), [object[]]$Connection = @())
  if (@($UserSid).Count -eq 0) { return (ConvertTo-ItcmdrResult -Ok $true -Skipped -Detail 'no user logged on') }
  $S = $Server.ToUpperInvariant(); $Q = $Queue.ToUpperInvariant()
  $missing = @($UserSid | Where-Object { $sid = $_; -not @($Connection | Where-Object { $_.sid -eq $sid -and $_.server -eq $S -and $_.queue -eq $Q }).Count })
  if ($missing.Count) { return (ConvertTo-ItcmdrResult -Ok $false -Detail "\\$Server\$Queue missing for $($missing -join ', ')") }
  return (ConvertTo-ItcmdrResult -Ok $true -Detail "\\$Server\$Queue connected for $(@($UserSid).Count) user(s)")
}

function Test-ItcmdrPrintQueueHealth {
  <#
    Server: the queue exists, is shared when expected, isn't in an error state, and has no
    job stuck in error or older than StuckMinutes. Printer/Jobs are Get-Printer/Get-PrintJob objects.
  #>
  param($Printer, [object[]]$Job = @(), [bool]$ExpectShared = $true, [int]$StuckMinutes = 60, [datetime]$Now = (Get-Date))
  if (-not $Printer) { return (ConvertTo-ItcmdrResult -Ok $false -Detail 'queue not found') }
  $issues = @()
  if ($ExpectShared -and -not $Printer.Shared) { $issues += 'not shared' }
  $st = [string]$Printer.PrinterStatus
  if ($st -match 'Error|Offline|PaperJam|NoToner|PaperOut|UserIntervention') { $issues += "status $st" }
  $stuck = @($Job | Where-Object { ([string]$_.JobStatus -match 'Error|Blocked') -or ($_.SubmittedTime -and $_.SubmittedTime -lt $Now.AddMinutes(-$StuckMinutes)) })
  if ($stuck.Count) { $issues += "$($stuck.Count) stuck job(s)" }
  if ($issues.Count) { return (ConvertTo-ItcmdrResult -Ok $false -Detail ($issues -join '; ')) }
  return (ConvertTo-ItcmdrResult -Ok $true -Detail "ok ($(@($Job).Count) job(s))")
}

function Test-ItcmdrSironaDrift {
  <#
    Read-only: report config files that still reference retired addresses/hosts.
    Files are opened with FileShare.ReadWrite so running Sirona services are never disturbed.
    Never modifies anything.
  #>
  param([string]$Path, [string[]]$Pattern = @())
  if (-not (Test-Path -LiteralPath $Path)) { return (ConvertTo-ItcmdrResult -Ok $true -Skipped -Detail 'file not present') }
  try {
    $fs = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
    try { $text = (New-Object IO.StreamReader($fs)).ReadToEnd() } finally { $fs.Dispose() }
  } catch {
    return (ConvertTo-ItcmdrResult -Ok $false -Detail "unreadable: $($_.Exception.Message)")
  }
  $hits = @($Pattern | Where-Object { $_ -and $text.IndexOf($_, [StringComparison]::OrdinalIgnoreCase) -ge 0 })
  if ($hits.Count) { return (ConvertTo-ItcmdrResult -Ok $false -Detail "stale reference(s): $($hits.Count) pattern(s) matched; fix with vendor tools, not by hand") }
  return (ConvertTo-ItcmdrResult -Ok $true -Detail 'no stale references')
}

function Resolve-ItcmdrPathList {
  <# Expand %ENV% variables and wildcards. #>
  param([string[]]$Path)
  $out = @()
  foreach ($p in @($Path | Where-Object { $_ })) {
    $e = [Environment]::ExpandEnvironmentVariables($p)
    if ($e -match '[\*\?]') { $out += @(Get-ChildItem -Path $e -File -ErrorAction SilentlyContinue | ForEach-Object FullName) } else { $out += $e }
  }
  return , $out
}

function Get-ItcmdrSafeId {
  param([string]$Text)
  return ($Text -replace '[^A-Za-z0-9\.-]+', '_').Trim('_')
}

function Test-ItcmdrRoleMatch {
  <# An entry with no "roles" list applies to every role. #>
  param($Entry, [string]$Role)
  $roles = @(Get-ItcmdrValue $Entry @('roles') | Where-Object { $_ })
  return ($roles.Count -eq 0 -or $roles -contains $Role)
}

# Scanners/imaging devices: TCP reachability (report only). Devices that may be off
# (e.g. the 3D unit) are handled with an exception rule, not special code.
function Invoke-ItcmdrScannerCheck {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  foreach ($s in @(Get-ItcmdrValue $Ctx.Site @('scanners') | Where-Object { $_ })) {
    if (-not (Test-ItcmdrRoleMatch $s $Ctx.Role)) { continue }
    $name = Get-ItcmdrValue $s @('name'); $addr = Get-ItcmdrValue $s @('address')
    foreach ($port in @(Get-ItcmdrValue $s @('ports') | Where-Object { $_ })) {
      $p = [int]$port
      Invoke-ItcmdrCheck -Id "SCANNER-$(Get-ItcmdrSafeId $name)-TCP-$p" -Category 'connectivity' -Test {
        ConvertTo-ItcmdrResult -Ok (Test-ItcmdrTcp -Address $addr -Port $p) -Detail "$($name):$p"
      } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
    }
  }
}

# Configuration checks: DNS records, drive maps, printer queues, Sirona drift.
function Invoke-ItcmdrConfigCheck {
  param([hashtable]$Ctx, [switch]$ReportOnlyLocal)
  $site = $Ctx.Site
  $resolver = {
    param($n)
    if (Get-Command Resolve-DnsName -ErrorAction SilentlyContinue) {
      Resolve-DnsName -Name $n -Type A -DnsOnly -QuickTimeout -ErrorAction SilentlyContinue | Where-Object { $_.Type -eq 'A' } | ForEach-Object IPAddress
    } else {
      [System.Net.Dns]::GetHostAddresses($n) | Where-Object { $_.AddressFamily -eq 'InterNetwork' } | ForEach-Object { $_.ToString() }
    }
  }
  foreach ($r in @(Get-ItcmdrValue $site @('dnsRecords') | Where-Object { $_ })) {
    if (-not (Test-ItcmdrRoleMatch $r $Ctx.Role)) { continue }
    $n = Get-ItcmdrValue $r @('name')
    $exp = @(Get-ItcmdrValue $r @('expected')); $forb = @(Get-ItcmdrValue $r @('forbidden'))
    Invoke-ItcmdrCheck -Id "DNS-$(Get-ItcmdrSafeId $n)" -Category 'configuration' -Test {
      Test-ItcmdrDnsRecord -Name $n -Expected $exp -Forbidden $forb -Resolver $resolver
    } -Repair {
      # Local cache only; server-side records are reported, never edited.
      ipconfig /flushdns | Out-Null
      (Test-ItcmdrDnsRecord -Name $n -Expected $exp -Forbidden $forb -Resolver $resolver).ok
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }

  if ($Ctx.Role -ne 'Server') {
    $sids = Get-ItcmdrUserSid
    $maps = Get-ItcmdrUserDriveMap
    foreach ($d in @(Get-ItcmdrValue $site @('driveMaps') | Where-Object { $_ })) {
      if (-not (Test-ItcmdrRoleMatch $d $Ctx.Role)) { continue }
      $letter = Get-ItcmdrValue $d @('letter'); $path = Get-ItcmdrValue $d @('path')
      Invoke-ItcmdrCheck -Id "DRIVE-$($letter.TrimEnd(':').ToUpperInvariant())" -Category 'configuration' -Test {
        Test-ItcmdrDriveMap -Letter $letter -Path $path -UserSid $sids -Mapping $maps
      } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
    }
  }

  $queues = @(Get-ItcmdrValue $site @('printerQueues') | Where-Object { $_ })
  if ($queues.Count -gt 0 -and (Get-Command Get-Service -ErrorAction SilentlyContinue)) {
    Invoke-ItcmdrCheck -Id 'SVC-Spooler' -Category 'services' -Test {
      (Get-Service -Name Spooler -ErrorAction SilentlyContinue).Status -eq 'Running'
    } -Repair {
      try { Start-Service -Name Spooler -ErrorAction Stop; $true } catch { $false }
    } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
  }
  $conns = if ($Ctx.Role -ne 'Server') { Get-ItcmdrUserPrinterConnection } else { @() }
  $sids2 = if ($Ctx.Role -ne 'Server') { Get-ItcmdrUserSid } else { @() }
  foreach ($q in $queues) {
    if (-not (Test-ItcmdrRoleMatch $q $Ctx.Role)) { continue }
    $qn = Get-ItcmdrValue $q @('name'); $qs = Get-ItcmdrValue $q @('server')
    $shared = Get-ItcmdrValue $q @('shared'); if ($null -eq $shared) { $shared = $true }
    if ($Ctx.Role -eq 'Server') {
      Invoke-ItcmdrCheck -Id "PRNQ-$(Get-ItcmdrSafeId $qn)" -Category 'configuration' -Test {
        $pr = Get-Printer -Name $qn -ErrorAction SilentlyContinue
        $jobs = if ($pr) { @(Get-PrintJob -PrinterName $qn -ErrorAction SilentlyContinue) } else { @() }
        Test-ItcmdrPrintQueueHealth -Printer $pr -Job $jobs -ExpectShared ([bool]$shared)
      } -Repair {
        # Only clears jobs that are in an error state; never touches drivers, ports or sharing.
        try {
          Get-PrintJob -PrinterName $qn -ErrorAction SilentlyContinue | Where-Object { [string]$_.JobStatus -match 'Error' } | Remove-PrintJob -ErrorAction SilentlyContinue
          $pr = Get-Printer -Name $qn -ErrorAction SilentlyContinue
          (Test-ItcmdrPrintQueueHealth -Printer $pr -Job @(Get-PrintJob -PrinterName $qn -ErrorAction SilentlyContinue) -ExpectShared ([bool]$shared)).ok
        } catch { $false }
      } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
    } elseif ($qs) {
      Invoke-ItcmdrCheck -Id "PRNC-$(Get-ItcmdrSafeId $qn)" -Category 'configuration' -Test {
        Test-ItcmdrPrinterConnection -Server $qs -Queue $qn -UserSid $sids2 -Connection $conns
      } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
    }
  }

  # Sirona drift: detect/report only. No -Repair on purpose.
  $patterns = @(Get-ItcmdrValue $site @('stalePatterns') | Where-Object { $_ })
  $files = Resolve-ItcmdrPathList @(Get-ItcmdrValue $site @('sirona', 'configFiles'))
  if ($patterns.Count -gt 0) {
    foreach ($f in $files) {
      Invoke-ItcmdrCheck -Id "SIRONA-DRIFT-$(Get-ItcmdrSafeId ([IO.Path]::GetFileName($f)))" -Category 'configuration' -Test {
        Test-ItcmdrSironaDrift -Path $f -Pattern $patterns
      } -ReportOnlyLocal:$ReportOnlyLocal -Ctx $Ctx
    }
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
  Invoke-ItcmdrScannerCheck -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective
  Invoke-ItcmdrConfigCheck -Ctx $ctx -ReportOnlyLocal:$reportOnlyEffective
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
