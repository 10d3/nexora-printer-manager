## Nexora Printer Manager v1.3.0

### What's New
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

### Previous Updates
- Added background running service functionality for the application
- Implemented auto-start on PC startup/login
- **Fix:** Resolved a compilation error with the system tray interacting with the `crossbeam_channel` receiver
- **Fix:** Removed invalid package dependencies (`windows-subsystem`) causing deployment issues

**Downloads:**
- `nexora-printer-manager.exe` - Windows x64 executable

### Installation
1. Download `nexora-printer-manager.exe`
2. Run the application
3. The HTTP server starts on port 8080
