Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . (Join-Path (Join-Path $PSScriptRoot '..') 'Invoke-ItcmdrAgent.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue

  function Get-TestContext([object[]]$Rules, [string[]]$Categories = @()) {
    $log = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-test-' + [guid]::NewGuid() + '.log')
    return @{ Site = @{}; RunId = 'run-1'; Computer = 'WS-01'; Role = 'Workstation'; LogFile = $log; Exceptions = $Rules; Categories = $Categories }
  }
  function Get-LastCheck([hashtable]$Ctx) {
    if (-not (Test-Path $Ctx.LogFile)) { return $null }
    Get-Content -LiteralPath $Ctx.LogFile | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.type -eq 'check' } | Select-Object -Last 1
  }
}

Describe 'Find-ItcmdrException' {
  It 'matches by machine, role, id, pattern and expiry' {
    $rules = @(
      @{ scope = @{ machines = @('WS-02') }; checkId = 'A'; action = 'ignore'; reason = 'other machine' },
      @{ scope = @{ role = 'Workstation' }; checkPattern = '^PRN-'; action = 'downgrade:warn'; reason = 'role' },
      @{ scope = @{ machines = @('WS-01') }; checkId = 'OLD'; action = 'ignore'; reason = 'expired'; expires = '2020-01-01' },
      @{ scope = @{}; checkId = 'ALL'; action = 'ignore'; reason = 'everyone' }
    )
    Find-ItcmdrException -Rules $rules -Id 'A' -Computer 'WS-01' -Role 'Workstation' | Should -BeNullOrEmpty
    (Find-ItcmdrException -Rules $rules -Id 'PRN-01-9100' -Computer 'WS-01' -Role 'Workstation').reason | Should -Be 'role'
    Find-ItcmdrException -Rules $rules -Id 'OLD' -Computer 'WS-01' -Role 'Workstation' | Should -BeNullOrEmpty
    (Find-ItcmdrException -Rules $rules -Id 'ALL' -Computer 'WS-07' -Role 'Server').reason | Should -Be 'everyone'
  }
  It 'treats a date-only expiry as valid through that whole day' {
    $r = @(@{ scope = @{}; checkId = 'X'; action = 'ignore'; reason = 'r'; expires = '2026-10-29' })
    Find-ItcmdrException -Rules $r -Id 'X' -Computer 'WS-01' -Role 'Workstation' -Now ([datetime]'2026-10-29T18:00:00') | Should -Not -BeNullOrEmpty
    Find-ItcmdrException -Rules $r -Id 'X' -Computer 'WS-01' -Role 'Workstation' -Now ([datetime]'2026-10-30T00:00:01') | Should -BeNullOrEmpty
  }
}

Describe 'Get-ItcmdrException' {
  It 'merges site.json rules with the server export and caches the export for offline runs' {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-ex-' + [guid]::NewGuid())
    $share = Join-Path $tmp 'share'; $cache = Join-Path $tmp 'cache'
    New-Item -ItemType Directory -Path $share -Force | Out-Null
    $export = '{"version":1,"generated":"2026-10-29T00:00:00Z","rules":[{"id":"x1","scope":{"machines":["WS-01"]},"checkId":"NET-SERVER-TCP-445","action":"ignore","reason":"from server","expires":null}]}'
    Set-Content -LiteralPath (Join-Path $share 'exceptions.json') -Value $export
    $site = @{ shares = @{ maintenanceRuns = $share }; exceptions = @(@{ scope = @{}; checkId = 'LOCAL'; action = 'ignore'; reason = 'from site' }) }
    try {
      $rules = Get-ItcmdrException -Site $site -CacheDir $cache
      @($rules).Count | Should -Be 2
      (Find-ItcmdrException -Rules $rules -Id 'NET-SERVER-TCP-445' -Computer 'WS-01' -Role 'Workstation').reason | Should -Be 'from server'
      Remove-Item -LiteralPath (Join-Path $share 'exceptions.json')
      $offline = Get-ItcmdrException -Site $site -CacheDir $cache
      @($offline).Count | Should -Be 2
    } finally { Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue }
  }
}

Describe 'Invoke-ItcmdrCheck' {
  It 'ignores a failed check when exception action=ignore' {
    $ctx = Get-TestContext @(@{ scope = @{ machines = @('WS-01') }; checkId = 'CHK-IGNORE'; action = 'ignore'; reason = 'unit test' })
    Invoke-ItcmdrCheck -Id 'CHK-IGNORE' -Category 'network' -Test { $false } -Repair { $false } -Ctx $ctx
    $rec = Get-LastCheck $ctx
    $rec.status | Should -Be 'ok'
    $rec.excepted | Should -BeTrue
    $rec.attemptedRepair | Should -BeFalse
    Remove-Item $ctx.LogFile -Force
  }
  It 'repairs when not report-only and records the result' {
    $ctx = Get-TestContext @()
    Invoke-ItcmdrCheck -Id 'CHK-R' -Category 'network' -Test { $false } -Repair { $true } -Ctx $ctx
    $rec = Get-LastCheck $ctx
    $rec.status | Should -Be 'ok'; $rec.attemptedRepair | Should -BeTrue; $rec.result | Should -Be 'success'
    Remove-Item $ctx.LogFile -Force
  }
  It 'does not attempt a repair in report-only mode or when no repair exists' {
    $ctx = Get-TestContext @()
    Invoke-ItcmdrCheck -Id 'CHK-RO' -Category 'network' -Test { $false } -Repair { $true } -ReportOnlyLocal -Ctx $ctx
    (Get-LastCheck $ctx).attemptedRepair | Should -BeFalse
    Invoke-ItcmdrCheck -Id 'CHK-NR' -Category 'network' -Test { $false } -Ctx $ctx
    $rec = Get-LastCheck $ctx
    $rec.status | Should -Be 'failed'; $rec.attemptedRepair | Should -BeFalse
    Remove-Item $ctx.LogFile -Force
  }
  It 'honours -Categories' {
    $ctx = Get-TestContext @() @('services')
    Invoke-ItcmdrCheck -Id 'CHK-NET' -Category 'network' -Test { $true } -Ctx $ctx
    Get-LastCheck $ctx | Should -BeNullOrEmpty
    Invoke-ItcmdrCheck -Id 'CHK-SVC' -Category 'services' -Test { $true } -Ctx $ctx
    (Get-LastCheck $ctx).checkId | Should -Be 'CHK-SVC'
    Remove-Item $ctx.LogFile -Force
  }
}

Describe 'Resolve-ItcmdrRepairMode' {
  It 'combines -ReportOnly, -Repair, SYSTEM and the clinical guard' {
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $false -ReportOnly $false -Repair $false -IsSystem $true -Quiet $true -Force $false | Should -BeFalse
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $false -ReportOnly $false -Repair $false -IsSystem $false -Quiet $true -Force $false | Should -BeTrue
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $false -ReportOnly $false -Repair $true -IsSystem $false -Quiet $true -Force $false | Should -BeFalse
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $true -ReportOnly $true -Repair $true -IsSystem $true -Quiet $true -Force $false | Should -BeTrue
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $false -ReportOnly $false -Repair $true -IsSystem $true -Quiet $false -Force $false | Should -BeTrue
    Resolve-ItcmdrRepairMode -ReportOnlyGiven $false -ReportOnly $false -Repair $true -IsSystem $true -Quiet $false -Force $true | Should -BeFalse
  }
}

Describe 'ConvertFrom-ItcmdrJson' {
  It 'returns nested hashtables (works without -AsHashtable)' {
    $h = ConvertFrom-ItcmdrJson '{"a":{"b":[{"c":1},{"c":2}]},"s":"x"}'
    $h | Should -BeOfType [hashtable]
    $h.a | Should -BeOfType [hashtable]
    @($h.a.b).Count | Should -Be 2
    $h.a.b[1].c | Should -Be 2
  }
}
