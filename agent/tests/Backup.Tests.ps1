Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . (Join-Path (Join-Path $PSScriptRoot '..') 'Invoke-ItcmdrAgent.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  $script:now = Get-Date '2026-10-30T08:00:00'
}

Describe 'Test-ItcmdrBackupTask' {
  It 'passes a recent successful task and explains every failure' {
    (Test-ItcmdrBackupTask -Info @{ exists = $true; state = 'Ready'; lastRun = $now.AddHours(-5); lastResult = 0 } -Now $now).ok | Should -BeTrue
    (Test-ItcmdrBackupTask -Info @{ exists = $false } -Now $now).detail | Should -Be 'task not found'
    $r = Test-ItcmdrBackupTask -Info @{ exists = $true; state = 'Disabled'; lastRun = $now.AddHours(-50); lastResult = 2147942402 } -Now $now
    $r.ok | Should -BeFalse
    $r.detail | Should -Match 'disabled'
    $r.detail | Should -Match '50h ago'
    $r.detail | Should -Match '0x80070002'
    (Test-ItcmdrBackupTask -Info @{ exists = $true; state = 'Ready'; lastRun = (Get-Date '1999-11-30'); lastResult = 267011 } -Now $now).detail | Should -Match 'never ran'
  }
}

Describe 'Test-ItcmdrBackupDestination' {
  It 'checks the newest matching file for age and size' {
    $d = Join-Path $TestDrive 'dest'; New-Item -ItemType Directory -Path (Join-Path $d 'sub') -Force | Out-Null
    (Test-ItcmdrBackupDestination -Path (Join-Path $TestDrive 'missing') -Now $now).detail | Should -Be 'destination unreachable'
    (Test-ItcmdrBackupDestination -Path $d -Pattern '*.bak' -Now $now).ok | Should -BeFalse
    $old = Join-Path $d 'old.bak'; Set-Content $old ('x' * 100); (Get-Item $old).LastWriteTime = $now.AddDays(-3)
    (Test-ItcmdrBackupDestination -Path $d -Pattern '*.bak' -Now $now).detail | Should -Match '72h old'
    $new = Join-Path $d 'sub/new.bak'; Set-Content $new ('x' * 100); (Get-Item $new).LastWriteTime = $now.AddHours(-2)
    (Test-ItcmdrBackupDestination -Path $d -Pattern '*.bak' -Now $now).ok | Should -BeTrue
    (Test-ItcmdrBackupDestination -Path $d -Pattern '*.bak' -MinBytes 1MB -Now $now).detail | Should -Match 'bytes'
  }
}

Describe 'Invoke-ItcmdrBackupCheck' {
  It 'runs on the server only, records each check and never repairs' {
    $dest = Join-Path $TestDrive 'nightly'; New-Item -ItemType Directory -Path $dest | Out-Null
    Set-Content (Join-Path $dest 'a.bak') 'data'
    $site = @{ backup = @{
        expectedBackupSources = @($TestDrive, (Join-Path $TestDrive 'nope'))
        tasks = @(@{ name = 'Nightly DB Backup'; maxAgeHours = 26 })
        destinations = @(@{ name = 'NIGHTLY'; path = $dest; pattern = '*.bak' })
      } }
    $info = { param($n) $null = $n; @{ exists = $true; state = 'Ready'; lastRun = (Get-Date).AddHours(-1); lastResult = 0 } }
    $log = Join-Path $TestDrive 'srv.log'
    Invoke-ItcmdrBackupCheck -Ctx @{ RunId = 'r'; Computer = 'SERVER01'; Role = 'Server'; Site = $site; LogFile = $log } -TaskInfo $info
    $r = @(Get-Content $log | ForEach-Object { $_ | ConvertFrom-Json })
    ($r | ForEach-Object checkId) | Should -Be @('BACKUP-SRC-A', 'BACKUP-SRC-B', 'BACKUP-TASK-Nightly_DB_Backup', 'BACKUP-DEST-NIGHTLY')
    ($r | ForEach-Object status) | Should -Be @('ok', 'failed', 'ok', 'ok')
    @($r | Where-Object category -ne 'backup').Count | Should -Be 0
    @($r | Where-Object attemptedRepair).Count | Should -Be 0

    $wlog = Join-Path $TestDrive 'ws.log'
    Invoke-ItcmdrBackupCheck -Ctx @{ RunId = 'r'; Computer = 'WS-01'; Role = 'Workstation'; Site = $site; LogFile = $wlog } -TaskInfo $info
    Test-Path $wlog | Should -BeFalse
  }
}
