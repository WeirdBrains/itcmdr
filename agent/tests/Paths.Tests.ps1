Set-StrictMode -Version Latest

# Decision 15: configurable local paths (site.json "paths").
Describe 'Get-ItcmdrPath' {
  BeforeAll {
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path (Join-Path $PSScriptRoot '..') 'Invoke-ItcmdrAgent.ps1')
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  }
  It 'defaults when site.json has no paths' {
    Get-ItcmdrPath -Site @{} -Name localLogs | Should -Be 'C:\maintenanceRuns'
    Get-ItcmdrPath -Site @{} -Name agentData | Should -Be 'C:\ProgramData\itcmdr'
    Get-ItcmdrPath -Site @{ paths = @{ localLogs = '  ' } } -Name localLogs | Should -Be 'C:\maintenanceRuns'
  }
  It 'uses the override' {
    $s = @{ paths = @{ localLogs = 'D:\itcmdr\logs'; agentData = 'D:\itcmdr\data' } }
    Get-ItcmdrPath -Site $s -Name localLogs | Should -Be 'D:\itcmdr\logs'
    Get-ItcmdrPath -Site $s -Name agentData | Should -Be 'D:\itcmdr\data'
  }
  It 'writes the run log under paths.localLogs' {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-paths-' + [guid]::NewGuid())
    $data = Join-Path $root 'data'
    try {
      $ctx = New-ItcmdrRunContext -Site @{ paths = @{ localLogs = $root; agentData = $data } } -Categories @()
      Split-Path -Parent $ctx.LogFile | Should -Be $root
      Test-Path -LiteralPath $root | Should -BeTrue
    } finally { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
  }
  It 'caches exceptions and spools under paths.agentData' {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-paths-' + [guid]::NewGuid())
    $share = Join-Path $root 'share'
    $data = Join-Path $root 'data'
    New-Item -ItemType Directory -Path $share -Force | Out-Null
    try {
      Set-Content -LiteralPath (Join-Path $share 'exceptions.json') -Value '{"rules":[]}'
      $site = @{ shares = @{ maintenanceRuns = $share }; paths = @{ agentData = $data; localLogs = $root } }
      Get-ItcmdrException -Site $site | Out-Null
      Test-Path -LiteralPath (Join-Path $data 'exceptions.json') | Should -BeTrue
      # unreachable share: the log is queued in <agentData>\spool\outbox
      $log = Join-Path $root 'WS-01_x.log'
      Set-Content -LiteralPath $log -Value '{}'
      $bad = @{ Site = @{ shares = @{ maintenanceRuns = (Join-Path $root 'missing\deeper') }; paths = @{ agentData = $data } }; LogFile = $log }
      Push-ItcmdrLog -Ctx $bad
      Test-Path -LiteralPath (Join-Path (Join-Path (Join-Path $data 'spool') 'outbox') 'WS-01_x.log') | Should -BeTrue
    } finally { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
  }
}

Describe 'poller and rotation honour paths.localLogs' {
  BeforeAll {
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path (Join-Path $PSScriptRoot '..') 'Poll-ItcmdrRequests.ps1')
    . (Join-Path (Join-Path $PSScriptRoot '..') 'Rotate-ItcmdrLogs.ps1')
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  }
  It 'poller: default and override' {
    @(Get-ItcmdrRequestDirectory -Site $null)[0] | Should -Be ([IO.Path]::Combine('C:\maintenanceRuns', 'requests'))
    $s = [pscustomobject]@{ paths = [pscustomobject]@{ localLogs = 'D:\logs' }; shares = [pscustomobject]@{ maintenanceRuns = '\\SERVER01\share' } }
    $d = @(Get-ItcmdrRequestDirectory -Site $s)
    $d[0] | Should -Be ([IO.Path]::Combine('D:\logs', 'requests'))
    $d[1] | Should -Be ([IO.Path]::Combine('\\SERVER01\share', 'requests'))
  }
  It 'rotation: share first, then paths.localLogs, then the default' {
    Get-ItcmdrRotationRoot ([pscustomobject]@{ shares = [pscustomobject]@{ maintenanceRuns = '\\SERVER01\share' }; paths = [pscustomobject]@{ localLogs = 'D:\logs' } }) | Should -Be '\\SERVER01\share'
    Get-ItcmdrRotationRoot ([pscustomobject]@{ paths = [pscustomobject]@{ localLogs = 'D:\logs' } }) | Should -Be 'D:\logs'
    Get-ItcmdrRotationRoot $null | Should -Be 'C:\maintenanceRuns'
  }
}
