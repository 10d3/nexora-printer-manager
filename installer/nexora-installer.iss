[Setup]
AppName=Nexora Printer Manager
AppVersion=1.3.0
AppPublisher=Nexora Team
AppPublisherURL=https://github.com/nexora/printer-manager
DefaultDirName={autopf}\Nexora\Printer Manager
DefaultGroupName=Nexora
OutputDir=Output
OutputBaseFilename=nexora-printer-manager-setup
Compression=lzma2
SolidCompression=yes
; SetupIconFile=..\assets\favicon.ico
UninstallDisplayIcon={app}\nexora-printer-manager.exe
ArchitecturesInstallIn64BitMode=x64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startmenuicon"; Description: "Create a Start Menu shortcut"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\nexora-printer-manager.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs
; Add other needed assets if any (e.g., UI layout files if they are not embedded)

[Icons]
Name: "{group}\Nexora Printer Manager"; Filename: "{app}\nexora-printer-manager.exe"
Name: "{group}\Uninstall Nexora Printer Manager"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Nexora Printer Manager"; Filename: "{app}\nexora-printer-manager.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\nexora-printer-manager.exe"; Description: "{cm:LaunchProgram,Nexora Printer Manager}"; Flags: nowait postinstall skipifsilent

[Registry]
; Configure autostart on user login by default via installer
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "NexoraPrinterManager"; ValueData: """{app}\nexora-printer-manager.exe"" --minimized"; Flags: uninsdeletevalue
