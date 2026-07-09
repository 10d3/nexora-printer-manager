# Barcode Printer Integration Plan
> **Status:** Planning  
> **Last Updated:** 2026-07-09  
> **Target Version:** v1.6.0

---

## 🎯 Goal

Add a second, independent printer slot dedicated to a **thermal barcode/label printer** (e.g., Xprinter, HPRT, Zebra) that can print product barcodes on demand via a new `POST /print-barcode` API endpoint — without breaking the existing receipt printer functionality.

---

## 🔍 What We Already Have (No Need to Touch)

| Existing Asset | Location | Role |
|---|---|---|
| `PrinterConfig` struct | `src/main.rs:30` | Connection config (type, path) |
| `PrinterManager` struct | `src/main.rs:100` | Receipt printer state + methods |
| `PrinterConnection` enum | `src/main.rs:92` | USB / Network / LPT / System / Console |
| `print_raw()` method | `src/main.rs:236` | Writes raw bytes to any connection type |
| `AppState` | `src/http_server.rs:127` | Shared state injected into all Axum handlers |
| Axum router | `src/http_server.rs:653` | All HTTP routes registered here |
| `ui/main.slint` | `ui/main.slint` | The single Slint UI file |

---

## 📦 What We Need to Add

### New Cargo Dependencies (`Cargo.toml`)

> **Good news:** No new Cargo dependencies are needed!  
> TSPL and ZPL are plain-text command languages. We write the command strings directly
> in Rust — no barcode rendering crate required. The bytes are sent via the existing
> `print_raw()` method over USB/Network/LPT, just like receipts.

---

## 🏗️ Phase 1 — Data Models (`src/main.rs`)

**What to add:**

1. **`BarcodePrinterConfig` struct** — mirrors `PrinterConfig` but holds barcode-specific settings:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct BarcodePrinterConfig {
       pub connection_type: String,   // "USB", "Network", "LPT"
       pub device_path: String,       // COM port, IP:port, or LPT1
       pub protocol: String,          // "TSPL", "ZPL", or "EPL"
       pub label_width_mm: u32,       // physical label width  (e.g. 50)
       pub label_height_mm: u32,      // physical label height (e.g. 30)
       pub dpi: u32,                  // printer DPI (203 or 300)
   }
   ```

2. **`BarcodePrinterManager` struct** — a self-contained manager for the barcode printer,
   completely separate from `PrinterManager` so both printers are 100% independent:
   ```rust
   pub struct BarcodePrinterManager {
       pub connection: Option<PrinterConnection>,  // reuse existing enum
       pub config: Option<BarcodePrinterConfig>,
   }
   ```
   Methods to implement: `connect()`, `disconnect()`, `is_connected()`, `print_raw()`.
   The `print_raw()` here is a thin clone of the one already in `PrinterManager`.

3. **Update `AppState`** in `src/http_server.rs` to hold the second manager:
   ```rust
   pub struct AppState {
       pub printer_manager: Arc<Mutex<PrinterManager>>,
       pub barcode_manager: Arc<Mutex<BarcodePrinterManager>>,  // NEW
   }
   ```

---

## 🖨️ Phase 2 — Barcode Command Module (`src/barcode_printer.rs`)

**Create a new file:** `src/barcode_printer.rs`

This module translates a structured label request into raw command bytes using TSPL or ZPL.
No image conversion needed — the printer handles barcode rendering internally.

**Key types:**

```rust
pub enum BarcodeType { Code128, Code39, Ean13, Ean8, Upca, Qr }

pub struct BarcodeLabelRequest {
    pub barcode_data: String,
    pub barcode_type: BarcodeType,
    pub label_text: Option<String>,   // human-readable text printed below barcode
    pub copies: Option<u32>,
    // width/height/dpi are read from BarcodePrinterConfig, not the request
}
```

**TSPL command output** (for Xprinter XP-365B, HPRT, most budget label printers):
```
SIZE 50 mm, 30 mm
GAP 2 mm, 0 mm
DIRECTION 0
CLS
BARCODE 10,5,"128",60,1,0,2,2,"123456789012"
TEXT 10,70,"3",0,1,1,"Product Name - $10.00"
PRINT 1,1
```

**ZPL command output** (for Zebra printers):
```
^XA
^FO50,30^BY2^BCN,60,Y,N,N^FD123456789012^FS
^FO50,100^A0N,20,20^FDProduct Name - $10.00^FS
^XZ
```

The builder will be a simple `match` on `config.protocol`:
```rust
pub fn build_label(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    match config.protocol.as_str() {
        "TSPL" => build_tspl(config, req),
        "ZPL"  => build_zpl(config, req),
        "EPL"  => build_epl(config, req),
        _      => build_tspl(config, req), // safe default
    }
}
```

---

## 🌐 Phase 3 — New API Endpoints (`src/http_server.rs`)

**New request/response types to add:**

```rust
#[derive(Debug, Deserialize)]
pub struct PrintBarcodeRequest {
    pub barcode_data: String,
    pub barcode_type: String,         // "CODE128", "EAN13", "QR", etc.
    pub label_text: Option<String>,
    pub copies: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct BarcodeStatusResponse {
    pub connected: bool,
    pub protocol: Option<String>,
    pub label_width_mm: Option<u32>,
    pub label_height_mm: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct BarcodePrinterConnectRequest {
    pub connection_type: String,
    pub device_path: String,
    pub protocol: String,
    pub label_width_mm: u32,
    pub label_height_mm: u32,
    pub dpi: u32,
}
```

**New routes to register** in the Axum router (`src/http_server.rs:653`):

| Method | Path | Description |
|---|---|---|
| `POST` | `/barcode/connect` | Connect to the barcode printer |
| `POST` | `/barcode/disconnect` | Disconnect the barcode printer |
| `GET`  | `/barcode/status` | Get barcode printer connection status |
| `POST` | `/print-barcode` | 🖨️ Print a barcode label |
| `POST` | `/barcode/test-print` | Print a test label |

---

## 🖥️ Phase 4 — UI Updates (`ui/main.slint`)

Add a **second panel** (tab or expandable card) in the existing Slint UI alongside the receipt printer panel. The user should be able to:

- Select **connection type** (USB / Network / LPT) and enter **device path**.
- Select **protocol** (TSPL / ZPL / EPL) from a dropdown.
- Enter **label width**, **label height**, and **DPI**.
- Click **Connect** / **Disconnect**.
- See a live status indicator (🟢 Connected / 🔴 Disconnected).
- Click **Test Print** to fire a sample barcode label.

The barcode panel and the receipt printer panel must be **completely separate** and independently operable.

---

## 💾 Phase 5 — Config Persistence (`src/main.rs`)

The app already saves `PrinterConfig` to a JSON file on disk. The same pattern applies:

- Add `barcode_config: Option<BarcodePrinterConfig>` to the saved settings JSON struct.
- On app startup, load it and **auto-reconnect** the barcode printer (same behavior as the receipt printer).
- On connect/disconnect from the UI or API, **save** the updated config to disk immediately.

---

## 📋 Implementation Order

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5
Data Models → Command Module → API Routes → UI → Persistence
```

Each phase is independently testable. After Phase 3 you can already test with `curl`
before touching the UI at all.

---

## ❓ Open Questions (Need Your Input Before We Start)

1. **What is the make and model of your barcode printer?**
   This determines the protocol (TSPL for most Xprinter/HPRT, ZPL for Zebra).

2. **What label size do you use?**
   e.g., 50×30 mm, 40×20 mm — this sets the default config values.

3. **How does it connect to the PC?**
   USB (COM port / Windows system printer name), Network (IP:Port), or LPT?

> Once these 3 questions are answered, Phases 1–3 can be implemented in a single session.
