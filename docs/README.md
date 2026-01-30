# Nexora Printer Manager

A professional ESC/POS thermal printer management application for Windows. Designed to integrate with the Nexora POS web application for seamless receipt printing.

## Overview

Nexora Printer Manager is a desktop application that:
- Connects to ESC/POS compatible thermal printers (USB, Network, LPT)
- Runs an HTTP server on port 8080 for integration with web applications
- Supports template-based receipt printing via JSON API
- Provides a graphical interface for printer setup and testing

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Nexora Printer Manager                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Slint UI      │    │       HTTP Server (Axum)        │ │
│  │                 │    │                                 │ │
│  │  - Connection   │    │  localhost:8080                 │ │
│  │  - Device Scan  │◄──►│  - /health                      │ │
│  │  - Test Print   │    │  - /status                      │ │
│  └────────┬────────┘    │  - /template (POST)             │ │
│           │             │  - /templates (GET)             │ │
│           ▼             │  - /print-template (POST)       │ │
│  ┌─────────────────┐    │  - /test-print (POST)           │ │
│  │ PrinterManager  │◄──►│  - /cache (DELETE)              │ │
│  │                 │    └─────────────────────────────────┘ │
│  │  - connect()    │                                        │
│  │  - print_test() │                                        │
│  │  - templates    │                                        │
│  └────────┬────────┘                                        │
│           │                                                 │
│           ▼                                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Printer Connection Layer                   ││
│  │  USB (Serial) │ Network (TCP/IP) │ LPT (Parallel)       ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
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

### Tested Models
- Epson TM-T88 Series (TM-T88III, TM-T88IV, TM-T88V, TM-T88VI)
- Star TSP143, TSP654
- Bixolon SRP-350

### Connection Types

| Type | Description | Example |
|------|-------------|---------|
| **USB** | Serial over USB | COM3, /dev/ttyUSB0 |
| **Network** | TCP/IP Ethernet | 192.168.1.100:9100 |
| **LPT** | Parallel port (Windows) | LPT1 |

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

## Integration Example

### JavaScript/Fetch

```javascript
// Check printer status
const status = await fetch('http://localhost:8080/status');
const { connected } = await status.json();

if (connected) {
  // Print receipt
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

### cURL

```bash
# Check health
curl http://localhost:8080/health

# Print with data
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
