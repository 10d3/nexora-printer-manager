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
            "CODE128" | "128"                   => Ok(BarcodeType::Code128),
            "CODE39"  | "39"                    => Ok(BarcodeType::Code39),
            "EAN13"   | "EAN-13"                => Ok(BarcodeType::Ean13),
            "EAN8"    | "EAN-8"                 => Ok(BarcodeType::Ean8),
            "UPCA"    | "UPC-A" | "UPC"         => Ok(BarcodeType::Upca),
            "QR"      | "QRCODE" | "QR_CODE"    => Ok(BarcodeType::Qr),
            // Default to Code128 for unknown types
            _                                   => Ok(BarcodeType::Code128),
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
        _     => build_tspl(config, req),
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
        _     => build_test_label_tspl(config),
    }
}

// ---------------------------------------------------------------------------
// Dynamic layout engine
// ---------------------------------------------------------------------------

/// Convert millimetres to dots at the given DPI.
fn mm_to_dots(mm: u32, dpi: u32) -> u32 {
    // Use f64 internally to avoid integer rounding errors.
    ((mm as f64) * (dpi as f64) / 25.4).round() as u32
}

/// Estimate the number of symbol modules (unit-wide columns) for a barcode.
///
/// These are conservative upper-bound estimates used to pick a narrow-bar
/// width that guarantees the symbol fits within the printable area.
fn estimate_modules(barcode_type: &BarcodeType, data_len: usize) -> u32 {
    let n = data_len as u32;
    match barcode_type {
        // CODE128: START(11) + ceil(N/2) code words × 11 + CHECK(11) + STOP(13) + 2×quiet(10)
        BarcodeType::Code128 => 11 * ((n + 1) / 2) + 55,
        // CODE39: each char = 10 modules, plus inter-char gap, plus start/stop + quiet
        BarcodeType::Code39  => 10 * (n + 2) + (n + 1) + 20,
        // Fixed-width symbologies (quiet zones included)
        BarcodeType::Ean13   => 113,
        BarcodeType::Ean8    => 81,
        BarcodeType::Upca    => 113,
        // QR is handled separately; modules are square — return 0 sentinel.
        BarcodeType::Qr      => 0,
    }
}

/// All dot-unit coordinates needed to lay out a label.
///
/// Computed once from the label's physical dimensions and DPI, then used by
/// all three protocol builders so no magic numbers appear in command strings.
struct LabelLayout {
    // ── label canvas ──────────────────────────────────────────────────────────
    /// Total printable width in dots (after margins).
    printable_w: u32,
    /// Total printable height in dots (after margins).
    printable_h: u32,
    // ── barcode position ─────────────────────────────────────────────────────
    /// Left edge of barcode / QR, in dots from label origin.
    barcode_x: u32,
    /// Top edge of barcode / QR, in dots from label origin.
    barcode_y: u32,
    /// Height of the barcode bars (or QR cell count) in dots.
    barcode_h: u32,
    /// Narrow bar width in dots (≥ 1).
    narrow: u32,
    /// Wide bar width in dots (= narrow × 2, ≥ 2).
    wide: u32,
    /// QR cell size in dots (only meaningful for QR barcodes).
    qr_cell: u32,
    // ── text position ────────────────────────────────────────────────────────
    /// Left edge of the label-text line, in dots.
    text_x: u32,
    /// Top edge of the label-text line, in dots.
    text_y: u32,
    /// TSPL font ID ("2" or "3").
    tspl_font: &'static str,
    /// ZPL/EPL font height in dots.
    font_h: u32,
}

impl LabelLayout {
    /// Compute the layout for a label.
    ///
    /// # Arguments
    /// * `width_mm` / `height_mm` – physical label size
    /// * `dpi`                    – printer resolution
    /// * `barcode_type`           – determines module count formula
    /// * `data_len`               – number of characters in the barcode value
    /// * `has_text`               – whether a text line will be printed
    fn compute(
        width_mm: u32,
        height_mm: u32,
        dpi: u32,
        barcode_type: &BarcodeType,
        data_len: usize,
        has_text: bool,
    ) -> Self {
        let total_w = mm_to_dots(width_mm, dpi);
        let total_h = mm_to_dots(height_mm, dpi);

        // 3 % margin on each side, minimum 3 dots.
        let margin_x = ((total_w as f64 * 0.03).round() as u32).max(3);
        let margin_y = ((total_h as f64 * 0.03).round() as u32).max(3);

        let printable_w = total_w.saturating_sub(2 * margin_x);
        let printable_h = total_h.saturating_sub(2 * margin_y);

        // ── font metrics ──────────────────────────────────────────────────
        // Use smaller font on narrow labels (< 38 mm).
        let (tspl_font, font_h) = if printable_w > mm_to_dots(35, dpi) {
            ("3", 24_u32)
        } else {
            ("2", 16_u32)
        };

        let text_gap: u32 = 4; // dots between barcode bottom and text top

        // ── vertical split ────────────────────────────────────────────────
        let (barcode_h, text_y) = if has_text {
            let bh = printable_h
                .saturating_sub(font_h)
                .saturating_sub(text_gap)
                .max(20); // never shorter than 20 dots (barely readable)
            let ty = margin_y + bh + text_gap;
            (bh, ty)
        } else {
            (printable_h, 0) // text_y unused when has_text=false
        };

        // ── narrow bar width ─────────────────────────────────────────────
        let modules = estimate_modules(barcode_type, data_len);
        let narrow = if modules == 0 {
            1 // QR — irrelevant, handled via qr_cell
        } else {
            let n = printable_w / modules;
            n.max(1).min(3) // clamp 1–3; warn if 0
        };
        let wide = (narrow * 2).max(2);

        // ── QR cell size ──────────────────────────────────────────────────
        // Assume auto-version with ~29 modules per side for short data.
        let qr_area = printable_w.min(barcode_h);
        let qr_cell = (qr_area / 29).max(1).min(10);

        LabelLayout {
            printable_w,
            printable_h,
            barcode_x: margin_x,
            barcode_y: margin_y,
            barcode_h,
            narrow,
            wide,
            qr_cell,
            text_x: margin_x,
            text_y,
            tspl_font,
            font_h,
        }
    }
}

// ---------------------------------------------------------------------------
// TSPL builder
// ---------------------------------------------------------------------------

fn build_tspl(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies = req.copies.unwrap_or(1);
    let width  = req.label_width_mm.unwrap_or(config.label_width_mm);
    let height = req.label_height_mm.unwrap_or(config.label_height_mm);
    let has_text = req.label_text.is_some();

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(), has_text,
    );

    let mut cmds = String::new();

    // Label header
    cmds.push_str(&format!("SIZE {} mm, {} mm\r\n", width, height));
    cmds.push_str("GAP 2 mm, 0 mm\r\n");
    cmds.push_str("DIRECTION 0\r\n");
    cmds.push_str("CLS\r\n");

    // Barcode / QR
    match req.barcode_type {
        BarcodeType::Qr => {
            // QRCODE x, y, ECC, cell_width, mode, rotation, [options], "data"
            cmds.push_str(&format!(
                "QRCODE {},{},M,{},A,0,M2,S3,\"{}\"\r\n",
                l.barcode_x, l.barcode_y, l.qr_cell, req.barcode_data
            ));
        }
        _ => {
            let type_str = tspl_barcode_type(&req.barcode_type);
            // BARCODE x, y, "type", height, readable, rotation, narrow, wide, "data"
            // readable=0: we emit our own TEXT command below so text is always
            // positioned correctly relative to the computed layout.
            cmds.push_str(&format!(
                "BARCODE {},{},\"{}\",{},0,0,{},{},\"{}\"\r\n",
                l.barcode_x, l.barcode_y, type_str,
                l.barcode_h, l.narrow, l.wide,
                req.barcode_data
            ));
        }
    }

    // Optional label text — placed just below the barcode
    if let Some(ref text) = req.label_text {
        cmds.push_str(&format!(
            "TEXT {},{},\"{}\",0,1,1,\"{}\"\r\n",
            l.text_x, l.text_y, l.tspl_font, text
        ));
    }

    cmds.push_str(&format!("PRINT 1,{}\r\n", copies));
    cmds.into_bytes()
}

/// Map a [`BarcodeType`] to its TSPL barcode identifier string.
fn tspl_barcode_type(barcode_type: &BarcodeType) -> &'static str {
    match barcode_type {
        BarcodeType::Code128 => "128",
        BarcodeType::Code39  => "39",
        BarcodeType::Ean13   => "EAN13",
        BarcodeType::Ean8    => "EAN8",
        BarcodeType::Upca    => "UPCA",
        // QR is handled separately; this branch is unreachable in practice.
        BarcodeType::Qr      => "QRCODE",
    }
}

fn build_test_label_tspl(config: &BarcodePrinterConfig) -> Vec<u8> {
    // Use CODE128 with 12-char data as representative test barcode.
    let l = LabelLayout::compute(
        config.label_width_mm, config.label_height_mm, config.dpi,
        &BarcodeType::Code128, 12, true,
    );

    // Reserve the top ~20% of the barcode area for a title text line.
    let title_h = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);

    let mut cmds = String::new();
    cmds.push_str(&format!(
        "SIZE {} mm, {} mm\r\n",
        config.label_width_mm, config.label_height_mm
    ));
    cmds.push_str("GAP 2 mm, 0 mm\r\n");
    cmds.push_str("DIRECTION 0\r\n");
    cmds.push_str("CLS\r\n");
    cmds.push_str(&format!(
        "TEXT {},{},\"{}\",0,1,1,\"NEXORA BARCODE TEST\"\r\n",
        l.text_x, l.barcode_y, l.tspl_font
    ));
    cmds.push_str(&format!(
        "BARCODE {},{},\"128\",{},0,0,{},{},\"123456789012\"\r\n",
        l.barcode_x, barcode_y2, barcode_h2, l.narrow, l.wide
    ));
    cmds.push_str(&format!(
        "TEXT {},{},\"{}\",0,1,1,\"Test Label - OK\"\r\n",
        l.text_x, l.text_y, l.tspl_font
    ));
    cmds.push_str("PRINT 1,1\r\n");
    cmds.into_bytes()
}

// ---------------------------------------------------------------------------
// ZPL builder
// ---------------------------------------------------------------------------

fn build_zpl(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies  = req.copies.unwrap_or(1);
    let width   = req.label_width_mm.unwrap_or(config.label_width_mm);
    let height  = req.label_height_mm.unwrap_or(config.label_height_mm);
    let has_text = req.label_text.is_some();

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(), has_text,
    );

    let mut cmds = String::new();
    cmds.push_str("^XA\n");

    match req.barcode_type {
        BarcodeType::Qr => {
            // ^BQN,model,magnification  (magnification = qr_cell)
            cmds.push_str(&format!(
                "^FO{},{}^BQN,2,{}^FDMM,A{}^FS\n",
                l.barcode_x, l.barcode_y, l.qr_cell, req.barcode_data
            ));
        }
        _ => {
            // ^BY sets bar width; ^BCN sets barcode height; N = no human-readable
            // (we emit our own ^FO text below for correct placement)
            cmds.push_str(&format!(
                "^FO{},{}^BY{}^BCN,{},N,N,N^FD{}^FS\n",
                l.barcode_x, l.barcode_y,
                l.narrow, l.barcode_h,
                req.barcode_data
            ));
        }
    }

    // Optional label text
    if let Some(ref text) = req.label_text {
        let fh = l.font_h;
        cmds.push_str(&format!(
            "^FO{},{}^A0N,{},{}^FD{}^FS\n",
            l.text_x, l.text_y, fh, fh, text
        ));
    }

    cmds.push_str(&format!("^PQ{}\n", copies));
    cmds.push_str("^XZ\n");
    cmds.into_bytes()
}

fn build_test_label_zpl(config: &BarcodePrinterConfig) -> Vec<u8> {
    let l = LabelLayout::compute(
        config.label_width_mm, config.label_height_mm, config.dpi,
        &BarcodeType::Code128, 12, true,
    );

    let title_h  = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);
    let fh = l.font_h;

    let mut cmds = String::new();
    cmds.push_str("^XA\n");
    cmds.push_str(&format!(
        "^FO{},{}^A0N,{},{}^FDNEXORA BARCODE TEST^FS\n",
        l.text_x, l.barcode_y, fh, fh
    ));
    cmds.push_str(&format!(
        "^FO{},{}^BY{}^BCN,{},N,N,N^FD123456789012^FS\n",
        l.barcode_x, barcode_y2, l.narrow, barcode_h2
    ));
    cmds.push_str(&format!(
        "^FO{},{}^A0N,{},{}^FDTest Label - OK^FS\n",
        l.text_x, l.text_y, fh, fh
    ));
    cmds.push_str("^PQ1\n");
    cmds.push_str("^XZ\n");
    cmds.into_bytes()
}

// ---------------------------------------------------------------------------
// EPL builder
// ---------------------------------------------------------------------------

fn build_epl(config: &BarcodePrinterConfig, req: &BarcodeLabelRequest) -> Vec<u8> {
    let copies  = req.copies.unwrap_or(1);
    let width   = req.label_width_mm.unwrap_or(config.label_width_mm);
    let height  = req.label_height_mm.unwrap_or(config.label_height_mm);
    let has_text = req.label_text.is_some();

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(), has_text,
    );

    // EPL font ID: 1 (small) or 3 (medium) based on label width
    let epl_font: u32 = if l.tspl_font == "2" { 1 } else { 3 };

    let mut cmds = String::new();
    cmds.push_str("N\n");

    // B x, y, rotation, type_id, narrow, wide, height, human_readable, "data"
    // type_id 1 = CODE128, readable=N means no auto-text
    cmds.push_str(&format!(
        "B{},{},0,1,{},{},{},N,\"{}\"\n",
        l.barcode_x, l.barcode_y,
        l.narrow, l.wide, l.barcode_h,
        req.barcode_data
    ));

    // Optional label text
    if let Some(ref text) = req.label_text {
        // A x, y, rotation, font_id, h_mult, v_mult, reverse, "data"
        cmds.push_str(&format!(
            "A{},{},0,{},1,1,N,\"{}\"\n",
            l.text_x, l.text_y, epl_font, text
        ));
    }

    cmds.push_str(&format!("P{}\n", copies));
    cmds.into_bytes()
}

fn build_test_label_epl(config: &BarcodePrinterConfig) -> Vec<u8> {
    let l = LabelLayout::compute(
        config.label_width_mm, config.label_height_mm, config.dpi,
        &BarcodeType::Code128, 12, true,
    );

    let title_h  = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);
    let epl_font: u32 = if l.tspl_font == "2" { 1 } else { 3 };

    let mut cmds = String::new();
    cmds.push_str("N\n");
    cmds.push_str(&format!(
        "A{},{},0,{},1,1,N,\"NEXORA BARCODE TEST\"\n",
        l.text_x, l.barcode_y, epl_font
    ));
    cmds.push_str(&format!(
        "B{},{},0,1,{},{},{},N,\"123456789012\"\n",
        l.barcode_x, barcode_y2, l.narrow, l.wide, barcode_h2
    ));
    cmds.push_str(&format!(
        "A{},{},0,{},1,1,N,\"Test Label - OK\"\n",
        l.text_x, l.text_y, epl_font
    ));
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

    fn small_config(protocol: &str) -> BarcodePrinterConfig {
        BarcodePrinterConfig {
            connection_type: "USB".to_string(),
            device_path: "/dev/usb/lp0".to_string(),
            protocol: protocol.to_string(),
            label_width_mm: 32,
            label_height_mm: 25,
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

    // ── helpers ──────────────────────────────────────────────────────────────

    #[test]
    fn test_mm_to_dots_203dpi() {
        // 25.4 mm = exactly 203 dots at 203 DPI
        // 25 mm × (203/25.4) = 199.8 → rounds to 200
        assert_eq!(mm_to_dots(25, 203), 200);
        assert_eq!(mm_to_dots(32, 203), 256);
        assert_eq!(mm_to_dots(50, 203), 400);
    }

    #[test]
    fn test_estimate_modules_ean13_is_fixed() {
        assert_eq!(estimate_modules(&BarcodeType::Ean13, 13), 113);
        assert_eq!(estimate_modules(&BarcodeType::Ean8,  8),  81);
        assert_eq!(estimate_modules(&BarcodeType::Upca,  12), 113);
    }

    #[test]
    fn test_estimate_modules_code128_scales_with_data() {
        let m6  = estimate_modules(&BarcodeType::Code128, 6);
        let m12 = estimate_modules(&BarcodeType::Code128, 12);
        assert!(m12 > m6, "longer data → more modules");
    }

    // ── LabelLayout ──────────────────────────────────────────────────────────

    #[test]
    fn test_layout_32x25_narrow_is_1() {
        // On a 32×25 mm label the bar must narrow to 1 dot to fit CODE128.
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, true);
        assert_eq!(l.narrow, 1, "narrow bar must be 1 dot on 32mm label");
        assert_eq!(l.wide, 2);
    }

    #[test]
    fn test_layout_32x25_barcode_fits_width() {
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, true);
        let modules = estimate_modules(&BarcodeType::Code128, 12);
        let barcode_w = modules * l.narrow;
        assert!(
            barcode_w <= l.printable_w,
            "barcode ({} dots) must fit in printable width ({} dots)",
            barcode_w, l.printable_w
        );
    }

    #[test]
    fn test_layout_32x25_text_fits_height() {
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, true);
        let total_h = mm_to_dots(25, 203); // 200 dots
        let text_bottom = l.text_y + l.font_h;
        assert!(
            text_bottom <= total_h,
            "text bottom ({} dots) must fit in label height ({} dots)",
            text_bottom, total_h
        );
    }

    #[test]
    fn test_layout_no_text_uses_full_height() {
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, false);
        // Without text the barcode should get the full printable height.
        let expected = mm_to_dots(25, 203).saturating_sub(2 * 6_u32.max(3));
        assert_eq!(l.barcode_h, expected.max(20));
    }

    #[test]
    fn test_layout_50x30_wider_narrow() {
        // 50 mm gives more room → narrow should be > 1
        let l = LabelLayout::compute(50, 30, 203, &BarcodeType::Code128, 12, false);
        assert!(l.narrow >= 2, "wider label should allow narrow ≥ 2");
    }

    // ── BarcodeType::from_str ─────────────────────────────────────────────────

    #[test]
    fn test_barcode_type_from_str_known() {
        assert!(matches!("CODE128".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
        assert!(matches!("128".parse::<BarcodeType>().unwrap(),     BarcodeType::Code128));
        assert!(matches!("code39".parse::<BarcodeType>().unwrap(),  BarcodeType::Code39));
        assert!(matches!("EAN13".parse::<BarcodeType>().unwrap(),   BarcodeType::Ean13));
        assert!(matches!("EAN-13".parse::<BarcodeType>().unwrap(),  BarcodeType::Ean13));
        assert!(matches!("EAN8".parse::<BarcodeType>().unwrap(),    BarcodeType::Ean8));
        assert!(matches!("UPCA".parse::<BarcodeType>().unwrap(),    BarcodeType::Upca));
        assert!(matches!("upc-a".parse::<BarcodeType>().unwrap(),   BarcodeType::Upca));
        assert!(matches!("QR".parse::<BarcodeType>().unwrap(),      BarcodeType::Qr));
        assert!(matches!("qrcode".parse::<BarcodeType>().unwrap(),  BarcodeType::Qr));
    }

    #[test]
    fn test_barcode_type_from_str_default() {
        assert!(matches!("UNKNOWN_TYPE".parse::<BarcodeType>().unwrap(), BarcodeType::Code128));
        assert!(matches!("".parse::<BarcodeType>().unwrap(),             BarcodeType::Code128));
    }

    // ── TSPL builder ─────────────────────────────────────────────────────────

    #[test]
    fn test_build_tspl_contains_size() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("SIZE 100 mm, 50 mm\r\n"));
    }

    #[test]
    fn test_build_tspl_size_override() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            label_width_mm: Some(70),
            label_height_mm: Some(40),
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(
            output.contains("SIZE 70 mm, 40 mm\r\n"),
            "Expected SIZE 70 mm, 40 mm but got: {}", output
        );
    }

    #[test]
    fn test_build_tspl_contains_barcode() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        // Must contain a BARCODE command with 128 and the data.
        assert!(output.contains("BARCODE "), "should emit BARCODE command");
        assert!(output.contains("\"128\""),  "should use CODE128 type string");
        assert!(output.contains("\"123456789012\""));
    }

    #[test]
    fn test_build_tspl_readable_is_zero() {
        // readable must be 0 so the printer doesn't auto-print text;
        // we control text placement via our own TEXT command.
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        // Format: BARCODE x,y,"type",h,readable,rotation,...
        // Check that the 5th field (readable) is 0, not 1.
        assert!(output.contains(",0,0,"), "readable and rotation must both be 0");
    }

    #[test]
    fn test_build_tspl_contains_label_text() {
        let config = test_config("TSPL");
        let req = test_request();
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("TEXT "),       "should emit TEXT command");
        assert!(output.contains("\"Test Item\""));
    }

    #[test]
    fn test_build_tspl_no_label_text_when_none() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest { label_text: None, ..test_request() };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(!output.contains("TEXT "), "should not emit TEXT when label_text is None");
    }

    #[test]
    fn test_build_tspl_qr_uses_qrcode_command() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest {
            barcode_type: BarcodeType::Qr,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("QRCODE "),     "QR should use QRCODE command");
        assert!(output.contains("\"123456789012\""));
        assert!(!output.contains("BARCODE "),   "QR must not emit BARCODE command");
    }

    #[test]
    fn test_build_tspl_print_command() {
        let config = test_config("TSPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("PRINT 1,1\r\n"));
    }

    #[test]
    fn test_build_tspl_print_copies() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest { copies: Some(3), ..test_request() };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("PRINT 1,3\r\n"));
    }

    #[test]
    fn test_build_tspl_default_copies() {
        let config = test_config("TSPL");
        let req = BarcodeLabelRequest { copies: None, ..test_request() };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("PRINT 1,1\r\n"));
    }

    // 32×25 mm integration test — the original bug report
    #[test]
    fn test_build_tspl_32x25_barcode_fits() {
        let config = small_config("TSPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("SIZE 32 mm, 25 mm\r\n"));
        // narrow=1 must appear in the BARCODE command
        assert!(output.contains(",1,2,\"123456789012\""),
            "Expected narrow=1 wide=2, got: {}", output);
    }

    // ── ZPL builder ───────────────────────────────────────────────────────────

    #[test]
    fn test_build_zpl_wraps_with_xa_xz() {
        let config = test_config("ZPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.starts_with("^XA\n"));
        assert!(output.ends_with("^XZ\n"));
    }

    #[test]
    fn test_build_zpl_contains_barcode() {
        let config = test_config("ZPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("^BCN,"),           "should use ^BC barcode command");
        assert!(output.contains("^FD123456789012^FS"));
    }

    #[test]
    fn test_build_zpl_qr_uses_bq_command() {
        let config = test_config("ZPL");
        let req = BarcodeLabelRequest {
            barcode_type: BarcodeType::Qr,
            ..test_request()
        };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^BQN,"),      "QR should use ^BQ command");
        assert!(output.contains("123456789012"));
        assert!(!output.contains("^BCN,"),     "QR must not emit ^BC command");
    }

    #[test]
    fn test_build_zpl_label_text() {
        let config = test_config("ZPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("^A0N,"),      "should emit ^A0 font command");
        assert!(output.contains("^FDTest Item^FS"));
    }

    #[test]
    fn test_build_zpl_copies() {
        let config = test_config("ZPL");
        let req = BarcodeLabelRequest { copies: Some(5), ..test_request() };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("^PQ5\n"));
    }

    // ── EPL builder ───────────────────────────────────────────────────────────

    #[test]
    fn test_build_epl_starts_with_n() {
        let config = test_config("EPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.starts_with("N\n"));
    }

    #[test]
    fn test_build_epl_contains_barcode() {
        let config = test_config("EPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("B"),               "should emit B barcode command");
        assert!(output.contains("\"123456789012\""));
    }

    #[test]
    fn test_build_epl_label_text() {
        let config = test_config("EPL");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.contains("A"),               "should emit A text command");
        assert!(output.contains("\"Test Item\""));
    }

    #[test]
    fn test_build_epl_print_command() {
        let config = test_config("EPL");
        let req = BarcodeLabelRequest { copies: Some(2), ..test_request() };
        let output = String::from_utf8(build_label(&config, &req)).unwrap();
        assert!(output.contains("P2\n"));
    }

    // ── Test label builders ───────────────────────────────────────────────────

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

    // ── Protocol dispatch ─────────────────────────────────────────────────────

    #[test]
    fn test_unknown_protocol_falls_back_to_tspl() {
        let config = test_config("UNKNOWN");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.starts_with("SIZE"), "unknown protocol must fall back to TSPL");
    }

    #[test]
    fn test_protocol_matching_is_case_insensitive() {
        let config = test_config("zpl");
        let output = String::from_utf8(build_label(&config, &test_request())).unwrap();
        assert!(output.starts_with("^XA\n"));
    }
}
