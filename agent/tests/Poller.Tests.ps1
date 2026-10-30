Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . (Join-Path (Join-Path $PSScriptRoot '..') 'Poll-ItcmdrRequests.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
}

Describe 'Get-ItcmdrAgentArgument' {
  It 'passes repair, force and categories through; reportOnly wins' {
    $a = Get-ItcmdrAgentArgument -Request @{ reportOnly = $false; repair = $true; force = $true; categories = @('network', 'services') }
    $a.Repair | Should -BeTrue
    $a.Force | Should -BeTrue
    $a.ContainsKey('ReportOnly') | Should -BeFalse
    $a.Categories | Should -Be @('network', 'services')
    $b = Get-ItcmdrAgentArgument -Request @{ reportOnly = $true; repair = $true; categories = @() }
    $b.ReportOnly | Should -BeTrue
    $b.ContainsKey('Repair') | Should -BeFalse
    $b.ContainsKey('Categories') | Should -BeFalse
  }
}

Describe 'Invoke-ItcmdrPoll' {
  BeforeEach {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-poll-' + [guid]::NewGuid())
    $req = Join-Path $root 'requests'
    New-Item -ItemType Directory -Path $req -Force | Out-Null
    $calls = Join-Path $root 'calls.txt'
    $agent = Join-Path $root 'FakeAgent.ps1'
    Set-Content -LiteralPath $agent -Value @"
param([switch]`$ReportOnly, [switch]`$Repair, [switch]`$Force, [string[]]`$Categories)
Add-Content -LiteralPath '$calls' -Value ("ro=`$ReportOnly repair=`$Repair cats=" + (`$Categories -join ','))
if (`$Categories -contains 'boom') { throw 'agent failed' }
"@
  }
  AfterEach { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }

  It 'runs a matching request once and records done' {
    Set-Content -LiteralPath (Join-Path $req 'r1.json') -Value '{"machines":["WS-01"],"reportOnly":false,"repair":true,"categories":["network"]}'
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-01' -AgentPath $agent | Should -Be 1
    Get-Content $calls | Should -Be 'ro=False repair=True cats=network'
    $resp = Get-Content (Join-Path $root 'responses/r1.WS-01.json') -Raw | ConvertFrom-Json
    $resp.status | Should -Be 'done'
    $resp.machine | Should -Be 'WS-01'
    # Second poll: already answered, nothing runs again.
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-01' -AgentPath $agent | Should -Be 0
    @(Get-Content $calls).Count | Should -Be 1
    @(Get-ChildItem $req -Filter '*.lock').Count | Should -Be 0
  }

  It 'ignores requests for other machines and lets every machine answer an all-machines request' {
    Set-Content -LiteralPath (Join-Path $req 'r2.json') -Value '{"machines":["WS-09"]}'
    Set-Content -LiteralPath (Join-Path $req 'r3.json') -Value '{"machines":[]}'
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-01' -AgentPath $agent | Should -Be 1
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-02' -AgentPath $agent | Should -Be 1
    Test-Path (Join-Path $root 'responses/r3.WS-01.json') | Should -BeTrue
    Test-Path (Join-Path $root 'responses/r3.WS-02.json') | Should -BeTrue
    Test-Path (Join-Path $root 'responses/r2.WS-01.json') | Should -BeFalse
  }

  It 'records failed when the agent throws' {
    Set-Content -LiteralPath (Join-Path $req 'r4.json') -Value '{"machines":[],"categories":["boom"]}'
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-01' -AgentPath $agent | Out-Null
    (Get-Content (Join-Path $root 'responses/r4.WS-01.json') -Raw | ConvertFrom-Json).status | Should -Be 'failed'
  }

  It 'skips a request another poll on this machine has locked' {
    Set-Content -LiteralPath (Join-Path $req 'r5.json') -Value '{"machines":[]}'
    New-Item -ItemType File -Path (Join-Path $req 'r5.json.WS-01.lock') | Out-Null
    Invoke-ItcmdrPoll -RequestDirectory $req -Computer 'WS-01' -AgentPath $agent | Should -Be 0
  }
}
