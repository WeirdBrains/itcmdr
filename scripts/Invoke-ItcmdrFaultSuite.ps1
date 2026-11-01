<#
.SYNOPSIS
  Run the v0.15.2 fault-injection suite (agent/tests/fault) on a disposable Windows TEST BED.
.DESCRIPTION
  Makes real breaks (stops/disables a service, corrupts/deletes configs, locks a log folder, breaks folder ACLs,
  kills the agent mid-run, drops the server link) against generic, configurable targets and checks detection,
  repair or report-only behaviour. Never run it on a production server: it refuses unless -IAmATestBed is given.
  Afterwards it checks the service is back in its original state and removes the scratch folder.
.PARAMETER Service   A harmless service the suite may stop and disable (default W32Time).
.PARAMETER Root      Scratch folder (default C:\itcmdr-fault); deleted at the end.
#>
param([switch]$IAmATestBed, [string]$Service = 'W32Time', [string]$Root = 'C:\itcmdr-fault', [string]$ResultXml)
$ErrorActionPreference = 'Stop'
if (-not $IAmATestBed) { throw 'Refusing: pass -IAmATestBed (this suite breaks things on purpose).' }
$s0 = Get-CimInstance Win32_Service -Filter "Name='$Service'"
if (-not $s0) { throw "service $Service not found" }
$env:ITCMDR_FAULT_TESTBED = '1'; $env:ITCMDR_FAULT_SERVICE = $Service; $env:ITCMDR_FAULT_ROOT = $Root
try {
  $c = New-PesterConfiguration
  $c.Run.Path = (Join-Path $PSScriptRoot '..\agent\tests\fault'); $c.Run.PassThru = $true; $c.Output.Verbosity = 'Detailed'
  if ($ResultXml) { $c.TestResult.Enabled = $true; $c.TestResult.OutputPath = $ResultXml }
  $r = Invoke-Pester -Configuration $c
} finally {
  Remove-Item Env:ITCMDR_FAULT_TESTBED, Env:ITCMDR_FAULT_SERVICE, Env:ITCMDR_FAULT_ROOT -ErrorAction SilentlyContinue
  $s1 = Get-CimInstance Win32_Service -Filter "Name='$Service'"
  "service restored: start $($s0.StartMode)->$($s1.StartMode), state $($s0.State)->$($s1.State)"
  if (Test-Path $Root) { Remove-Item $Root -Recurse -Force -ErrorAction SilentlyContinue }
  "scratch removed: $(-not (Test-Path $Root))"
}
"FAULT $($r.PassedCount)/$($r.TotalCount) failed $($r.FailedCount)"
foreach ($f in $r.Failed) { "FAILED: $($f.ExpandedPath): $($f.ErrorRecord[0])" }
if ($r.FailedCount) { exit 1 }
