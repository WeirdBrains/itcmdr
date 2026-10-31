<#
.SYNOPSIS
  itcmdr agent side of v0.11.0: run the built-in check plugins, compare them with the task's
  built-in checks (parity), and follow the nightly cutover (task <-> default schedule).

.DESCRIPTION
  Dot-sourced after Itcmdr.Checks.ps1 by Invoke-ItcmdrAgent.ps1 and Poll-ItcmdrRequests.ps1.

  Plugins: agents run only the built-in plugin copies installed with the agent
  (<agent>\plugins\<name>, or the pinned itcmdr-plugins copy in a source tree). Agents never download plugins;
  plugin packages are fetched (plain HTTPS) and run by the server only. Each plugin runs as a child PowerShell process with the plugin protocol (stdin JSON, one
  ITCMDR-RESULT line), like itcmdr-plugin-ps on the server.

  Cutover: the server writes <share>\schedule-state.json every scheduler pass. Local state is
  <agentData>\cutover.json (task | schedule, optionally pinned by Set-ItcmdrCutover.ps1).
  Windows PowerShell 5.1 and PowerShell 7.
#>
Set-StrictMode -Version Latest

$script:ItcmdrNightlyTaskName = 'itcmdr Agent Nightly'

function Get-ItcmdrSitePath {
  <# The site.json Get-ItcmdrSiteConfig would read (or $null). #>
  $c = @()
  if ($env:ITCMDR_SITE_PATH) { $c += $env:ITCMDR_SITE_PATH }
  $c += @((Join-Path -Path (Get-Location) -ChildPath 'config/site.json'), 'C:\ProgramData\itcmdr\site.json')
  foreach ($p in $c) { if (Test-Path -LiteralPath $p) { return (Resolve-Path -LiteralPath $p).Path } }
  return $null
}

function Read-ItcmdrJsonFile {
  param([string]$Path)
  if (-not $Path -or -not (Test-Path -LiteralPath $Path)) { return $null }
  return ((Get-Content -LiteralPath $Path -Raw).TrimStart([char]0xFEFF) | ConvertFrom-Json)
}

function Write-ItcmdrJsonFile {
  param([string]$Path, $Object)
  $dir = Split-Path -Parent $Path
  if ($dir -and -not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
  $tmp = "$Path.tmp"
  [IO.File]::WriteAllText($tmp, ($Object | ConvertTo-Json -Depth 8), (New-Object Text.UTF8Encoding($false)))
  Move-Item -LiteralPath $tmp -Destination $Path -Force
}

# Source tree only: the itcmdr-plugins checkout pinned in plugins.pin.json (scripts/fetch-plugins.sh).
function Get-ItcmdrSourcePluginsDir([string]$AgentDir) {
  return (Join-Path (Join-Path (Join-Path (Join-Path $AgentDir '..') 'third_party') 'itcmdr-plugins') 'plugins')
}

function Get-ItcmdrBuiltinPluginSet {
  <#
    One entry per built-in check plugin: Name, Version, Main, Source (bundled | missing),
    CheckFunction. Agents run ONLY the copies installed next to the agent scripts
    (<agent>\plugins\<name>, or third_party\itcmdr-plugins\plugins\<name> in a source tree, fetched
    from the pinned itcmdr-plugins tag by scripts/fetch-plugins.sh). They never download or
    copy plugins from the share or anywhere else: any PC can write to the share, so a planted copy
    would run as SYSTEM everywhere. Signed packages are fetched (HTTPS) and run by the server only.
  #>
  param([string]$AgentDir = $PSScriptRoot)
  $set = @()
  foreach ($name in @($script:ItcmdrBuiltinPlugins.Keys)) {
    $main = $null
    foreach ($d in @((Join-Path (Join-Path $AgentDir 'plugins') $name), (Join-Path (Get-ItcmdrSourcePluginsDir $AgentDir) $name))) {
      $m = Join-Path $d 'main.ps1'
      if (Test-Path -LiteralPath $m) { $main = (Resolve-Path -LiteralPath $m).Path; break }
    }
    $src = if ($main) { 'bundled' } else { 'missing' }
    $set += [pscustomobject]@{ Name = $name; Version = $script:AgentVersionForPlugins; Main = $main; Source = $src; CheckFunction = $script:ItcmdrBuiltinPlugins[$name] }
  }
  return $set
}
$script:AgentVersionForPlugins = '0.11.0'

function Get-ItcmdrPowerShellHost {
  try { $p = (Get-Process -Id $PID).Path; if ($p) { return $p } } catch { Write-Verbose 'no process path' }
  return 'powershell.exe'
}

function Invoke-ItcmdrPluginStage {
  <# Run one stage of a PowerShell plugin in a child process; returns the parsed result. #>
  param(
    [Parameter(Mandatory)][string]$Main,
    [Parameter(Mandatory)][string]$Stage,
    [hashtable]$Ctx = @{},
    [hashtable]$Environment = @{},
    [int]$TimeoutSec = 1800
  )
  $psi = New-Object Diagnostics.ProcessStartInfo
  $psi.FileName = Get-ItcmdrPowerShellHost
  $psi.Arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$Main`""
  $psi.UseShellExecute = $false
  $psi.RedirectStandardInput = $true
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.WorkingDirectory = (Get-Location).Path
  foreach ($k in $Environment.Keys) { $psi.EnvironmentVariables[[string]$k] = [string]$Environment[$k] }
  $c = @{}
  foreach ($k in $Ctx.Keys) { $c[$k] = $Ctx[$k] }
  $c['stage'] = $Stage
  $req = @{ stage = $Stage; ctx = $c } | ConvertTo-Json -Depth 6 -Compress
  $p = [Diagnostics.Process]::Start($psi)
  $errTask = $p.StandardError.ReadToEndAsync()
  $p.StandardInput.Write($req)
  $p.StandardInput.Close()
  $outTask = $p.StandardOutput.ReadToEndAsync()
  if (-not $p.WaitForExit($TimeoutSec * 1000)) {
    try { $p.Kill() } catch { Write-Verbose 'kill failed' }
    throw "$Stage timed out after $TimeoutSec s"
  }
  $out = $outTask.Result
  $line = @($out -split "`r?`n" | Where-Object { $_.StartsWith('ITCMDR-RESULT|') } | Select-Object -Last 1)
  if ($line.Count -eq 0) {
    $err = ($errTask.Result + ' ' + $out).Trim()
    if ($err.Length -gt 400) { $err = $err.Substring(0, 400) }
    throw "$Stage gave no result (exit $($p.ExitCode)): $err"
  }
  $r = $line[0].Substring('ITCMDR-RESULT|'.Length) | ConvertFrom-Json
  if ($r.PSObject.Properties['error']) { throw "$Stage failed: $($r.error)" }
  return $r
}

function Invoke-ItcmdrBuiltinPluginRun {
  <#
    Run the built-in check plugins (Verify, then Execute). -AgentLog makes them append their
    agent-format records to the nightly log (after the switch). Returns Results (Plugin, CheckId,
    Status), Ran ('name@version (source)'), Errors (Plugin, Error).
  #>
  param(
    $Site,
    [switch]$ReportOnly,
    [string]$AgentLog,
    [string]$RunId,
    [string[]]$Categories,
    [switch]$Force,
    [bool]$Quiet = $true,
    [string]$AgentDir = $PSScriptRoot,
    [object[]]$PluginSet,
    [string]$Role,
    [string]$Computer = $env:COMPUTERNAME
  )
  if ($null -eq $PluginSet) { $PluginSet = @(Get-ItcmdrBuiltinPluginSet -AgentDir $AgentDir) }
  $logRoot = Join-Path (Get-ItcmdrPath -Site $Site -Name localLogs) 'plugin-logs'
  $sitePath = Get-ItcmdrSitePath
  $out = @{ Results = @(); Ran = @(); Errors = @() }
  foreach ($pl in $PluginSet) {
    if (-not $pl.Main) { $out.Errors += [pscustomobject]@{ Plugin = $pl.Name; Error = 'plugin not found' }; continue }
    $log = Join-Path (Join-Path $logRoot $pl.Name) ("{0}_{1}.jsonl" -f $Computer, (Get-Date).ToString('yyyy-MM-dd'))
    $envs = @{
      ITCMDR_PS_MODULE = (Join-Path $AgentDir 'Itcmdr.Plugin.psm1')
      ITCMDR_CHECKS_LIB = (Join-Path $AgentDir 'Itcmdr.Checks.ps1')
      ITCMDR_LOG_PATH = $log
    }
    if ($sitePath) { $envs['ITCMDR_SITE_PATH'] = $sitePath }
    $pctx = @{
      run_id = [guid]::NewGuid().ToString(); log_path = $log; machine = $Computer; report_only = [bool]$ReportOnly
      args = @{
        agent_log = [string]$AgentLog; agent_run_id = [string]$RunId; categories = (@($Categories | Where-Object { $_ }) -join ',')
        force = $(if ($Force) { '1' } else { '0' }); quiet = $(if ($Quiet) { '1' } else { '0' })
        role = [string]$Role; computer = [string]$Computer
      }
    }
    try {
      $v = Invoke-ItcmdrPluginStage -Main $pl.Main -Stage 'verify' -Ctx $pctx -Environment $envs
      if (-not $v.ok) { throw "verify: $($v.message)" }
      $x = Invoke-ItcmdrPluginStage -Main $pl.Main -Stage 'execute' -Ctx $pctx -Environment $envs
      foreach ($r in @($x.results)) {
        if ($null -eq $r) { continue }
        $out.Results += [pscustomobject]@{ Plugin = $pl.Name; CheckId = [string]$r.check_id; Status = [string]$r.status }
      }
      $out.Ran += "$($pl.Name)@$($pl.Version) ($($pl.Source))"
    } catch {
      $out.Errors += [pscustomobject]@{ Plugin = $pl.Name; Error = $_.ToString() }
    }
  }
  return $out
}

function Compare-ItcmdrParity {
  <#
    Built-in records (the nightly log's check lines) vs plugin results, by check id, on what each
    saw before repairs (ConvertTo-ItcmdrParityStatus). A check missing on one side is a mismatch.
  #>
  param([object[]]$BuiltinRecords, [object[]]$PluginResults)
  $b = @{}
  foreach ($r in @($BuiltinRecords)) { if ($r) { $b[[string]$r.checkId] = ConvertTo-ItcmdrParityStatus (ConvertTo-ItcmdrPluginStatus $r) } }
  $p = @{}
  foreach ($r in @($PluginResults)) { if ($r) { $p[[string]$r.CheckId] = ConvertTo-ItcmdrParityStatus ([string]$r.Status) } }
  $matched = 0
  $mis = @()
  foreach ($id in @(@($b.Keys) + @($p.Keys) | Sort-Object -Unique)) {
    $bs = if ($b.ContainsKey($id)) { $b[$id] } else { 'missing' }
    $ps = if ($p.ContainsKey($id)) { $p[$id] } else { 'missing' }
    if ($bs -eq $ps) { $matched++ } else { $mis += [pscustomobject]@{ checkId = $id; builtin = $bs; plugin = $ps } }
  }
  return @{ Matched = $matched; Mismatched = @($mis) }
}

# --- Cutover -------------------------------------------------------------------------------

function Get-ItcmdrScheduleState {
  <#
    The server's schedule-state.json (share; cached to <agentData>). Fresh = written within
    agent.serverStaleMinutes (else the server counts as unreachable).
  #>
  param($Site, [datetime]$Now = (Get-Date))
  $cache = Join-Path (Get-ItcmdrPath -Site $Site -Name agentData) 'schedule-state.json'
  $doc = $null; $source = 'none'
  $share = Get-ItcmdrValue $Site @('shares', 'maintenanceRuns')
  if ($share) {
    try {
      $doc = Read-ItcmdrJsonFile (Join-Path ([string]$share) 'schedule-state.json')
      if ($doc) { $source = 'share'; Write-ItcmdrJsonFile -Path $cache -Object $doc }
    } catch { $doc = $null }
  }
  if (-not $doc) { try { $doc = Read-ItcmdrJsonFile $cache; if ($doc) { $source = 'cache' } } catch { $doc = $null } }
  $ts = $null
  if ($doc -and $doc.PSObject.Properties['ts']) { try { $ts = [DateTimeOffset]::Parse([string]$doc.ts, [Globalization.CultureInfo]::InvariantCulture).LocalDateTime } catch { $ts = $null } }
  $stale = [int](Get-ItcmdrSetting -Site $Site -Key 'agent.serverStaleMinutes')
  $fresh = [bool]($source -eq 'share' -and $ts -and ($Now - $ts).TotalMinutes -le $stale)
  return [pscustomobject]@{ Doc = $doc; Ts = $ts; Fresh = $fresh; Source = $source }
}

function Get-ItcmdrCutoverState {
  param($Site)
  $p = Join-Path (Get-ItcmdrPath -Site $Site -Name agentData) 'cutover.json'
  try { $d = Read-ItcmdrJsonFile $p } catch { $d = $null }
  $state = 'task'; $pinned = $false
  if ($d) {
    if ($d.PSObject.Properties['state'] -and [string]$d.state -in @('task', 'schedule')) { $state = [string]$d.state }
    if ($d.PSObject.Properties['pinned']) { $pinned = [bool]$d.pinned }
  }
  return [pscustomobject]@{ State = $state; Pinned = $pinned; Path = $p }
}

function Set-ItcmdrCutoverState {
  [CmdletBinding(SupportsShouldProcess)]
  param($Site, [ValidateSet('task', 'schedule')][string]$State, [switch]$Pinned, [string]$Reason = '')
  $p = Join-Path (Get-ItcmdrPath -Site $Site -Name agentData) 'cutover.json'
  if ($PSCmdlet.ShouldProcess($p, "cutover $State")) {
    Write-ItcmdrJsonFile -Path $p -Object ([pscustomobject]@{ state = $State; pinned = [bool]$Pinned; changed = (Get-Date).ToString('o'); reason = $Reason })
  }
}

function Set-ItcmdrNightlyTaskEnabled {
  <# Enable or disable (never delete) the SYSTEM nightly task. False when it can't be changed here. #>
  [CmdletBinding(SupportsShouldProcess)]
  param([bool]$Enabled, [string]$TaskName = $script:ItcmdrNightlyTaskName)
  if (-not (Get-Command -Name Get-ScheduledTask -ErrorAction SilentlyContinue)) { return $false }
  $t = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
  if (-not $t) { return $false }
  if ($PSCmdlet.ShouldProcess($TaskName, $(if ($Enabled) { 'Enable' } else { 'Disable' }))) {
    if ($Enabled) { Enable-ScheduledTask -TaskName $TaskName | Out-Null } else { Disable-ScheduledTask -TaskName $TaskName | Out-Null }
  }
  return $true
}

function Sync-ItcmdrCutover {
  <#
    Follow the server: when schedule-state.json is fresh and says a different state, disable the
    task (schedule) or re-enable it (task). A pinned local state (Set-ItcmdrCutover.ps1) wins.
    Returns the local state after the sync.
  #>
  param($Site, $ScheduleState)
  if ($null -eq $ScheduleState) { $ScheduleState = Get-ItcmdrScheduleState -Site $Site }
  $cur = Get-ItcmdrCutoverState -Site $Site
  if ($cur.Pinned -or -not $ScheduleState.Fresh -or -not $ScheduleState.Doc) { return $cur }
  $want = [string]$ScheduleState.Doc.cutover
  if ($want -notin @('task', 'schedule') -or $want -eq $cur.State) { return $cur }
  $null = Set-ItcmdrNightlyTaskEnabled -Enabled ($want -eq 'task')
  Set-ItcmdrCutoverState -Site $Site -State $want -Reason 'server schedule-state.json'
  return (Get-ItcmdrCutoverState -Site $Site)
}

function Resolve-ItcmdrPluginMode {
  <# agent.pluginMode: auto = parallel before the switch, plugins after it. #>
  param($Site, [string]$CutoverState = 'task')
  $m = [string](Get-ItcmdrSetting -Site $Site -Key 'agent.pluginMode')
  if ($m -in @('off', 'parallel', 'plugins')) { return $m }
  if ($CutoverState -eq 'schedule') { return 'plugins' }
  return 'parallel'
}

function Get-ItcmdrNightlyMarker {
  param($Site)
  try { return Read-ItcmdrJsonFile (Join-Path (Get-ItcmdrPath -Site $Site -Name agentData) 'last-nightly.json') } catch { return $null }
}

function Set-ItcmdrNightlyMarker {
  [CmdletBinding(SupportsShouldProcess)]
  param($Site, [string]$Trigger, [string]$RunId, [datetime]$Now = (Get-Date))
  $p = Join-Path (Get-ItcmdrPath -Site $Site -Name agentData) 'last-nightly.json'
  if ($PSCmdlet.ShouldProcess($p, 'Write nightly marker')) {
    Write-ItcmdrJsonFile -Path $p -Object ([pscustomobject]@{ date = $Now.ToString('yyyy-MM-dd'); trigger = $Trigger; runId = $RunId; ts = $Now.ToString('o') })
  }
}

function Test-ItcmdrFallbackDue {
  <#
    After the switch: run the nightly locally when it is agent.fallbackGraceMinutes past
    agent.nightlyTime, no nightly ran today, and the server is unreachable (stale state) or its
    nightly schedule is on but nothing arrived. A schedule paused on purpose is respected.
  #>
  param($Site, [datetime]$Now = (Get-Date), $ScheduleState, $Cutover)
  if ($null -eq $Cutover) { $Cutover = Get-ItcmdrCutoverState -Site $Site }
  if ($Cutover.State -ne 'schedule') { return $false }
  $at = ConvertTo-ItcmdrNightlyTime -Time ([string](Get-ItcmdrSetting -Site $Site -Key 'agent.nightlyTime')) -Today $Now.Date
  $grace = [int](Get-ItcmdrSetting -Site $Site -Key 'agent.fallbackGraceMinutes')
  if ($Now -lt $at.AddMinutes($grace)) { return $false }
  $m = Get-ItcmdrNightlyMarker -Site $Site
  if ($m -and [string]$m.date -eq $Now.ToString('yyyy-MM-dd')) { return $false }
  if ($null -eq $ScheduleState) { $ScheduleState = Get-ItcmdrScheduleState -Site $Site -Now $Now }
  if ($ScheduleState.Fresh -and $ScheduleState.Doc) {
    $n = $ScheduleState.Doc.nightly
    if ($null -eq $n -or -not [bool]$n.enabled) { return $false }
  }
  return $true
}
