; RapidR's Windows installer (Inno Setup 6: free for any use, commercial too;
; the setup.exe it makes carries no licence obligations). tools/release/
; windows.ps1 compiles it once per kind and architecture:
;
;   ISCC.exe /DVersion=2.116.0 /DKind=sdk|runtime /DArch=x64|arm64
;            /DStage=<staged prefix> /DOutDir=<folder> rapidr.iss
;
; Per user, no administrator: %LOCALAPPDATA%\Programs\RapidR, bin\ on the
; user's PATH (a task, on by default), a Start menu entry for RapidR Studio (SDK),
; the file types under HKCU\Software\Classes — .rrbc runs (rapidrw.exe: a
; console program gets a console, a windowed one none), .rr and .rrproj (a
; project; SDK only) open in RapidR Studio, .rr with a "Run" action (the runtime
; alone: runs); .bas lists RapidR under
; "Open with", and is RapidR's by default only when its task is ticked (off by
; default: .bas is other BASICs' too). The uninstaller removes all of it.
; The SDK and the runtime share one AppId: installing one replaces the other.

#ifndef Version
  #error Pass /DVersion=x.y.z
#endif
#if Kind == "sdk"
  #define AppName "RapidR"
  #define FileBase "RapidR"
  #define AppIcon "rapidr-ide.ico"
#else
  #define AppName "RapidR Runtime"
  #define FileBase "RapidR-Runtime"
  #define AppIcon "rapidr-runtime.ico"
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
AppPublisher=Ruta Internet SRL
AppCopyright=Copyright (c) 2025-2026 Ruta Internet SRL
AppPublisherURL=https://github.com/iBobX/RapidR
AppSupportURL=https://github.com/iBobX/RapidR/issues
DefaultDirName={autopf}\RapidR
PrivilegesRequired=lowest
ArchitecturesAllowed={#Allowed}
ArchitecturesInstallIn64BitMode={#Allowed}
DisableProgramGroupPage=yes
DisableDirPage=auto
LicenseFile={#Stage}\share\doc\rapidr\LICENSE
InfoBeforeFile={#Stage}\share\doc\rapidr\LEGAL.md
OutputDir={#OutDir}
OutputBaseFilename={#FileBase}-{#Version}-windows-{#Arch}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
ChangesAssociations=yes
SetupIconFile={#Stage}\share\icons\{#AppIcon}
UninstallDisplayIcon={app}\share\icons\{#AppIcon}
UninstallDisplayName={#AppName} {#Version}

[Tasks]
Name: "basdefault"; Description: "Open .bas, .rqw, .rqb and .rq files with RapidR by default"; GroupDescription: "File types (.rrbc and .rr are always RapidR's):"; Flags: unchecked
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
Name: "{autoprograms}\RapidR Studio"; Filename: "{app}\bin\rapidrw.exe"; Parameters: "--ide"; WorkingDir: "{userdocs}"; IconFilename: "{app}\share\icons\rapidr-ide.ico"; Comment: "Write, run and build RapidR and RapidQ programs"
#endif

[Registry]
; .rrbc: a RapidR program — runs (rapidrw.exe: a console for a console program)
Root: HKA; Subkey: "Software\Classes\.rrbc"; ValueType: string; ValueName: ""; ValueData: "RapidR.Program"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\RapidR.Program"; ValueType: string; ValueName: ""; ValueData: "RapidR program"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\RapidR.Program\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\share\icons\rapidr-program.ico"""
Root: HKA; Subkey: "Software\Classes\RapidR.Program\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""
; .rr: RapidR source — always RapidR's: opens in the IDE (SDK), "Run" runs it
Root: HKA; Subkey: "Software\Classes\.rr\OpenWithProgids"; ValueType: string; ValueName: "RapidR.Source"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.rr"; ValueType: string; ValueName: ""; ValueData: "RapidR.Source"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\RapidR.Source"; ValueType: string; ValueName: ""; ValueData: "RapidR source"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\RapidR.Source\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\share\icons\rapidr-source.ico"""
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\run"; ValueType: string; ValueName: ""; ValueData: "&Run"
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\run\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""
#if Kind == "sdk"
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell"; ValueType: string; ValueName: ""; ValueData: "open"
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\open"; ValueType: string; ValueName: ""; ValueData: "&Open in RapidR Studio"
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" --ide ""%1"""
#else
Root: HKA; Subkey: "Software\Classes\RapidR.Source\shell"; ValueType: string; ValueName: ""; ValueData: "run"
#endif
; .bas: BASIC source — RapidR is always under "Open with"; the default only by its task
#if Kind == "sdk"
; RapidR Studio's project files (.rrproj) open in Studio.
Root: HKA; Subkey: "Software\Classes\.rrproj"; ValueType: string; ValueName: ""; ValueData: "RapidR.Project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\RapidR.Project"; ValueType: string; ValueName: ""; ValueData: "RapidR project"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\RapidR.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\share\icons\rapidr-source.ico"""
Root: HKA; Subkey: "Software\Classes\RapidR.Project\shell"; ValueType: string; ValueName: ""; ValueData: "open"
Root: HKA; Subkey: "Software\Classes\RapidR.Project\shell\open"; ValueType: string; ValueName: ""; ValueData: "&Open in RapidR Studio"
Root: HKA; Subkey: "Software\Classes\RapidR.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" --ide ""%1"""
#endif
Root: HKA; Subkey: "Software\Classes\.bas\OpenWithProgids"; ValueType: string; ValueName: "RapidR.BasicSource"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.bas"; ValueType: string; ValueName: ""; ValueData: "RapidR.BasicSource"; Flags: uninsdeletevalue; Tasks: basdefault
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource"; ValueType: string; ValueName: ""; ValueData: "BASIC source"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\share\icons\basic-source.ico"""
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell\run"; ValueType: string; ValueName: ""; ValueData: "&Run"
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell\run\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""
#if Kind == "sdk"
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell"; ValueType: string; ValueName: ""; ValueData: "open"
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell\open"; ValueType: string; ValueName: ""; ValueData: "&Open in RapidR Studio"
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" --ide ""%1"""
#else
Root: HKA; Subkey: "Software\Classes\RapidR.BasicSource\shell"; ValueType: string; ValueName: ""; ValueData: "run"
#endif

; .rqw / .rqb / .rq: RapidQ's window programs and libraries — RapidR under "Open with"; the default by the same task as .bas
Root: HKA; Subkey: "Software\Classes\.rqw\OpenWithProgids"; ValueType: string; ValueName: "RapidR.RapidQSource"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.rqw"; ValueType: string; ValueName: ""; ValueData: "RapidR.RapidQSource"; Flags: uninsdeletevalue; Tasks: basdefault
Root: HKA; Subkey: "Software\Classes\.rqb\OpenWithProgids"; ValueType: string; ValueName: "RapidR.RapidQSource"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.rqb"; ValueType: string; ValueName: ""; ValueData: "RapidR.RapidQSource"; Flags: uninsdeletevalue; Tasks: basdefault
Root: HKA; Subkey: "Software\Classes\.rq\OpenWithProgids"; ValueType: string; ValueName: "RapidR.RapidQSource"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.rq"; ValueType: string; ValueName: ""; ValueData: "RapidR.RapidQSource"; Flags: uninsdeletevalue; Tasks: basdefault
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource"; ValueType: string; ValueName: ""; ValueData: "RapidQ BASIC source"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\share\icons\basic-source.ico"""
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell\run"; ValueType: string; ValueName: ""; ValueData: "&Run"
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell\run\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" ""%1"""
#if Kind == "sdk"
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell"; ValueType: string; ValueName: ""; ValueData: "open"
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell\open"; ValueType: string; ValueName: ""; ValueData: "&Open in RapidR Studio"
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\bin\rapidrw.exe"" --ide ""%1"""
#else
Root: HKA; Subkey: "Software\Classes\RapidR.RapidQSource\shell"; ValueType: string; ValueName: ""; ValueData: "run"
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
