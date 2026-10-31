Set-StrictMode -Version Latest
# v0.15.0 plugin error contract in the PowerShell module: New-ItcmdrPluginError,
# Write-ItcmdrPluginError and how Invoke-ItcmdrPlugin reports errors to itcmdr-plugin-ps.

BeforeAll {
  Import-Module (Join-Path (Join-Path $PSScriptRoot '..') 'Itcmdr.Plugin.psm1') -Force
  function Invoke-WithExecute([scriptblock]$Execute) {
    $ctx = '{"run_id":"r-1","log_path":"","machine":"SERVER01","report_only":true,"args":{},"stage":"execute"}'
    $out = Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.2.0' } -Execute $Execute -InputJson ('{"stage":"execute","ctx":' + $ctx + '}')
    $line = @($out)[-1]
    return ($line.Substring('ITCMDR-RESULT|'.Length) | ConvertFrom-Json)
  }
}

Describe 'plugin errors' {
  It 'New-ItcmdrPluginError builds a record with a backtrace' {
    $e = New-ItcmdrPluginError -Message 'share unreachable' -Criticality Critical -Context @{ share = 'x' }
    $e.criticality | Should -Be 'critical'
    $e.error | Should -Be 'share unreachable'
    $e.kind | Should -Be 'ItcmdrPluginError'
    $e.backtrace | Should -Not -BeNullOrEmpty
    $e.context.share | Should -Be 'x'
  }

  It 'rejects an unknown criticality' {
    { New-ItcmdrPluginError -Message 'x' -Criticality 'Fatal' } | Should -Throw
  }

  It 'a thrown plugin error fails the stage with its criticality' {
    $r = Invoke-WithExecute { param($Ctx) $null = $Ctx; throw (New-ItcmdrPluginError -Message 'backup share is gone' -Criticality Critical -Kind 'ShareMissing') }
    $r.error | Should -Be 'backup share is gone'
    $r.plugin_error.criticality | Should -Be 'critical'
    $r.plugin_error.kind | Should -Be 'ShareMissing'
    $r.plugin_error.service_plugin | Should -Be 'disk-free'
    $r.plugin_error.backtrace | Should -Match 'line'
  }

  It 'an uncaught exception leaves the criticality to the server default' {
    $r = Invoke-WithExecute { param($Ctx) $null = $Ctx; [int]::Parse('nope') | Out-Null }
    $r.error | Should -Not -BeNullOrEmpty
    $r.plugin_error.criticality | Should -Be ''
    $r.plugin_error.kind | Should -Match 'Exception'
    $r.plugin_error.backtrace | Should -Not -BeNullOrEmpty
  }

  It 'a returned plugin error fails the stage' {
    $r = Invoke-WithExecute { param($Ctx) $null = $Ctx; New-ItcmdrPluginError -Message 'access denied' -Criticality Cautious }
    $r.ok | Should -BeFalse
    $r.plugin_error.criticality | Should -Be 'cautious'
  }

  It 'Write-ItcmdrPluginError is non-fatal and sends the worst one' {
    $r = Invoke-WithExecute { param($Ctx) $null = $Ctx
      Write-ItcmdrPluginError -Message 'slow share' -Criticality Informational
      Write-ItcmdrPluginError -Message 'one disk is 91% full' -Criticality Cautious
      @{ ok = $true; message = 'done' } }
    $r.ok | Should -BeTrue
    $r.message | Should -Be 'done'
    $r.plugin_error.error | Should -Be 'one disk is 91% full'
    $r.plugin_error.criticality | Should -Be 'cautious'
  }

  It 'a stage without errors sends no plugin_error' {
    $r = Invoke-WithExecute { param($Ctx) $null = $Ctx; @{ ok = $true; message = 'fine' } }
    $r.PSObject.Properties.Name | Should -Not -Contain 'plugin_error'
  }

  It 'long messages and backtraces are cut' {
    $e = New-ItcmdrPluginError -Message ('x' * 10000) -Criticality Informational
    $e.error.Length | Should -BeLessOrEqual 4096
  }
}
