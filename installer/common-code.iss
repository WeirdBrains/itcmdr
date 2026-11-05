// Included inside each package's [Code] section.
const
  UninstallRoot = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\';
  CombinedKey = 'itcmdr_is1'; // the combined installer up to v0.14.2 (no AppId: AppName "itcmdr")

function ServiceExists(Name: string): Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec(ExpandConstant('{sys}\sc.exe'), 'query ' + Name, '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
end;

function Q(S: string): string;
begin
  StringChangeEx(S, '"', '', True);
  Result := '"' + S + '"';
end;

function PackageInstalled(Guid: string): Boolean;
begin
  Result := RegKeyExists(HKLM64, UninstallRoot + '{' + Guid + '}_is1');
end;

function DataDir(): string;
begin
  Result := ExpandConstant('{commonappdata}\itcmdr');
end;

// Moving off the combined installer: it recorded its components ("server,app,agent"). Once every
// one of them is installed as a split package (counting the one being installed now), its
// uninstall entry and unins000.* are removed. Files, data, keys and the service are untouched
// (each split package now owns its files). Until then the old entry stays and a log line says
// which packages are still missing.
// The combined installer did not reference-count Itcmdr.Plugin.psm1; the split packages mark it
// shared, so Inno counted the pre-existing file as one extra reference. Drop that one on takeover
// so uninstalling both server and agent removes it.
procedure DropCombinedSharedRef();
var
  Key, F: string;
  N: Cardinal;
begin
  Key := 'SOFTWARE\Microsoft\Windows\CurrentVersion\SharedDLLs';
  F := ExpandConstant('{pf}\{#InstallDirName}\agent\Itcmdr.Plugin.psm1');
  if RegQueryDWordValue(HKLM64, Key, F, N) and (N > 1) then
    RegWriteDWordValue(HKLM64, Key, F, N - 1);
end;

procedure MigrateCombined(ThisPart: string);
var
  Comps, Missing: string;
begin
  if not RegKeyExists(HKLM64, UninstallRoot + CombinedKey) then exit;
  if not RegQueryStringValue(HKLM64, UninstallRoot + CombinedKey, 'Inno Setup: Selected Components', Comps) then
    Comps := 'server,app,agent';
  Missing := '';
  if (Pos('server', Comps) > 0) and (ThisPart <> 'server') and not PackageInstalled('{#ServerAppId}') then Missing := Missing + ' server';
  if (Pos('agent', Comps) > 0) and (ThisPart <> 'agent') and not PackageInstalled('{#AgentAppId}') then Missing := Missing + ' agent';
  if (Pos('app', Comps) > 0) and (ThisPart <> 'app') and not PackageInstalled('{#AppAppId}') then Missing := Missing + ' app';
  if Missing <> '' then
  begin
    Log('Combined installer entry kept until these packages are installed too:' + Missing);
    exit;
  end;
  RegDeleteKeyIncludingSubkeys(HKLM64, UninstallRoot + CombinedKey);
  if (Pos('server', Comps) > 0) or (Pos('agent', Comps) > 0) then DropCombinedSharedRef();
  DeleteFile(ExpandConstant('{pf}\{#InstallDirName}\unins000.exe'));
  DeleteFile(ExpandConstant('{pf}\{#InstallDirName}\unins000.dat'));
  DeleteFile(ExpandConstant('{pf}\{#InstallDirName}\unins000.msg'));
  Log('Combined installer entry taken over (files kept).');
end;

// ---- Uninstall: data is kept unless asked ----
// Interactive: two questions, both default to keeping. Silent: /REMOVEDATA=yes removes this
// package's data, /REMOVEKEYS=yes also the server's keys\ folder. backups\ and any configured
// backup folder are never removed by an uninstaller.
var
  RemoveDataChosen, RemoveKeysChosen: Boolean;

// Read from the raw command line ({param:} is a Setup-side constant; this works in both).
function YesParam(Name: string): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/' + Name + '=yes') = 0 then Result := True;
end;

procedure AskRemoveData(WithKeys: Boolean);
begin
  RemoveDataChosen := YesParam('REMOVEDATA');
  RemoveKeysChosen := WithKeys and RemoveDataChosen and YesParam('REMOVEKEYS');
  if UninstallSilent() then exit;
  RemoveDataChosen := MsgBox('Keep this package''s data in ' + DataDir() + '?' + #13#10#13#10 +
    'Choose Yes (recommended) to keep settings and data for a later reinstall.' + #13#10 +
    'Backups are never removed.', mbConfirmation, MB_YESNO or MB_DEFBUTTON1) = IDNO;
  if RemoveDataChosen and WithKeys then
    RemoveKeysChosen := MsgBox('Also delete the keys folder (' + DataDir() + '\keys)?' + #13#10#13#10 +
      'Without it, a restore needs your 24 recovery words. Have them ready before saying Yes.' + #13#10 +
      'Choose No to keep the keys.', mbError, MB_YESNO or MB_DEFBUTTON2) = IDYES;
end;

procedure RemoveItem(Name: string);
var
  P: string;
begin
  P := DataDir() + '\' + Name;
  if DirExists(P) then
  begin
    if DelTree(P, True, True, True) then Log('removed ' + P) else Log('could not remove ' + P);
  end
  else if FileExists(P) then
  begin
    if DeleteFile(P) then Log('removed ' + P) else Log('could not remove ' + P);
  end;
end;

procedure RemoveDataDirIfEmpty();
begin
  // RemoveDir only succeeds on an empty folder: anything left (keys, backups, the other
  // package's files) keeps it.
  if RemoveDir(DataDir()) then Log('removed empty ' + DataDir());
end;

// ---- v1.2.1: persistent install/upgrade log, loud failure, services back on the old version ----
// Every package writes <LogRoot>\<part>-<version>-<yyyymmdd-hhnnss>.log (each step with a time,
// then Inno Setup's own log appended) and <LogRoot>\last-<part>.json (ok, step, error, exit code).
// LogRoot is %ProgramData%\itcmdr\install-logs (adminsvr: %ProgramData%\itcmdr-adminsvr\install-logs).
// Services are stopped and waited for (state STOPPED and the process gone), the files are probed
// for locks with retries, and if anything fails the services that were running are started again
// on the version still installed, an Application event (source itcmdr-setup) is written and Setup
// exits non-zero (7: failed before any file was replaced; 8: installed but a service did not start;
// Inno's own codes otherwise). Test hook: /SIMULATEFAIL=lock or /SIMULATEFAIL=start.
var
  ILogPath, IPart, IFrom, IStep, IError, ILogRoot, IServices, IExes, IWasRunning: string;
  IStarted, IOk, IPostFail: Boolean;

function IStamp(): string;
begin
  Result := GetDateTimeString('yyyy-mm-dd hh:nn:ss', '-', ':');
end;

procedure ILog(S: string);
begin
  Log(S);
  if ILogPath <> '' then SaveStringToFile(ILogPath, IStamp() + '  ' + S + #13#10, True);
end;

function JsonStr(S: string): string;
begin
  StringChangeEx(S, '\', '\\', True);
  StringChangeEx(S, '"', '\"', True);
  StringChangeEx(S, #13, ' ', True);
  StringChangeEx(S, #10, ' ', True);
  Result := '"' + S + '"';
end;

function Simulate(Step: string): Boolean;
begin
  Result := CompareText(ExpandConstant('{param:SIMULATEFAIL|}'), Step) = 0;
end;

// Next item of a comma list (removed from L).
function NextItem(var L: string): string;
var
  I: Integer;
begin
  I := Pos(',', L);
  if I = 0 then begin Result := Trim(L); L := ''; end
  else begin Result := Trim(Copy(L, 1, I - 1)); Delete(L, 1, I); end;
end;

function RunRC(Exe, Params: string): Integer;
begin
  if not Exec(Exe, Params, '', SW_HIDE, ewWaitUntilTerminated, Result) then Result := -1;
end;

function SvcIs(Name, State: string): Boolean;
begin
  Result := RunRC(ExpandConstant('{cmd}'), '/C ' + ExpandConstant('{sys}\sc.exe') + ' query ' + Name + ' | ' + ExpandConstant('{sys}\find.exe') + ' "' + State + '"') = 0;
end;

function ProcRunning(ExeName: string): Boolean;
begin
  Result := RunRC(ExpandConstant('{cmd}'), '/C ' + ExpandConstant('{sys}\tasklist.exe') + ' /FI "IMAGENAME eq ' + ExeName + '" /NH | ' + ExpandConstant('{sys}\find.exe') + ' /I "' + ExeName + '"') = 0;
end;

function InstalledVersion(Guid: string): string;
begin
  if not RegQueryStringValue(HKLM64, UninstallRoot + '{' + Guid + '}_is1', 'DisplayVersion', Result) then Result := '';
end;

// v1.3.0: keep the newest InstallLogKeep logs of this part; last-<part>.json and the current run's
// log are never touched. "admin-*" also matches "admin-app-*", so the name must continue with a digit.
const
  InstallLogKeep = 20;

function IsPartLog(Name, Part: string): Boolean;
var
  C: string;
begin
  Result := False;
  if (Length(Name) <= Length(Part) + 1) or (CompareText(Copy(Name, 1, Length(Part) + 1), Part + '-') <> 0) then exit;
  C := Copy(Name, Length(Part) + 2, 1);
  Result := (C >= '0') and (C <= '9') and (CompareText(ExtractFileExt(Name), '.log') = 0);
end;

procedure InstallLogPrune(LogRoot, Part, Current: string);
var
  FR: TFindRec;
  Count, Removed: Integer;
  Oldest: string;
  OH, OL: Cardinal;
begin
  Removed := 0;
  repeat
    Count := 0; Oldest := '';
    if FindFirst(LogRoot + '\' + Part + '-*.log', FR) then
    try
      repeat
        if IsPartLog(FR.Name, Part) and (CompareText(LogRoot + '\' + FR.Name, Current) <> 0) then
        begin
          Count := Count + 1;
          if (Oldest = '') or (FR.LastWriteTime.dwHighDateTime < OH) or ((FR.LastWriteTime.dwHighDateTime = OH) and (FR.LastWriteTime.dwLowDateTime < OL)) then
          begin Oldest := FR.Name; OH := FR.LastWriteTime.dwHighDateTime; OL := FR.LastWriteTime.dwLowDateTime; end;
        end;
      until not FindNext(FR);
    finally
      FindClose(FR);
    end;
    // Count excludes the current log, so keep InstallLogKeep - 1 older ones.
    if (Count < InstallLogKeep) or (Oldest = '') then break;
    if not DeleteFile(LogRoot + '\' + Oldest) then begin ILog('could not prune ' + Oldest); break; end;
    Removed := Removed + 1;
  until False;
  if Removed > 0 then ILog('pruned ' + IntToStr(Removed) + ' old install log(s), keeping the newest ' + IntToStr(InstallLogKeep));
end;

// Called from InitializeSetup. Services: stop order (e.g. watchdog first). Exes: files under {app}
// that are replaced (relative paths), probed for locks.
procedure InstallLogBegin(Part, Guid, LogRoot, Services, Exes: string);
begin
  IPart := Part; ILogRoot := LogRoot; IServices := Services; IExes := Exes;
  IStep := 'start'; IError := ''; IWasRunning := '';
  ForceDirectories(LogRoot);
  ILogPath := LogRoot + '\' + Part + '-{#MyAppVersion}-' + GetDateTimeString('yyyymmdd-hhnnss', #0, #0) + '.log';
  IFrom := InstalledVersion(Guid);
  if IFrom = '' then ILog('itcmdr ' + Part + ' setup {#MyAppVersion}: fresh install')
  else ILog('itcmdr ' + Part + ' setup {#MyAppVersion}: upgrade from ' + IFrom);
  if WizardSilent() then ILog('mode: silent') else ILog('mode: interactive');
  InstallLogPrune(LogRoot, Part, ILogPath);
end;

procedure IFail(Step, Err: string);
begin
  IStep := Step; IError := Err;
  ILog('FAILED at ' + Step + ': ' + Err);
end;

function StopOne(Name: string; TimeoutSec: Integer): Boolean;
var
  I, RC: Integer;
begin
  Result := True;
  if not ServiceExists(Name) then begin ILog('service ' + Name + ': not installed'); exit; end;
  if SvcIs(Name, 'STOPPED') then begin ILog('service ' + Name + ': already stopped'); exit; end;
  IWasRunning := IWasRunning + Name + ',';
  RC := RunRC(ExpandConstant('{sys}\sc.exe'), 'stop ' + Name);
  ILog('service ' + Name + ': stop requested (sc exit ' + IntToStr(RC) + ')');
  for I := 1 to TimeoutSec do
  begin
    if SvcIs(Name, 'STOPPED') then begin ILog('service ' + Name + ': stopped after ' + IntToStr(I) + ' s'); exit; end;
    if I mod 10 = 0 then ILog('service ' + Name + ': still stopping (' + IntToStr(I) + ' s)');
    Sleep(1000);
  end;
  Result := False;
end;

procedure WaitGone(ExeName: string);
var
  I, RC: Integer;
begin
  for I := 1 to 30 do
  begin
    if not ProcRunning(ExeName) then begin if I > 1 then ILog(ExeName + ': process exited'); exit; end;
    if I = 1 then ILog(ExeName + ': process still running, waiting');
    Sleep(1000);
  end;
  RC := RunRC(ExpandConstant('{sys}\taskkill.exe'), '/F /IM ' + ExeName);
  ILog(ExeName + ': still running after 30 s, taskkill /F (exit ' + IntToStr(RC) + ')');
  Sleep(2000);
end;

function Locked(P: string): Boolean;
begin
  Result := Simulate('lock') or (FileExists(P) and not RenameFile(P, P + '.lockprobe'));
  if (not Result) and FileExists(P + '.lockprobe') then RenameFile(P + '.lockprobe', P);
end;

// Called from PrepareToInstall: returns '' or the error (Setup then exits 7).
function InstallStopAndProbe(): string;
var
  L, S, P, F: string;
  I: Integer;
  Free: Boolean;
begin
  Result := '';
  IStep := 'stop services';
  L := IServices;
  while L <> '' do
  begin
    S := NextItem(L);
    if (S <> '') and not StopOne(S, 120) then
    begin
      ILog('service ' + S + ': not stopped after 120 s, ending its process');
    end;
  end;
  IStep := 'wait for processes';
  L := IExes;
  while L <> '' do
  begin
    F := NextItem(L);
    if F <> '' then WaitGone(ExtractFileName(F));
  end;
  IStep := 'file locks';
  L := IExes;
  while L <> '' do
  begin
    F := NextItem(L);
    if F = '' then continue;
    P := ExpandConstant('{app}\') + F;
    Free := False;
    for I := 1 to 30 do
    begin
      if not Locked(P) then begin Free := True; break; end;
      ILog(F + ': locked (attempt ' + IntToStr(I) + ' of 30), retrying in 2 s');
      Sleep(2000);
    end;
    if not Free then
    begin
      IFail('file locks', F + ' is locked by another process (antivirus, backup or a program still running)');
      Result := 'itcmdr ' + IPart + ' setup: ' + IError + '. Nothing was changed; the services were started again. Log: ' + ILogPath;
      exit;
    end;
    if FileExists(P) then ILog(F + ': not locked');
  end;
  IStep := 'replace files';
  ILog('replacing files (each file is in the Inno Setup log below)');
end;

procedure InstallFilesDone();
begin
  IOk := True;
  IStep := 'start services';
  ILog('files replaced; version now {#MyAppVersion}');
end;

// Called at ssDone: every service that was running before must be running again.
procedure InstallVerifyServices();
var
  L, S: string;
  I, RC: Integer;
  Up: Boolean;
begin
  L := IWasRunning;
  while L <> '' do
  begin
    S := NextItem(L);
    if S = '' then continue;
    Up := False;
    for I := 1 to 60 do
    begin
      if SvcIs(S, 'RUNNING') and not Simulate('start') then begin Up := True; break; end;
      if I = 20 then
      begin
        RC := RunRC(ExpandConstant('{sys}\sc.exe'), 'start ' + S);
        ILog('service ' + S + ': not running after 20 s, start again (sc exit ' + IntToStr(RC) + ')');
      end;
      Sleep(1000);
    end;
    if Up then ILog('service ' + S + ': running')
    else
    begin
      IPostFail := True;
      IFail('start services', 'service ' + S + ' did not start after the upgrade to {#MyAppVersion}');
    end;
  end;
  if not IPostFail then IStep := 'done';
end;

// Failure before or during the file copy: Inno restored the old files; start what was running.
procedure InstallRestoreServices();
var
  L, S: string;
  RC, I: Integer;
begin
  L := IWasRunning;
  // reverse of the stop order: the service first, then its watchdog
  S := '';
  while L <> '' do S := NextItem(L) + ',' + S;
  L := S;
  while L <> '' do
  begin
    S := NextItem(L);
    if S = '' then continue;
    RC := RunRC(ExpandConstant('{sys}\sc.exe'), 'start ' + S);
    for I := 1 to 30 do
    begin
      if SvcIs(S, 'RUNNING') then break;
      Sleep(1000);
    end;
    if SvcIs(S, 'RUNNING') then ILog('service ' + S + ': started again on version ' + IFrom + ' (sc exit ' + IntToStr(RC) + ')')
    else ILog('service ' + S + ': COULD NOT be started again (sc exit ' + IntToStr(RC) + '); start it by hand');
  end;
end;

function GetCustomSetupExitCode(): Integer;
begin
  if IPostFail then Result := 8 else Result := 0;
end;

procedure DeinitializeSetup();
var
  Code, OkS: string;
  RC: Integer;
begin
  if ILogPath = '' then exit;
  OkS := 'false';
  if IOk and not IPostFail then
  begin
    ILog('OK: ' + IPart + ' ' + IFrom + ' -> {#MyAppVersion}; exit code 0');
    Code := '0';
    OkS := 'true';
  end
  else
  begin
    if IError = '' then IFail(IStep, 'setup was aborted (see the Inno Setup log below); the files were rolled back');
    if IPostFail then Code := '8' else if IStep = 'file locks' then Code := '7' else Code := 'non-zero';
    ILog('UPGRADE FAILED: ' + IPart + ' stays at ' + IFrom + '; exit code ' + Code);
    RC := RunRC(ExpandConstant('{sys}\eventcreate.exe'), '/L APPLICATION /T ERROR /SO itcmdr-setup /ID 100 /D ' + Q('itcmdr ' + IPart + ' setup {#MyAppVersion} failed at ' + IStep + ': ' + IError + ' Log: ' + ILogPath));
    ILog('Application event written (eventcreate exit ' + IntToStr(RC) + ')');
  end;
  SaveStringToFile(ILogRoot + '\last-' + IPart + '.json',
    '{"part":' + JsonStr(IPart) + ',"from":' + JsonStr(IFrom) + ',"to":' + JsonStr('{#MyAppVersion}') +
    ',"ok":' + OkS + ',"step":' + JsonStr(IStep) +
    ',"error":' + JsonStr(IError) + ',"exit":' + JsonStr(Code) + ',"log":' + JsonStr(ILogPath) +
    ',"time":' + JsonStr(IStamp()) + '}', False);
  // After last-<part>.json, so the restarted service sees the result at once.
  if not IOk then InstallRestoreServices();
  // Setup still holds its own log open (deny write), so copy it with cmd's type, which shares.
  SaveStringToFile(ILogPath, #13#10 + '---- Inno Setup log ----' + #13#10, True);
  if RunRC(ExpandConstant('{cmd}'), '/C type "' + ExpandConstant('{log}') + '" >> "' + ILogPath + '"') <> 0 then
    SaveStringToFile(ILogPath, '(could not copy ' + ExpandConstant('{log}') + ')' + #13#10, True);
end;
