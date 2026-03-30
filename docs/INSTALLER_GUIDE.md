# Installer Creation Guide
## Nexora Printer Manager v1.4.0

Creating a professional Windows installer (.exe or .msi) is the recommended way to distribute the application. It handles file placement, shortcuts, and autostart registration automatically.

### Recommended Tool: Inno Setup
[Inno Setup](https://jrsoftware.org/isinfo.php) is a free and powerful tool for creating Windows installers.

#### 1. Configuration Script (setup.iss)
Create a file named `setup.iss` with the following configuration:

```iss
[Setup]
AppName=Nexora Printer Manager
AppVersion=1.3.0
DefaultDirName={autopf}\Nexora\PrinterManager
DefaultGroupName=Nexora POS
OutputDir=installer
OutputBaseFilename=nexora-printer-manager-setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "target\release\nexora-printer-manager.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\Nexora Printer Manager"; Filename: "{app}\nexora-printer-manager.exe"
Name: "{autodesktop}\Nexora Printer Manager"; Filename: "{app}\nexora-printer-manager.exe"

[Registry]
; Register the app to start with Windows
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
    ValueType: string; ValueName: "NexoraPrinterManager"; \
    ValueData: """{app}\nexora-printer-manager.exe"" --minimized"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\nexora-printer-manager.exe"; Description: "Launch Nexora Printer Manager"; Flags: nowait postinstall skipifsilent
```

#### 2. Build the Installer
1. Build the release version of the Rust app:
   ```powershell
   cargo build --release
   ```
2. Open Inno Setup Compiler.
3. Open `setup.iss`.
4. Press **Compile** (Ctrl+F9).
5. Your installer will be generated in the `installer/` folder.

---

### Alternative: WiX Toolset
For advanced enterprise deployment (.msi), use the **WiX Toolset**. 
- Best for **Active Directory** environments.
- Supports **Silent Installation** across a network.
- Requires more complex XML configuration.

**Recommendation**: Start with **Inno Setup** as it is easier to maintain and perfect for your current needs.
