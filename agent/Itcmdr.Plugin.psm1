<#
.SYNOPSIS
  itcmdr PowerShell plugin module (decisions 6 and 11). Windows PowerShell 5.1 and PowerShell 7.

.DESCRIPTION
  A PowerShell service plugin is a .ps1 run by itcmdr-plugin-ps for each stage:

    Import-Module $env:ITCMDR_PS_MODULE
    Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.0.0'; features = @('verify', 'execute') } `
      -Verify  { param($Ctx) @{ ok = $true; message = 'ready' } } `
      -Execute { param($Ctx)
                 Write-ItcmdrMetric -Name 'disk_free' -Value 12.5 -Unit 'GiB' -Labels @{ drive = 'C:' }
                 @{ ok = $true; message = 'done' } }

  Write-ItcmdrLog / Write-ItcmdrMetric write itcmdr.log/v1 lines (docs/LOG-STANDARD.md) to the
  run's log path ($Ctx.log_path, else $env:ITCMDR_LOG_PATH).
#>
Set-StrictMode -Version Latest

$script:ItcmdrPluginCtx = $null
$script:ItcmdrPluginInfo = @{ name = ''; version = '' }

function Get-ItcmdrLogTimestamp {
  param([datetime]$Now = (Get-Date))
  $off = [TimeZoneInfo]::Local.GetUtcOffset($Now)
  $sign = if ($off.Ticks -lt 0) { '-' } else { '+' }
  return $Now.ToString('yyyy-MM-ddTHH:mm:ss.fff', [Globalization.CultureInfo]::InvariantCulture) + ('{0}{1:00}:{2:00}' -f $sign, [Math]::Abs($off.Hours), [Math]::Abs($off.Minutes))
}

function Get-ItcmdrCtxValue {
  param([string]$Name)
  $c = $script:ItcmdrPluginCtx
  if ($null -eq $c) { return '' }
  if ($c -is [System.Collections.IDictionary]) { if ($c.Contains($Name)) { return $c[$Name] } else { return '' } }
  $p = $c.PSObject.Properties[$Name]
  if ($p) { return $p.Value }
  return ''
}

function New-ItcmdrLogRecord {
  [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseShouldProcessForStateChangingFunctions', '', Justification = 'Builds an object; changes nothing')]
  <# Build one itcmdr.log/v1 record (ordered dictionary). #>
  param(
    [ValidateSet('debug', 'info', 'warn', 'error')][string]$Level = 'info',
    [Parameter(Mandatory)][string]$EventName,
    [string]$Message = '',
    [string]$CheckId,
    [ValidateSet('ok', 'fail', 'repaired', 'skipped', 'excepted')][string]$Status,
    [Nullable[long]]$DurationMs,
    [object[]]$Metrics
  )
  $stage = [string](Get-ItcmdrCtxValue 'stage'); if (-not $stage) { $stage = 'execute' }
  $r = [ordered]@{
    schema         = 'itcmdr.log/v1'
    ts             = Get-ItcmdrLogTimestamp
    level          = $Level
    plugin         = [string]$script:ItcmdrPluginInfo.name
    plugin_version = [string]$script:ItcmdrPluginInfo.version
    run_id         = [string](Get-ItcmdrCtxValue 'run_id')
    machine        = [string](Get-ItcmdrCtxValue 'machine')
    stage          = $stage
    event          = $EventName
    message        = $Message
  }
  if ($PSBoundParameters.ContainsKey('CheckId')) { $r['check_id'] = $CheckId }
  if ($PSBoundParameters.ContainsKey('Status')) { $r['status'] = $Status }
  if ($null -ne $DurationMs) { $r['duration_ms'] = [long]$DurationMs }
  if ($Metrics) { $r['metrics'] = @($Metrics) }
  return $r
}

function Get-ItcmdrLogPath {
  $p = [string](Get-ItcmdrCtxValue 'log_path')
  if (-not $p) { $p = [string]$env:ITCMDR_LOG_PATH }
  return $p
}

function Write-ItcmdrLog {
  <# Append one itcmdr.log/v1 line to the run's log path. #>
  [CmdletBinding()]
  param(
    [ValidateSet('debug', 'info', 'warn', 'error')][string]$Level = 'info',
    [Parameter(Mandatory)][string]$EventName,
    [string]$Message = '',
    [string]$CheckId,
    [ValidateSet('ok', 'fail', 'repaired', 'skipped', 'excepted')][string]$Status,
    [Nullable[long]]$DurationMs,
    [object[]]$Metrics,
    [string]$Path
  )
  $null = $PSBoundParameters.Remove('Path')
  $rec = New-ItcmdrLogRecord @PSBoundParameters
  if (-not $Path) { $Path = Get-ItcmdrLogPath }
  if (-not $Path) { throw 'No log path: run under itcmdr (ITCMDR_LOG_PATH) or pass -Path' }
  $dir = Split-Path -Parent $Path
  if ($dir -and -not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
  $line = ($rec | ConvertTo-Json -Depth 6 -Compress)
  [IO.File]::AppendAllText($Path, $line + "`n", (New-Object Text.UTF8Encoding($false)))
}

function New-ItcmdrMetric {
  [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSUseShouldProcessForStateChangingFunctions', '', Justification = 'Builds an object; changes nothing')]
  param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][double]$Value,
    [string]$Unit = '',
    [ValidateSet('counter', 'gauge', 'histogram')][string]$Kind = 'gauge',
    [hashtable]$Labels = @{}
  )
  $l = [ordered]@{}
  foreach ($k in @($Labels.Keys | Sort-Object)) { $l[[string]$k] = [string]$Labels[$k] }
  return [ordered]@{ name = $Name; value = $Value; unit = $Unit; kind = $Kind; labels = $l }
}

function Write-ItcmdrMetric {
  <# Append one line carrying a single metric. #>
  [CmdletBinding()]
  param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][double]$Value,
    [string]$Unit = '',
    [ValidateSet('counter', 'gauge', 'histogram')][string]$Kind = 'gauge',
    [hashtable]$Labels = @{},
    [string]$Path
  )
  $m = New-ItcmdrMetric -Name $Name -Value $Value -Unit $Unit -Kind $Kind -Labels $Labels
  Write-ItcmdrLog -EventName 'metrics' -Message "$Name = $Value" -Metrics @($m) -Path $Path
}

$script:ItcmdrPluginErrors = $null

# ---- v0.15.0: the plugin error contract (docs/design/SERVER-AGENT.md section 6) ----
# Criticality: Critical | Cautious | Informational. The host (itcmdr-plugin-ps) passes the error to
# the server as PluginError; the server writes the plugin-error log line (plugins do not).

function ConvertTo-ItcmdrCriticality {
  param([string]$Value)
  switch -Regex ($Value.Trim()) {
    '^(?i)(critical|crit)$' { return 'critical' }
    '^(?i)(cautious|caution|warn|warning)$' { return 'cautious' }
    '^(?i)(informational|info)$' { return 'informational' }
    default { return '' }
  }
}

function Limit-ItcmdrText {
  param([string]$Text, [int]$Max)
  if ($null -eq $Text) { return '' }
  if ($Text.Length -le $Max) { return $Text }
  $mark = "`n...[truncated]"
  return $Text.Substring(0, $Max - $mark.Length) + $mark
}

function New-ItcmdrPluginError {
  <#
    A plugin error. `throw (New-ItcmdrPluginError -Message 'share unreachable' -Criticality Critical)`
    fails the stage with it; Write-ItcmdrPluginError records a non-fatal one. The backtrace is the
    PowerShell call stack here (a thrown one also gets the catch site's ScriptStackTrace).
  #>
  [CmdletBinding()]
  param(
    [Parameter(Mandatory)][string]$Message,
    [Parameter(Mandatory)][ValidateSet('Critical', 'Cautious', 'Informational')][string]$Criticality,
    [string]$Kind = 'ItcmdrPluginError',
    [hashtable]$Context = @{}
  )
  $plugin = ''
  if ($script:ItcmdrPluginInfo) { $plugin = [string]$script:ItcmdrPluginInfo.name }
  $stack = (Get-PSCallStack | Select-Object -Skip 1 | ForEach-Object { "at $($_.Command), $($_.Location)" }) -join "`n"
  $ctx = [ordered]@{}
  foreach ($k in $Context.Keys) { $ctx[[string]$k] = [string]$Context[$k] }
  [pscustomobject]@{
    PSTypeName     = 'ItcmdrPluginError'
    service_plugin = $plugin
    kind           = $Kind
    criticality    = (ConvertTo-ItcmdrCriticality $Criticality)
    error          = (Limit-ItcmdrText $Message 4096)
    backtrace      = (Limit-ItcmdrText $stack 16384)
    context        = $ctx
  }
}

function Write-ItcmdrPluginError {
  <#
    Record a non-fatal plugin error for the current stage (the stage still succeeds). The worst
    one of the stage is sent to the server with the stage result.
  #>
  [CmdletBinding()]
  param(
    [Parameter(Mandatory, ParameterSetName = 'New')][string]$Message,
    [Parameter(Mandatory, ParameterSetName = 'New')][ValidateSet('Critical', 'Cautious', 'Informational')][string]$Criticality,
    [Parameter(ParameterSetName = 'New')][string]$Kind = 'ItcmdrPluginError',
    [Parameter(ParameterSetName = 'New')][hashtable]$Context = @{},
    [Parameter(Mandatory, ParameterSetName = 'Object')]$PluginError
  )
  if ($PSCmdlet.ParameterSetName -eq 'New') {
    $PluginError = New-ItcmdrPluginError -Message $Message -Criticality $Criticality -Kind $Kind -Context $Context
  }
  if ($null -eq $script:ItcmdrPluginErrors) { $script:ItcmdrPluginErrors = New-Object System.Collections.ArrayList }
  [void]$script:ItcmdrPluginErrors.Add($PluginError)
}

function Test-ItcmdrPluginError {
  param($Value)
  return ($null -ne $Value) -and ($Value -is [psobject]) -and ($Value.PSObject.TypeNames -contains 'ItcmdrPluginError')
}

function Get-ItcmdrWorstPluginError {
  param($Errors)
  $rank = @{ critical = 0; cautious = 1; informational = 2; '' = 1 }
  return @($Errors) | Sort-Object { $rank[[string]$_.criticality] } | Select-Object -First 1
}

function ConvertTo-ItcmdrPluginErrorRecord {
  # An exception caught by Invoke-ItcmdrPlugin. A thrown ItcmdrPluginError keeps its criticality;
  # anything else gets an empty one, so the server applies the package's default_error_criticality
  # (cautious unless set).
  param($ErrorRecord)
  $site = "$($ErrorRecord.ScriptStackTrace)`n$($ErrorRecord.InvocationInfo.PositionMessage)".Trim()
  $t = $ErrorRecord.TargetObject
  if (Test-ItcmdrPluginError $t) {
    $bt = (("$($t.backtrace)`n$site").Trim())
    return [ordered]@{ service_plugin = $t.service_plugin; kind = $t.kind; criticality = $t.criticality
      error = $t.error; backtrace = (Limit-ItcmdrText $bt 16384); context = $t.context }
  }
  $plugin = ''
  if ($script:ItcmdrPluginInfo) { $plugin = [string]$script:ItcmdrPluginInfo.name }
  return [ordered]@{ service_plugin = $plugin; kind = $ErrorRecord.Exception.GetType().FullName; criticality = ''
    error = (Limit-ItcmdrText $ErrorRecord.ToString() 4096); backtrace = (Limit-ItcmdrText $site 16384); context = [ordered]@{} }
}

function ConvertTo-ItcmdrStageResult {
  param($Value)
  if ($Value -is [array]) { $Value = $Value[-1] }
  if (Test-ItcmdrPluginError $Value) {
    # A returned (not thrown) plugin error fails the stage too.
    return [ordered]@{ ok = $false; message = $Value.error; results = @(); plugin_error = $Value }
  }
  if ($Value -is [bool]) { return [ordered]@{ ok = $Value; message = '' } }
  if ($Value -is [string]) { return [ordered]@{ ok = $true; message = $Value } }
  if ($Value -is [System.Collections.IDictionary]) {
    $ok = if ($Value.Contains('ok')) { [bool]$Value['ok'] } else { $true }
    $msg = if ($Value.Contains('message')) { [string]$Value['message'] } else { '' }
    # Not `$res = if ...`: a statement's output is unrolled, and a single result would stop
    # being an array in the JSON.
    $res = @()
    if ($Value.Contains('results')) { $res = @($Value['results']) }
    return [ordered]@{ ok = $ok; message = $msg; results = $res }
  }
  return [ordered]@{ ok = $true; message = '' }
}

function Invoke-ItcmdrPlugin {
  <#
    Entry point of a plugin script. Reads the host's JSON request (stdin, or -InputJson), runs the
    stage's scriptblock and prints one ITCMDR-RESULT|{json} line. An exception in a stage is
    reported as an error (the host fails that stage; a failed Verify stops Execute).
  #>
  [CmdletBinding()]
  param(
    [Parameter(Mandatory)][hashtable]$Info,
    [scriptblock]$Verify,
    [Parameter(Mandatory)][scriptblock]$Execute,
    [scriptblock]$Log,
    [string]$InputJson
  )
  if (-not $PSBoundParameters.ContainsKey('InputJson')) { $InputJson = [Console]::In.ReadToEnd() }
  $req = $InputJson | ConvertFrom-Json
  $script:ItcmdrPluginInfo = $Info
  $stage = [string]$req.stage
  $out = $null
  try {
    if ($stage -eq 'describe') {
      $f = @('execute')
      if ($Info.ContainsKey('features')) { $f = @($Info.features) }
      $d = if ($Info.ContainsKey('description')) { [string]$Info.description } else { '' }
      $out = [ordered]@{ name = [string]$Info.name; version = [string]$Info.version; features = $f; description = $d }
    } else {
      $script:ItcmdrPluginCtx = $req.ctx
      $script:ItcmdrPluginErrors = New-Object System.Collections.ArrayList
      switch ($stage) {
        'verify' { $out = if ($Verify) { ConvertTo-ItcmdrStageResult (& $Verify $req.ctx) } else { [ordered]@{ ok = $true; message = 'no verify stage' } } }
        'execute' { $out = ConvertTo-ItcmdrStageResult (& $Execute $req.ctx) }
        'log' {
          if ($Log) { & $Log $req.ctx ([string]$req.failed_stage) ([string]$req.error) | Out-Null }
          $out = [ordered]@{ ok = $true; message = '' }
        }
        default { throw "unknown stage '$stage'" }
      }
    }
    if ($out -is [System.Collections.IDictionary] -and -not $out.Contains('plugin_error') -and $script:ItcmdrPluginErrors -and $script:ItcmdrPluginErrors.Count -gt 0) {
      $out['plugin_error'] = Get-ItcmdrWorstPluginError $script:ItcmdrPluginErrors
    }
  } catch {
    $pe = ConvertTo-ItcmdrPluginErrorRecord $_
    $out = [ordered]@{ error = [string]$pe.error; plugin_error = $pe }
  }
  $line = 'ITCMDR-RESULT|' + ($out | ConvertTo-Json -Depth 6 -Compress)
  Write-Output $line
}

Export-ModuleMember -Function New-ItcmdrPluginError, Write-ItcmdrPluginError, Invoke-ItcmdrPlugin, Write-ItcmdrLog, Write-ItcmdrMetric, New-ItcmdrMetric, New-ItcmdrLogRecord, Get-ItcmdrLogTimestamp
