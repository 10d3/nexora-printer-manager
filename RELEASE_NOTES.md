## Nexora Printer Manager v1.4.1

### What's New
- **Professional Setup Installer**: Added a robust Windows Setup Executable (`nexora-printer-manager-setup.exe`) powered by Inno Setup.
  - Automatically installs to `Program Files`.
  - Creates Start Menu and Desktop shortcuts.
  - Generates an uninstaller in Add/Remove Programs.
- **True Background Execution**: The application now seamlessly minimizes and stays running strictly in the system tray when closed, acting as a background daemon without appearing in the taskbar. 
  - Bypassed Slint event loop quitting limitations using native Win32 `HWND` extraction and `ShowWindow` for flawless background behavior.
- **Boot Autostart**: Integrated Windows Autostart capability toggleable directly from the System Tray menu.
- **CI/CD Integration**: Setup executables are now automatically compiled & attached to GitHub Releases via GitHub Actions.
- **File Logging**: Output logs are now correctly captured in local Application Data instead of dumping into an invisible console.
### Previous Updates

- **Logo Printing**: Added full image printing support for ESC/POS thermal printers.
  Receipts and invoices can now include a business logo printed directly at the
  top of the page.
  - Accepts base64-encoded PNG or JPEG (with or without `data:image/...;base64,` prefix)
  - Automatically scales images to fit the paper width while preserving aspect ratio —
    small images (like favicons) are scaled up, large images are scaled down
  - Supports `max_width` to constrain logo size (e.g. `200 dots` for a compact header logo)
  - Supports `align`: `left`, `center`, or `right` — alignment is baked into the
    bitmap row bytes directly since ESC/POS ignores text alignment commands for images
  - New `PrintCommand::Image(Vec<u8>)` variant for passing raw ESC/POS raster bytes
    through the render pipeline
  - `Element::Logo` in receipt templates now fully wired — pass `{{logo}}` as a
    variable resolved from custom data fields
  - New `/print-image` HTTP endpoint for printing standalone images without a template
  - ASCII art preview in `generate_image_preview()` also respects alignment and `max_width`
  
### Previous Updates
- **Robust Printing**: Switched to the Windows Print Spooler API (`OpenPrinterW`, `StartDocPrinterW`) for reliable support across all USB, Network, and LPT printers.
- **Template System**: Implemented a professional JSON-based template system for receipts, reports, and invoices.
- **Visual Improvements**:
    - Added **ESC/POS Reverse Mode** support for professional solid black bar charts and highlighting.
    - Enhanced **Table Alignment** using incremental scaling (error diffusion) for pixel-perfect vertical alignment between headers and rows.
    - Optimized for thermal printers: Replaced Unicode characters with ASCII equivalents to prevent gibberish on legacy devices.
- **Layout Precision**:
    - Added automatic **Safety Margins** (6 characters) to prevent unwanted line wrapping.
    - New `Write` command for inline text styling without automatic newlines.
    - Improved paper feed and cut reliability.
- **Integration**:
    - Enhanced HTTP API with `/template`, `/print-template`, and `/preview-template` endpoints.
    - Provided PowerShell integration scripts for automated testing and remote printing.
- Added background running service functionality for the application
- Implemented auto-start on PC startup/login
- **Fix:** Resolved a compilation error with the system tray interacting with the `crossbeam_channel` receiver
- **Fix:** Removed invalid package dependencies (`windows-subsystem`) causing deployment issues

**Downloads:**
- `nexora-printer-manager-setup.exe` - Windows Installer (Recommended)
- `nexora-printer-manager.exe` - Windows x64 executable (Portable)

### Installation
1. Download `nexora-printer-manager-setup.exe` and follow the installation wizard.
2. The application will launch safely in the background (visible in the System Tray).
3. The HTTP server starts automatically on port `8080`.
4. Right-click the printer icon in the taskbar tray to Show the Manager to configure devices or Toggle Launch at Startup.
