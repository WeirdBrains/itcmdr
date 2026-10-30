Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . (Join-Path $PSScriptRoot '..' 'Invoke-ItcmdrAgent.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue

  function Get-TestContext([hashtable]$Site, [string]$Role = 'Workstation') {
    $log = Join-Path $TestDrive "run-$([guid]::NewGuid()).log"
    return @{ RunId = 'r1'; Computer = 'WS-01'; Role = $Role; Site = $Site; LogFile = $log; Exceptions = @(Get-ItcmdrValue $Site @('exceptions')) }
  }
  function Get-TestRecord($Ctx) { @(Get-Content $Ctx.LogFile | ForEach-Object { $_ | ConvertFrom-Json }) }
}

Describe 'Test-ItcmdrTcp' {
  It 'is true for a listening port and false for a closed one' {
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $l.Start()
    try {
      $port = $l.LocalEndpoint.Port
      Test-ItcmdrTcp -Address '127.0.0.1' -Port $port | Should -BeTrue
    } finally { $l.Stop() }
    Test-ItcmdrTcp -Address '127.0.0.1' -Port $port -TimeoutMs 500 | Should -BeFalse
    Test-ItcmdrTcp -Address '' -Port 1 | Should -BeFalse
  }
}

Describe 'scanner reachability' {
  It 'fails an unreachable scanner but the 3D unit exception downgrades it to ok' {
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start()
    $open = $l.LocalEndpoint.Port
    $closedL = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $closedL.Start()
    $closed = $closedL.LocalEndpoint.Port; $closedL.Stop()
    try {
      $site = @{
        scanners = @(
          @{ name = 'SCAN-01'; address = '127.0.0.1'; ports = @($open) },
          @{ name = 'SCAN-02'; address = '127.0.0.1'; ports = @($closed) },
          @{ name = 'SCAN-03'; address = '127.0.0.1'; ports = @($closed) },
          @{ name = 'FRONTONLY'; address = '127.0.0.1'; ports = @($closed); roles = @('Server') })
        exceptions = @(@{ scope = @{ machines = @() }; checkPattern = '^SCANNER-SCAN-03-'; action = 'downgrade:info'; reason = '3D unit may be off' })
      }
      $ctx = Get-TestContext $site
      $env:ITCMDR_TCP_TIMEOUT_MS = '500'
      Invoke-ItcmdrScannerCheck -Ctx $ctx
    } finally { $l.Stop() }
    $r = Get-TestRecord $ctx
    $r.Count | Should -Be 3
    ($r | Where-Object checkId -eq "SCANNER-SCAN-01-TCP-$open").status | Should -Be 'ok'
    ($r | Where-Object checkId -eq "SCANNER-SCAN-02-TCP-$closed").status | Should -Be 'failed'
    $c = $r | Where-Object checkId -eq "SCANNER-SCAN-03-TCP-$closed"
    $c.status | Should -Be 'ok'
    $c.excepted | Should -BeTrue
    @($r | Where-Object { $_.attemptedRepair }).Count | Should -Be 0
  }
}

Describe 'Test-ItcmdrDnsRecord' {
  It 'passes when every expected address is present' {
    (Test-ItcmdrDnsRecord -Name 'srv' -Expected @('198.51.100.2') -Resolver { @('198.51.100.2') }).ok | Should -BeTrue
  }
  It 'fails on a missing, stale, or unresolvable record' {
    $r = Test-ItcmdrDnsRecord -Name 'srv' -Expected @('198.51.100.2') -Resolver { @('198.51.100.9') }
    $r.ok | Should -BeFalse; $r.detail | Should -Match 'missing'
    $r = Test-ItcmdrDnsRecord -Name 'srv' -Expected @('198.51.100.2') -Forbidden @('198.51.100.3') -Resolver { @('198.51.100.2', '198.51.100.3') }
    $r.ok | Should -BeFalse; $r.detail | Should -Match 'stale 198.51.100.3'
    (Test-ItcmdrDnsRecord -Name 'srv' -Resolver { throw 'nx' }).ok | Should -BeFalse
  }
}

Describe 'Test-ItcmdrDriveMap' {
  It 'skips when nobody is logged on' {
    $r = Test-ItcmdrDriveMap -Letter 'P:' -Path '\\SERVER01\PDATA'
    $r.skipped | Should -BeTrue
  }
  It 'checks every logged-on user' {
    $maps = @(@{ sid = 'S-1'; letter = 'P'; remotePath = '\\SERVER01\PDATA' }, @{ sid = 'S-2'; letter = 'P'; remotePath = '\\OLD\PDATA' })
    (Test-ItcmdrDriveMap -Letter 'p:' -Path '\\SERVER01\PDATA' -UserSid @('S-1') -Mapping $maps).ok | Should -BeTrue
    $r = Test-ItcmdrDriveMap -Letter 'P:' -Path '\\SERVER01\PDATA' -UserSid @('S-1', 'S-2', 'S-3') -Mapping $maps
    $r.ok | Should -BeFalse
    $r.detail | Should -Match 'S-2: P: \\\\OLD'
    $r.detail | Should -Match 'S-3: P: not mapped'
  }
}

Describe 'printer queues' {
  It 'checks per-user connections on workstations' {
    $c = @(@{ sid = 'S-1'; server = 'SERVER01'; queue = 'PRN-03' })
    (Test-ItcmdrPrinterConnection -Server 'server01' -Queue 'prn-03' -UserSid @('S-1') -Connection $c).ok | Should -BeTrue
    (Test-ItcmdrPrinterConnection -Server 'SERVER01' -Queue 'PRN-03' -UserSid @('S-1', 'S-2') -Connection $c).ok | Should -BeFalse
    (Test-ItcmdrPrinterConnection -Server 'SERVER01' -Queue 'PRN-03').skipped | Should -BeTrue
  }
  It 'flags missing, unshared, errored queues and stuck jobs on the server' {
    $now = Get-Date '2026-10-30T09:00:00'
    $p = [pscustomobject]@{ Shared = $true; PrinterStatus = 'Normal' }
    (Test-ItcmdrPrintQueueHealth -Printer $p -Now $now).ok | Should -BeTrue
    (Test-ItcmdrPrintQueueHealth -Printer $null).ok | Should -BeFalse
    (Test-ItcmdrPrintQueueHealth -Printer ([pscustomobject]@{ Shared = $false; PrinterStatus = 'Normal' })).detail | Should -Match 'not shared'
    (Test-ItcmdrPrintQueueHealth -Printer ([pscustomobject]@{ Shared = $true; PrinterStatus = 'Error' })).ok | Should -BeFalse
    $jobs = @([pscustomobject]@{ JobStatus = 'Printing'; SubmittedTime = $now.AddHours(-3) })
    (Test-ItcmdrPrintQueueHealth -Printer $p -Job $jobs -Now $now).detail | Should -Match '1 stuck'
    $jobs = @([pscustomobject]@{ JobStatus = 'Printing'; SubmittedTime = $now.AddMinutes(-2) })
    (Test-ItcmdrPrintQueueHealth -Printer $p -Job $jobs -Now $now).ok | Should -BeTrue
  }
}

Describe 'Sirona drift (report only)' {
  It 'reports stale references without modifying the file' {
    $f = Join-Path $TestDrive 'sidexis.ini'
    Set-Content -Path $f -Value "[Server]`nHost=EXAMPLE-OLD-SERVER`nPort=1999"
    $before = (Get-FileHash $f).Hash
    $r = Test-ItcmdrSironaDrift -Path $f -Pattern @('example-old-server', '192.0.2.')
    $r.ok | Should -BeFalse
    $r.detail | Should -Match '1 pattern'
    (Get-FileHash $f).Hash | Should -Be $before
    (Test-ItcmdrSironaDrift -Path $f -Pattern @('192.0.2.')).ok | Should -BeTrue
    (Test-ItcmdrSironaDrift -Path (Join-Path $TestDrive 'nope.ini') -Pattern @('x')).skipped | Should -BeTrue
  }
  It 'reads a file another process holds open for writing' {
    $f = Join-Path $TestDrive 'locked.ini'
    Set-Content -Path $f -Value 'Host=EXAMPLE-OLD-SERVER'
    $h = [IO.File]::Open($f, 'Open', 'ReadWrite', 'ReadWrite')
    try { (Test-ItcmdrSironaDrift -Path $f -Pattern @('EXAMPLE-OLD-SERVER')).ok | Should -BeFalse } finally { $h.Dispose() }
  }
  It 'never attempts a repair even when repairs are allowed' {
    $f = Join-Path $TestDrive 'Sirona.config'
    Set-Content -Path $f -Value '<add key="srv" value="192.0.2.7"/>'
    $ctx = Get-TestContext @{ stalePatterns = @('192.0.2.'); sirona = @{ configFiles = @((Join-Path $TestDrive '*.config')) } } 'Server'
    Invoke-ItcmdrConfigCheck -Ctx $ctx
    $r = @(Get-TestRecord $ctx | Where-Object { $_.checkId -like 'SIRONA-DRIFT-*' })
    $r.Count | Should -Be 1
    $r[0].checkId | Should -Be 'SIRONA-DRIFT-Sirona.config'
    $r[0].status | Should -Be 'failed'
    $r[0].attemptedRepair | Should -BeFalse
  }
}

Describe 'rich results in Invoke-ItcmdrCheck' {
  It 'records skipped checks without failing or repairing' {
    $ctx = Get-TestContext @{}
    $script:repaired = $false
    Invoke-ItcmdrCheck -Id 'X' -Category 'configuration' -Test { ConvertTo-ItcmdrResult -Ok $false -Skipped -Detail 'nobody' } -Repair { $script:repaired = $true; $true } -Ctx $ctx
    $r = Get-TestRecord $ctx
    $r[0].status | Should -Be 'skipped'
    $r[0].detail | Should -Be 'nobody'
    $script:repaired | Should -BeFalse
  }
  It 'honours the configuration category filter' {
    $ctx = Get-TestContext @{}
    $ctx.Categories = @('network')
    Invoke-ItcmdrCheck -Id 'X' -Category 'configuration' -Test { $true } -Ctx $ctx
    Test-Path $ctx.LogFile | Should -BeFalse
  }
}
