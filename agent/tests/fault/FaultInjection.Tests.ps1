Set-StrictMode -Version Latest
<#
  v0.15.2 fault-injection suite: REAL breaks on a disposable Windows test bed, never on a production server.
  Runs only when ITCMDR_FAULT_TESTBED=1 on Windows as an administrator (scripts/Invoke-ItcmdrFaultSuite.ps1 sets it).
  Targets are generic and configurable:
    ITCMDR_FAULT_SERVICE  a harmless service the suite may stop/disable (default W32Time)
    ITCMDR_FAULT_ROOT     scratch folder for configs, logs, share and drift files (default C:\itcmdr-fault)
  Every test restores what it broke (AfterEach/finally); the runner checks the service and folder state at the end.
#>
BeforeDiscovery {
  $script:enabled = ($env:ITCMDR_FAULT_TESTBED -eq '1') -and ($env:OS -eq 'Windows_NT')
  if ($script:enabled) {
    $id = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    $script:enabled = $id.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
  }
}

Describe 'fault injection on the test bed' -Skip:(-not $script:enabled) {
  BeforeAll {
    $script:agentDir = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path $script:agentDir 'Invoke-ItcmdrAgent.ps1')
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
    $script:svc = if ($env:ITCMDR_FAULT_SERVICE) { $env:ITCMDR_FAULT_SERVICE } else { 'W32Time' }
    $script:root = if ($env:ITCMDR_FAULT_ROOT) { $env:ITCMDR_FAULT_ROOT } else { 'C:\itcmdr-fault' }
    New-Item -ItemType Directory -Force -Path $script:root, "$($script:root)\logs", "$($script:root)\data", "$($script:root)\share" | Out-Null
    $s0 = Get-CimInstance Win32_Service -Filter "Name='$($script:svc)'"
    $script:svcStart = $s0.StartMode; $script:svcState = $s0.State

    function Get-FaultSite([hashtable]$Extra = @{}) {
      $s = @{ paths = @{ localLogs = "$($script:root)\logs"; agentData = "$($script:root)\data" }
        watchedServices = @(@{ id = 'Fault'; name = $script:svc; role = 'Any'; restart = $true }) }
      foreach ($k in $Extra.Keys) { $s[$k] = $Extra[$k] }
      return $s
    }
    function Get-FaultCtx([hashtable]$Site) {
      @{ RunId = [guid]::NewGuid().ToString(); Computer = $env:COMPUTERNAME; Role = 'Server'; Site = $Site
        LogFile = "$($script:root)\logs\fault-$([guid]::NewGuid()).log"; Exceptions = @() }
    }
    function Read-FaultRec($Ctx, [string]$Id) { @(Get-Content $Ctx.LogFile | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.checkId -eq $Id }) }
    function Restore-FaultService {
      $mode = switch ($script:svcStart) { 'Auto' { 'Automatic' } 'Disabled' { 'Disabled' } default { 'Manual' } }
      Set-Service -Name $script:svc -StartupType $mode
      if ($script:svcState -eq 'Running') { Start-Service -Name $script:svc -ErrorAction SilentlyContinue } else { Stop-Service -Name $script:svc -Force -ErrorAction SilentlyContinue }
    }
    function Invoke-FaultAgent([string]$SitePath, [string[]]$AgentArgs, [int]$KillAfterMs = 0) {
      $ps = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
      $out = "$($script:root)\agent-$([guid]::NewGuid()).out"
      $env:ITCMDR_SITE_PATH = $SitePath
      try {
        $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $script:agentDir 'Invoke-ItcmdrAgent.ps1')) + $AgentArgs
        $p = Start-Process $ps -ArgumentList $a -PassThru -NoNewWindow -RedirectStandardError $out -RedirectStandardOutput "$out.1"
      $null = $p.Handle  # without this, ExitCode stays $null on Windows PowerShell
        if ($KillAfterMs -gt 0) { Start-Sleep -Milliseconds $KillAfterMs; Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue; $p.WaitForExit() }
        else { if (-not $p.WaitForExit(600000)) { Stop-Process -Id $p.Id -Force; throw 'agent hung' } }
        return @{ Exit = $p.ExitCode; Err = (Get-Content $out -Raw -ErrorAction SilentlyContinue) }
      } finally { Remove-Item Env:ITCMDR_SITE_PATH -ErrorAction SilentlyContinue }
    }
    function Save-FaultSite([hashtable]$Site, [string]$Name = 'site.json') {
      $p = Join-Path $script:root $Name; $Site | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $p -Encoding UTF8; return $p
    }
  }

  AfterEach { Restore-FaultService }

  It 'stopped watched service: detected and restarted (repair mode)' {
    Set-Service -Name $script:svc -StartupType Manual; Stop-Service -Name $script:svc -Force
    $ctx = Get-FaultCtx (Get-FaultSite)
    Invoke-ItcmdrServiceCheck -Ctx $ctx
    $r = Read-FaultRec $ctx 'SVC-Fault'
    $r[0].attemptedRepair | Should -BeTrue
    $r[0].result | Should -Be 'success'
    $r[0].status | Should -Be 'ok'
    (Get-Service $script:svc).Status | Should -Be 'Running'
  }

  It 'stopped watched service in report-only mode: reported, left alone' {
    Set-Service -Name $script:svc -StartupType Manual; Stop-Service -Name $script:svc -Force
    $ctx = Get-FaultCtx (Get-FaultSite)
    Invoke-ItcmdrServiceCheck -Ctx $ctx -ReportOnlyLocal
    $r = Read-FaultRec $ctx 'SVC-Fault'
    $r[0].status | Should -Be 'failed'; $r[0].attemptedRepair | Should -BeFalse
    (Get-Service $script:svc).Status | Should -Be 'Stopped'
  }

  It 'disabled watched service: repair fails, recorded as failed repair and a Critical plugin error' {
    Stop-Service -Name $script:svc -Force; Set-Service -Name $script:svc -StartupType Disabled
    $m = Import-Module (Join-Path $script:agentDir 'Itcmdr.Plugin.psm1') -Force -PassThru
    & $m { $script:ItcmdrPluginErrors = New-Object System.Collections.ArrayList }
    $sp = Save-FaultSite (Get-FaultSite)
    $env:ITCMDR_SITE_PATH = $sp; $env:ITCMDR_LOG_PATH = "$($script:root)\logs\plugin-v1.log"
    try { $res = Invoke-ItcmdrBuiltinCheckSet -CheckFunction 'Invoke-ItcmdrServiceCheck' -PluginCtx @{ report_only = $false; args = @{ role = 'Server'; quiet = '1' } } }
    finally { Remove-Item Env:ITCMDR_SITE_PATH, Env:ITCMDR_LOG_PATH -ErrorAction SilentlyContinue }
    @($res.results | Where-Object { $_.check_id -eq 'SVC-Fault' })[0].status | Should -Be 'fail'
    $e = @(& $m { $script:ItcmdrPluginErrors }) | Where-Object { ($_ | ConvertTo-Json -Depth 5) -match 'SVC-Fault' } | Select-Object -First 1
    ($e | ConvertTo-Json -Depth 5) | Should -Match 'Critical'
    ($e | ConvertTo-Json -Depth 5) | Should -Match 'RepairFailed'
    (Get-Service $script:svc).Status | Should -Be 'Stopped'
    Remove-Module $m.Name -Force
    # clean re-run after the admin re-enables it
    Set-Service -Name $script:svc -StartupType Manual
    $ctx = Get-FaultCtx (Get-FaultSite)
    Invoke-ItcmdrServiceCheck -Ctx $ctx
    (Read-FaultRec $ctx 'SVC-Fault')[0].status | Should -Be 'ok'
  }

  It 'corrupt site.json: the agent stops with a clear parse error' {
    $p = Join-Path $script:root 'corrupt.json'; Set-Content -LiteralPath $p -Value '{ "paths": { "localLogs": '
    $r = Invoke-FaultAgent $p @('-ReportOnly', '-Categories', 'services')
    $r.Exit | Should -Not -Be 0
    $r.Err | Should -Match 'Failed to parse site config'
  }

  It 'deleted site.json: the agent refuses instead of falling back to another config' {
    $p = Join-Path $script:root 'deleted.json'; Remove-Item $p -ErrorAction SilentlyContinue
    $r = Invoke-FaultAgent $p @('-ReportOnly', '-Categories', 'services')
    $r.Exit | Should -Not -Be 0
    $r.Err | Should -Match 'Site config not found at ITCMDR_SITE_PATH'
  }

  It 'log folder not writable: the run fails loudly and a later run is clean' {
    $logs = "$($script:root)\locked-logs"; New-Item -ItemType Directory -Force $logs | Out-Null
    $site = Get-FaultSite; $site.paths.localLogs = $logs
    $sp = Save-FaultSite $site 'site-locked.json'
    $acl0 = Get-Acl $logs
    $acl = Get-Acl $logs; $acl.SetAccessRuleProtection($true, $false)
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule('Everyone', 'Write,AppendData,CreateFiles', 'ContainerInherit,ObjectInherit', 'None', 'Deny')))
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule('Administrators', 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
    Set-Acl $logs $acl
    try {
      $r = Invoke-FaultAgent $sp @('-ReportOnly', '-Categories', 'services')
      Write-Information -InformationAction Continue "locked-log run: exit $($r.Exit); stderr: $(([string]$r.Err).Trim() -replace '\s+', ' ' | Select-Object -First 1)"
      $r.Exit | Should -Not -BeNullOrEmpty
      $r.Exit | Should -Not -Be 0
    } finally { Set-Acl $logs $acl0 }
    $r2 = Invoke-FaultAgent $sp @('-ReportOnly', '-Categories', 'services')
    $r2.Exit | Should -Be 0
    @(Get-ChildItem $logs -Filter *.log).Count | Should -BeGreaterThan 0
  }

  It 'share folder with broken permissions: SHARE check fails, report only, ACL untouched' {
    $sh = "$($script:root)\share"
    $acl0 = Get-Acl $sh; $sddl0 = $acl0.Sddl
    $acl = Get-Acl $sh; $acl.SetAccessRuleProtection($true, $false)
    foreach ($r0 in @($acl.Access)) { [void]$acl.RemoveAccessRule($r0) }
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule('Everyone', 'FullControl', 'Allow')))
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule('Everyone', 'ReadData,ListDirectory,ReadAttributes', 'Deny')))
    Set-Acl $sh $acl
    $broken = (Get-Acl $sh).Sddl
    try {
      # run as a non-admin view: test the folder content listing, which the deny blocks for everyone
      $ctx = Get-FaultCtx (Get-FaultSite @{ shares = @{ fault = "$sh\*" }; network = @{ serverHostname = '127.0.0.1'; serverPorts = @(1) } })
      Invoke-ItcmdrNetworkCheck -Ctx $ctx
      $rec = Read-FaultRec $ctx 'SHARE-FAULT'
      $rec[0].status | Should -Be 'failed'
      $rec[0].attemptedRepair | Should -BeFalse
      (Get-Acl $sh).Sddl | Should -Be $broken
    } finally { $a = Get-Acl $sh; $a.SetSecurityDescriptorSddlForm($sddl0); Set-Acl $sh $a }
  }

  It 'vendor config drift: reported, never repaired, file unchanged; locked file reported unreadable' {
    $f = "$($script:root)\vendor.ini"; Set-Content $f 'host=192.0.2.44'
    $h0 = (Get-FileHash $f).Hash
    $ctx = Get-FaultCtx (Get-FaultSite @{ stalePatterns = @('192.0.2.'); configDrift = @{ files = @($f) } })
    Invoke-ItcmdrConfigCheck -Ctx $ctx
    $r = Read-FaultRec $ctx 'DRIFT-vendor.ini'
    $r[0].status | Should -Be 'failed'; $r[0].attemptedRepair | Should -BeFalse
    (Get-FileHash $f).Hash | Should -Be $h0
    $lock = [IO.File]::Open($f, 'Open', 'ReadWrite', 'None')
    try {
      $ctx2 = Get-FaultCtx (Get-FaultSite @{ stalePatterns = @('192.0.2.'); configDrift = @{ files = @($f) } })
      Invoke-ItcmdrConfigCheck -Ctx $ctx2
      $r2 = Read-FaultRec $ctx2 'DRIFT-vendor.ini'
      $r2[0].status | Should -Be 'failed'; $r2[0].detail | Should -Match 'unreadable'
    } finally { $lock.Dispose() }
  }

  It 'agent killed mid-run: the next run completes cleanly' {
    $site = Get-FaultSite @{ network = @{ serverHostname = '192.0.2.1'; serverPorts = @(445, 53, 9) } }
    $sp = Save-FaultSite $site 'site-kill.json'
    $null = Invoke-FaultAgent $sp @('-ReportOnly') -KillAfterMs 4000
    $r = Invoke-FaultAgent $sp @('-ReportOnly', '-Categories', 'services')
    $r.Exit | Should -Be 0
    $last = Get-ChildItem "$($script:root)\logs" -Filter "$($env:COMPUTERNAME)_*.log" | Sort-Object LastWriteTime | Select-Object -Last 1
    (Get-Content $last.FullName | ForEach-Object { ($_ | ConvertFrom-Json).type }) | Should -Contain 'run-complete'
  }

  It 'server link dropped: server port checks fail and are reported, nothing repaired' {
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start(); $port = $l.LocalEndpoint.Port; $l.Stop()
    $ctx = Get-FaultCtx (Get-FaultSite @{ network = @{ serverHostname = '127.0.0.1'; serverPorts = @($port) } })
    $env:ITCMDR_TCP_TIMEOUT_MS = '500'
    try { Invoke-ItcmdrNetworkCheck -Ctx $ctx -ReportOnlyLocal } finally { Remove-Item Env:ITCMDR_TCP_TIMEOUT_MS }
    $r = Read-FaultRec $ctx "NET-SERVER-TCP-$port"
    $r[0].status | Should -Be 'failed'; $r[0].attemptedRepair | Should -BeFalse
  }
}
