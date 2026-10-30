<#
.SYNOPSIS
  Rotate itcmdr maintenanceRuns logs into weekly, 4-week block and yearly zips.

.DESCRIPTION
  Everything is aligned to ISO 8601 weeks (Monday-Sunday; the ISO year is the year
  of the week's Thursday). Archives live under <Root>\archive\:
    weekly\itcmdr-<isoYear>-W<ww>.zip   one finished ISO week of run logs
    blocks\itcmdr-<isoYear>-B<NN>.zip   block N = weekly zips of ISO weeks 4N-3..4N
                                        (13 blocks per ISO year; in 53-week years
                                        week 53 goes into block 13)
    yearly\itcmdr-<isoYear>.zip         all block zips of a finished ISO year
  A zip is built only once its whole period has finished (the current week is never
  touched). Every zip carries manifest.json (kind, period, start/end dates, and the
  SHA256 of every member); it is re-opened and verified before originals are removed.
  Logs that arrive late for an already-zipped period go into a numbered supplement
  (itcmdr-2026-W10-2.zip). Block zips are kept after the yearly zip unless
  retention.keepBlockZipsAfterYearly is false. The rotation run itself is logged as a
  run in <Root>. Compatible with Windows PowerShell 5.1 and PowerShell 7.

.PARAMETER Root
  maintenanceRuns folder. Default: site.json shares.maintenanceRuns, else paths.localLogs, else C:\maintenanceRuns.

.PARAMETER Today
  Override today's date (yyyy-MM-dd) for testing.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
  [string]$Root,
  [string]$Today,
  [bool]$DoWeekly = $true,
  [bool]$DoBlocks = $true,
  [bool]$DoYearly = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ItcmdrRotationSite {
  foreach ($p in @('config/site.json', 'C:\ProgramData\itcmdr\site.json')) {
    if (Test-Path $p) {
      try { return ((Get-Content $p -Raw).TrimStart([char]0xFEFF) | ConvertFrom-Json) } catch { Write-Warning "Failed to read site config: $_" }
    }
  }
  return $null
}

function Get-ItcmdrRotationRoot {
  param($Site)
  $r = Get-ItcmdrProp $Site @('shares', 'maintenanceRuns')
  if (-not $r) { $r = Get-ItcmdrProp $Site @('paths', 'localLogs') }
  if (-not $r) { $r = 'C:\maintenanceRuns' }
  return [string]$r
}

function Get-ItcmdrProp {
  param($Obj, [string[]]$Path)
  $cur = $Obj
  foreach ($n in $Path) {
    if ($null -eq $cur) { return $null }
    if ($cur -is [System.Collections.IDictionary]) { $cur = $cur[$n]; continue }
    $p = $cur.PSObject.Properties[$n]
    if (-not $p) { return $null }
    $cur = $p.Value
  }
  return $cur
}

function Get-ItcmdrIsoWeek {
  param([datetime]$Date)
  $d = $Date.Date
  $thursday = $d.AddDays(3 - ((([int]$d.DayOfWeek) + 6) % 7))
  [pscustomobject]@{ Year = $thursday.Year; Week = [int][math]::Floor(($thursday.DayOfYear - 1) / 7) + 1 }
}

function Get-ItcmdrIsoMonday {
  param([int]$Year, [int]$Week)
  $jan4 = New-Object DateTime $Year, 1, 4
  $mon1 = $jan4.AddDays(-((([int]$jan4.DayOfWeek) + 6) % 7))
  return $mon1.AddDays(7 * ($Week - 1))
}

function Get-ItcmdrIsoWeeksInYear {
  param([int]$Year)
  return (Get-ItcmdrIsoWeek (New-Object DateTime $Year, 12, 28)).Week
}

function Get-ItcmdrBlockOfWeek {
  param([int]$Week)
  return [int][math]::Floor(([math]::Min($Week, 52) - 1) / 4) + 1
}

function Get-ItcmdrBlockRange {
  <# Start (Monday) and end (Sunday) dates of 4-week block N of an ISO year. #>
  param([int]$Year, [int]$Block)
  $start = Get-ItcmdrIsoMonday $Year (4 * $Block - 3)
  $lastWeek = if ($Block -eq 13) { Get-ItcmdrIsoWeeksInYear $Year } else { 4 * $Block }
  $end = (Get-ItcmdrIsoMonday $Year $lastWeek).AddDays(6)
  [pscustomobject]@{ Start = $start; End = $end; FirstWeek = 4 * $Block - 3; LastWeek = $lastWeek }
}

function Get-ItcmdrArchiveName {
  <# base.zip, or base-2.zip, base-3.zip... when earlier ones exist. #>
  param([string]$Dir, [string]$Base)
  $p = Join-Path $Dir "$Base.zip"
  $n = 2
  while (Test-Path -LiteralPath $p) { $p = Join-Path $Dir "$Base-$n.zip"; $n++ }
  return $p
}

function New-ItcmdrZipWithManifest {
  [CmdletBinding(SupportsShouldProcess)]
  param([string]$ZipPath, [string[]]$Files, [hashtable]$Meta, [string[]]$EntryNames)
  if (-not $EntryNames) { $EntryNames = @($Files | ForEach-Object { [IO.Path]::GetFileName($_) }) }
  if (-not $PSCmdlet.ShouldProcess($ZipPath, 'Create archive')) { return $false }
  Add-Type -AssemblyName System.IO.Compression
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $entries = @()
  for ($i = 0; $i -lt $Files.Count; $i++) {
    $entries += [pscustomobject]@{ path = $EntryNames[$i]; sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $Files[$i]).Hash }
  }
  $manifest = [ordered]@{}
  foreach ($k in $Meta.Keys) { $manifest[$k] = $Meta[$k] }
  $manifest['created'] = (Get-Date).ToString('o')
  $manifest['count'] = $entries.Count
  $manifest['files'] = $entries
  $tmp = "$ZipPath.partial"
  if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force }
  $zip = [IO.Compression.ZipFile]::Open($tmp, [IO.Compression.ZipArchiveMode]::Create)
  try {
    for ($i = 0; $i -lt $Files.Count; $i++) {
      [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $Files[$i], $EntryNames[$i], [IO.Compression.CompressionLevel]::Optimal)
    }
    $e = $zip.CreateEntry('manifest.json')
    $w = New-Object IO.StreamWriter($e.Open())
    try { $w.Write(($manifest | ConvertTo-Json -Depth 6)) } finally { $w.Dispose() }
  } finally { $zip.Dispose() }
  Test-ItcmdrZipManifest -ZipPath $tmp | Out-Null
  Move-Item -LiteralPath $tmp -Destination $ZipPath
  return $true
}

function Test-ItcmdrZipManifest {
  <# Throws unless every manifest entry is present with a matching SHA256. #>
  param([string]$ZipPath)
  Add-Type -AssemblyName System.IO.Compression
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $zip = [IO.Compression.ZipFile]::OpenRead($ZipPath)
  try {
    $me = $zip.GetEntry('manifest.json')
    if (-not $me) { throw "manifest.json missing in $ZipPath" }
    $r = New-Object IO.StreamReader($me.Open())
    try { $m = $r.ReadToEnd() | ConvertFrom-Json } finally { $r.Dispose() }
    $files = @($m.files)
    if ($files.Count -ne [int]$m.count) { throw "manifest count mismatch in $ZipPath" }
    foreach ($f in $files) {
      $e = $zip.GetEntry($f.path)
      if (-not $e) { throw "missing $($f.path) in $ZipPath" }
      $s = $e.Open()
      try {
        $sha = [Security.Cryptography.SHA256]::Create()
        $hash = ([BitConverter]::ToString($sha.ComputeHash($s))).Replace('-', '')
      } finally { $s.Dispose() }
      if ($hash -ne $f.sha256) { throw "hash mismatch for $($f.path) in $ZipPath" }
    }
    return $m
  } finally { $zip.Dispose() }
}

function Invoke-ItcmdrRotation {
  [CmdletBinding(SupportsShouldProcess)]
  param([string]$Root, [datetime]$Today, [bool]$DoWeekly = $true, [bool]$DoBlocks = $true, [bool]$DoYearly = $true, [bool]$KeepBlocks = $true)
  $weeklyDir = Join-Path (Join-Path $Root 'archive') 'weekly'
  $blocksDir = Join-Path (Join-Path $Root 'archive') 'blocks'
  $yearlyDir = Join-Path (Join-Path $Root 'archive') 'yearly'
  foreach ($d in @($weeklyDir, $blocksDir, $yearlyDir)) { if (-not (Test-Path -LiteralPath $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null } }
  $events = @()
  $thisMonday = $Today.Date.AddDays(-((([int]$Today.DayOfWeek) + 6) % 7))

  if ($DoWeekly) {
    $groups = @{}
    foreach ($f in @(Get-ChildItem -LiteralPath $Root -Filter '*.log' -File -ErrorAction SilentlyContinue)) {
      if ($f.BaseName -notmatch '_(\d{8})-\d{6}_') { continue }
      $d = [datetime]::ParseExact($Matches[1], 'yyyyMMdd', [Globalization.CultureInfo]::InvariantCulture)
      if ($d -ge $thisMonday) { continue }
      $iw = Get-ItcmdrIsoWeek $d
      $key = '{0}-W{1:D2}' -f $iw.Year, $iw.Week
      if (-not $groups.ContainsKey($key)) { $groups[$key] = [pscustomobject]@{ Year = $iw.Year; Week = $iw.Week; Files = @(); Names = @() } }
      $groups[$key].Files += $f.FullName
      $groups[$key].Names += $f.Name
    }
    # Plugin logs mirrored by the server: <Root>\plugin-logs\<plugin>\<yyyy-MM-dd>.jsonl (decision 11).
    $pl = Join-Path $Root 'plugin-logs'
    if (Test-Path -LiteralPath $pl) {
      foreach ($f in @(Get-ChildItem -LiteralPath $pl -Recurse -Filter '*.jsonl' -File -ErrorAction SilentlyContinue)) {
        if ($f.BaseName -notmatch '^\d{4}-\d{2}-\d{2}$') { continue }
        $d = [datetime]::ParseExact($f.BaseName, 'yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture)
        if ($d -ge $thisMonday) { continue }
        $iw = Get-ItcmdrIsoWeek $d
        $key = '{0}-W{1:D2}' -f $iw.Year, $iw.Week
        if (-not $groups.ContainsKey($key)) { $groups[$key] = [pscustomobject]@{ Year = $iw.Year; Week = $iw.Week; Files = @(); Names = @() } }
        $groups[$key].Files += $f.FullName
        $groups[$key].Names += ('plugin-logs/{0}/{1}' -f $f.Directory.Name, $f.Name)
      }
    }
    foreach ($key in @($groups.Keys | Sort-Object)) {
      $g = $groups[$key]
      $mon = Get-ItcmdrIsoMonday $g.Year $g.Week
      $zipPath = Get-ItcmdrArchiveName $weeklyDir "itcmdr-$key"
      $meta = @{ kind = 'weekly'; isoYear = $g.Year; isoWeek = $g.Week; start = $mon.ToString('yyyy-MM-dd'); end = $mon.AddDays(6).ToString('yyyy-MM-dd') }
      if (New-ItcmdrZipWithManifest -ZipPath $zipPath -Files $g.Files -Meta $meta -EntryNames $g.Names) {
        $g.Files | Remove-Item -Force
        $events += [pscustomobject]@{ type = 'rotation'; scope = 'weekly'; period = $key; archive = [IO.Path]::GetFileName($zipPath); files = $g.Files.Count }
      }
    }
  }

  if ($DoBlocks) {
    $groups = @{}
    foreach ($z in @(Get-ChildItem -LiteralPath $weeklyDir -Filter 'itcmdr-*.zip' -File -ErrorAction SilentlyContinue)) {
      if ($z.BaseName -notmatch '^itcmdr-(\d{4})-W(\d{2})(-\d+)?$') { continue }
      $y = [int]$Matches[1]; $w = [int]$Matches[2]
      $b = Get-ItcmdrBlockOfWeek $w
      $key = '{0}-B{1:D2}' -f $y, $b
      if (-not $groups.ContainsKey($key)) { $groups[$key] = [pscustomobject]@{ Year = $y; Block = $b; Files = @() } }
      $groups[$key].Files += $z.FullName
    }
    foreach ($key in @($groups.Keys | Sort-Object)) {
      $g = $groups[$key]
      $range = Get-ItcmdrBlockRange $g.Year $g.Block
      if ($Today.Date -le $range.End) { continue }   # block not finished yet
      $zipPath = Get-ItcmdrArchiveName $blocksDir "itcmdr-$key"
      $files = @($g.Files | Sort-Object)
      $meta = @{ kind = 'block4'; isoYear = $g.Year; block = $g.Block; firstWeek = $range.FirstWeek; lastWeek = $range.LastWeek; start = $range.Start.ToString('yyyy-MM-dd'); end = $range.End.ToString('yyyy-MM-dd') }
      if (New-ItcmdrZipWithManifest -ZipPath $zipPath -Files $files -Meta $meta) {
        $files | Remove-Item -Force
        $events += [pscustomobject]@{ type = 'rotation'; scope = 'block4'; period = $key; archive = [IO.Path]::GetFileName($zipPath); files = $files.Count }
      }
    }
  }

  if ($DoYearly) {
    $groups = @{}
    foreach ($z in @(Get-ChildItem -LiteralPath $blocksDir -Filter 'itcmdr-*.zip' -File -ErrorAction SilentlyContinue)) {
      if ($z.BaseName -notmatch '^itcmdr-(\d{4})-B(\d{2})(-\d+)?$') { continue }
      $y = [int]$Matches[1]
      if (-not $groups.ContainsKey($y)) { $groups[$y] = @() }
      $groups[$y] += $z.FullName
    }
    foreach ($y in @($groups.Keys | Sort-Object)) {
      $nextStart = Get-ItcmdrIsoMonday ($y + 1) 1
      if ($Today.Date -lt $nextStart) { continue }    # ISO year not finished yet
      $existing = Join-Path $yearlyDir "itcmdr-$y.zip"
      if ($KeepBlocks -and (Test-Path -LiteralPath $existing)) { continue }  # blocks kept on purpose
      $zipPath = Get-ItcmdrArchiveName $yearlyDir "itcmdr-$y"
      $files = @($groups[$y] | Sort-Object)
      $start = Get-ItcmdrIsoMonday $y 1
      $meta = @{ kind = 'yearly'; isoYear = $y; weeks = (Get-ItcmdrIsoWeeksInYear $y); start = $start.ToString('yyyy-MM-dd'); end = $nextStart.AddDays(-1).ToString('yyyy-MM-dd') }
      if (New-ItcmdrZipWithManifest -ZipPath $zipPath -Files $files -Meta $meta) {
        if (-not $KeepBlocks) { $files | Remove-Item -Force }
        $events += [pscustomobject]@{ type = 'rotation'; scope = 'yearly'; period = "$y"; archive = [IO.Path]::GetFileName($zipPath); files = $files.Count }
      }
    }
  }
  return , $events
}

if (-not $env:ITCMDR_IMPORT_ONLY) {
  $site = Get-ItcmdrRotationSite
  if (-not $Root -or $Root.Trim() -eq '') {
    $Root = Get-ItcmdrRotationRoot $site
  }
  $now = if ($Today) { [datetime]::ParseExact($Today, 'yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture) } else { Get-Date }
  $keep = $true
  $k1 = Get-ItcmdrProp $site @('retention', 'keepBlockZipsAfterYearly')
  $k2 = Get-ItcmdrProp $site @('retention', 'yearlyKeepForever')
  if ($null -ne $k1) { $keep = [bool]$k1 } elseif ($null -ne $k2) { $keep = [bool]$k2 }

  $computer = $env:COMPUTERNAME
  if (-not $computer) { $computer = [Environment]::MachineName }
  $runId = [guid]::NewGuid().ToString()
  $logFile = Join-Path $Root ('{0}_{1}_{2}.log' -f $computer, $now.ToString('yyyyMMdd-HHmmss'), $runId)
  $line = { param($o) Add-Content -LiteralPath $logFile -Value ($o | ConvertTo-Json -Depth 6 -Compress) -Encoding UTF8 }
  & $line ([pscustomobject]@{ type = 'run-start'; runId = $runId; computer = $computer; role = 'Server'; rotation = $true; started = (Get-Date).ToString('o') })
  $exit = 0
  try {
    foreach ($e in (Invoke-ItcmdrRotation -Root $Root -Today $now -DoWeekly $DoWeekly -DoBlocks $DoBlocks -DoYearly $DoYearly -KeepBlocks $keep)) { & $line $e }
  } catch {
    $exit = 1
    & $line ([pscustomobject]@{ type = 'rotation-error'; runId = $runId; computer = $computer; error = $_.ToString() })
    throw
  } finally {
    & $line ([pscustomobject]@{ type = 'run-complete'; runId = $runId; computer = $computer; finished = (Get-Date).ToString('o'); exitCode = $exit })
  }
}
