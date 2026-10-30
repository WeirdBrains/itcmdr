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

function ConvertTo-ItcmdrStageResult {
  param($Value)
  if ($Value -is [array]) { $Value = $Value[-1] }
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
  } catch {
    $out = [ordered]@{ error = $_.ToString() }
  }
  $line = 'ITCMDR-RESULT|' + ($out | ConvertTo-Json -Depth 6 -Compress)
  Write-Output $line
}

Export-ModuleMember -Function Invoke-ItcmdrPlugin, Write-ItcmdrLog, Write-ItcmdrMetric, New-ItcmdrMetric, New-ItcmdrLogRecord, Get-ItcmdrLogTimestamp
