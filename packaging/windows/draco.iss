; Inno Setup 6 Script for Draco Code Editor (Per-User Installer, No Admin Required)
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#ifndef MyAppTag
  #define MyAppTag "v0.1.0"
#endif
#ifndef MyExePath
  #define MyExePath "..\..\target\x86_64-pc-windows-msvc\release\draco.exe"
#endif

[Setup]
AppId={{8E9A3F42-5C21-4B8F-9A63-7D2F4C1E8B10}
AppName=Draco
AppVersion={#MyAppVersion}
AppPublisher=Draco Contributors
AppPublisherURL=https://github.com/chaungoclong/draco
AppSupportURL=https://github.com/chaungoclong/draco/issues
DefaultDirName={localappdata}\Programs\Draco
DefaultGroupName=Draco
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ChangesEnvironment=yes
OutputDir=..\..\dist
OutputBaseFilename=draco-{#MyAppTag}-x86_64-setup
SetupIconFile=..\..\assets\icons\draco.ico
UninstallDisplayIcon={app}\draco.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "Add 'draco' to User PATH (run from terminal)"; GroupDescription: "System Integration:"
Name: "contextmenu"; Description: "Add 'Open with Draco' to file & folder context menu"; GroupDescription: "System Integration:"

[Files]
Source: "{#MyExePath}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icons\draco.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\docs\guides\user-guide.md"; DestDir: "{app}"; DestName: "USER_GUIDE.md"; Flags: ignoreversion

[Icons]
Name: "{group}\Draco"; Filename: "{app}\draco.exe"; IconFilename: "{app}\draco.ico"
Name: "{group}\Uninstall Draco"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Draco"; Filename: "{app}\draco.exe"; IconFilename: "{app}\draco.ico"; Tasks: desktopicon

[Registry]
; Optional "Open with Draco" right-click menu for files and folders (HKCU — no admin required)
Root: HKCU; Subkey: "Software\Classes\*\shell\OpenWithDraco"; ValueType: string; ValueName: ""; ValueData: "Open with Draco"; Tasks: contextmenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\*\shell\OpenWithDraco"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\draco.exe"",0"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\*\shell\OpenWithDraco\command"; ValueType: string; ValueName: ""; ValueData: """{app}\draco.exe"" ""%1"""; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\OpenWithDraco"; ValueType: string; ValueName: ""; ValueData: "Open Folder with Draco"; Tasks: contextmenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\OpenWithDraco"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\draco.exe"",0"; Tasks: contextmenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\OpenWithDraco\command"; ValueType: string; ValueName: ""; ValueData: """{app}\draco.exe"" ""%V"""; Tasks: contextmenu

[Run]
Filename: "{app}\draco.exe"; Description: "{cm:LaunchProgram,Draco}"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  OldPath, NewPath, AppDir: string;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
  begin
    AppDir := ExpandConstant('{app}');
    if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OldPath) then
      OldPath := '';
    if Pos(';' + UpperCase(AppDir) + ';', ';' + UpperCase(OldPath) + ';') = 0 then
    begin
      if OldPath <> '' then
        NewPath := OldPath + ';' + AppDir
      else
        NewPath := AppDir;
      RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', NewPath);
    end;
  end;
end;
