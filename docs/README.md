# Nexora Printer Manager

A professional dual-printer management application for Windows. Designed to integrate with the Nexora POS web application for seamless receipt **and** barcode label printing.

## Overview

Nexora Printer Manager is a desktop application that:
- Connects to **ESC/POS receipt printers** (USB, Network, LPT) for receipts
- Connects to **thermal label printers** (TSPL/ZPL/EPL) for product barcodes
- Runs an HTTP server on port 8080 for integration with web applications
- Supports template-based receipt printing and direct barcode label printing via JSON API
- Provides a graphical interface for independent setup and testing of both printers
- Persists both printer configurations and auto-reconnects on startup

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Nexora Printer Manager                       │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐    ┌────────────────────────────────────┐   │
│  │   Slint UI      │    │       HTTP Server (Axum)           │   │
│  │                 │    │       localhost:8080                │   │
│  │  Receipt Panel  │◄──►│  GET  /health                      │   │
│  │  Barcode Panel  │    │  GET  /status                      │   │
│  └────────┬────────┘    │  POST /print-template              │   │
│           │             │  POST /test-print                  │   │
│           ▼             │  GET  /barcode/status              │   │
│  ┌──────────────────┐   │  POST /barcode/connect             │   │
│  │  PrinterManager  │   │  POST /barcode/disconnect          │   │
│  │  (ESC/POS)       │◄──│  POST /print-barcode               │   │
│  └──────────────────┘   │  POST /barcode/test-print          │   │
│  ┌──────────────────┐   └────────────────────────────────────┘   │
│  │BarcodePrinter    │                                            │
│  │Manager           │                                            │
│  │(TSPL/ZPL/EPL)    │                                            │
│  └──────────────────┘                                            │
│  USB │ Network (TCP/IP) │ LPT — for both managers               │
└──────────────────────────────────────────────────────────────────┘
```

---

## Getting Started

### Build & Run

```powershell
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run the application
cargo run
```

### First-Time Setup

1. **Launch the application** - The UI window opens and HTTP server starts on port 8080
2. **Select connection type** - Choose USB, Network, or LPT
3. **Scan for devices** - Click "Scan" to detect available printers
4. **Select your printer** - Choose from the detected devices list
5. **Connect** - Click "Connect Printer" to establish connection
6. **Test** - Use "Print Test Page" to verify the connection

---

## HTTP API Reference

The application exposes a REST API on `http://localhost:8080` for integration with web applications.

### Health Check

```http
GET /health
```

**Response:**
```json
{
  "status": "ok",
  "version": "1.0.0"
}
```

---

### Printer Status

```http
GET /status
```

**Response:**
```json
{
  "connected": true,
  "connection_type": "USB",
  "device_path": "COM3",
  "active_template": "receipt-v1",
  "cached_templates": 2
}
```

---

### Set Template

Upload and cache a receipt template for printing.

```http
POST /template
Content-Type: application/json
```

**Request Body:**
```json
{
  "template": {
    "id": "receipt-v1",
    "name": "Standard Receipt",
    "version": "1.0",
    "paper_width": 80,
    "layout": {
      "sections": [
        {
          "type": "header",
          "elements": [
            {
              "type": "text",
              "content": "{{store_name}}",
              "align": "center",
              "bold": true
            }
          ]
        }
      ]
    }
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "Template 'receipt-v1' set successfully"
}
```

---

### List Cached Templates

```http
GET /templates
```

**Response:**
```json
{
  "templates": [
    {
      "id": "receipt-v1",
      "name": "Standard Receipt",
      "version": "1.0"
    }
  ],
  "active_template": "receipt-v1"
}
```

---

### Get Specific Template

```http
GET /template/{id}
```

**Response:** The full template JSON or 404 if not found.

---

### Print with Template

Print a receipt using the active template and provided data.

```http
POST /print-template
Content-Type: application/json
```

**Request Body:**
```json
{
  "data": {
    "order_id": "ORD-001",
    "timestamp": "2026-01-30 11:00:00",
    "items": [
      {"name": "Burger", "quantity": 2, "price": 9.99, "total": 19.98},
      {"name": "Fries", "quantity": 1, "price": 3.99, "total": 3.99}
    ],
    "subtotal": 23.97,
    "tax": 2.40,
    "total": 26.37,
    "payment_method": "Card"
  }
}
```

**Response:**
```json
{
  "success": true,
  "message": "Receipt printed successfully"
}
```

---

### Test Print

Print a test page using the active template.

```http
POST /test-print
```

**Response:**
```json
{
  "success": true,
  "message": "Test print completed"
}
```

---

### Clear Template Cache

```http
DELETE /cache
```

**Response:**
```json
{
  "success": true,
  "message": "Cache cleared"
}
```

---

## Barcode Printer API Reference

The barcode printer operates **independently** of the receipt printer. It targets thermal label printers (e.g. Aokia AK-3001) using TSPL, ZPL, or EPL command protocols.

> **Note:** The barcode printer must be connected before printing. Connection settings are persisted — the app auto-reconnects on next launch.

---

### Barcode Printer Status

```http
GET /barcode/status
```

**Response:**
```json
{
  "connected": true,
  "protocol": "TSPL",
  "label_width_mm": 50,
  "label_height_mm": 30,
  "dpi": 203
}
```

---

### Connect Barcode Printer

```http
POST /barcode/connect
Content-Type: application/json
```

**Request Body:**
```json
{
  "connection_type": "USB",
  "device_path": "COM5",
  "protocol": "TSPL",
  "label_width_mm": 50,
  "label_height_mm": 30,
  "dpi": 203
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `connection_type` | string | ✅ | `"USB"`, `"Network"`, or `"LPT"` |
| `device_path` | string | ✅ | `"COM5"`, `"192.168.1.101:9100"`, or `"LPT1"` |
| `protocol` | string | ✅ | `"TSPL"` (default), `"ZPL"`, or `"EPL"` |
| `label_width_mm` | number | ✅ | Label width in millimeters |
| `label_height_mm` | number | ✅ | Label height in millimeters |
| `dpi` | number | ✅ | Printer DPI — usually `203` or `300` |

**Response:**
```json
{ "success": true, "message": "Barcode printer connected" }
```

---

### Disconnect Barcode Printer

```http
POST /barcode/disconnect
```

**Response:**
```json
{ "success": true, "message": "Barcode printer disconnected" }
```

---

### Print a Barcode Label

```http
POST /print-barcode
Content-Type: application/json
```

**Request Body:**
```json
{
  "barcode_data": "123456789012",
  "barcode_type": "CODE128",
  "label_text": "Cola 330ml",
  "copies": 1,
  "label_width_mm": 50,
  "label_height_mm": 30
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `barcode_data` | string | ✅ | The value to encode in the barcode |
| `barcode_type` | string | ❌ | Barcode format — defaults to `"CODE128"` |
| `label_text` | string | ❌ | Human-readable text printed below the barcode |
| `copies` | number | ❌ | Number of labels to print — defaults to `1` |
| `label_width_mm` | number | ❌ | Override label width for this job only (mm) |
| `label_height_mm` | number | ❌ | Override label height for this job only (mm) |

**How label layout works (auto-calculated):**

The printer engine automatically fits the barcode to your label — you do not need to specify dot coordinates or bar widths. For every print job it computes:

| Calculated value | How it is derived |
|---|---|
| Narrow bar width (dots) | `floor(printable_width / symbol_modules)`, clamped 1–3 |
| Barcode height (dots) | Capped at 55% of printable height so bars don't dominate the label |
| Horizontal alignment | Barcode and text are both horizontally centered based on their estimated widths |
| Vertical alignment | The entire content block (barcode + text) is vertically centered on the label |
| Text Y position (dots) | Barcode bottom + 4-dot gap — never overflows the label |
| Margins | 3 % of each dimension, minimum 3 dots per side |

> **Small label tip:** On a 32×25 mm label the engine automatically reduces the narrow bar to **1 dot** so that a 12-digit CODE128 barcode (121 symbol modules) fits within the 240 available dots. On wider labels (e.g. 50 mm) it uses 2–3 dots per bar for better scannability.

**Supported `barcode_type` values:**

| Value | Format | Use case |
|---|---|---|
| `CODE128` | Code 128 | General purpose (default) |
| `CODE39` | Code 39 | Alphanumeric, older systems |
| `EAN13` | EAN-13 | Standard retail products |
| `EAN8` | EAN-8 | Small retail products |
| `UPCA` | UPC-A | North American retail |
| `QR` | QR Code | URLs, product links |

**Response:**
```json
{ "success": true, "message": "Barcode label printed: 123456789012" }
```

---

### Print Test Label

Prints a test label to verify the barcode printer is working correctly.

```http
POST /barcode/test-print
```

**Response:**
```json
{ "success": true, "message": "Barcode test label printed" }
```

---

### Batch Printing

The API does not have a dedicated batch endpoint. Call `/print-barcode` in a loop from your application:

```javascript
const labels = [
  { barcode_data: '111111111111', label_text: 'Apple Juice', copies: 5 },
  { barcode_data: '222222222222', label_text: 'Orange Juice', copies: 3 },
  { barcode_data: '333333333333', label_text: 'Water Bottle', copies: 12 },
];

for (const label of labels) {
  await fetch('http://localhost:8080/print-barcode', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ barcode_type: 'CODE128', ...label })
  });
}
```

---

## Template System


Templates define the layout of receipts using JSON. They support:

### Element Types

| Type | Description |
|------|-------------|
| `text` | Plain or formatted text with alignment, bold, underline |
| `logo` | Image/logo placement |
| `divider` | Horizontal line separator |
| `row` | Left/right aligned row (e.g., item and price) |
| `qr` | QR code |
| `barcode` | Barcode with various formats |
| `table` | Data table with columns |
| `space` | Vertical spacing |

### Variable Substitution

Use `{{variable_name}}` in template content to insert data:

```json
{
  "type": "text",
  "content": "Order #{{order_id}}",
  "align": "center"
}
```

### Conditional Elements

Elements can have conditions:

```json
{
  "type": "text",
  "content": "Loyalty Points: {{points}}",
  "condition": "points > 0"
}
```

---

## Supported Printers

### Receipt Printers (ESC/POS)
- Epson TM-T88 Series (TM-T88III, TM-T88IV, TM-T88V, TM-T88VI)
- Star TSP143, TSP654
- Bixolon SRP-350
- Any ESC/POS compatible thermal printer

### Barcode / Label Printers
- **Aokia AK-3001** (TSPL — recommended)
- Any TSPL, ZPL, or EPL2 compatible label printer
  - TSPL: TSC, Argox, Godex, Bixolon label printers
  - ZPL: Zebra GX/ZD/ZT series
  - EPL2: Older Zebra / Eltron models

### Connection Types

| Type | Description | Example |
|------|-------------|---------|
| **USB** | Serial over USB | `COM3`, `COM5` |
| **Network** | TCP/IP Ethernet | `192.168.1.100:9100` |
| **LPT** | Parallel port (Windows only) | `LPT1` |

---

## Configuration

Settings are stored in:
```
%APPDATA%\nexora\printer-manager\config.json
```

Example:
```json
{
  "connection_type": "USB",
  "device_path": "COM3"
}
```

---

## Integration Examples

### Receipt Printing (JavaScript/Fetch)

```javascript
// Check receipt printer status
const status = await fetch('http://localhost:8080/status');
const { connected } = await status.json();

if (connected) {
  await fetch('http://localhost:8080/print-template', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      data: {
        order_id: 'ORD-12345',
        timestamp: new Date().toISOString(),
        items: cart.items,
        subtotal: cart.subtotal,
        tax: cart.tax,
        total: cart.total,
        payment_method: 'Card'
      }
    })
  });
}
```

### Barcode Printing (JavaScript/Fetch)

```javascript
// Check barcode printer status
const { connected } = await fetch('http://localhost:8080/barcode/status').then(r => r.json());

if (connected) {
  // Single label
  await fetch('http://localhost:8080/print-barcode', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      barcode_data: product.barcode,
      barcode_type: 'EAN13',
      label_text: product.name,
      copies: 1
    })
  });
}
```

### cURL Examples

```bash
# Check receipt printer
curl http://localhost:8080/status

# Check barcode printer
curl http://localhost:8080/barcode/status

# Connect barcode printer (Aokia AK-3001 on COM5)
curl -X POST http://localhost:8080/barcode/connect \
  -H "Content-Type: application/json" \
  -d '{"connection_type":"USB","device_path":"COM5","protocol":"TSPL","label_width_mm":50,"label_height_mm":30,"dpi":203}'

# Print a barcode label
curl -X POST http://localhost:8080/print-barcode \
  -H "Content-Type: application/json" \
  -d '{"barcode_data":"123456789012","barcode_type":"CODE128","label_text":"My Product","copies":1}'

# Print receipt
curl -X POST http://localhost:8080/print-template \
  -H "Content-Type: application/json" \
  -d '{"data":{"order_id":"TEST-001","timestamp":"2026-01-30","items":[],"subtotal":0,"tax":0,"total":0,"payment_method":"Cash"}}'
```

---

## Troubleshooting

### Printer Not Detected
- Ensure printer is powered on and connected
- Check USB cable or network connection
- Try running as Administrator (for USB access)

### Print Fails
- Verify printer is in "ready" state (green LED)
- Check paper roll is installed correctly
- Ensure correct connection type is selected

### HTTP Server Not Accessible
- Check Windows Firewall allows port 8080
- Verify no other application is using port 8080
- Try `netstat -an | findstr 8080`

---

## Technology Stack

- **UI**: [Slint](https://slint.dev) - Native cross-platform UI
- **HTTP**: [Axum](https://github.com/tokio-rs/axum) - Async web framework
- **Runtime**: [Tokio](https://tokio.rs) - Async runtime
- **Printer**: [escpos](https://crates.io/crates/escpos) - ESC/POS commands

---

## License

MIT License - Nexora Team
