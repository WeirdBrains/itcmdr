<#
.SYNOPSIS
  First-run site setup for an agent PC (v0.14.0): writes site values into site.json and checks
  that the folders and the server are usable. Existing values are kept unless -Overwrite.
.DESCRIPTION
  Values: -SiteName, -ServerHostname, -ServerAddress, -MaintenanceRunsShare, -LocalLogs.
  Checks: site.json parses, the local log folder exists (created) and is writable, the
  maintenanceRuns share exists and is writable, and the server's gRPC port answers (warning only).
  Windows PowerShell 5.1 compatible. Returns check objects; exit code 1 from the installer's call
  when a check fails (-FailExitCode).
#>
[CmdletBinding()]
param(
  [string]$SitePath = (Join-Path $env:ProgramData 'itcmdr\site.json'),
  [string]$SiteName,
  [string]$ServerHostname,
  [string]$ServerAddress,
  [string]$MaintenanceRunsShare,
  [string]$LocalLogs,
  [int]$ServerPort = 50051,
  [switch]$Overwrite,
  [switch]$CheckOnly,
  [switch]$SkipServerCheck,
  [string]$Report,
  [switch]$FailExitCode
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Read-ItcmdrSiteJson([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return [pscustomobject]@{} }
  $t = [IO.File]::ReadAllText($Path).TrimStart([char]0xFEFF)
  if (-not $t.Trim()) { return [pscustomobject]@{} }
  return ($t | ConvertFrom-Json)
}

function Merge-ItcmdrSiteValue($Obj, [string]$Dotted, $Value, [bool]$Force) {
  $parts = $Dotted.Split('.')
  $o = $Obj
  for ($i = 0; $i -lt $parts.Count - 1; $i++) {
    $p = $parts[$i]
    if (-not $o.PSObject.Properties[$p] -or $null -eq $o.$p) {
      $o | Add-Member -NotePropertyName $p -NotePropertyValue ([pscustomobject]@{}) -Force
    }
    $o = $o.$p
  }
  $k = $parts[-1]
  $cur = if ($o.PSObject.Properties[$k]) { $o.$k } else { $null }
  if (-not $Force -and $null -ne $cur -and "$cur".Trim()) { return $false }
  if ("$cur" -eq "$Value") { return $false }
  $o | Add-Member -NotePropertyName $k -NotePropertyValue $Value -Force
  return $true
}

function Test-ItcmdrWritableDir([string]$Path, [bool]$Create) {
  if (-not $Path) { return 'not set' }
  if (-not (Test-Path -LiteralPath $Path)) {
    if (-not $Create) { return "$Path does not exist or is not reachable" }
    try { New-Item -ItemType Directory -Path $Path -Force | Out-Null } catch { return "cannot create ${Path}: $($_.Exception.Message)" }
  }
  if (-not (Test-Path -LiteralPath $Path -PathType Container)) { return "$Path is not a folder" }
  $probe = Join-Path $Path (".itcmdr-write-test-$PID")
  try { [IO.File]::WriteAllText($probe, 'x'); Remove-Item -LiteralPath $probe -Force } catch { return "cannot write in ${Path}: $($_.Exception.Message)" }
  return $null
}

function ConvertTo-ItcmdrCheck([string]$Name, [string]$Level, [string]$Detail) {
  [pscustomobject]@{ name = $Name; level = $Level; detail = $Detail }
}

$changed = @()
$site = $null
$checks = @()
try {
  $site = Read-ItcmdrSiteJson $SitePath
  $checks += ConvertTo-ItcmdrCheck 'site.json' 'ok' $SitePath
} catch {
  $checks += ConvertTo-ItcmdrCheck 'site.json' 'fail' "cannot parse ${SitePath}: $($_.Exception.Message)"
}

if ($site -and -not $CheckOnly) {
  $vals = [ordered]@{
    'siteName' = $SiteName; 'network.serverHostname' = $ServerHostname; 'network.serverAddress' = $ServerAddress
    'shares.maintenanceRuns' = $MaintenanceRunsShare; 'paths.localLogs' = $LocalLogs
  }
  foreach ($k in $vals.Keys) {
    if ($vals[$k]) { if (Merge-ItcmdrSiteValue $site $k $vals[$k] ([bool]$Overwrite)) { $changed += $k } }
  }
  if ($changed.Count -gt 0 -or -not (Test-Path -LiteralPath $SitePath)) {
    $dir = Split-Path -Parent $SitePath
    if (-not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    $tmp = "$SitePath.tmp"
    [IO.File]::WriteAllText($tmp, ($site | ConvertTo-Json -Depth 20), (New-Object Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $tmp -Destination $SitePath -Force
  }
}

if ($site) {
  $get = { param($a, $b) if ($site.PSObject.Properties[$a] -and $site.$a -and $site.$a.PSObject.Properties[$b]) { [string]$site.$a.$b } else { '' } }
  $local = & $get 'paths' 'localLogs'; if (-not $local) { $local = 'C:\maintenanceRuns' }
  $e = Test-ItcmdrWritableDir $local $true
  $checks += if ($e) { ConvertTo-ItcmdrCheck 'localLogs' 'fail' $e } else { ConvertTo-ItcmdrCheck 'localLogs' 'ok' "$local (writable)" }
  $share = & $get 'shares' 'maintenanceRuns'
  if (-not $share) {
    $checks += ConvertTo-ItcmdrCheck 'share' 'warn' 'shares.maintenanceRuns not set: logs stay local and requests from the app are not picked up'
  } else {
    $e = Test-ItcmdrWritableDir $share $false
    $checks += if ($e) { ConvertTo-ItcmdrCheck 'share' 'fail' $e } else { ConvertTo-ItcmdrCheck 'share' 'ok' "$share (writable)" }
  }
  $srv = & $get 'network' 'serverAddress'; if (-not $srv) { $srv = & $get 'network' 'serverHostname' }
  if ($SkipServerCheck) {
  } elseif (-not $srv) {
    $checks += ConvertTo-ItcmdrCheck 'server' 'warn' 'network.serverHostname / serverAddress not set'
  } else {
    $ok = $false
    try {
      $c = New-Object Net.Sockets.TcpClient
      $ok = $c.ConnectAsync($srv, $ServerPort).Wait(3000) -and $c.Connected
      $c.Close()
    } catch { $ok = $false }
    $checks += if ($ok) { ConvertTo-ItcmdrCheck 'server' 'ok' "${srv}:$ServerPort answers" } else { ConvertTo-ItcmdrCheck 'server' 'warn' "${srv}:$ServerPort does not answer (server not installed yet, firewall, or name)" }
  }
}

$lines = @()
if ($changed.Count) { $lines += "site.json: set $($changed -join ', ')" }
$lines += $checks | ForEach-Object { '{0,-4} {1,-10} {2}' -f $_.level.ToUpper(), $_.name, $_.detail }
if ($Report) { [IO.File]::WriteAllText($Report, ($lines -join "`r`n") + "`r`n") }
$lines | ForEach-Object { Write-Information $_ -InformationAction Continue }
$failed = @($checks | Where-Object { $_.level -eq 'fail' }).Count -gt 0
if ($FailExitCode) { if ($failed) { exit 1 } else { exit 0 } }
[pscustomobject]@{ changed = $changed; checks = $checks; failed = $failed }
