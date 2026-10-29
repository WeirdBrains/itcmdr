Set-StrictMode -Version Latest

Describe 'Exceptions handling in Invoke-ItcmdrCheck' {
  AfterAll {
    Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
  }

  BeforeAll {
    $env:ITCMDR_IMPORT_ONLY = '1'
    . (Join-Path $PSScriptRoot '..' 'Invoke-ItcmdrAgent.ps1')
  }

  It 'ignores a failed check when exception action=ignore' {
    $comp = if ($env:COMPUTERNAME) { $env:COMPUTERNAME } else { 'WS-01' }
    $ctx = @{
      Site = @{
        exceptions = @(@{
          scope = @{ machines = @($comp) }
          checkId = 'CHK-IGNORE'
          action = 'ignore'
          reason = 'unit test'
        })
      }
      RunId = 'run-1'; Computer=$comp; LogFile = (Join-Path $PSScriptRoot 'itcmdr_test.log'); Role='Workstation'
    }
    if (Test-Path $ctx.LogFile) { Remove-Item $ctx.LogFile -Force }
    Invoke-ItcmdrCheck -Id 'CHK-IGNORE' -Category 'test' -Test { $false } -Repair { $false } -ReportOnlyLocal:$false -Ctx $ctx
    $lines = Get-Content -LiteralPath $ctx.LogFile
    $rec = $lines | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.type -eq 'check' } | Select-Object -Last 1
    $rec | Should -Not -BeNullOrEmpty
    $rec.status | Should -Be 'ok'
    $rec.excepted | Should -BeTrue
    Remove-Item $ctx.LogFile -Force -ErrorAction SilentlyContinue
  }
}

