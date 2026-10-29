<#
.SYNOPSIS
  Poll for on-demand itcmdr requests and execute runs with provided options.

.DESCRIPTION
  Looks for JSON request files under:
    - Local: C:\maintenanceRuns\requests\
    - Remote share: shares.maintenanceRuns\requests from site.json (optional)
  Claims matching requests (machines includes this computer or empty=all), runs Invoke-ItcmdrAgent.ps1 with flags,
  and writes a response file with basic status. Idempotent and safe to run every minute.
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Load-Site {
  $candidates = @('config/site.json','C:\ProgramData\itcmdr\site.json')
  foreach ($p in $candidates) { if (Test-Path $p) { return Get-Content $p -Raw | ConvertFrom-Json -AsHashtable } }
  return @{} 
}

function Get-RequestDirs($site) {
  $dirs = @('C:\maintenanceRuns\requests')
  if ($site.shares -and $site.shares.maintenanceRuns) {
    $shareReq = Join-Path $site.shares.maintenanceRuns 'requests'
    $dirs += $shareReq
  }
  $dirs | ForEach-Object { if (-not (Test-Path $_)) { New-Item -ItemType Directory -Path $_ -Force | Out-Null }; $_ }
}

function Claim-RequestFile($path) {
  $lock = "$path.lock"
  try {
    return (New-Item -ItemType File -Path $lock -Force -ErrorAction Stop)
  } catch { return $null }
}

$site = Load-Site
$computer = $env:COMPUTERNAME
$agent = Join-Path $PSScriptRoot 'Invoke-ItcmdrAgent.ps1'

foreach ($dir in Get-RequestDirs -site $site) {
  Get-ChildItem -LiteralPath $dir -Filter '*.json' -File -ErrorAction SilentlyContinue | ForEach-Object {
    $path = $_.FullName
    $req = $null
    try { $req = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -AsHashtable } catch { return }
    $machines = @($req.machines) | Where-Object { $_ } 
    $match = ($machines.Count -eq 0) -or ($machines -contains $computer)
    if (-not $match) { return }
    if (-not (Claim-RequestFile -path $path)) { return }
    $rid = [IO.Path]::GetFileNameWithoutExtension($path)
    $args = @('-ReportOnly:$' + [bool]$req.reportOnly, '-Force:$' + [bool]$req.force)
    if ($req.categories) { $args += @('-Categories', ($req.categories -join ',')) }
    try {
      & $agent @args
      $status = 'done'
    } catch {
      $status = 'failed'
    }
    # Simple response file
    $respDir = Join-Path $dir '..\\responses' | Resolve-Path -ErrorAction SilentlyContinue
    if (-not $respDir) { $respDir = Join-Path $dir '..\\responses' }
    if (-not (Test-Path $respDir)) { New-Item -ItemType Directory -Path $respDir -Force | Out-Null }
    $out = [pscustomobject]@{ requestId=$rid; machine=$computer; status=$status; ts=(Get-Date).ToString('o') }
    $out | ConvertTo-Json -Compress | Out-File -LiteralPath (Join-Path $respDir "$rid.$computer.json") -Encoding utf8
    Remove-Item -LiteralPath "$path.lock" -Force -ErrorAction SilentlyContinue
  }
}

