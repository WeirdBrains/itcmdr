<#
.SYNOPSIS
  Windows VM upgrade smoke: FromVer -> ToVer, stamps, failureflag, self-heal (watchdog-only <=90s), /EnrollToken=.
.DESCRIPTION
  Disposable test-bed only. Prints OK/FAIL lines and never secrets (no tokens, no machine names).
  Paths and versions are parameters; VM host details stay out of this file (see HANDOVER.local.md).

  Example (1.3.6 -> 1.3.7):
    .\e2e\v137.ps1 -FromVer 1.3.6 -ToVer 1.3.7 `
      -Commit <tag-commit> -Help <help-hash> `
      -RelDir C:\path\to\rel-1.3.7 -PublishedDir C:\path\to\published `
      -DocsDir C:\path\to\tag\docs

.PARAMETER FromVer          Installed version before the upgrade (e.g. 1.3.6).
.PARAMETER ToVer            Target version (e.g. 1.3.7).
.PARAMETER Commit           Full git commit of the ToVer tag (for ITCMDR-BUILD stamps).
.PARAMETER Help             16-hex help hash from the ToVer stamp.
.PARAMETER RelDir           Folder with itcmdr-server-Setup-<ToVer>.exe and itcmdr-agent-Setup-<ToVer>.exe.
.PARAMETER PublishedDir     Folder with a published itcmdr-agent-Setup-<ToVer>.exe for /EnrollToken= checks.
.PARAMETER DocsDir          Optional tag checkout docs/ for help-vs-docs (skipped if missing).
.PARAMETER InstallRoot      Install root (default: under ProgramFiles\itcmdr).
.PARAMETER WatchdogOnlySecs Max seconds for a server-watchdog-only kill to return Running (default 90).
#>
param(
  [Parameter(Mandatory)][string]$FromVer,
  [Parameter(Mandatory)][string]$ToVer,
  [Parameter(Mandatory)][string]$Commit,
  [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{16}$')][string]$Help,
  [Parameter(Mandatory)][string]$RelDir,
  [Parameter(Mandatory)][string]$PublishedDir,
  [string]$DocsDir = '',
  [string]$InstallRoot = (Join-Path $env:ProgramFiles 'itcmdr'),
  [int]$WatchdogOnlySecs = 90
)

$ErrorActionPreference = 'Continue'
$fails = 0
$skipped = @()
function Chk([bool]$ok, [string]$what) { if ($ok) { "OK   $what" } else { "FAIL $what"; $script:fails++ } }
function WaitSvc($n, $sec) {
  for ($i = 0; $i -lt $sec; $i++) {
    $s = Get-Service $n -ErrorAction SilentlyContinue
    if ($s -and $s.Status -eq 'Running') { return $true }
    Start-Sleep 1
  }
  $false
}
function SvcPid($n) { (Get-CimInstance Win32_Service -Filter "Name='$n'").ProcessId }
function StampOf($path) {
  if (-not (Test-Path $path)) { return '' }
  $m = [regex]::Match(
    [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($path)),
    'ITCMDR-BUILD v=\S+ commit=[0-9a-zA-Z]+ help=[0-9a-f]{16} END'
  )
  if ($m.Success) { $m.Value } else { '' }
}
function LastJson($part) {
  $L = Join-Path $env:ProgramData 'itcmdr\install-logs'
  foreach ($name in @("last-$part.json", "last-$part.reported.json")) {
    $f = Join-Path $L $name
    if (Test-Path $f) {
      try { return Get-Content $f -Raw | ConvertFrom-Json } catch { return $null }
    }
  }
  return $null
}
function AgentOnlineAny {
  $lst = (& $script:cli agent list 2>&1 | Out-String)
  return ($lst -match '(?i)\bonline\b')
}
function StatusMachineEqualsComputer {
  $raw = & $script:agent status 2>&1 | Out-String
  try {
    $st = $raw | ConvertFrom-Json
    return ([string]$st.machine -eq $env:COMPUTERNAME)
  } catch {
    return $false
  }
}
function ListLineComputerOnline {
  $lst = & $script:cli agent list 2>&1 | Out-String
  foreach ($line in ($lst -split "`n")) {
    if ($line -match ('^' + [regex]::Escape($env:COMPUTERNAME) + '\s+online')) { return $true }
  }
  return $false
}
function FailureFlagOn($n) {
  $o = & sc.exe qfailureflag $n 2>&1 | Out-String
  return ($o -match 'FAILURE_ACTIONS_ON_NONCRASH_FAILURES:\s+TRUE')
}

# Escape versions for -match
$fromRe = [regex]::Escape($FromVer)
$toRe = [regex]::Escape($ToVer)

if (Test-Path $RelDir) { Get-ChildItem $RelDir -Filter '*.exe' -ErrorAction SilentlyContinue | Unblock-File }
if (Test-Path $PublishedDir) { Get-ChildItem $PublishedDir -Filter '*.exe' -ErrorAction SilentlyContinue | Unblock-File }

$want = "ITCMDR-BUILD v=$ToVer commit=$Commit help=$Help END"
$script:cli = Join-Path $InstallRoot 'server\itcmdr-cli.exe'
$script:agent = Join-Path $InstallRoot 'agent\current\itcmdr-agent.exe'
$svcs = 'itcmdr-server', 'itcmdr-server-watchdog', 'itcmdr-agent', 'itcmdr-agent-watchdog'
$L = Join-Path $env:ProgramData 'itcmdr\install-logs'
$cn = $env:COMPUTERNAME

"== 0 before"
$before = & $cli --version 2>&1 | Out-String
Chk ($before -match $fromRe) ("before cli matches $FromVer")
$ab = & $agent --version 2>&1 | Out-String
Chk ($ab -match $fromRe) ("before agent matches $FromVer")
foreach ($s in $svcs) { Chk (WaitSvc $s 30) "$s Running before upgrade" }

"== 1 upgrade server and agent to $ToVer"
foreach ($p in 'server', 'agent') {
  $setup = Join-Path $RelDir "itcmdr-$p-Setup-$ToVer.exe"
  Chk (Test-Path $setup) "$p installer present locally"
  $rc = (Start-Process $setup -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru).ExitCode
  Chk ($rc -eq 0) "$p setup exit $rc"
}
foreach ($s in $svcs) { Chk (WaitSvc $s 90) "$s Running after upgrade" }
$after = & $cli --version 2>&1 | Out-String
Chk ($after -match $toRe) ("after cli matches $ToVer")
$aa = & $agent --version 2>&1 | Out-String
Chk ($aa -match $toRe) ("after agent matches $ToVer")

"== 2 stamps match the $ToVer tag"
$bins = @{
  'cli' = $cli
  'server' = Join-Path $InstallRoot 'server\itcmdr-server.exe'
  'server-watchdog' = Join-Path $InstallRoot 'server-watchdog\itcmdr-server-watchdog.exe'
  'agent' = $agent
  'agent-watchdog' = Join-Path $InstallRoot 'agent\watchdog\itcmdr-agent-watchdog.exe'
}
foreach ($k in $bins.Keys) {
  $st = StampOf $bins[$k]
  Chk ($st -eq $want) "$k stamp matches tag"
}

"== 3 failureflag 1 restored on upgrade"
foreach ($s in $svcs) {
  Chk (FailureFlagOn $s) "$s failureflag TRUE"
}

"== 4 installed help matches the tag docs"
if ($DocsDir -and (Test-Path $DocsDir)) {
  $n = 0; $bad = @()
  foreach ($f in Get-ChildItem $DocsDir -Filter *.md) {
    $id = $f.BaseName.ToLower(); if ($id -eq 'readme') { $id = 'index' }
    $tmp = Join-Path $env:TEMP 'itcmdr-h.txt'
    cmd /c "`"$cli`" help $id > `"$tmp`" 2>&1"
    $h = ([IO.File]::ReadAllText($tmp, [Text.Encoding]::UTF8) -replace "`r", '').Trim()
    $d = ([IO.File]::ReadAllText($f.FullName, [Text.Encoding]::UTF8) -replace "`r", '').Trim()
    if ($h -ne $d) { $bad += $id }
    $n++
  }
  Chk ($bad.Count -eq 0) "installed itcmdr-cli help = tag docs for $n topics"
} else {
  $skipped += 'help-vs-docs (DocsDir missing)'
  "SKIP help-vs-docs"
}
$e = (& $cli help environment 2>&1) -join "`n"
Chk ($e -match 'ITCMDR_KEEP_BUILD' -and $e -match 'ITCMDR_BUILD_COMMIT') 'help environment has ITCMDR_KEEP_BUILD and ITCMDR_BUILD_COMMIT'

"== 5 installer logs"
foreach ($p in 'server', 'agent') {
  $j = LastJson $p
  Chk ($null -ne $j -and $j.ok -eq $true -and "$($j.to)" -match $toRe) "last-$p log ok to $ToVer"
  $cur = Get-ChildItem $L -Filter "$p-*.log" -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match "^$p-" } |
    Sort-Object LastWriteTime |
    Select-Object -Last 1
  Chk ($null -ne $cur -and $cur.Name -match $toRe) "$p newest log is this run"
}

"== 6 self-heal (watchdog-only must be back Running within ${WatchdogOnlySecs}s)"
Start-Sleep 10
WaitSvc itcmdr-server 60 | Out-Null
$p1 = SvcPid itcmdr-server
if ($p1) { Stop-Process -Id $p1 -Force }
$back = $false
for ($i = 0; $i -lt 180; $i++) {
  Start-Sleep 1
  $p2 = SvcPid itcmdr-server
  if ($p2 -and $p2 -ne $p1 -and (Get-Service itcmdr-server).Status -eq 'Running') { $back = $true; break }
}
Chk $back "killed itcmdr-server came back in ${i}s"
Start-Sleep 8
$w1 = SvcPid itcmdr-server-watchdog
if ($w1) { Stop-Process -Id $w1 -Force }
$back = $false
for ($i = 0; $i -lt $WatchdogOnlySecs; $i++) {
  Start-Sleep 1
  $w2 = SvcPid itcmdr-server-watchdog
  $st = (Get-Service itcmdr-server-watchdog -ErrorAction SilentlyContinue).Status
  if ($w2 -and $w2 -ne $w1 -and $st -eq 'Running') { $back = $true; break }
}
Chk $back "killed server watchdog came back Running in ${i}s (want <=$WatchdogOnlySecs)"
Start-Sleep 8
& sc.exe stop itcmdr-server | Out-Null
Chk (WaitSvc itcmdr-server 180) 'stopped itcmdr-server started again by the watchdog'
Start-Sleep 8
$p1 = SvcPid itcmdr-server; $w1 = SvcPid itcmdr-server-watchdog
if ($p1 -and $w1) { Stop-Process -Id $p1, $w1 -Force }
$back = $false
for ($i = 0; $i -lt 240; $i++) {
  Start-Sleep 1
  if ((Get-Service itcmdr-server).Status -eq 'Running' -and (Get-Service itcmdr-server-watchdog).Status -eq 'Running') {
    $back = $true; break
  }
}
Chk $back "server and watchdog killed together: both back in ${i}s"
Start-Sleep 8
$a1 = SvcPid itcmdr-agent
if ($a1) { Stop-Process -Id $a1 -Force }
$back = $false
for ($i = 0; $i -lt 180; $i++) {
  Start-Sleep 1
  $a2 = SvcPid itcmdr-agent
  if ($a2 -and $a2 -ne $a1 -and (Get-Service itcmdr-agent).Status -eq 'Running') { $back = $true; break }
}
Chk $back "killed itcmdr-agent came back in ${i}s"
Start-Sleep 8
$aw = SvcPid itcmdr-agent-watchdog
if ($aw) { Stop-Process -Id $aw -Force }
$back = $false
for ($i = 0; $i -lt 180; $i++) {
  Start-Sleep 1
  $x = SvcPid itcmdr-agent-watchdog
  if ($x -and $x -ne $aw) { $back = $true; break }
}
Chk $back "killed agent watchdog came back in ${i}s"
$on = $false
for ($i = 0; $i -lt 60; $i++) { Start-Sleep 2; if (AgentOnlineAny) { $on = $true; break } }
Chk $on 'agent online again after the kills'
$o = (& $cli setup check 2>&1 | Out-String)
Chk ($o -match 'install\.server') 'setup check runs and shows install.server'

"== 7 re-enroll already-enrolled machine with PUBLISHED installer /EnrollToken="
$pubSetup = Join-Path $PublishedDir "itcmdr-agent-Setup-$ToVer.exe"
Chk (Test-Path $pubSetup) "published $ToVer agent installer present"
$tok = $null
try {
  $js = & $cli --json agent enroll-token --machine $cn 2>$null
  $tj = $js | ConvertFrom-Json
  if ($tj.token) { $tok = [string]$tj.token }
} catch { }
if (-not $tok) {
  $skipped += 're-enroll (could not issue a token)'
  "SKIP re-enroll (no token)"
} else {
  $rc = (Start-Process $pubSetup -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/EnrollToken=$tok" -Wait -PassThru).ExitCode
  Chk ($rc -eq 0) "already-enrolled /EnrollToken= published setup exit $rc (want 0)"
  $tok = $null
  $onlineAsCn = $false
  $machineEq = $false
  $listLine = $false
  for ($i = 0; $i -lt 90; $i++) {
    Start-Sleep 2
    $machineEq = StatusMachineEqualsComputer
    $listLine = ListLineComputerOnline
    if ($machineEq -and $listLine) { $onlineAsCn = $true; break }
  }
  Chk $machineEq 'agent status JSON .machine equals COMPUTERNAME (strict, not substring)'
  Chk $listLine 'agent list has a line starting with COMPUTERNAME then online (strict)'
  Chk $onlineAsCn 'agent online under the current computer name after published /EnrollToken='
}

"== 8 bad token: fail loudly and keep the working enrollment"
$beforeMachine = StatusMachineEqualsComputer
$beforeList = ListLineComputerOnline
$rcBad = (Start-Process $pubSetup -ArgumentList '/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/EnrollToken=itcmdr-enroll:not-a-valid-token' -Wait -PassThru).ExitCode
Chk ($rcBad -ne 0) "bad-token setup exit $rcBad (must not be 0)"
Chk ($rcBad -eq 9) "bad-token setup exit is 9"
$j = LastJson 'agent'
Chk ($null -ne $j -and $j.ok -eq $false -and "$($j.exit)" -eq '9') 'last-agent.json records enroll failure exit 9'
Start-Sleep 8
$stillMachine = $false
$stillList = $false
for ($i = 0; $i -lt 60; $i++) {
  Start-Sleep 2
  $stillMachine = StatusMachineEqualsComputer
  $stillList = ListLineComputerOnline
  if ($stillMachine -and $stillList) { break }
}
Chk $stillMachine 'old enrollment status .machine still equals COMPUTERNAME after a bad token'
Chk $stillList 'old enrollment still listed as COMPUTERNAME online after a bad token'
Chk ($beforeMachine -and $beforeList) 'enrollment was healthy under COMPUTERNAME before the bad-token run'

"== 9 final state"
foreach ($s in $svcs) { Chk (WaitSvc $s 60) "$s Running at the end" }
Remove-Item (Join-Path $env:ProgramData 'itcmdr\agent-enroll.token') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:ProgramData 'itcmdr\agent\last-enroll-error.txt') -ErrorAction SilentlyContinue
Remove-Item (Join-Path $env:TEMP 'itcmdr-h.txt') -ErrorAction SilentlyContinue
"FAILS $fails"
if ($skipped.Count) { "SKIPPED $($skipped -join ' | ')" } else { 'SKIPPED none' }
if ($fails -ne 0) { exit 1 } else { exit 0 }
