use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// BarcodeType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarcodeType {
    Code128,
    Code39,
    Ean13,
    Ean8,
    Upca,
    Qr,
}

impl FromStr for BarcodeType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CODE128" | "128" => Ok(BarcodeType::Code128),
            "CODE39" | "39" => Ok(BarcodeType::Code39),
            "EAN13" | "EAN-13" => Ok(BarcodeType::Ean13),
            "EAN8" | "EAN-8" => Ok(BarcodeType::Ean8),
            "UPCA" | "UPC-A" | "UPC" => Ok(BarcodeType::Upca),
            "QR" | "QRCODE" | "QR_CODE" => Ok(BarcodeType::Qr),
            // Default to Code128 for unknown types
            _ => Ok(BarcodeType::Code128),
        }
    }
}

// ---------------------------------------------------------------------------
// BarcodePrinterConfig
// ---------------------------------------------------------------------------

/// Configuration for a barcode printer connection and label dimensions.
///
/// `protocol` should be one of `"TSPL"`, `"ZPL"`, or `"EPL"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodePrinterConfig {
    pub connection_type: String,
    pub device_path: String,
    /// Printer command protocol: "TSPL", "ZPL", or "EPL"
    pub protocol: String,
    pub label_width_mm: u32,
    pub label_height_mm: u32,
    pub dpi: u32,
}

// ---------------------------------------------------------------------------
// BarcodeLabelRequest
// ---------------------------------------------------------------------------

/// A request to print a single barcode label.
#[derive(Debug, Clone)]
pub struct BarcodeLabelRequest {
    pub barcode_data: String,
    pub barcode_type: BarcodeType,
    /// Optional human-readable text printed below the barcode.
    pub label_text: Option<String>,
    /// Number of copies to print. Defaults to 1 when `None`.
    pub copies: Option<u32>,
    /// Override the configured label width for this job only (mm).
    /// Falls back to `BarcodePrinterConfig::label_width_mm` when `None`.
    pub label_width_mm: Option<u32>,
    /// Override the configured label height for this job only (mm).
    /// Falls back to `BarcodePrinterConfig::label_height_mm` when `None`.
    pub label_height_mm: Option<u32>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a raw label byte payload for the given protocol.
///
/// Dispatches to [`build_tspl`], [`build_zpl`], or [`build_epl`] based on
/// `config.protocol`. Unknown protocols fall back to TSPL.
pub fn build_label(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    match config.protocol.to_uppercase().as_str() {
        "ZPL" => build_zpl(config, req),
        "EPL" => build_epl(config, req),
        _ => build_tspl(config, req),
    }
}

/// Build a test label byte payload for the given protocol.
///
/// Useful for verifying printer connectivity and alignment without requiring
/// real barcode data.
pub fn build_test_label(config: &BarcodePrinterConfig) -> Vec<u8> {
    match config.protocol.to_uppercase().as_str() {
        "ZPL" => build_test_label_zpl(config),
        "EPL" => build_test_label_epl(config),
        _ => build_test_label_tspl(config),
    }
}

// ---------------------------------------------------------------------------
// TSPL builder
// ---------------------------------------------------------------------------

fn build_tspl(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies = req.copies.unwrap_or(1);
    let width  = req.label_width_mm.unwrap_or(config.label_width_mm);
    let height = req.label_height_mm.unwrap_or(config.label_height_mm);
    let mut cmds = String::new();

    // Label setup
    cmds.push_str(&format!(
        "SIZE {} mm, {} mm\r\n",
        width, height
    ));
    cmds.push_str("GAP 2 mm, 0 mm\r\n");
    cmds.push_str("DIRECTION 0\r\n");
    cmds.push_str("CLS\r\n");

    // Barcode command — QR uses a different TSPL instruction
    match req.barcode_type {
        BarcodeType::Qr => {
            cmds.push_str(&format!(
                "QRCODE 10,5,M,4,A,0,M2,S3,\"{}\"\r\n",
                req.barcode_data
            ));
        }
        _ => {
            let type_str = tspl_barcode_type(&req.barcode_type);
            cmds.push_str(&format!(
                "BARCODE 10,5,\"{}\",60,1,0,2,2,\"{}\"\r\n",
                type_str, req.barcode_data
            ));
        }
    }

    // Optional human-readable text
    if let Some(ref text) = req.label_text {
        cmds.push_str(&format!("TEXT 10,80,\"3\",0,1,1,\"{}\"\r\n", text));
    }

    // Print command
    cmds.push_str(&format!("PRINT 1,{}\r\n", copies));

    cmds.into_bytes()
}

/// Map a [`BarcodeType`] to its TSPL barcode identifier string.
fn tspl_barcode_type(barcode_type: &BarcodeType) -> &'static str {
    match barcode_type {
        BarcodeType::Code128 => "128",
        BarcodeType::Code39 => "39",
        BarcodeType::Ean13 => "EAN13",
        BarcodeType::Ean8 => "EAN8",
        BarcodeType::Upca => "UPCA",
        // QR is handled separately; this branch is unreachable in practice.
        BarcodeType::Qr => "QRCODE",
    }
}

fn build_test_label_tspl(config: &BarcodePrinterConfig) -> Vec<u8> {
    let mut cmds = String::new();

    cmds.push_str(&format!(
        "SIZE {} mm, {} mm\r\n",
        config.label_width_mm, config.label_height_mm
    ));
    cmds.push_str("GAP 2 mm, 0 mm\r\n");
    cmds.push_str("DIRECTION 0\r\n");
    cmds.push_str("CLS\r\n");
    cmds.push_str("TEXT 10,5,\"3\",0,1,1,\"NEXORA BARCODE TEST\"\r\n");
    cmds.push_str("BARCODE 10,30,\"128\",60,1,0,2,2,\"123456789012\"\r\n");
    cmds.push_str("TEXT 10,110,\"3\",0,1,1,\"Test Label - OK\"\r\n");
    cmds.push_str("PRINT 1,1\r\n");

    cmds.into_bytes()
}

// ---------------------------------------------------------------------------
// ZPL builder
// ---------------------------------------------------------------------------

fn build_zpl(_config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies = req.copies.unwrap_or(1);
    let mut cmds = String::new();

    cmds.push_str("^XA\n");

    // Barcode field — QR uses a different ZPL command
    match req.barcode_type {
        BarcodeType::Qr => {
            cmds.push_str(&format!(
                "^FO50,30^BQN,2,4^FDMM,A{}^FS\n",
                req.barcode_data
            ));
        }
        _ => {
            cmds.push_str(&format!(
                "^FO50,30^BY2^BCN,60,Y,N,N^FD{}^FS\n",
                req.barcode_data
            ));
        }
    }

    // Optional human-readable text
    if let Some(ref text) = req.label_text {
        cmds.push_str(&format!("^FO50,100^A0N,20,20^FD{}^FS\n", text));
    }

    // ZPL quantity is set via the print quantity field in ^PQ
    cmds.push_str(&format!("^PQ{}\n", copies));
    cmds.push_str("^XZ\n");

    cmds.into_bytes()
}

fn build_test_label_zpl(_config: &BarcodePrinterConfig) -> Vec<u8> {
    let mut cmds = String::new();

    cmds.push_str("^XA\n");
    cmds.push_str("^FO50,10^A0N,25,25^FDNEXORA BARCODE TEST^FS\n");
    cmds.push_str("^FO50,40^BY2^BCN,60,Y,N,N^FD123456789012^FS\n");
    cmds.push_str("^FO50,120^A0N,20,20^FDTest Label - OK^FS\n");
    cmds.push_str("^PQ1\n");
    cmds.push_str("^XZ\n");

    cmds.into_bytes()
}

// ---------------------------------------------------------------------------
// EPL builder
// ---------------------------------------------------------------------------

fn build_epl(_config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies = req.copies.unwrap_or(1);
    let mut cmds = String::new();

    cmds.push_str("N\n");
    cmds.push_str(&format!(
        "B10,10,0,1,2,5,60,N,\"{}\"\n",
        req.barcode_data
    ));

    // Optional human-readable text
    if let Some(ref text) = req.label_text {
        cmds.push_str(&format!("A10,80,0,3,1,1,N,\"{}\"\n", text));
    }

    cmds.push_str(&format!("P{}\n", copies));

    cmds.into_bytes()
}

fn build_test_label_epl(_config: &BarcodePrinterConfig) -> Vec<u8> {
    let mut cmds = String::new();

    cmds.push_str("N\n");
    cmds.push_str("A10,10,0,3,1,1,N,\"NEXORA BARCODE TEST\"\n");
    cmds.push_str("B10,35,0,1,2,5,60,N,\"123456789012\"\n");
    cmds.push_str("A10,110,0,3,1,1,N,\"Test Label - OK\"\n");
    cmds.push_str("P1\n");

    cmds.into_bytes()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(protocol: &str) -> BarcodePrinterConfig {
        BarcodePrinterConfig {
            connection_type: "USB".to_string(),
            device_path: "/dev/usb/lp0".to_string(),
            protocol: protocol.to_string(),
            label_width_mm: 100,
            label_height_mm: 50,
            dpi: 203,
        }
    }

    fn test_request() -> BarcodeLabelRequest {
        BarcodeLabelRequest {
            barcode_data: "123456789012".to_string(),
            barcode_type: BarcodeType::Code128,
            label_text: Some("Test Item".to_string()),
            copies: Some(1),
            label_width_mm: None,
            label_height_mm: None,
        }
    }

    // ---- BarcodeType::from_str ----

    #[test]
    fn test_barcode_type_from_str_known() {
        assert!(matches!("CODE128".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
        assert!(matches!("128".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
        assert!(matches!("code39".parse::<BarcodeType>().unwrap(), BarcodeType::Code39));
        assert!(matches!("EAN13".parse::<BarcodeType>().unwrap(), BarcodeType::Ean13));
        assert!(matches!("EAN-13".parse::<BarcodeType>().unwrap(), BarcodeType::Ean13));
        assert!(matches!("EAN8".parse::<BarcodeType>().unwrap(), BarcodeType::Ean8));
        assert!(matches!("UPCA".parse::<BarcodeType>().unwrap(), BarcodeType::Upca));
        assert!(matches!("upc-a".parse::<BarcodeType>().unwrap(), BarcodeType::Upca));
        assert!(matches!("QR".parse::<BarcodeType>().unwrap(), BarcodeType::Qr));
        assert!(matches!("qrcode".parse::<BarcodeType>().unwrap(), BarcodeType::Qr));
    }

    #[test]
    fn test_barcode_type_from_str_default() {
        // Unknown strings should default to Code128
        assert!(matches!("UNKNOWN_TYPE".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
        assert!(matches!("".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
    }

    // ---- TSPL builder ----

    #[test]
    fn test_build_tspl_contains_size() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("SIZE 100 mm, 50 mm\r\n"));
    }

    #[test]
    fn test_build_tspl_size_override() {
        // Per-job size should override the config values
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            label_width_mm: Some(70),
            label_height_mm: Some(40),
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("SIZE 70 mm, 40 mm\r\n"),
            "Expected SIZE 70 mm, 40 mm but got: {}", output);
    }

    #[test]
    fn test_build_tspl_contains_barcode() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("BARCODE 10,5,\"128\",60,1,0,2,2,\"123456789012\""));
    }

    #[test]
    fn test_build_tspl_contains_label_text() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("TEXT 10,80,\"3\",0,1,1,\"Test Item\""));
    }

    #[test]
    fn test_build_tspl_no_label_text_when_none() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            label_text: None,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(!output.contains("TEXT 10,80"));
    }

    #[test]
    fn test_build_tspl_qr_uses_qrcode_command() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            barcode_type: BarcodeType::Qr,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("QRCODE 10,5,M,4,A,0,M2,S3,\"123456789012\""));
        assert!(!output.contains("BARCODE"));
    }

    #[test]
    fn test_build_tspl_print_command() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("PRINT 1,1\r\n"));
    }

    #[test]
    fn test_build_tspl_print_copies() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            copies: Some(3),
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("PRINT 1,3\r\n"));
    }

    #[test]
    fn test_build_tspl_default_copies() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            copies: None,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("PRINT 1,1\r\n"));
    }

    // ---- ZPL builder ----

    #[test]
    fn test_build_zpl_wraps_with_xa_xz() {
        let config = test_config("ZPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.starts_with("^XA\n"));
        assert!(output.ends_with("^XZ\n"));
    }

    #[test]
    fn test_build_zpl_contains_barcode() {
        let config = test_config("ZPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^FO50,30^BY2^BCN,60,Y,N,N^FD123456789012^FS"));
    }

    #[test]
    fn test_build_zpl_qr_uses_bq_command() {
        let config = test_config("ZPL");
        let req = BarcodeLabelRequest {
            barcode_type: BarcodeType::Qr,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^FO50,30^BQN,2,4^FDMM,A123456789012^FS"));
        assert!(!output.contains("^BCN"));
    }

    #[test]
    fn test_build_zpl_label_text() {
        let config = test_config("ZPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^FO50,100^A0N,20,20^FDTest Item^FS"));
    }

    #[test]
    fn test_build_zpl_copies() {
        let config = test_config("ZPL");
        let req = BarcodeLabelRequest {
            copies: Some(5),
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^PQ5\n"));
    }

    // ---- EPL builder ----

    #[test]
    fn test_build_epl_starts_with_n() {
        let config = test_config("EPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.starts_with("N\n"));
    }

    #[test]
    fn test_build_epl_contains_barcode() {
        let config = test_config("EPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("B10,10,0,1,2,5,60,N,\"123456789012\""));
    }

    #[test]
    fn test_build_epl_label_text() {
        let config = test_config("EPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("A10,80,0,3,1,1,N,\"Test Item\""));
    }

    #[test]
    fn test_build_epl_print_command() {
        let config = test_config("EPL");
        let req = BarcodeLabelRequest {
            copies: Some(2),
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("P2\n"));
    }

    // ---- Test label builders ----

    #[test]
    fn test_build_test_label_tspl() {
        let config = test_config("TSPL");
        let output = String::from_utf8(build_test_label(&config)).unwrap();
        assert!(output.contains("NEXORA BARCODE TEST"));
        assert!(output.contains("Test Label - OK"));
        assert!(output.contains("PRINT 1,1\r\n"));
    }

    #[test]
    fn test_build_test_label_zpl() {
        let config = test_config("ZPL");
        let output = String::from_utf8(build_test_label(&config)).unwrap();
        assert!(output.contains("NEXORA BARCODE TEST"));
        assert!(output.contains("Test Label - OK"));
        assert!(output.starts_with("^XA\n"));
        assert!(output.ends_with("^XZ\n"));
    }

    #[test]
    fn test_build_test_label_epl() {
        let config = test_config("EPL");
        let output = String::from_utf8(build_test_label(&config)).unwrap();
        assert!(output.contains("NEXORA BARCODE TEST"));
        assert!(output.contains("Test Label - OK"));
        assert!(output.starts_with("N\n"));
    }

    // ---- Protocol dispatch ----

    #[test]
    fn test_unknown_protocol_falls_back_to_tspl() {
        let config = test_config("UNKNOWN");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        // TSPL output starts with SIZE
        assert!(output.starts_with("SIZE"));
    }

    #[test]
    fn test_protocol_matching_is_case_insensitive() {
        let config = test_config("zpl");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.starts_with("^XA\n"));
    }
}
