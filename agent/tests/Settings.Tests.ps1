Set-StrictMode -Version Latest

# v0.9.1: agent settings served centrally (agent-settings.json), site.json as local override.
Describe 'Get-ItcmdrSetting' {
  BeforeAll {
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path (Join-Path $PSScriptRoot '..') 'Invoke-ItcmdrAgent.ps1')
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
    $script:central = @{
      schema   = 'itcmdr-agent-settings/v1'
      global   = @{ 'agent.quietMinutes' = 15; 'retention.agentLocalDays' = 200 }
      machines = @{ 'WS-01' = @{ 'agent.quietMinutes' = 30 } }
    }
  }
  BeforeEach { $script:ItcmdrCentral = $null }

  It 'defaults equal the old hard-coded values' {
    $none = @{}
    $want = @{
      'paths.localLogs' = 'C:\maintenanceRuns'; 'paths.agentData' = 'C:\ProgramData\itcmdr'
      'retention.agentLocalDays' = 400; 'agent.quietMinutes' = 10; 'agent.printerStuckMinutes' = 60
      'agent.backupMaxAgeHours' = 26; 'agent.backupMinBytes' = 1; 'agent.tcpTimeoutMs' = 3000
      'agent.staleLockMinutes' = 150; 'agent.nightlyTime' = '01:30'; 'rotation.timeLocal' = '02:10'
    }
    foreach ($k in $want.Keys) { Get-ItcmdrSetting -Site @{} -Key $k -Central $none | Should -Be $want[$k] }
  }
  It 'applies central global, then per machine, then site.json' {
    Get-ItcmdrSetting -Site @{} -Key 'agent.quietMinutes' -Computer 'WS-02' -Central $central | Should -Be 15
    Get-ItcmdrSetting -Site @{} -Key 'agent.quietMinutes' -Computer 'ws-01' -Central $central | Should -Be 30
    Get-ItcmdrSetting -Site @{ agent = @{ quietMinutes = 5 } } -Key 'agent.quietMinutes' -Computer 'WS-01' -Central $central | Should -Be 5
    Get-ItcmdrSetting -Site @{} -Key 'retention.agentLocalDays' -Computer 'WS-01' -Central $central | Should -Be 200
  }
  It 'rejects unknown keys' {
    { Get-ItcmdrSetting -Site @{} -Key 'agent.nope' -Central @{} } | Should -Throw
  }
  It 'picks up agent-settings.json from the share and caches it for offline runs' {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-set-' + [guid]::NewGuid())
    $share = Join-Path $root 'share'; $data = Join-Path $root 'data'
    New-Item -ItemType Directory -Path $share -Force | Out-Null
    try {
      ($central | ConvertTo-Json -Depth 5) | Set-Content -LiteralPath (Join-Path $share 'agent-settings.json')
      $site = @{ shares = @{ maintenanceRuns = $share }; paths = @{ agentData = $data } }
      Get-ItcmdrSetting -Site $site -Key 'agent.quietMinutes' -Computer 'WS-01' | Should -Be 30
      Test-Path -LiteralPath (Join-Path $data 'agent-settings.json') | Should -BeTrue
      # share gone: the cache still answers
      Remove-Item -LiteralPath (Join-Path $share 'agent-settings.json')
      $script:ItcmdrCentral = $null
      Get-ItcmdrSetting -Site $site -Key 'agent.quietMinutes' -Computer 'WS-02' | Should -Be 15
      # central paths.localLogs moves the run logs
      $script:ItcmdrCentral = @{ global = @{ 'paths.localLogs' = $root } }
      Get-ItcmdrPath -Site @{} -Name localLogs | Should -Be $root
    } finally { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue; $script:ItcmdrCentral = $null }
  }
  It 'parses the nightly time' {
    $d = [datetime]'2026-10-30'
    (ConvertTo-ItcmdrNightlyTime -Time '01:30' -Today $d) | Should -Be $d.AddMinutes(90)
    { ConvertTo-ItcmdrNightlyTime -Time '25:00' } | Should -Throw
  }
}

Describe 'Get-ItcmdrPollSetting' {
  BeforeAll {
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path (Join-Path $PSScriptRoot '..') 'Poll-ItcmdrRequests.ps1')
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  }
  It 'reads staleLockMinutes and localLogs from the central file, site.json wins' {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-pset-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    try {
      '{"global":{"agent.staleLockMinutes":90,"paths.localLogs":"D:\\runs"},"machines":{"WS-01":{"agent.staleLockMinutes":45}}}' |
        Set-Content -LiteralPath (Join-Path $root 'agent-settings.json')
      $site = [pscustomobject]@{ shares = [pscustomobject]@{ maintenanceRuns = $root } }
      Get-ItcmdrPollSetting -Site $site -Key 'agent.staleLockMinutes' -Default 150 -Computer 'WS-02' | Should -Be 90
      Get-ItcmdrPollSetting -Site $site -Key 'agent.staleLockMinutes' -Default 150 -Computer 'WS-01' | Should -Be 45
      Get-ItcmdrPollSetting -Site $null -Key 'agent.staleLockMinutes' -Default 150 | Should -Be 150
      @(Get-ItcmdrRequestDirectory -Site $site)[0] | Should -Be ([IO.Path]::Combine('D:\runs', 'requests'))
      $site2 = [pscustomobject]@{ paths = [pscustomobject]@{ localLogs = 'E:\x' }; shares = [pscustomobject]@{ maintenanceRuns = $root } }
      @(Get-ItcmdrRequestDirectory -Site $site2)[0] | Should -Be ([IO.Path]::Combine('E:\x', 'requests'))
    } finally { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
  }
}
