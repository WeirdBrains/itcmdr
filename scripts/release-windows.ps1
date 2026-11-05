param([Parameter(Mandatory)][string]$Zip, [Parameter(Mandatory)][string]$Ver, [Parameter(Mandatory)][string]$Commit,
      [string]$Work = 'C:\e2e', [string]$Share = '\\host.lan\Data')
# Builds the 7 Windows installers (Rust binaries with the static C runtime; the apps carry its DLLs) from a source zip of the tag (git archive vX.Y.Z + third_party).
# v1.3.1 release guard: $Commit (the tag's commit) goes into every binary's build stamp; after the
# build each binary is checked for "ITCMDR-BUILD v=$Ver commit=$Commit ..." and the stamps are
# written to <share>\rel-$Ver\windows-stamps.txt (one "<installer>: <stamp>" line per installer)
# for `release-local.sh check` (dist/$Ver.windows-stamps.txt). Build caches are removed at the end.
$ErrorActionPreference = 'Stop'
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ';C:\tools\flutter\bin;' + "$env:USERPROFILE\.cargo\bin"
$env:ITCMDR_BUILD_COMMIT = $Commit
$src = "C:\src\rel-$Ver"
if (Test-Path $src) { Remove-Item -Recurse -Force $src }
# A git checkout of the tag (from a bundle), so build scripts that run without our environment
# (Flutter native-asset hooks) still find the commit with `git rev-parse HEAD`; the zip adds third_party/.
$bundle = "$Share\itcmdr-v$Ver.bundle"
if (Test-Path $bundle) {
  git -c core.autocrlf=false clone -q -c core.autocrlf=false $bundle $src; if ($LASTEXITCODE -ne 0) { throw "clone failed" }
  git -C $src checkout -q "v$Ver"; if ($LASTEXITCODE -ne 0) { throw "checkout v$Ver failed" }
  if ((git -C $src rev-parse HEAD) -ne $Commit) { throw "GUARD: bundle v$Ver is not $Commit" }
  $tmp = "$src-zip"; Expand-Archive "$Work\$Zip" $tmp -Force
  Copy-Item "$tmp\third_party" $src -Recurse -Force; Remove-Item -Recurse -Force $tmp
} else { Expand-Archive "$Work\$Zip" $src -Force }
Set-Location $src
try {
  cargo build --release --locked -p itcmdr-server -p itcmdr-agent -p itcmdr-watchdog -p itcmdr-admin-cli -p itcmdr-adminsvr --bins
  if ($LASTEXITCODE -ne 0) { throw "cargo failed $LASTEXITCODE" }
  foreach ($d in 'app','admin-app') {
    Push-Location $d
    flutter config --enable-windows-desktop | Out-Null
    flutter pub get --enforce-lockfile; if ($LASTEXITCODE -ne 0) { throw "pub get $d failed" }
    flutter build windows --release; if ($LASTEXITCODE -ne 0) { throw "flutter build $d failed" }
    Pop-Location
  }
  # v1.3.2: the Rust binaries link the C runtime statically (.cargo/config.toml); the Flutter apps'
  # runners still need it, so its DLLs ship next to them (app-local, Microsoft's redistributable CRT).
  $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath
  $crt = Get-ChildItem "$vs\VC\Redist\MSVC\*\x64\Microsoft.VC14*.CRT" -Directory | Sort-Object FullName | Select-Object -Last 1
  if (-not $crt) { throw "no VC++ redistributable CRT folder under $vs" }
  foreach ($d in 'app','admin-app') { Copy-Item "$($crt.FullName)\*.dll" "$d\build\windows\x64\runner\Release\" -Force }
  # Guard: no Rust binary may need the VC++ runtime DLLs (a fresh Windows does not have them).
  foreach ($f in Get-ChildItem target\release\*.exe) {
    if ([Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($f.FullName)) -match '(?i)vcruntime140') { throw "GUARD: $($f.Name) imports the VC++ runtime" }
  }
  # Release guard: the stamp of each binary that goes into an installer.
  $re = [regex]"ITCMDR-BUILD v=(\S+) commit=([0-9a-zA-Z]+) help=([0-9a-f]{16}) END"
  function Stamp($f) { $m = $re.Match([Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($f))); if ($m.Success) { $m.Value } else { '' } }
  $parts = @{
    'server' = 'target\release\itcmdr-server.exe'; 'agent' = 'target\release\itcmdr-agent.exe'
    'cli' = 'target\release\itcmdr-cli.exe'; 'admin' = 'target\release\itcmdr-admin.exe'
    'adminsvr' = 'target\release\itcmdr-adminsvr.exe'
    'app' = 'app\build\windows\x64\runner\Release\*.dll'; 'admin-app' = 'admin-app\build\windows\x64\runner\Release\*.dll'
  }
  $lines = @()
  foreach ($p in $parts.Keys) {
    $s = Get-ChildItem $parts[$p] | ForEach-Object { Stamp $_.FullName } | Where-Object { $_ } | Select-Object -First 1
    if (-not $s) { throw "GUARD: no build stamp in $($parts[$p])" }
    if ($s -notmatch " v=$([regex]::Escape($Ver)) commit=$Commit ") { throw "GUARD: $p stamp '$s' is not v$Ver at $Commit" }
    $lines += "itcmdr-$p-Setup-$Ver.exe: $s"
  }
  $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
  foreach ($p in 'server','agent','cli','app','admin','adminsvr','admin-app') {
    & $iscc /Q "/DMyAppVersion=$Ver" "installer\itcmdr-$p.iss"
    if ($LASTEXITCODE -ne 0) { throw "ISCC $p failed $LASTEXITCODE" }
  }
  $out = "$Work\rel-$Ver"; New-Item -ItemType Directory -Force $out | Out-Null
  Copy-Item installer\out\*.exe $out -Force
  $lines | Set-Content "$out\windows-stamps.txt" -Encoding ascii
  if (Test-Path $Share) { New-Item -ItemType Directory -Force "$Share\rel-$Ver" | Out-Null; Copy-Item "$out\*" "$Share\rel-$Ver\" -Force }
  Get-ChildItem $out | Select-Object Name, Length | Out-String
  "BUILD-EXIT 0"
} finally {
  # Standing rule: clear the build caches and targets of every build (sources and installers stay).
  Set-Location $Work
  foreach ($d in "$src\target", "$src\app\build", "$src\admin-app\build", "$src\app\.dart_tool\hooks_runner", "$src\admin-app\.dart_tool\hooks_runner") {
    if (Test-Path $d) { Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue }
  }
  "cleaned build caches"
}
