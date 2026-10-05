; RapidR's Windows installer (Inno Setup 6: free for any use, commercial too;
; the setup.exe it makes carries no licence obligations). tools/release/
; windows.ps1 compiles it once per kind and architecture:
;
;   ISCC.exe /DVersion=2.116.0 /DKind=sdk|runtime /DArch=x64|arm64
;            /DStage=<staged prefix> /DOutDir=<folder> rapidr.iss
;
; Per user, no administrator: %LOCALAPPDATA%\Programs\RapidR, bin\ on the
; user's PATH, a Start menu entry for the IDE (SDK), the file types under
; HKCU\Software\Classes — .rrbc runs (rapidrw.exe: a console program gets a
; console, a windowed one none), .rr / .bas open in the IDE with a "Run"
; action (the runtime alone: they run). The uninstaller removes all of it.
; The SDK and the runtime share one AppId: installing one replaces the other.

#ifndef Version
  #error Pass /DVersion=x.y.z
#endif
#if Kind == "sdk"
  #define AppName "RapidR"
  #define FileBase "RapidR"
#else
  #define AppName "RapidR Runtime"
  #define FileBase "RapidR-Runtime"
#endif
#if Arch == "arm64"
  #define Allowed "arm64"
#else
  #define Allowed "x64compatible"
#endif

[Setup]
AppId={{6F3B2C1E-8A4D-4E5F-9B7A-1C2D3E4F5A6B}
AppName={#AppName}
AppVersion={#Version}
AppVerName={#AppName} {#Version}
AppPublisher=RapidR
AppPublisherURL=https://github.com/iBobX/RapidR
AppSupportURL=https://github.com/iBobX/RapidR/issues
DefaultDirName={autopf}\RapidR
PrivilegesRequired=lowest
ArchitecturesAllowed={#Allowed}
ArchitecturesInstallIn64BitMode={#Allowed}
DisableProgramGroupPage=yes
DisableDirPage=auto
LicenseFile={#Stage}\share\doc\rapidr\LICENSE
OutputDir={#OutDir}
OutputBaseFilename={#FileBase}-{#Version}-windows-{#Arch}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
ChangesAssociations=yes
UninstallDisplayIcon={app}\bin\rapidrw.exe
UninstallDisplayName={#AppName} {#Version}

[Tasks]
Name: "associate"; Description: "Open .rrbc, .rr and .bas files with RapidR"; GroupDescription: "File types:"
Name: "addtopath"; Description: "Add rapidr to PATH (for the command line)"; GroupDescription: "Command line:"

[Files]
Source: "{#Stage}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[InstallDelete]
; (an SDK replaced by the runtime, or an older version: no stale files)
Type: filesandordirs; Name: "{app}\lib"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\lib"

[Icons]
#if Kind == "sdk"
Name: "{autoprograms}\RapidR IDE"; Filename: "{app}\bin\rapidrw.exe"; Parameters: "--ide"; WorkingDir: "{userdocs}"; Comment: "Write, run and build RapidR and RapidQ programs"
#endif

[Registry]
; .rrbc: a RapidR program — runs
Root: HKA; Subkey: "Software\Classes\.rrbc"; ValueType: string; ValueName: ""; ValueData: "RapidR.Program"; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Program"; ValueType: string; ValueName: ""; ValueData: "RapidR program"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Program\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"",0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Program\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""; Tasks: associate
; .rr / .bas: source — opens in the IDE (SDK), "Run" runs it
Root: HKA; Subkey: "Software\Classes\.rr"; ValueType: string; ValueName: ""; ValueData: "RapidR.Source"; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\.bas"; ValueType: string; ValueName: ""; ValueData: "RapidR.Source"; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\.rr\OpenWithProgids"; ValueType: string; ValueName: "RapidR.Source"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\.bas\OpenWithProgids"; ValueType: string; ValueName: "RapidR.Source"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source"; ValueType: string; ValueName: ""; ValueData: "RapidR source"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"",0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\run"; ValueType: string; ValueName: ""; ValueData: "&Run"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\run\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""; Tasks: associate
#if Kind == "sdk"
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell"; ValueType: string; ValueName: ""; ValueData: "open"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\open"; ValueType: string; ValueName: ""; ValueData: "&Open in RapidR IDE"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" --ide ""%1"""; Tasks: associate
#else
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell"; ValueType: string; ValueName: ""; ValueData: "run"; Tasks: associate
#endif

[Code]
const
  EnvKey = 'Environment';

{ bin\ on the user's PATH (HKCU\Environment\Path), once. }
procedure AddToPath(Dir: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths) then
    Paths := '';
  if Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';') > 0 then
    exit;
  if (Paths <> '') and (Copy(Paths, Length(Paths), 1) <> ';') then
    Paths := Paths + ';';
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths + Dir);
end;

procedure RemoveFromPath(Dir: string);
var
  Paths: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths) then
    exit;
  P := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';');
  if P = 0 then
    exit;
  Delete(Paths, P, Length(Dir) + 1);
  if (Length(Paths) > 0) and (Copy(Paths, Length(Paths), 1) = ';') then
    Delete(Paths, Length(Paths), 1);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToPath(ExpandConstant('{app}\bin'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}\bin'));
end;
