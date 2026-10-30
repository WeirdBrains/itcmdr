Set-StrictMode -Version Latest
# v0.10.0 PowerShell plugin module: itcmdr.log/v1 helpers and the stage dispatcher used by
# itcmdr-plugin-ps.

BeforeAll {
  Import-Module (Join-Path (Join-Path $PSScriptRoot '..') 'Itcmdr.Plugin.psm1') -Force
  $script:Required = 'schema', 'ts', 'level', 'plugin', 'plugin_version', 'run_id', 'machine', 'stage', 'event', 'message'
  function Invoke-Stage([string]$Json) {
    $out = Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.2.0'; features = @('verify', 'execute', 'log') } `
      -Verify { param($Ctx) if ($Ctx.args.PSObject.Properties['fail']) { throw 'not ready' }; @{ ok = $true; message = 'ready' } } `
      -Execute { param($Ctx)
        $null = $Ctx
        Write-ItcmdrLog -EventName 'check' -Message 'C: ok' -CheckId 'DISK-C' -Status 'ok' -DurationMs 12
        Write-ItcmdrMetric -Name 'disk_free' -Value 42.5 -Unit 'GiB' -Labels @{ drive = 'C:' }
        @{ ok = $true; message = 'done'; results = @(@{ check_id = 'DISK-C'; status = 'ok'; message = '42.5' }) } } `
      -Log { param($Ctx, $Stage, $Err) $null = $Ctx; Write-ItcmdrLog -Level 'error' -EventName 'stage-failed' -Message "${Stage}: $Err" } `
      -InputJson $Json
    $line = @($out)[-1]
    $line | Should -BeLike 'ITCMDR-RESULT|*'
    return ($line.Substring('ITCMDR-RESULT|'.Length) | ConvertFrom-Json)
  }
}

Describe 'Itcmdr.Plugin module' {
  BeforeEach {
    $dir = Join-Path ([IO.Path]::GetTempPath()) ('itcmdr-plug-' + [guid]::NewGuid())
    $log = Join-Path (Join-Path $dir 'disk-free') '2026-10-30.jsonl'
    $script:CtxJson = '{"run_id":"r-1","log_path":' + ($log | ConvertTo-Json) + ',"machine":"SERVER01","report_only":true,"args":{},"stage":"execute"}'
  }
  AfterEach { Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue }

  It 'formats timestamps as RFC 3339 with an offset' {
    Get-ItcmdrLogTimestamp | Should -Match '^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}[+-]\d\d:\d\d$'
  }

  It 'describes the plugin' {
    $r = Invoke-Stage '{"stage":"describe"}'
    $r.name | Should -Be 'disk-free'
    $r.version | Should -Be '1.2.0'
    @($r.features) | Should -Be @('verify', 'execute', 'log')
  }

  It 'executes and writes valid itcmdr.log/v1 lines with metrics' {
    $r = Invoke-Stage ('{"stage":"execute","ctx":' + $script:CtxJson + '}')
    $r.ok | Should -BeTrue
    @($r.results).Count | Should -Be 1
    $r.results[0].check_id | Should -Be 'DISK-C'
    $lines = @(Get-Content -LiteralPath $log | ForEach-Object { $_ | ConvertFrom-Json })
    $lines.Count | Should -Be 2
    foreach ($l in $lines) {
      foreach ($k in $script:Required) { $l.PSObject.Properties.Name | Should -Contain $k }
      $l.schema | Should -Be 'itcmdr.log/v1'
      $l.plugin | Should -Be 'disk-free'
      $l.plugin_version | Should -Be '1.2.0'
      $l.run_id | Should -Be 'r-1'
      $l.machine | Should -Be 'SERVER01'
    }
    $lines[0].status | Should -Be 'ok'
    $lines[0].duration_ms | Should -Be 12
    $m = @($lines[1].metrics)
    $m.Count | Should -Be 1
    $m[0].name | Should -Be 'disk_free'
    $m[0].value | Should -Be 42.5
    $m[0].kind | Should -Be 'gauge'
    $m[0].labels.drive | Should -Be 'C:'
  }

  It 'reports a failing stage as an error and runs Log' {
    $bad = $script:CtxJson.Replace('"args":{}', '"args":{"fail":"1"}')
    (Invoke-Stage ('{"stage":"verify","ctx":' + $bad + '}')).error | Should -Match 'not ready'
    $r = Invoke-Stage ('{"stage":"log","ctx":' + $bad + ',"failed_stage":"verify","error":"not ready"}')
    $r.ok | Should -BeTrue
    $l = Get-Content -LiteralPath $log | ConvertFrom-Json
    $l.level | Should -Be 'error'
    $l.event | Should -Be 'stage-failed'
    $l.message | Should -Be 'verify: not ready'
  }

  It 'rejects unknown stages and bad values' {
    (Invoke-Stage '{"stage":"dance","ctx":{}}').error | Should -Match 'unknown stage'
    { New-ItcmdrMetric -Name x -Value 1 -Kind 'pie' } | Should -Throw
    { Write-ItcmdrLog -EventName x -Status 'meh' -Path (Join-Path $dir 'x.jsonl') } | Should -Throw
  }

  It 'needs a log path' {
    $env:ITCMDR_LOG_PATH = $null
    { Write-ItcmdrLog -EventName x } | Should -Throw '*No log path*'
    Write-ItcmdrLog -EventName x -Path (Join-Path $dir 'p.jsonl')
    Test-Path (Join-Path $dir 'p.jsonl') | Should -BeTrue
  }
}
