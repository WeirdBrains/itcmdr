<#
.SYNOPSIS
  Rotate itcmdr maintenanceRuns into weekly, monthly, and yearly archives with verification.

.DESCRIPTION
  - Weekly: zip previous ISO week into archive\weekly\<yyyy>-W<ww>.zip
  - Monthly: zip that month's weekly zips into archive\monthly\<yyyy>-<MM>.zip
  - Yearly: zip that year's monthly zips into archive\yearly\<yyyy>.zip
  - Verify by count and SHA256 manifest before removing originals.
  - Log rotation as a run in maintenanceRuns (type='rotation').

.PARAMETER Root
  Root folder of maintenanceRuns. Default: from site config shares.maintenanceRuns or C:\maintenanceRuns

.PARAMETER Today
  Override today's date for testing (yyyy-MM-dd).

.PARAMETER DoWeekly
  Perform weekly rollup (default: true).

.PARAMETER DoMonthly
  Perform monthly rollup (default: true).

.PARAMETER DoYearly
  Perform yearly rollup (default: true).
#>
[CmdletBinding()]
param(
  [string]$Root,
  [string]$Today,
  [bool]$DoWeekly = $true,
  [bool]$DoMonthly = $true,
  [bool]$DoYearly = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-Site { 
  try {
    $candidates = @('config/site.json','C:\ProgramData\itcmdr\site.json')
    foreach ($p in $candidates) { if (Test-Path $p) { return Get-Content $p -Raw | ConvertFrom-Json -AsHashtable } }
  } catch { Write-Warning "Failed to read site config: $_" }
  return @{} 
}

function ConvertTo-JsonLine { param($Obj); return ($Obj | ConvertTo-Json -Depth 6 -Compress) }

function Get-IsoWeekInfo([datetime]$date) {
  $cal = [System.Globalization.CultureInfo]::InvariantCulture.Calendar
  $week = $cal.GetWeekOfYear($date, [System.Globalization.CalendarWeekRule]::FirstFourDayWeek, [DayOfWeek]::Monday)
  $year = $date.Year
  # ISO weeks that span year start/end: adjust Thursday rule
  $thursday = $date.AddDays(3 - (($date.DayOfWeek.value__ + 6) % 7)) # move to Thursday
  $year = $thursday.Year
  [pscustomobject]@{ Year = $year; Week = $week }
}

function Get-PrevIsoWeekInfo([datetime]$date) {
  $prev = $date.AddDays(-7)
  return Get-IsoWeekInfo $prev
}

function Get-RunFilesForIsoWeek($root, [int]$year, [int]$week) {
  Get-ChildItem -LiteralPath $root -Filter '*.log' -File -ErrorAction SilentlyContinue | Where-Object {
    $name = $_.BaseName # COMPUTER_yyyyMMdd-HHmmss_runId
    if ($name -match '_(\d{8})-') {
      $d = [datetime]::ParseExact($Matches[1], 'yyyyMMdd', $null)
      $info = Get-IsoWeekInfo $d
      return ($info.Year -eq $year -and $info.Week -eq $week)
    }
    $false
  }
}

function Get-MonthForIsoWeek([int]$year, [int]$week) {
  # ISO rule: month containing Thursday
  $firstThursday = (Get-Date -Date (Get-Date -Year $year -Month 1 -Day 4)).Date
  $mon1 = $firstThursday.AddDays(-((($firstThursday.DayOfWeek.value__ + 6) % 7)))
  $start = $mon1.AddDays(7 * ($week - 1))
  $thursday = $start.AddDays(3)
  return $thursday.ToString('yyyy-MM')
}

function New-ZipWithManifest {
  [CmdletBinding(SupportsShouldProcess)]
  param($zipPath, $files)
  if (-not $PSCmdlet.ShouldProcess($zipPath, 'Create archive')) { return }
  $tmpManifest = [System.IO.Path]::GetTempFileName()
  $manifest = @()
  foreach ($f in $files) {
    $h = Get-FileHash -Algorithm SHA256 -LiteralPath $f
    $manifest += [pscustomobject]@{ path = [System.IO.Path]::GetFileName($f); sha256 = $h.Hash }
  }
  $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $tmpManifest -Encoding UTF8
  if (Test-Path $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
  Compress-Archive -LiteralPath ($files + $tmpManifest) -DestinationPath $zipPath -CompressionLevel Optimal
  # Verify counts and hashes by re-opening zip
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
  try {
    $entry = $zip.Entries | Where-Object { $_.Name -eq [System.IO.Path]::GetFileName($tmpManifest) } | Select-Object -First 1
    if (-not $entry) { throw "Manifest not found in zip $zipPath" }
    $tmp = [System.IO.Path]::GetTempFileName()
    [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $tmp, $true)
    $m = Get-Content -LiteralPath $tmp -Raw | ConvertFrom-Json
    foreach ($e in $m) {
      $z = $zip.Entries | Where-Object { $_.Name -eq $e.path } | Select-Object -First 1
      if (-not $z) { throw "Entry missing: $($e.path)" }
      $tmp2 = [System.IO.Path]::GetTempFileName()
      [System.IO.Compression.ZipFileExtensions]::ExtractToFile($z, $tmp2, $true)
      $h = Get-FileHash -Algorithm SHA256 -LiteralPath $tmp2
      if ($h.Hash -ne $e.sha256) { throw "Hash mismatch for $($e.path)" }
      Remove-Item $tmp2 -Force
    }
    Remove-Item $tmp -Force
  } finally {
    $zip.Dispose()
  }
  Remove-Item $tmpManifest -Force
}

function Initialize-Directory($p) { if (-not (Test-Path -LiteralPath $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null } }

$site = Get-Site
if (-not $Root -or $Root.Trim() -eq '') {
  $Root = $site.shares.maintenanceRuns
  if (-not $Root) { $Root = 'C:\maintenanceRuns' }
}
$today = if ($Today) { [datetime]::Parse($Today) } else { (Get-Date) }
$computer = $env:COMPUTERNAME

$archiveWeekly = Join-Path $Root 'archive\weekly'
$archiveMonthly = Join-Path $Root 'archive\monthly'
$archiveYearly = Join-Path $Root 'archive\yearly'
Initialize-Directory $archiveWeekly; Initialize-Directory $archiveMonthly; Initialize-Directory $archiveYearly

# Run log
$runId = [guid]::NewGuid().ToString()
$stamp = $today.ToString('yyyyMMdd-HHmmss')
$logFile = Join-Path $Root "$computer`_$stamp`_$runId.log"
ConvertTo-JsonLine ([pscustomobject]@{ type='run-start'; runId=$runId; computer=$computer; role='Server'; rotation=$true; started=(Get-Date).ToString('o') }) | Add-Content -LiteralPath $logFile

try {
  if ($DoWeekly) {
    $wi = Get-PrevIsoWeekInfo $today
    $weekStr = ('{0}-W{1:D2}' -f $wi.Year, $wi.Week)
    $weekZip = Join-Path $archiveWeekly "$weekStr.zip"
    $weekFiles = @(Get-RunFilesForIsoWeek -root $Root -year $wi.Year -week $wi.Week | ForEach-Object { $_.FullName })
    if ($weekFiles.Count -gt 0 -and -not (Test-Path $weekZip)) {
      New-ZipWithManifest -zipPath $weekZip -files $weekFiles
      # Remove originals after verification
      $weekFiles | Remove-Item -Force
      ConvertTo-JsonLine ([pscustomobject]@{ type='rotation'; scope='weekly'; week=$weekStr; files=$weekFiles.Count }) | Add-Content -LiteralPath $logFile
    }
  }

  if ($DoMonthly) {
    $wi = Get-PrevIsoWeekInfo $today
    $monthStr = Get-MonthForIsoWeek -year $wi.Year -week $wi.Week
    $monthZip = Join-Path $archiveMonthly "$monthStr.zip"
    $weeklyZips = Get-ChildItem -LiteralPath $archiveWeekly -Filter '*.zip' -File | Where-Object { $_.BaseName -like "$($wi.Year)-W*" } | Where-Object {
      # Include those weeks whose Thursday falls in this month
      if ($_.BaseName -match '^(\d{4})-W(\d{2})$') {
        (Get-MonthForIsoWeek -year [int]$Matches[1] -week [int]$Matches[2]) -eq $monthStr
      } else { $false }
    } | ForEach-Object { $_.FullName }
    if ($weeklyZips.Count -gt 0 -and -not (Test-Path $monthZip)) {
      New-ZipWithManifest -zipPath $monthZip -files $weeklyZips
      $weeklyZips | Remove-Item -Force
      ConvertTo-JsonLine ([pscustomobject]@{ type='rotation'; scope='monthly'; month=$monthStr; files=$weeklyZips.Count }) | Add-Content -LiteralPath $logFile
    }
  }

  if ($DoYearly) {
    $year = ($today.Year - 1)
    $yearZip = Join-Path $archiveYearly "$year.zip"
    $monthlyZips = Get-ChildItem -LiteralPath $archiveMonthly -Filter "$year-*.zip" -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }
    if ($monthlyZips.Count -gt 0 -and -not (Test-Path $yearZip)) {
      New-ZipWithManifest -zipPath $yearZip -files $monthlyZips
      # Respect retention: keep yearly forever by default; do not delete monthly if config says to keep
      $keepForever = $true
      if ($null -ne $site.retention.yearlyKeepForever) { $keepForever = [bool]$site.retention.yearlyKeepForever }
      if (-not $keepForever) {
        $monthlyZips | Remove-Item -Force
      }
      ConvertTo-JsonLine ([pscustomobject]@{ type='rotation'; scope='yearly'; year=$year; files=$monthlyZips.Count }) | Add-Content -LiteralPath $logFile
    }
  }
} finally {
  ConvertTo-JsonLine ([pscustomobject]@{ type='run-complete'; runId=$runId; computer=$computer; finished=(Get-Date).ToString('o'); exitCode=0 }) | Add-Content -LiteralPath $logFile
}

