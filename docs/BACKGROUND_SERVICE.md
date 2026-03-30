# Background Service & Autostart Report
## Nexora Printer Manager v1.4.0

This report explains how the application handles background execution and automatic startup on Windows.

### 1. Background Execution (System Tray)
The application is designed to stay active even when the main window is closed. This ensures that the HTTP server remains available for print requests from your POS system.

#### How it works:
- **`tray-icon` & `crossbeam-channel`**: The app uses the `tray-icon` crate to create an icon in the Windows System Tray (near the clock).
- **Event Loop**: A background thread monitors the tray icon for clicks.
- **Minimize to Tray**: When you click the "X" on the main window, the application hides the window instead of terminating the process.
- **Restoring**: Double-clicking the tray icon brings the management interface back to the foreground.
- **Exit**: To fully close the application, you must right-click the tray icon and select **Exit**.

### 2. Automatic Startup
To ensure the printer manager is always ready when the computer turns on, we use the `auto-launch` crate.

#### Implementation:
- **Registry Entry**: The app adds an entry to the Windows Registry at `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Silent Start**: When the PC starts, the app launches in a "minimized" state directly to the system tray without popping up the main window.
- **Manual Toggle**: You can enable or disable this feature directly within the application settings.

### 3. Professional Installer (Recommended)
While the app can manage its own autostart, using a professional installer is the best way to deploy the application to multiple computers.

#### Why use an installer?
- **Dependency Management**: Ensures all required Windows components are present.
- **Standard Path**: Installs the app to `C:\Program Files\Nexora\`, which is more secure.
- **Uninstaller**: Provides a clean way to remove the app and its registry entries.
- **Firewall Rules**: An installer can automatically whitelist port `8080` so the POS can communicate with the printer manager without manual setup.

#### Recommended Tool: **Inno Setup** or **WiX Toolset**
We can provide a script for **Inno Setup** that will:
1. Package `nexora-printer-manager.exe` and its assets.
2. Create Desktop and Start Menu shortcuts.
3. Configure the "Run on Startup" registry key during installation.
4. Set up Windows Firewall rules for the HTTP server.

---
**Status**: Both Background Service and Autostart are currently implemented in the source code using `tray-icon` and `auto-launch`.
