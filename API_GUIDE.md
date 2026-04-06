# Nexora Printer Manager API Guide
## v1.5.0

This guide explains how to integrate your POS or web application with the Nexora Printer Manager HTTP API.

### **Base URL**
The server runs locally on: `http://127.0.0.1:8080`

---

### **1. Basic Printing (Legacy)**
Use this endpoint for simple receipts without complex layouts. It uses a fixed internal template.

- **Endpoint**: `POST /print`
- **Content-Type**: `application/json`
- **Payload**:
```json
{
  "order_id": "ORD-123",
  "timestamp": "2026-03-28 14:30:00",
  "items": [
    { "name": "Espresso", "quantity": 2, "price": 3.50 },
    { "name": "Croissant", "quantity": 1, "price": 4.00 }
  ],
  "subtotal": 11.00,
  "tax": 0.88,
  "total": 11.88,
  "payment_method": "Credit Card"
}
```

---

### **2. Template Management**
You can "cache" templates on the server so you don't have to send the full layout with every print job.

#### **Set/Cache a Template**
- **Endpoint**: `POST /template`
- **Payload**:
```json
{
  "template": {
    "id": "my-custom-receipt",
    "name": "Custom Receipt",
    "version": "1.0.0",
    "paper_width": 48,
    "layout": {
      "sections": [
        {
          "type": "header",
          "elements": [
            { "type": "text", "content": "MY STORE", "align": "center", "font_size": 2, "bold": true }
          ]
        },
        {
          "type": "items",
          "elements": [
            { "type": "table", "data_source": "items", "columns": [
                { "field": "name", "width": 30, "align": "left" },
                { "field": "total", "width": 12, "align": "right", "format": "currency" }
            ]}
          ]
        }
      ]
    }
  }
}
```

---

### **3. Professional Template Printing**
This is the recommended way to print. You can refer to a cached template ID or send an inline template.

- **Endpoint**: `POST /print-template`
- **Payload (using Cached Template)**:
```json
{
  "template_id": "my-custom-receipt",
  "data": {
    "order_id": "ORD-456",
    "items": [
      { "name": "Latte", "quantity": 1, "price": 4.50, "total": 4.50 }
    ],
    "total": 4.50,
    "payment_method": "Cash"
  }
}
```

- **Payload (using Inline Template)**:
Send both `template` and `data` in the same request.
```json
{
  "template": { ... template definition ... },
  "data": { ... receipt data ... }
}
```

---

### **4. Preview Template (No Printer Required)**
Useful for debugging your layouts. It returns the generated ESC/POS commands and a text-based preview.

- **Endpoint**: `POST /preview-template`
- **Payload**: Same as `print-template` (requires both `template` and `data`).

---

### **5. Logo Caching (Fast Printing)**
Cache logos on the server to avoid re-encoding large images with every print request. Cached logos are stored both in-memory and persisted to disk at `./cache/logos/`.

#### **Cache a Logo Explicitly**
- **Endpoint**: `POST /cache-logo`
- **Payload**:
```json
{
  "id": "company-logo",  // Optional: user-friendly ID. If omitted, auto-generated from content hash
  "base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
}
```
- **Response**:
```json
{
  "id": "company-logo",
  "content_hash": "a1b2c3d4e5f6...",
  "cached": true,
  "file_path": "./cache/logos/company-logo.b64"
}
```
- **Note**: If the same image is cached twice, the second request returns `"cached": false` (reused existing).

#### **List All Cached Logos**
- **Endpoint**: `GET /logos`
- **Response**:
```json
{
  "logos": [
    {
      "id": "company-logo",
      "content_hash": "a1b2c3d4...",
      "metadata": {
        "file_size_bytes": 2048,
        "original_width": 200,
        "original_height": 100,
        "usage_count": 5,
        "cached_dimensions": null
      },
      "created_at": "2026-04-06T10:30:00Z",
      "last_used": "2026-04-06T14:15:00Z"
    }
  ]
}
```

#### **Delete a Cached Logo**
- **Endpoint**: `DELETE /logos/{id}`
- **Example**: `DELETE /logos/company-logo`
- **Response**:
```json
{
  "success": true,
  "message": "Logo deleted: company-logo"
}
```

#### **Auto-Caching in Templates**
When you POST a template with inline base64 logos to `/template`, they are automatically cached:
- **Endpoint**: `POST /template`
- **Behavior**:
  1. System scans the template for all logo elements
  2. Inline base64 images are cached with auto-generated hash-based IDs
  3. Template is updated to reference the cached logo IDs
  4. Response includes count of auto-cached logos

#### **Using Cached Logos in Templates**
There are three ways to reference a cached logo:

**Method 1: Explicit `logo_id` field**
```json
{
  "type": "logo",
  "logo_id": "company-logo",
  "align": "center",
  "max_width": 288
}
```

**Method 2: Simple source string (smart fallback)**
```json
{
  "type": "logo",
  "source": "company-logo",  // If not base64-like, system checks cache first
  "align": "center"
}
```

**Method 3: Inline base64 (backward compatible)**
```json
{
  "type": "logo",
  "source": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
  "align": "center"
}
```

---

### **6. Status & Health**
- **Check Health**: `GET /health` (Returns `{"status": "healthy"}`)
- **Check Printer Status**: `GET /status`
  - Returns connection status, active template ID, cached template count, and logo cache statistics.
  - Response includes `logo_cache_info` with `count`, `total_size_bytes`, and `disk_usage_bytes`.

---

### **7. Cache Management**
- **Clear Template Cache**: `DELETE /cache`
- **Clear Template & Logo Cache**: `DELETE /cache?include_logos=true`

---

### **8. Converting TypeScript Templates to JSON**
If you are using the templates defined in `pro-template.ts`, you need to convert them to valid JSON before sending them to the `/template` or `/print-template` endpoints.

#### **Key Differences**
1. **Property Names**: In JSON, all keys MUST be in double quotes (e.g., `"id"` instead of `id`).
2. **Trailing Commas**: JSON does not allow trailing commas after the last property in an object or the last item in an array.
3. **Strings**: Use double quotes (`"`) only. JSON does not support single quotes (`'`) or backticks (`` ` ``).
4. **Variables**: Ensure all `{{variable}}` placeholders match the keys in your `data` object.

#### **Example: Converting "luxury-spa"**
**TypeScript Source:**
```typescript
"luxury-spa": {
    id: "luxury-spa",
    name: "Luxury Spa & Wellness",
    paper_width: 48,
    layout: {
        sections: [
            {
                type: "header",
                elements: [
                    { type: "text", content: "WELLNESS & BEAUTY", align: "center", font_size: 1 },
                    { type: "divider", style: "gradient" }
                ]
            }
        ]
    }
}
```

**JSON Payload (Ready for API):**
```json
{
  "template": {
    "id": "luxury-spa",
    "name": "Luxury Spa & Wellness",
    "version": "1.0.0",
    "paper_width": 48,
    "layout": {
      "sections": [
        {
          "type": "header",
          "elements": [
            { "type": "text", "content": "WELLNESS & BEAUTY", "align": "center", "font_size": 1 },
            { "type": "divider", "style": "gradient" }
          ]
        }
      ]
    }
  }
}
```

---

### **9. Template Element Reference**
Below are the supported elements and their main properties:

| Element | Properties | Notes |
| :--- | :--- | :--- |
| **`text`** | `content`, `align` (left/center/right), `font_size` (1-8), `bold`, `italic`, `invert` | Use `{{var}}` for dynamic content. |
| **`divider`** | `style` (solid/dashed/thin/gradient), `character`, `thickness` | `gradient` uses ASCII shading. |
| **`row`** | `left`, `right`, `center`, `bold`, `font_size` | Perfect for key-value pairs like `Total: $10.00`. |
| **`table`** | `data_source`, `columns` (field, width, align, format), `show_header` | `format: "currency"` adds `$` automatically. |
| **`box`** | `elements`, `style` (filled/shaded/bordered), `padding`, `border` | Use `style: "filled"` for solid black bars. |
| **`grid`** | `columns`, `data` (label, value), `gap` | Two-column layout for info blocks. |
| **`qr`** | `content`, `size`, `align` | Generates a QR code from content. |
| **`barcode`** | `content`, `format` (CODE39/EAN13), `height`, `width` | Standard linear barcodes. |
| **`space`** | `lines` | Adds empty lines (vertical spacing). |
| **`bar_chart`** | `data_source`, `value_field`, `height` | Renders a horizontal bar chart. |

---

### **Technical Tips**
1. **Safety Margins**: The renderer automatically applies a 6-character safety margin to prevent text wrapping on physical printers.
2. **Reverse Mode**: For charts or headers, set `style: "filled"` or `style: "shaded"` in a `box` element to get a solid black background.
3. **Data Types**: 
   - `currency` format in tables automatically adds the `$` sign and fixes to 2 decimal places.
   - Use `{{variable_name}}` in text or rows to inject data from your JSON payload.
