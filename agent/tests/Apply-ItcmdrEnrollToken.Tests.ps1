Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  $script:Script = Join-Path $PSScriptRoot '..\Apply-ItcmdrEnrollToken.ps1'
  . $script:Script
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  $script:Pwsh = (Get-Command pwsh -ErrorAction SilentlyContinue).Source
  if (-not $script:Pwsh) { $script:Pwsh = (Get-Command powershell).Source }
}

Describe 'Get-ItcmdrNormalizedEnrollToken' {
  It 'trims whitespace and surrounding quotes' {
    Get-ItcmdrNormalizedEnrollToken -Raw "  `"itcmdr-enroll:abc`" `n" | Should -Be 'itcmdr-enroll:abc'
  }
  It 'picks the token line out of mixed stderr' {
    $raw = @"
one-time token for WS-01 until tomorrow; agents dial SERVER01:50052 and pin abcd
itcmdr-enroll:THESECRETTOKENVALUE
"@
    Get-ItcmdrNormalizedEnrollToken -Raw $raw | Should -Be 'itcmdr-enroll:THESECRETTOKENVALUE'
  }
  It 'finds a token that shares a line with other text' {
    Get-ItcmdrNormalizedEnrollToken -Raw "note itcmdr-enroll:XYZ123 more" | Should -Be 'itcmdr-enroll:XYZ123'
  }
  It 'returns empty for blank input' {
    Get-ItcmdrNormalizedEnrollToken -Raw "   `n`n" | Should -Be ''
  }
}

Describe 'Get-ItcmdrEnrollTokenFromInput' {
  It 'reads a token file that contains stderr noise and blank lines' {
    $d = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-tok-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $d | Out-Null
    try {
      $tf = Join-Path $d 'tok.txt'
      @"
one-time token for OLDNAME until later; agents dial SERVER01:50052 and pin deadbeef

  itcmdr-enroll:FILETOKENVALUE  

"@ | Set-Content -LiteralPath $tf
      Get-ItcmdrEnrollTokenFromInput -TokenFile $tf | Should -Be 'itcmdr-enroll:FILETOKENVALUE'
    } finally { Remove-Item -LiteralPath $d -Recurse -Force -ErrorAction SilentlyContinue }
  }
}

Describe 'Invoke-ItcmdrEnrollToken' {
  BeforeEach {
    $script:d = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-enroll-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $d | Out-Null
    $script:agentJson = Join-Path $d 'agent.json'
    '{"machine":"OLDNAME","role":"workstation","agent_id":"old"}' | Set-Content -LiteralPath $agentJson
    New-Item -ItemType Directory -Path (Join-Path $d 'keys') | Out-Null
    'oldkey' | Set-Content -LiteralPath (Join-Path $d 'keys\agent.key')
  }
  AfterEach { Remove-Item -LiteralPath $d -Recurse -Force -ErrorAction SilentlyContinue }

  It 'rolls back agent.json and keys when enroll fails' {
    Mock Invoke-ItcmdrAgentEnrollExe { throw 'itcmdr-agent enroll exited 1' }
    { Invoke-ItcmdrEnrollToken -Token 'itcmdr-enroll:AAAABBBBCCCCDDDD' -AgentExe 'fake' `
        -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $true } | Should -Throw '*enroll exited 1*'
    $j = Get-Content -LiteralPath $agentJson -Raw | ConvertFrom-Json
    $j.machine | Should -Be 'OLDNAME'
    (Get-Content -LiteralPath (Join-Path $d 'keys\agent.key') -Raw).Trim() | Should -Be 'oldkey'
  }

  It 'replaces enrollment under the current computer name on success' {
    Mock Invoke-ItcmdrAgentEnrollExe {
      '{"machine":"NEWPC","role":"workstation","agent_id":"new"}' | Set-Content -LiteralPath (Join-Path $DataDir 'agent.json')
      'newkey' | Set-Content -LiteralPath (Join-Path $DataDir 'keys\agent.key')
    }
    Invoke-ItcmdrEnrollToken -Token 'itcmdr-enroll:AAAABBBBCCCCDDDD' -AgentExe 'fake' `
      -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $true
    $j = Get-Content -LiteralPath $agentJson -Raw | ConvertFrom-Json
    $j.machine | Should -Be 'NEWPC'
    (Get-Content -LiteralPath (Join-Path $d 'keys\agent.key') -Raw).Trim() | Should -Be 'newkey'
    Test-Path -LiteralPath (Join-Path $d 'agent.json.bak-enroll') | Should -BeFalse
  }

  It 'rejects a successful enroll that kept the old machine name' {
    Mock Invoke-ItcmdrAgentEnrollExe {
      '{"machine":"OLDNAME","role":"workstation","agent_id":"new"}' | Set-Content -LiteralPath (Join-Path $DataDir 'agent.json')
    }
    { Invoke-ItcmdrEnrollToken -Token 'itcmdr-enroll:AAAABBBBCCCCDDDD' -AgentExe 'fake' `
        -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $true } | Should -Throw '*current computer name*'
    (Get-Content -LiteralPath $agentJson -Raw | ConvertFrom-Json).machine | Should -Be 'OLDNAME'
  }

  It 'deletes the token file only after a successful enroll' {
    $tf = Join-Path $d 'agent-enroll.token'
    'itcmdr-enroll:AAAABBBBCCCCDDDD' | Set-Content -LiteralPath $tf
    Mock Invoke-ItcmdrAgentEnrollExe { throw 'boom' }
    { Invoke-ItcmdrEnrollToken -TokenFile $tf -AgentExe 'fake' `
        -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $true } | Should -Throw
    Test-Path -LiteralPath $tf | Should -BeTrue
    Mock Invoke-ItcmdrAgentEnrollExe {
      '{"machine":"NEWPC","role":"workstation"}' | Set-Content -LiteralPath (Join-Path $DataDir 'agent.json')
    }
    Invoke-ItcmdrEnrollToken -TokenFile $tf -AgentExe 'fake' `
      -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $true
    Test-Path -LiteralPath $tf | Should -BeFalse
  }

  It 'fails the connect check unless a new linked line appears' {
    New-Item -ItemType Directory -Path (Join-Path $d 'logs') | Out-Null
    'old linked to the server' | Set-Content -LiteralPath (Join-Path $d 'logs\agent.log')
    Mock Invoke-ItcmdrAgentEnrollExe {
      '{"machine":"NEWPC","role":"workstation"}' | Set-Content -LiteralPath (Join-Path $DataDir 'agent.json')
    }
    { Invoke-ItcmdrEnrollToken -Token 'itcmdr-enroll:AAAABBBBCCCCDDDD' -AgentExe 'fake' `
        -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl $true -SkipConnectCheck $false -ConnectTimeoutSec 1 } | Should -Throw '*did not link*'
    (Get-Content -LiteralPath $agentJson -Raw | ConvertFrom-Json).machine | Should -Be 'OLDNAME'
  }
}

Describe 'Apply-ItcmdrEnrollToken.ps1 exit code' {
  It 'exits 9 when enroll fails (never 0)' {
    $d = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-enroll-exit-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $d | Out-Null
    try {
      '{"machine":"OLDNAME","role":"workstation"}' | Set-Content -LiteralPath (Join-Path $d 'agent.json')
      $stub = Join-Path $d 'fail.ps1'
      'exit 1' | Set-Content -LiteralPath $stub
      & $script:Pwsh -NoProfile -File $script:Script -Token 'itcmdr-enroll:AAAABBBBCCCCDDDD' `
        -AgentExe $stub -DataDir $d -ComputerName 'NEWPC' -SkipServiceControl -SkipConnectCheck
      $LASTEXITCODE | Should -Be 9
      (Get-Content -LiteralPath (Join-Path $d 'agent.json') -Raw | ConvertFrom-Json).machine | Should -Be 'OLDNAME'
    } finally { Remove-Item -LiteralPath $d -Recurse -Force -ErrorAction SilentlyContinue }
  }
}
