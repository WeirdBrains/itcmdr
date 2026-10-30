<#
.SYNOPSIS
  Poll for on-demand itcmdr requests and run the agent with the requested options.

.DESCRIPTION
  Request files (<requestId>.json, written by the server) are read from:
    - <paths.localLogs>\requests\ (site.json, default C:\maintenanceRuns)
    - <shares.maintenanceRuns>\requests\ from site.json
  A request applies when its "machines" list is empty (all) or contains this computer.
  For each applicable request this computer:
    1. claims it with a per-machine lock (<requestId>.json.<COMPUTER>.lock),
    2. writes responses\<requestId>.<COMPUTER>.json with status "running",
    3. runs Invoke-ItcmdrAgent.ps1 with -ReportOnly / -Repair / -Force / -Categories,
    4. rewrites the response with "done" or "failed".
  A request already answered done/failed by this computer is skipped, so running
  every minute is safe. Compatible with Windows PowerShell 5.1 and PowerShell 7.
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function ConvertFrom-ItcmdrPollJson {
  param([string]$Text)
  $o = $Text.TrimStart([char]0xFEFF) | ConvertFrom-Json
  $h = @{}
  foreach ($p in $o.PSObject.Properties) { $h[$p.Name] = $p.Value }
  return $h
}

function Get-ItcmdrPollSite {
  foreach ($p in @('config/site.json', 'C:\ProgramData\itcmdr\site.json')) {
    if (Test-Path $p) {
      $o = (Get-Content $p -Raw).TrimStart([char]0xFEFF) | ConvertFrom-Json
      return $o
    }
  }
  return $null
}

function Get-ItcmdrRequestDirectory {
  param($Site)
  $local = 'C:\maintenanceRuns'
  if ($Site -and $Site.PSObject.Properties['paths'] -and $Site.paths.PSObject.Properties['localLogs'] -and $Site.paths.localLogs) {
    $local = [string]$Site.paths.localLogs
  }
  $dirs = @([IO.Path]::Combine($local, 'requests'))
  if ($Site -and $Site.PSObject.Properties['shares'] -and $Site.shares.PSObject.Properties['maintenanceRuns'] -and $Site.shares.maintenanceRuns) {
    $dirs += ([IO.Path]::Combine([string]$Site.shares.maintenanceRuns, 'requests'))
  }
  return $dirs
}

function Get-ItcmdrAgentArgument {
  <# Map request options to agent parameters. reportOnly wins over repair. #>
  param([hashtable]$Request)
  $a = @{}
  if ($Request['reportOnly']) { $a['ReportOnly'] = $true }
  elseif ($Request['repair']) { $a['Repair'] = $true }
  if ($Request['force']) { $a['Force'] = $true }
  $cats = @($Request['categories'] | Where-Object { $_ })
  if ($cats.Count -gt 0) { $a['Categories'] = [string[]]$cats }
  return $a
}

function Write-ItcmdrResponse {
  param([string]$Dir, [string]$RequestId, [string]$Computer, [string]$Status, [string]$Message = '')
  if (-not (Test-Path -LiteralPath $Dir)) { New-Item -ItemType Directory -Path $Dir -Force | Out-Null }
  $out = [pscustomobject]@{ requestId = $RequestId; machine = $Computer; status = $Status; message = $Message; ts = (Get-Date).ToString('o') }
  $path = Join-Path $Dir "$RequestId.$Computer.json"
  $tmp = "$path.tmp"
  [IO.File]::WriteAllText($tmp, ($out | ConvertTo-Json -Compress))
  Move-Item -LiteralPath $tmp -Destination $path -Force
}

function Invoke-ItcmdrPoll {
  # StaleLockMinutes: a lock older than this was left by a crashed/killed poller (the task's
  # limit is 2h), so it is removed and the request is claimed again.
  param([string[]]$RequestDirectory, [string]$Computer, [string]$AgentPath, [int]$StaleLockMinutes = 150)
  $handled = 0
  foreach ($dir in $RequestDirectory) {
    if (-not (Test-Path -LiteralPath $dir)) { continue }
    $respDir = Join-Path (Split-Path -Parent $dir) 'responses'
    foreach ($file in @(Get-ChildItem -LiteralPath $dir -Filter '*.json' -File -ErrorAction SilentlyContinue)) {
      $rid = [IO.Path]::GetFileNameWithoutExtension($file.Name)
      $respPath = Join-Path $respDir "$rid.$Computer.json"
      if (Test-Path -LiteralPath $respPath) {
        try {
          $prev = ConvertFrom-ItcmdrPollJson (Get-Content -LiteralPath $respPath -Raw)
          if ($prev['status'] -in @('done', 'failed')) { continue }
        } catch { Write-Verbose "Unreadable response $respPath; re-running" }
      }
      try { $req = ConvertFrom-ItcmdrPollJson (Get-Content -LiteralPath $file.FullName -Raw) } catch { continue }
      $machines = @($req['machines'] | Where-Object { $_ })
      if ($machines.Count -gt 0 -and $machines -notcontains $Computer) { continue }
      $lock = "$($file.FullName).$Computer.lock"
      if (Test-Path -LiteralPath $lock) {
        $age = (Get-Date).ToUniversalTime() - (Get-Item -LiteralPath $lock).LastWriteTimeUtc
        if ($age.TotalMinutes -gt $StaleLockMinutes) {
          Write-Verbose "Removing stale lock $lock ($([int]$age.TotalMinutes) min old)"
          Remove-Item -LiteralPath $lock -Force -ErrorAction SilentlyContinue
        }
      }
      try { New-Item -ItemType File -Path $lock -ErrorAction Stop | Out-Null } catch { continue }
      try {
        Write-ItcmdrResponse -Dir $respDir -RequestId $rid -Computer $Computer -Status 'running'
        $agentArgs = Get-ItcmdrAgentArgument -Request $req
        $status = 'done'; $msg = ''
        try { & $AgentPath @agentArgs | Out-Null } catch { $status = 'failed'; $msg = $_.ToString() }
        Write-ItcmdrResponse -Dir $respDir -RequestId $rid -Computer $Computer -Status $status -Message $msg
        $handled++
      } finally {
        Remove-Item -LiteralPath $lock -Force -ErrorAction SilentlyContinue
      }
    }
  }
  return $handled
}

if (-not $env:ITCMDR_IMPORT_ONLY) {
  $site = Get-ItcmdrPollSite
  $dirs = Get-ItcmdrRequestDirectory -Site $site
  Invoke-ItcmdrPoll -RequestDirectory $dirs -Computer $env:COMPUTERNAME -AgentPath (Join-Path $PSScriptRoot 'Invoke-ItcmdrAgent.ps1') | Out-Null
}
