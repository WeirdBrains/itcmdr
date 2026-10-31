Set-StrictMode -Version Latest

# v0.11.0: the built-in checks as plugins. The plugin versions must give exactly the task's
# records (parity), run report-only in parallel, and the cutover/fallback logic must hold.
BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  $script:agentDir = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
  $pinned = Join-Path $script:agentDir '..\third_party\itcmdr-plugins\plugins\itcmdr-network\main.ps1'
  if (-not (Test-Path -LiteralPath $pinned)) { throw 'The pinned itcmdr-plugins checkout is missing: run scripts/fetch-plugins.sh first (plugins.pin.json).' }
  . (Join-Path $script:agentDir 'Invoke-ItcmdrAgent.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  $env:ITCMDR_TCP_TIMEOUT_MS = '500'

  function Get-TestSite([hashtable]$Extra = @{}) {
    $s = @{ paths = @{ localLogs = (Join-Path $TestDrive 'logs'); agentData = (Join-Path $TestDrive 'data') } }
    foreach ($k in $Extra.Keys) { $s[$k] = $Extra[$k] }
    return $s
  }
  function Get-Ctx([hashtable]$Site, [string]$Role = 'Workstation') {
    return @{ RunId = [guid]::NewGuid().ToString(); Computer = 'WS-01'; Role = $Role; Site = $Site; LogFile = (Join-Path $TestDrive "run-$([guid]::NewGuid()).log"); Exceptions = @(Get-ItcmdrValue $Site @('exceptions')) }
  }
  function Read-Rec([string]$Path) {
    if (-not (Test-Path $Path)) { return @() }
    @(Get-Content $Path | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.type -eq 'check' } |
        Select-Object runId, computer, category, checkId, severity, status, detail, attemptedRepair, action, result, excepted, exceptionReason)
  }
  # Run the built-in function and its plugin on the same context; return both record sets.
  function Invoke-Both([string]$Plugin, [hashtable]$Site, [string]$Role = 'Workstation', [switch]$ReportOnly) {
    $fn = $script:ItcmdrBuiltinPlugins[$Plugin]
    $a = Get-Ctx $Site $Role
    & $fn -Ctx $a -ReportOnlyLocal:$ReportOnly
    $b = Get-Ctx $Site $Role
    $b.RunId = $a.RunId
    $sp = Join-Path $TestDrive "site-$([guid]::NewGuid()).json"
    Set-Content -LiteralPath $sp -Value ($Site | ConvertTo-Json -Depth 10)
    $env:ITCMDR_SITE_PATH = $sp
    try {
      $set = @(Get-ItcmdrBuiltinPluginSet -AgentDir $script:agentDir | Where-Object Name -eq $Plugin)
      $run = Invoke-ItcmdrBuiltinPluginRun -Site $Site -PluginSet $set -AgentLog $b.LogFile -RunId $b.RunId -Role $Role -Computer 'WS-01' -Quiet $true -ReportOnly:$ReportOnly -AgentDir $script:agentDir
    } finally { Remove-Item Env:ITCMDR_SITE_PATH -ErrorAction SilentlyContinue }
    return @{ Builtin = @(Read-Rec $a.LogFile); Plugin = @(Read-Rec $b.LogFile); Run = $run }
  }
  function Assert-Same($r) {
    @($r.Run.Errors).Count | Should -Be 0 -Because (@($r.Run.Errors) | Out-String)
    $r.Plugin.Count | Should -Be $r.Builtin.Count
    ($r.Plugin | ConvertTo-Json -Depth 4) | Should -Be ($r.Builtin | ConvertTo-Json -Depth 4)
    @($r.Run.Results).Count | Should -Be $r.Builtin.Count
  }
}

AfterAll { Remove-Item Env:ITCMDR_TCP_TIMEOUT_MS -ErrorAction SilentlyContinue }

Describe 'built-in check plugins' {
  It 'ships all five, each mapped to its check function' {
    $set = @(Get-ItcmdrBuiltinPluginSet -AgentDir $script:agentDir)
    ($set | ForEach-Object Name) | Should -Be @('itcmdr-network', 'itcmdr-scanners', 'itcmdr-config', 'itcmdr-backup-coverage', 'itcmdr-services')
    @($set | Where-Object Source -ne 'bundled').Count | Should -Be 0
    foreach ($p in $set) { Get-Command $p.CheckFunction | Should -Not -BeNullOrEmpty }
  }

  It 'scanners: same records as the task, incl. the 3D unit exception and role filter' {
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start()
    $open = $l.LocalEndpoint.Port
    $c = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $c.Start(); $closed = $c.LocalEndpoint.Port; $c.Stop()
    try {
      $site = Get-TestSite @{
        scanners = @(
          @{ name = 'SCAN-01'; address = '127.0.0.1'; ports = @($open) },
          @{ name = 'SCAN-02'; address = '127.0.0.1'; ports = @($closed) },
          @{ name = 'SCAN-03'; address = '127.0.0.1'; ports = @($closed) },
          @{ name = 'FRONTONLY'; address = '127.0.0.1'; ports = @($closed); roles = @('Server') })
        exceptions = @(@{ scope = @{ machines = @() }; checkPattern = '^SCANNER-SCAN-03-'; action = 'downgrade:info'; reason = '3D unit may be off' })
      }
      $r = Invoke-Both 'itcmdr-scanners' $site
    } finally { $l.Stop() }
    Assert-Same $r
    $r.Plugin.Count | Should -Be 3
    ($r.Run.Results | Where-Object CheckId -like 'SCANNER-SCAN-03-*').Status | Should -Be 'excepted'
    ($r.Run.Results | Where-Object CheckId -like 'SCANNER-SCAN-02-*').Status | Should -Be 'fail'
  }

  It 'config: Sirona drift is reported, never repaired, and the file is untouched' {
    $f = Join-Path $TestDrive 'Sirona.config'
    Set-Content -Path $f -Value '<add key="srv" value="192.0.2.7"/>'
    $before = (Get-FileHash $f).Hash
    $site = Get-TestSite @{ stalePatterns = @('192.0.2.'); sirona = @{ configFiles = @((Join-Path $TestDrive '*.config')) } }
    $r = Invoke-Both 'itcmdr-config' $site 'Server'
    Assert-Same $r
    $d = @($r.Plugin | Where-Object checkId -eq 'SIRONA-DRIFT-Sirona.config')
    $d.Count | Should -Be 1
    $d[0].status | Should -Be 'failed'
    $d[0].attemptedRepair | Should -BeFalse
    (Get-FileHash $f).Hash | Should -Be $before
  }

  It 'backup coverage: same records on the server, nothing on a workstation' {
    $dest = Join-Path $TestDrive 'nightly'; New-Item -ItemType Directory -Path $dest -Force | Out-Null
    Set-Content (Join-Path $dest 'a.bak') 'data'
    $site = Get-TestSite @{ backup = @{
        expectedBackupSources = @($TestDrive, (Join-Path $TestDrive 'nope'))
        destinations = @(@{ name = 'NIGHTLY'; path = $dest; pattern = '*.bak' })
      } }
    $r = Invoke-Both 'itcmdr-backup-coverage' $site 'Server'
    Assert-Same $r
    ($r.Plugin | ForEach-Object checkId) | Should -Be @('BACKUP-SRC-A', 'BACKUP-SRC-B', 'BACKUP-DEST-NIGHTLY')
    ($r.Plugin | ForEach-Object status) | Should -Be @('ok', 'failed', 'ok')
    $w = Invoke-Both 'itcmdr-backup-coverage' $site 'Workstation'
    Assert-Same $w
    $w.Plugin.Count | Should -Be 0
  }

  It 'network and services: same records (whatever this host gives)' {
    $site = Get-TestSite @{ network = @{ serverHostname = '127.0.0.1'; serverPorts = @(1) } }
    Assert-Same (Invoke-Both 'itcmdr-network' $site -ReportOnly)
    Assert-Same (Invoke-Both 'itcmdr-services' $site 'Server' -ReportOnly)
  }

  It 'report-only plugin runs never repair and write itcmdr.log/v1 lines' {
    $site = Get-TestSite @{ network = @{ serverHostname = '127.0.0.1'; serverPorts = @(1) } }
    $r = Invoke-Both 'itcmdr-network' $site -ReportOnly
    @($r.Plugin | Where-Object attemptedRepair).Count | Should -Be 0
    $logs = @(Get-ChildItem -Recurse -File (Join-Path (Join-Path $TestDrive 'logs') 'plugin-logs') -Filter '*.jsonl')
    $logs.Count | Should -BeGreaterThan 0
    $line = Get-Content $logs[0].FullName | Select-Object -First 1 | ConvertFrom-Json
    $line.schema | Should -Be 'itcmdr.log/v1'
    $line.event | Should -Be 'check'
  }
}

Describe 'Compare-ItcmdrParity' {
  It 'compares what each side saw before repairs and flags missing checks' {
    $b = @(
      [pscustomobject]@{ checkId = 'A'; status = 'ok'; attemptedRepair = $true; result = 'success'; excepted = $false },
      [pscustomobject]@{ checkId = 'B'; status = 'ok'; attemptedRepair = $false; result = 'skipped'; excepted = $true },
      [pscustomobject]@{ checkId = 'C'; status = 'failed'; attemptedRepair = $false; result = 'skipped'; excepted = $false },
      [pscustomobject]@{ checkId = 'D'; status = 'ok'; attemptedRepair = $false; result = 'skipped'; excepted = $false })
    $p = @(
      [pscustomobject]@{ CheckId = 'A'; Status = 'fail' },
      [pscustomobject]@{ CheckId = 'B'; Status = 'excepted' },
      [pscustomobject]@{ CheckId = 'C'; Status = 'ok' },
      [pscustomobject]@{ CheckId = 'E'; Status = 'ok' })
    $c = Compare-ItcmdrParity -BuiltinRecords $b -PluginResults $p
    $c.Matched | Should -Be 2
    ($c.Mismatched | ForEach-Object checkId) | Should -Be @('C', 'D', 'E')
    ($c.Mismatched | Where-Object checkId -eq 'D').plugin | Should -Be 'missing'
  }
}

Describe 'cutover' {
  BeforeEach {
    $share = Join-Path $TestDrive "share-$([guid]::NewGuid())"
    New-Item -ItemType Directory -Path $share -Force | Out-Null
    $site = Get-TestSite @{ shares = @{ maintenanceRuns = $share } }
    $site.paths.agentData = Join-Path $TestDrive "data-$([guid]::NewGuid())"
    function Write-TestState([string]$Cutover, [datetime]$Ts, [bool]$NightlyOn = $true) {
      $doc = @{ v = 1; ts = $Ts.ToString('o'); cutover = $Cutover; nightly = @{ id = 'sid-1'; name = 'Nightly agent run'; enabled = $NightlyOn; when = 'daily 01:30' } }
      Set-Content -LiteralPath (Join-Path $share 'schedule-state.json') -Value ($doc | ConvertTo-Json -Depth 5)
    }
  }

  It 'resolves the plugin mode from the cutover state and agent.pluginMode' {
    Resolve-ItcmdrPluginMode -Site $site -CutoverState 'task' | Should -Be 'parallel'
    Resolve-ItcmdrPluginMode -Site $site -CutoverState 'schedule' | Should -Be 'plugins'
    $site.agent = @{ pluginMode = 'off' }
    Resolve-ItcmdrPluginMode -Site $site -CutoverState 'schedule' | Should -Be 'off'
  }

  It 'follows a fresh server state, ignores a stale one, and a pinned rollback wins' {
    Mock Set-ItcmdrNightlyTaskEnabled { $true }
    (Get-ItcmdrCutoverState -Site $site).State | Should -Be 'task'
    Write-TestState 'schedule' (Get-Date).AddHours(-2)
    (Sync-ItcmdrCutover -Site $site).State | Should -Be 'task'
    Should -Invoke Set-ItcmdrNightlyTaskEnabled -Times 0
    Write-TestState 'schedule' (Get-Date)
    (Sync-ItcmdrCutover -Site $site).State | Should -Be 'schedule'
    Should -Invoke Set-ItcmdrNightlyTaskEnabled -Times 1 -ParameterFilter { $Enabled -eq $false }
    # Local one-command rollback pins the task.
    Set-ItcmdrCutoverState -Site $site -State 'task' -Pinned
    (Sync-ItcmdrCutover -Site $site).State | Should -Be 'task'
    # Server rollback re-enables the task (after unpinning).
    Set-ItcmdrCutoverState -Site $site -State 'schedule'
    Write-TestState 'task' (Get-Date)
    (Sync-ItcmdrCutover -Site $site).State | Should -Be 'task'
    Should -Invoke Set-ItcmdrNightlyTaskEnabled -Times 1 -ParameterFilter { $Enabled -eq $true }
  }

  It 'runs the nightly locally only when it is overdue after the switch' {
    $now = (Get-Date).Date.AddHours(5)
    $sched = [pscustomobject]@{ State = 'schedule'; Pinned = $false }
    $task = [pscustomobject]@{ State = 'task'; Pinned = $false }
    $stale = [pscustomobject]@{ Doc = $null; Ts = $null; Fresh = $false; Source = 'none' }
    Test-ItcmdrFallbackDue -Site $site -Now $now -ScheduleState $stale -Cutover $task | Should -BeFalse
    Test-ItcmdrFallbackDue -Site $site -Now $now -ScheduleState $stale -Cutover $sched | Should -BeTrue
    Test-ItcmdrFallbackDue -Site $site -Now $now.Date.AddHours(2) -ScheduleState $stale -Cutover $sched | Should -BeFalse
    Write-TestState 'schedule' $now $false
    $paused = Get-ItcmdrScheduleState -Site $site -Now $now
    $paused.Fresh | Should -BeTrue
    Test-ItcmdrFallbackDue -Site $site -Now $now -ScheduleState $paused -Cutover $sched | Should -BeFalse
    Write-TestState 'schedule' $now $true
    Test-ItcmdrFallbackDue -Site $site -Now $now -ScheduleState (Get-ItcmdrScheduleState -Site $site -Now $now) -Cutover $sched | Should -BeTrue
    Set-ItcmdrNightlyMarker -Site $site -Trigger 'schedule' -RunId 'r1' -Now $now
    Test-ItcmdrFallbackDue -Site $site -Now $now -ScheduleState $stale -Cutover $sched | Should -BeFalse
  }

  It 'uses the cached state when the share is gone, but never as fresh' {
    Write-TestState 'schedule' (Get-Date)
    (Get-ItcmdrScheduleState -Site $site).Source | Should -Be 'share'
    Remove-Item -LiteralPath $share -Recurse -Force
    $s = Get-ItcmdrScheduleState -Site $site
    $s.Source | Should -Be 'cache'
    $s.Fresh | Should -BeFalse
  }
}

Describe 'nightly run end to end' {
  BeforeAll {
    function Invoke-AgentRun([hashtable]$Site, [string]$AgentDir = $script:agentDir, [string[]]$Arguments = @('-ReportOnly')) {
      $wd = Join-Path $TestDrive "wd-$([guid]::NewGuid())"
      New-Item -ItemType Directory -Path (Join-Path $wd 'config') -Force | Out-Null
      Set-Content -LiteralPath (Join-Path (Join-Path $wd 'config') 'site.json') -Value ($Site | ConvertTo-Json -Depth 10)
      $exe = (Get-Process -Id $PID).Path
      Push-Location $wd
      try { & $exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $AgentDir 'Invoke-ItcmdrAgent.ps1') @Arguments | Out-Null } finally { Pop-Location }
      $log = @(Get-ChildItem -LiteralPath $Site.paths.localLogs -Filter '*.log' -File | Sort-Object LastWriteTime | Select-Object -Last 1)
      return @(Get-Content $log[0].FullName | ForEach-Object { $_ | ConvertFrom-Json })
    }
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start()
    $script:listener = $l
    $script:runSite = {
      $s = Get-TestSite @{ scanners = @(@{ name = 'SCAN-01'; address = '127.0.0.1'; ports = @($script:listener.LocalEndpoint.Port) }) }
      $s.paths.localLogs = Join-Path $TestDrive "logs-$([guid]::NewGuid())"
      $s.paths.agentData = Join-Path $TestDrive "data-$([guid]::NewGuid())"
      $s
    }
  }
  AfterAll { $script:listener.Stop() }

  It 'before the switch: built-in checks are the run, plugins ran report-only first, parity matches' {
    $r = Invoke-AgentRun (& $script:runSite)
    $start = $r | Where-Object type -eq 'run-start'
    $start.trigger | Should -Be 'task'
    $start.cutover | Should -Be 'task'
    $start.pluginMode | Should -Be 'parallel'
    $checks = @($r | Where-Object type -eq 'check')
    $checks.Count | Should -BeGreaterThan 0
    $p = @($r | Where-Object type -eq 'parity')
    $p.Count | Should -Be 1
    @($p[0].errors).Count | Should -Be 0 -Because ($p[0].errors | ConvertTo-Json)
    @($p[0].mismatched).Count | Should -Be 0 -Because ($p[0].mismatched | ConvertTo-Json)
    $p[0].matched | Should -Be $checks.Count
    @($p[0].plugins).Count | Should -Be 5
  }

  It 'after the switch: the plugins are the run; with no plugins it falls back to the built-in checks' {
    $s = & $script:runSite
    $s.agent = @{ pluginMode = 'plugins' }
    $r = Invoke-AgentRun $s -Arguments @('-ReportOnly', '-Trigger', 'fallback')
    ($r | Where-Object type -eq 'run-start').trigger | Should -Be 'fallback'
    @($r | Where-Object type -eq 'parity').Count | Should -Be 0
    @($r | Where-Object type -eq 'plugin-fallback').Count | Should -Be 0
    $n = @($r | Where-Object type -eq 'check').Count
    $n | Should -BeGreaterThan 0
    (Get-Content (Join-Path $s.paths.agentData 'last-nightly.json') -Raw | ConvertFrom-Json).trigger | Should -Be 'fallback'

    # An agent copy without plugins: every check function runs locally instead.
    $copy = Join-Path $TestDrive "agent-$([guid]::NewGuid())"
    New-Item -ItemType Directory -Path $copy | Out-Null
    Copy-Item -Path (Join-Path $script:agentDir '*.ps*1') -Destination $copy
    $s2 = & $script:runSite
    $s2.agent = @{ pluginMode = 'plugins' }
    $r2 = Invoke-AgentRun $s2 -AgentDir $copy
    @($r2 | Where-Object type -eq 'plugin-fallback').Count | Should -Be 5
    @($r2 | Where-Object type -eq 'check').Count | Should -Be $n
  }
}

Describe 'scripts that dot-source the agent' {
  It 'never assign a variable named like an agent parameter (the ValidateSet sticks to it)' {
    $params = @('ReportOnly', 'Repair', 'Force', 'Categories', 'Trigger')
    foreach ($s in @('Install-ItcmdrAgent.ps1', 'Report-BackupCoverage.ps1')) {
      $ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $script:agentDir $s), [ref]$null, [ref]$null)
      $bad = @($ast.FindAll({ param($n) $n -is [Management.Automation.Language.AssignmentStatementAst] -and $n.Left -is [Management.Automation.Language.VariableExpressionAst] }, $true) |
          Where-Object { $params -contains $_.Left.VariablePath.UserPath })
      $bad.Count | Should -Be 0 -Because "$s assigns $(@($bad | ForEach-Object { $_.Left.VariablePath.UserPath }) -join ',')"
    }
  }
}
