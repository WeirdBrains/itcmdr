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
