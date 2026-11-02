; WinNote - installeur par utilisateur (aucun droit administrateur). Compiler : ISCC /DAppVersion=0.4.0 installer\winnote.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6E1B0C7A-5D1F-4E62-9B3C-7A2F5E4D8C11}
AppName=WinNote
AppVersion={#AppVersion}
AppPublisher=Heiphaistos
AppPublisherURL=https://github.com/Heiphaistos/WinNote
DefaultDirName={autopf}\WinNote
DefaultGroupName=WinNote
PrivilegesRequired=lowest
OutputDir=..\target\installer
OutputBaseFilename=WinNote-Setup-{#AppVersion}
SetupIconFile=..\winnote.ico
UninstallDisplayIcon={app}\winnote.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
DisableProgramGroupPage=yes

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "Créer un raccourci sur le Bureau"; Flags: unchecked
Name: "explorer"; Description: "Ajouter « Modifier avec WinNote » au menu contextuel de l'explorateur"

[Files]
Source: "..\target\release\winnote.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\WinNote"; Filename: "{app}\winnote.exe"
Name: "{autodesktop}\WinNote"; Filename: "{app}\winnote.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\*\shell\WinNote"; ValueType: string; ValueData: "Modifier avec WinNote"; Flags: uninsdeletekey; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\*\shell\WinNote"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\winnote.exe"",0"; Tasks: explorer
Root: HKCU; Subkey: "Software\Classes\*\shell\WinNote\command"; ValueType: string; ValueData: """{app}\winnote.exe"" ""%1"""; Tasks: explorer

[Run]
Filename: "{app}\winnote.exe"; Description: "Lancer WinNote"; Flags: nowait postinstall skipifsilent
