BeforeAll {
  $script:Script = Join-Path $PSScriptRoot '..\Initialize-ItcmdrSite.ps1'
}

Describe 'Initialize-ItcmdrSite (v0.14.0 agent first-run)' {
  BeforeEach {
    $script:d = Join-Path ([IO.Path]::GetTempPath()) ("itcmdr-site-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $d | Out-Null
    $script:site = Join-Path $d 'site.json'
    $script:share = Join-Path $d 'share'
    New-Item -ItemType Directory -Path $share | Out-Null
  }
  AfterEach { Remove-Item -LiteralPath $d -Recurse -Force -ErrorAction SilentlyContinue }

  It 'creates site.json with the given values and passes the checks' {
    $r = & $Script -SitePath $site -SiteName 'Example Office' -ServerHostname 'SERVER01' -MaintenanceRunsShare $share -LocalLogs (Join-Path $d 'logs') -SkipServerCheck 6>$null
    $r.failed | Should -BeFalse
    $r.changed | Should -Contain 'shares.maintenanceRuns'
    $j = Get-Content $site -Raw | ConvertFrom-Json
    $j.network.serverHostname | Should -Be 'SERVER01'
    $j.paths.localLogs | Should -Be (Join-Path $d 'logs')
    Test-Path (Join-Path $d 'logs') | Should -BeTrue
  }

  It 'keeps existing values on an upgrade unless -Overwrite' {
    '{"siteName":"Kept","paths":{"localLogs":""},"retention":{"serverDays":7}}' | Set-Content -LiteralPath $site
    $r = & $Script -SitePath $site -SiteName 'New' -LocalLogs (Join-Path $d 'l2') -SkipServerCheck 6>$null
    $j = Get-Content $site -Raw | ConvertFrom-Json
    $j.siteName | Should -Be 'Kept'
    $j.retention.serverDays | Should -Be 7
    $j.paths.localLogs | Should -Be (Join-Path $d 'l2')
    $r.changed | Should -Be @('paths.localLogs')
    & $Script -SitePath $site -SiteName 'New' -Overwrite -SkipServerCheck 6>$null | Out-Null
    (Get-Content $site -Raw | ConvertFrom-Json).siteName | Should -Be 'New'
  }

  It 'fails when the share is missing and warns when it is not set' {
    $r = & $Script -SitePath $site -MaintenanceRunsShare (Join-Path $d 'nope') -LocalLogs (Join-Path $d 'logs') -SkipServerCheck 6>$null
    $r.failed | Should -BeTrue
    ($r.checks | Where-Object name -eq 'share').level | Should -Be 'fail'
    Remove-Item $site
    $r = & $Script -SitePath $site -LocalLogs (Join-Path $d 'logs') -SkipServerCheck 6>$null
    ($r.checks | Where-Object name -eq 'share').level | Should -Be 'warn'
    $r.failed | Should -BeFalse
  }

  It 'reports an unreadable site.json and writes a report file' {
    'not json' | Set-Content -LiteralPath $site
    $rep = Join-Path $d 'r.txt'
    $r = & $Script -SitePath $site -CheckOnly -Report $rep 6>$null
    $r.failed | Should -BeTrue
    Get-Content $rep -Raw | Should -Match 'FAIL site.json'
  }

  It 'warns when the server does not answer' {
    $r = & $Script -SitePath $site -ServerAddress '127.0.0.1' -ServerPort 1 -LocalLogs (Join-Path $d 'logs') 6>$null
    ($r.checks | Where-Object name -eq 'server').level | Should -Be 'warn'
  }
}
