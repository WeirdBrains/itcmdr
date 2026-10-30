Set-StrictMode -Version Latest

BeforeAll {
  $env:ITCMDR_IMPORT_ONLY = '1'
  . (Join-Path (Join-Path $PSScriptRoot '..') 'Rotate-ItcmdrLogs.ps1')
  Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue

  function Add-FakeLog([string]$Root, [datetime]$Day, [string]$Machine = 'WS-01') {
    $p = Join-Path $Root ('{0}_{1}_{2}.log' -f $Machine, $Day.ToString('yyyyMMdd-010000'), [guid]::NewGuid())
    Set-Content -LiteralPath $p -Value ('{"type":"run-start","computer":"' + $Machine + '"}')
    return $p
  }
  function Get-ZipEntryName([string]$Path) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($Path)
    try { return @($z.Entries | ForEach-Object Name | Sort-Object) } finally { $z.Dispose() }
  }
  # Run the rotation once per day over a date range, like the daily scheduled task.
  function Invoke-RotationDay([string]$Root, [datetime]$From, [datetime]$To) {
    for ($d = $From; $d -le $To; $d = $d.AddDays(1)) { Invoke-ItcmdrRotation -Root $Root -Today $d | Out-Null }
  }
}

Describe 'ISO week helpers' {
  It 'computes ISO weeks and years' {
    (Get-ItcmdrIsoWeek ([datetime]'2026-10-29')).Week | Should -Be 44
    $w = Get-ItcmdrIsoWeek ([datetime]'2027-01-01'); $w.Year | Should -Be 2026; $w.Week | Should -Be 53
    $w = Get-ItcmdrIsoWeek ([datetime]'2024-12-30'); $w.Year | Should -Be 2025; $w.Week | Should -Be 1
    Get-ItcmdrIsoWeeksInYear 2026 | Should -Be 53
    Get-ItcmdrIsoWeeksInYear 2027 | Should -Be 52
    (Get-ItcmdrIsoMonday 2026 1).ToString('yyyy-MM-dd') | Should -Be '2025-12-29'
  }
  It 'maps weeks to 4-week blocks, week 53 into block 13' {
    Get-ItcmdrBlockOfWeek 1 | Should -Be 1
    Get-ItcmdrBlockOfWeek 4 | Should -Be 1
    Get-ItcmdrBlockOfWeek 5 | Should -Be 2
    Get-ItcmdrBlockOfWeek 52 | Should -Be 13
    Get-ItcmdrBlockOfWeek 53 | Should -Be 13
    $r = Get-ItcmdrBlockRange 2026 13
    $r.Start.ToString('yyyy-MM-dd') | Should -Be '2026-11-30'
    $r.End.ToString('yyyy-MM-dd') | Should -Be '2027-01-03'
    $r.LastWeek | Should -Be 53
    (Get-ItcmdrBlockRange 2027 13).End.ToString('yyyy-MM-dd') | Should -Be '2028-01-02'
  }
}

Describe 'Invoke-ItcmdrRotation' {
  BeforeEach {
    $root = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-rot-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $root | Out-Null
  }
  AfterEach { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }

  It 'never touches the current week and zips finished weeks with a verified manifest' {
    $old = Add-FakeLog $root ([datetime]'2026-10-21')   # W43
    $cur = Add-FakeLog $root ([datetime]'2026-10-27')   # W44 (current)
    Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2026-10-29') | Out-Null
    $wz = Join-Path $root 'archive/weekly/itcmdr-2026-W43.zip'
    Test-Path $wz | Should -BeTrue
    Test-Path $old | Should -BeFalse
    Test-Path $cur | Should -BeTrue
    $m = Test-ItcmdrZipManifest -ZipPath $wz
    $m.kind | Should -Be 'weekly'
    $m.start | Should -Be '2026-10-19'
    $m.end | Should -Be '2026-10-25'
  }

  It 'archives finished-week plugin logs (plugin-logs/NAME/DAY.jsonl) into the weekly zip' {
    Add-FakeLog $root ([datetime]'2026-10-21') | Out-Null
    $pd = Join-Path (Join-Path $root 'plugin-logs') 'disk-free'
    New-Item -ItemType Directory -Path $pd -Force | Out-Null
    $old = Join-Path $pd '2026-10-22.jsonl'; Set-Content -LiteralPath $old -Value '{"schema":"itcmdr.log/v1"}'
    $cur = Join-Path $pd '2026-10-28.jsonl'; Set-Content -LiteralPath $cur -Value '{"schema":"itcmdr.log/v1"}'
    $odd = Join-Path $pd 'notes.txt'; Set-Content -LiteralPath $odd -Value 'x'
    Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2026-10-29') | Out-Null
    $wz = Join-Path $root 'archive/weekly/itcmdr-2026-W43.zip'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($wz)
    try { $names = @($z.Entries | ForEach-Object FullName) } finally { $z.Dispose() }
    $names | Should -Contain 'plugin-logs/disk-free/2026-10-22.jsonl'
    Test-Path $old | Should -BeFalse
    Test-Path $cur | Should -BeTrue
    Test-Path $odd | Should -BeTrue
    (Test-ItcmdrZipManifest -ZipPath $wz).kind | Should -Be 'weekly'
  }

  It 'builds a block only after all 4 weeks finished, including every weekly' {
    # Block 11 of 2026 = W41..W44 = 2026-10-05 .. 2026-11-01
    foreach ($d in '2026-10-06', '2026-10-14', '2026-10-21', '2026-10-28', '2026-10-30') { Add-FakeLog $root ([datetime]$d) | Out-Null }
    Invoke-RotationDay $root ([datetime]'2026-10-12') ([datetime]'2026-11-01')
    Test-Path (Join-Path $root 'archive/blocks/itcmdr-2026-B11.zip') | Should -BeFalse
    Invoke-RotationDay $root ([datetime]'2026-11-02') ([datetime]'2026-11-03')
    $bz = Join-Path $root 'archive/blocks/itcmdr-2026-B11.zip'
    Test-Path $bz | Should -BeTrue
    Get-ZipEntryName $bz | Should -Be @('itcmdr-2026-W41.zip', 'itcmdr-2026-W42.zip', 'itcmdr-2026-W43.zip', 'itcmdr-2026-W44.zip', 'manifest.json')
    $m = Test-ItcmdrZipManifest -ZipPath $bz
    $m.kind | Should -Be 'block4'
    $m.start | Should -Be '2026-10-05'
    $m.end | Should -Be '2026-11-01'
    @(Get-ChildItem (Join-Path $root 'archive/weekly') -Filter *.zip).Count | Should -Be 0
  }

  It 'puts week 53 into block 13 of a 53-week year (5 weeklies) and builds the ISO yearly zip' {
    foreach ($d in '2026-12-01', '2026-12-08', '2026-12-15', '2026-12-22', '2026-12-29', '2027-01-02', '2026-03-03') { Add-FakeLog $root ([datetime]$d) | Out-Null }
    Invoke-RotationDay $root ([datetime]'2026-12-28') ([datetime]'2027-01-03')
    Test-Path (Join-Path $root 'archive/blocks/itcmdr-2026-B13.zip') | Should -BeFalse
    Test-Path (Join-Path $root 'archive/yearly/itcmdr-2026.zip') | Should -BeFalse
    Invoke-RotationDay $root ([datetime]'2027-01-04') ([datetime]'2027-01-04')
    $bz = Join-Path $root 'archive/blocks/itcmdr-2026-B13.zip'
    (Get-ZipEntryName $bz) | Should -Be @('itcmdr-2026-W49.zip', 'itcmdr-2026-W50.zip', 'itcmdr-2026-W51.zip', 'itcmdr-2026-W52.zip', 'itcmdr-2026-W53.zip', 'manifest.json')
    $m = Test-ItcmdrZipManifest -ZipPath $bz
    $m.start | Should -Be '2026-11-30'
    $m.end | Should -Be '2027-01-03'
    $m.lastWeek | Should -Be 53
    $yz = Join-Path $root 'archive/yearly/itcmdr-2026.zip'
    (Get-ZipEntryName $yz) | Should -Be @('itcmdr-2026-B03.zip', 'itcmdr-2026-B13.zip', 'manifest.json')
    $ym = Test-ItcmdrZipManifest -ZipPath $yz
    $ym.weeks | Should -Be 53
    $ym.start | Should -Be '2025-12-29'
    $ym.end | Should -Be '2027-01-03'
    # Blocks are kept by default; re-running is a no-op.
    Test-Path $bz | Should -BeTrue
    (Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2027-01-05')).Count | Should -Be 0
  }

  It 'puts late logs for an already-zipped week into a numbered supplement' {
    Add-FakeLog $root ([datetime]'2026-10-21') | Out-Null
    Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2026-10-29') | Out-Null
    Add-FakeLog $root ([datetime]'2026-10-22') | Out-Null
    Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2026-10-30') | Out-Null
    Test-Path (Join-Path $root 'archive/weekly/itcmdr-2026-W43-2.zip') | Should -BeTrue
  }

  It 'detects a tampered archive' {
    Add-FakeLog $root ([datetime]'2026-10-21') | Out-Null
    Invoke-ItcmdrRotation -Root $root -Today ([datetime]'2026-10-29') | Out-Null
    $wz = Join-Path $root 'archive/weekly/itcmdr-2026-W43.zip'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::Open($wz, [IO.Compression.ZipArchiveMode]::Update)
    try {
      $e = @($z.Entries | Where-Object Name -like '*.log')[0]
      $s = $e.Open(); $s.SetLength(0); $w = New-Object IO.StreamWriter($s); $w.Write('tampered'); $w.Dispose()
    } finally { $z.Dispose() }
    { Test-ItcmdrZipManifest -ZipPath $wz } | Should -Throw '*hash mismatch*'
  }
}
