#Requires -Version 5.1
<#
.SYNOPSIS
  Apply an agent enrollment token: stop the services, enroll (replacing any existing
  enrollment under the current computer name), roll back agent.json on failure, restart,
  and check that the agent links. Used by the agent installer when /EnrollToken= is set
  (v1.3.4). Exit 9 on any enroll failure so Setup never exits 0 after a silent no-op.
#>
[CmdletBinding()]
param(
  [string]$Token,
  [string]$TokenFile,
  [string]$AgentExe = $(if ($env:ProgramFiles) { Join-Path $env:ProgramFiles 'itcmdr\agent\current\itcmdr-agent.exe' } else { 'itcmdr-agent' }),
  [string]$DataDir = $(if ($env:ProgramData) { Join-Path $env:ProgramData 'itcmdr\agent' } else { '/var/lib/itcmdr-agent' }),
  [string]$ComputerName = $(if ($env:COMPUTERNAME) { $env:COMPUTERNAME } elseif ($env:HOSTNAME) { $env:HOSTNAME } else { [Environment]::MachineName }),
  [int]$ConnectTimeoutSec = 90,
  [switch]$SkipServiceControl,
  [switch]$SkipConnectCheck
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:EnrollFailExit = 9

function Get-ItcmdrNormalizedEnrollToken {
  param([Parameter(Mandatory)][AllowEmptyString()][string]$Raw)
  if ([string]::IsNullOrWhiteSpace($Raw)) { return '' }
  foreach ($line in ($Raw -split '\r?\n')) {
    $t = $line.Trim().Trim('"').Trim("'")
    if ($t.StartsWith('itcmdr-enroll:')) { return $t }
  }
  foreach ($line in ($Raw -split '\r?\n')) {
    $t = $line.Trim().Trim('"').Trim("'")
    if ($t -match 'itcmdr-enroll:\S+') { return $Matches[0] }
  }
  foreach ($line in ($Raw -split '\r?\n')) {
    $t = $line.Trim().Trim('"').Trim("'")
    if (-not $t) { continue }
    if ($t -match '^(?i)(one-time token|error:|usage:|itcmdr-cli)\b') { continue }
    if ($t -notmatch '\s' -and $t.Length -ge 16) { return $t }
  }
  return $Raw.Trim()
}

function Get-ItcmdrEnrollTokenFromInput {
  param([string]$Token, [string]$TokenFile)
  if ($TokenFile) {
    if (-not (Test-Path -LiteralPath $TokenFile)) {
      throw "enroll token file not found: $TokenFile"
    }
    $Token = [IO.File]::ReadAllText($TokenFile)
  }
  $n = Get-ItcmdrNormalizedEnrollToken -Raw ([string]$Token)
  if (-not $n) { throw 'enroll token is empty after trimming whitespace' }
  return $n
}

function Backup-ItcmdrAgentState {
  param([string]$DataDir)
  $src = Join-Path $DataDir 'agent.json'
  if (-not (Test-Path -LiteralPath $src)) { return $null }
  $bak = Join-Path $DataDir 'agent.json.bak-enroll'
  Copy-Item -LiteralPath $src -Destination $bak -Force
  $keys = Join-Path $DataDir 'keys'
  $keysBak = Join-Path $DataDir 'keys.bak-enroll'
  if (Test-Path -LiteralPath $keys) {
    if (Test-Path -LiteralPath $keysBak) { Remove-Item -LiteralPath $keysBak -Recurse -Force }
    Copy-Item -LiteralPath $keys -Destination $keysBak -Recurse -Force
  }
  return $bak
}

function Restore-ItcmdrAgentState {
  param([string]$DataDir, [string]$Backup)
  if (-not $Backup -or -not (Test-Path -LiteralPath $Backup)) { return }
  Copy-Item -LiteralPath $Backup -Destination (Join-Path $DataDir 'agent.json') -Force
  $keysBak = Join-Path $DataDir 'keys.bak-enroll'
  $keys = Join-Path $DataDir 'keys'
  if (Test-Path -LiteralPath $keysBak) {
    if (Test-Path -LiteralPath $keys) { Remove-Item -LiteralPath $keys -Recurse -Force }
    Copy-Item -LiteralPath $keysBak -Destination $keys -Recurse -Force
  }
}

function Remove-ItcmdrAgentStateBackup {
  param([string]$DataDir)
  foreach ($n in @('agent.json.bak-enroll', 'keys.bak-enroll')) {
    $p = Join-Path $DataDir $n
    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction SilentlyContinue }
  }
}

function Stop-ItcmdrAgentServices {
  foreach ($s in @('itcmdr-agent-watchdog', 'itcmdr-agent')) {
    try { Stop-Service -Name $s -Force -ErrorAction Stop } catch { }
  }
}

function Start-ItcmdrAgentServices {
  foreach ($s in @('itcmdr-agent', 'itcmdr-agent-watchdog')) {
    try { Start-Service -Name $s -ErrorAction Stop } catch { }
  }
}

function Get-ItcmdrAgentLogSize {
  param([string]$DataDir)
  $log = Join-Path $DataDir 'logs\agent.log'
  if (Test-Path -LiteralPath $log) { return [int64](Get-Item -LiteralPath $log).Length }
  return [int64]0
}

function Read-ItcmdrSharedText {
  param([string]$Path)
  # The agent keeps agent.log open for append; ReadAllText would throw and look like an enroll failure.
  $fs = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
  try {
    $sr = New-Object System.IO.StreamReader($fs)
    try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
  } finally { $fs.Dispose() }
}

function Wait-ItcmdrAgentLinked {
  param([string]$DataDir, [int]$TimeoutSec, [int64]$AfterBytes = 0)
  $log = Join-Path $DataDir 'logs\agent.log'
  $deadline = [datetime]::UtcNow.AddSeconds($TimeoutSec)
  do {
    if (Test-Path -LiteralPath $log) {
      try { $txt = Read-ItcmdrSharedText -Path $log } catch { $txt = '' }
      if ($txt) {
        $start = 0
        if ($AfterBytes -gt 0 -and $txt.Length -ge $AfterBytes) { $start = [int]$AfterBytes }
        if ($txt.Length -lt $AfterBytes) { $start = 0 }
        if ($txt.Substring($start) -match 'linked to the server') { return $true }
      }
    }
    Start-Sleep -Milliseconds 250
  } while ([datetime]::UtcNow -lt $deadline)
  return $false
}

function Invoke-ItcmdrAgentEnrollExe {
  param(
    [string]$AgentExe,
    [string]$Token,
    [string]$ComputerName,
    [string]$Role,
    [string]$DataDir
  )
  if (-not (Test-Path -LiteralPath $AgentExe)) { throw "agent executable not found: $AgentExe" }
  $old = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  & $AgentExe --dir $DataDir enroll $Token --machine $ComputerName --role $Role
  $rc = $LASTEXITCODE
  $ErrorActionPreference = $old
  if ($null -eq $rc) { $rc = 0 }
  if ($rc -ne 0) { throw "itcmdr-agent enroll exited $rc" }
}

function Invoke-ItcmdrEnrollToken {
  param(
    [string]$Token,
    [string]$TokenFile,
    [string]$AgentExe,
    [string]$DataDir,
    [string]$ComputerName,
    [int]$ConnectTimeoutSec,
    [bool]$SkipServiceControl,
    [bool]$SkipConnectCheck
  )
  $tok = Get-ItcmdrEnrollTokenFromInput -Token $Token -TokenFile $TokenFile
  if (-not $ComputerName) { $ComputerName = [Environment]::MachineName }
  New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
  $role = 'workstation'
  $existing = Join-Path $DataDir 'agent.json'
  if (Test-Path -LiteralPath $existing) {
    try {
      $prev = Get-Content -LiteralPath $existing -Raw | ConvertFrom-Json
      if ($prev.role) { $role = [string]$prev.role }
    } catch { }
  }
  if (-not $SkipServiceControl) { Stop-ItcmdrAgentServices }
  $bak = Backup-ItcmdrAgentState -DataDir $DataDir
  $enrolled = $false
  try {
    Invoke-ItcmdrAgentEnrollExe -AgentExe $AgentExe -Token $tok -ComputerName $ComputerName -Role $role -DataDir $DataDir
    $stPath = Join-Path $DataDir 'agent.json'
    if (Test-Path -LiteralPath $stPath) {
      $st = Get-Content -LiteralPath $stPath -Raw | ConvertFrom-Json
      if ($st.machine -and ($st.machine -ne $ComputerName)) {
        throw "enrolled as '$($st.machine)' instead of the current computer name '$ComputerName'"
      }
    }
    $enrolled = $true
    if ($TokenFile -and (Test-Path -LiteralPath $TokenFile)) {
      Remove-Item -LiteralPath $TokenFile -Force -ErrorAction SilentlyContinue
    }
    $mark = Get-ItcmdrAgentLogSize -DataDir $DataDir
    if (-not $SkipServiceControl) { Start-ItcmdrAgentServices }
    if (-not $SkipConnectCheck) {
      if (-not (Wait-ItcmdrAgentLinked -DataDir $DataDir -TimeoutSec $ConnectTimeoutSec -AfterBytes $mark)) {
        throw "agent did not link to the server within ${ConnectTimeoutSec}s after enroll"
      }
    }
  } catch {
    Restore-ItcmdrAgentState -DataDir $DataDir -Backup $bak
    if (-not $SkipServiceControl) { Start-ItcmdrAgentServices }
    throw
  } finally {
    if ($enrolled) { Remove-ItcmdrAgentStateBackup -DataDir $DataDir }
  }
}

if ($env:ITCMDR_IMPORT_ONLY -eq '1') { return }

try {
  Invoke-ItcmdrEnrollToken -Token $Token -TokenFile $TokenFile -AgentExe $AgentExe -DataDir $DataDir `
    -ComputerName $ComputerName -ConnectTimeoutSec $ConnectTimeoutSec `
    -SkipServiceControl:$SkipServiceControl.IsPresent -SkipConnectCheck:$SkipConnectCheck.IsPresent
  exit 0
} catch {
  $msg = $_.Exception.Message
  try {
    if ($DataDir) {
      New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
      [IO.File]::WriteAllText((Join-Path $DataDir 'last-enroll-error.txt'), $msg)
    }
  } catch { }
  [Console]::Error.WriteLine("error: $msg")
  exit $script:EnrollFailExit
}
