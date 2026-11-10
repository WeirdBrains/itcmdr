<#
.SYNOPSIS
  Test fixture only, never shipped: a stand-in for the built-in check plugins of the private
  itcmdr-plugins repo, so agent/tests/BuiltinPlugins.Tests.ps1 needs no third_party checkout.

.DESCRIPTION
  Dot-sourced by <name>/main.ps1 next to this file. Like the real plugins (which bundle
  Itcmdr.Checks.ps1), it speaks the plugin protocol through Invoke-ItcmdrPlugin (stdin JSON, one
  ITCMDR-RESULT line) and runs the agent's own check function through Test-ItcmdrBuiltinPluginReady
  (verify) and Invoke-ItcmdrBuiltinCheckSet (execute). Records are not canned: the parity tests still
  compare what the plugin process reports with what the task's check function writes.
  The tests point the agent here with ITCMDR_BUILTIN_PLUGINS_DIR.
#>
Set-StrictMode -Version Latest

# agent/tests/fixtures/builtin-plugins -> agent
$script:MockAgentDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../..')).Path
. (Join-Path $script:MockAgentDir 'Itcmdr.Checks.ps1')
Import-Module (Join-Path $script:MockAgentDir 'Itcmdr.Plugin.psm1') -Force

function Invoke-ItcmdrMockBuiltinPlugin {
  <# Run one protocol stage of the mock built-in plugin $Name (one of $script:ItcmdrBuiltinPlugins). #>
  param([Parameter(Mandatory)][string]$Name)
  if (-not $script:ItcmdrBuiltinPlugins.Contains($Name)) { throw "not a built-in plugin: $Name" }
  $script:MockCheckFunction = $script:ItcmdrBuiltinPlugins[$Name]
  $info = @{ name = $Name; version = '0.0.0-test'; features = @('verify', 'execute'); description = "test mock over $($script:MockCheckFunction)" }
  Invoke-ItcmdrPlugin -Info $info `
    -Verify { Test-ItcmdrBuiltinPluginReady -CheckFunction $script:MockCheckFunction } `
    -Execute { param($Ctx) Invoke-ItcmdrBuiltinCheckSet -CheckFunction $script:MockCheckFunction -PluginCtx $Ctx }
}
