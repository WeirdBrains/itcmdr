# itcmdr.log/v1: structured plugin logs (decision 11)

Every service plugin writes **JSON Lines**: one JSON object per line, UTF8 text, `\n` line ends. The
server validates each line, counts the bad ones and turns `metrics` into chart data. The same rules
apply to the built-in checks when they become plugins (v0.11.0).

## Where the log goes

- Each run gets a log path from the server: `RunContext.log_path`, also in the environment as
  `ITCMDR_LOG_PATH`.
- The path is `<data>/plugin-logs/<plugin>/<yyyy-MM-dd>.jsonl` (local date). One file per plugin per
  day; runs append, and each line carries its `run_id`.
- The server writes a `run-start` and a `run-end` line around every run, so even a plugin that logs
  nothing leaves a record.
- After the run the day file is copied to the share at `<maintenanceRuns>/plugin-logs/<plugin>/`.
  The share rotation (`Rotate-ItcmdrLogs.ps1`) puts finished weeks into the weekly zip as
  `plugin-logs/<plugin>/<day>.jsonl`, so they also go into the 4-week blocks and yearly archives.
- Server backups include `plugin-logs/**`. A restore merges them back without overwriting newer
  files.

## Fields

Required. Every one must be a string:

| Field | Meaning |
|---|---|
| `schema` | Always `itcmdr.log/v1` |
| `ts` | RFC 3339 time **with an offset**, e.g. `2026-10-30T02:00:05.123-04:00` |
| `level` | `debug`, `info`, `warn` or `error` |
| `plugin` | Plugin name, as in `package.toml` |
| `plugin_version` | Plugin version |
| `run_id` | From `RunContext.run_id` |
| `machine` | Machine the work is about, e.g. `SERVER01` |
| `stage` | `verify`, `execute`, `log` (or `run` for the server's lines) |
| `event` | Short machine-readable name: `check`, `metrics`, `stage-failed`, ... |
| `message` | Text for people (may be empty) |

Optional:

| Field | Type | Meaning |
|---|---|---|
| `check_id` | string | e.g. `DISK-C` |
| `status` | string | `ok`, `fail`, `repaired`, `skipped` or `excepted` |
| `duration_ms` | integer | |
| `metrics` | list | See below |

Other fields are allowed and ignored.

## Metrics

```json
{"name": "disk_free", "value": 42.5, "unit": "GiB", "kind": "gauge", "labels": {"drive": "C:"}}
```

- `name`: required, not empty.
- `value`: required, a number.
- `kind`: required. `gauge` is a level, `counter` is a count since the last line, `histogram` is one
  observation.
- `unit`: optional.
- `labels`: optional. All values must be strings.

`GetPluginMetrics` returns the raw points, or daily values when you ask for them. Daily values sum
counters and average gauges and histograms. You can filter by name and labels.

## Bad lines

A line that is not JSON, or that breaks a rule above, is a **reject**. It is counted for its file
(`plugin_log_files.rejects`) and reported by `GetPluginMetrics` (`rejects`). It never stops the
ingest, and the line's metrics are not used. Each line is read once: the server remembers how far it
got in each file.

## Example

```json
{"schema":"itcmdr.log/v1","ts":"2026-10-30T02:00:05.120-04:00","level":"info","plugin":"disk-free","plugin_version":"1.0.0","run_id":"5c3e...","machine":"SERVER01","stage":"execute","event":"check","message":"C: has room","check_id":"DISK-C","status":"ok","duration_ms":12}
{"schema":"itcmdr.log/v1","ts":"2026-10-30T02:00:05.200-04:00","level":"info","plugin":"disk-free","plugin_version":"1.0.0","run_id":"5c3e...","machine":"SERVER01","stage":"execute","event":"metrics","message":"disk_free = 42.5","metrics":[{"name":"disk_free","value":42.5,"unit":"GiB","kind":"gauge","labels":{"drive":"C:"}}]}
```

## Helpers

**PowerShell** (`agent/Itcmdr.Plugin.psm1`, Windows PowerShell 5.1 and 7):

```powershell
Import-Module $env:ITCMDR_PS_MODULE
Invoke-ItcmdrPlugin -Info @{ name = 'disk-free'; version = '1.0.0'; features = @('verify', 'execute', 'log') } `
  -Verify  { param($Ctx) @{ ok = $true; message = 'ready' } } `
  -Execute { param($Ctx)
             Write-ItcmdrLog -EventName 'check' -Message 'C: has room' -CheckId 'DISK-C' -Status 'ok'
             Write-ItcmdrMetric -Name 'disk_free' -Value 42.5 -Unit 'GiB' -Labels @{ drive = 'C:' }
             @{ ok = $true; message = 'done'; results = @(@{ check_id = 'DISK-C'; status = 'ok'; message = '42.5 GiB' }) } } `
  -Log     { param($Ctx, $Stage, $Err) Write-ItcmdrLog -Level 'error' -EventName 'stage-failed' -Message "${Stage}: $Err" }
```

Also available: `New-ItcmdrLogRecord`, `New-ItcmdrMetric` and `Get-ItcmdrLogTimestamp`.

**Rust** (`itcmdr-plugin` crate, `itcmdr_plugin::log`):

```rust
let log = Logger::for_ctx("disk-free", "1.0.0", &ctx);
log.check("DISK-C", "ok", "C: has room", Some(12))?;
log.metric(Metric::gauge("disk_free", 42.5, "GiB").label("drive", "C:"))?;
```

`itcmdr_plugin::log::validate(line)` is the validator the server uses.

## Plugin protocol (summary)

- A plugin is a separate process. It serves `itcmdr.plugin.PluginService` (`proto/plugin.proto`) on
  loopback and prints `ITCMDR-PLUGIN|1|tcp|127.0.0.1:<port>` on stdout.
- The server passes a per-launch secret in `ITCMDR_PLUGIN_SECRET`. Every call carries it in
  `x-itcmdr-plugin-secret`.
- The order is `Describe` (the name must match), then `Verify` (if the plugin lists the feature),
  then `Execute`.
- A failed `Verify` stops the run before `Execute`. `Log` is called on any failure, with the
  failed stage and the error.
- Every call times out after `plugins.timeout_secs`.
- The plugin exits when its stdin closes.
- PowerShell plugins (`type = "powershell"` main file) run through `itcmdr-plugin-ps`, which calls the
  script once per stage. The shell is the `plugins.powershell` setting (`powershell.exe` on Windows).
