Set-StrictMode -Version Latest

Describe 'Invoke-ItcmdrAgent.ps1' {
  It 'parses and runs in ReportOnly without throwing' {
    $script = Join-Path $PSScriptRoot '..' 'Invoke-ItcmdrAgent.ps1'
    { & $script -ReportOnly } | Should -Not -Throw
  }
}

