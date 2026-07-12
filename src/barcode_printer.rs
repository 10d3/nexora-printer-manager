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

struct TextLine {
    text: String,
    x: u32,
    y: u32,
}

/// All dot-unit coordinates needed to lay out a label.
///
/// Computed once from the label's physical dimensions and DPI, then used by
/// all three protocol builders so no magic numbers appear in command strings.
struct LabelLayout {
    // ── label canvas ──────────────────────────────────────────────────────────
    /// Total label width in dots (used by ZPL ^FB centering).
    total_w: u32,
    /// Usable width inside margins in dots.
    printable_w: u32,
    /// Usable height inside margins in dots.
    printable_h: u32,
    // ── barcode position ─────────────────────────────────────────────────────
    /// Left edge of the barcode, horizontally centred in the printable area.
    barcode_x: u32,
    /// Top edge of the barcode, part of the vertically centred content block.
    barcode_y: u32,
    /// Height of the barcode bars in dots (≤ 55 % of printable height).
    barcode_h: u32,
    /// Narrow bar width in dots (≥ 1).
    narrow: u32,
    /// Wide bar width in dots (= narrow × 2, ≥ 2).
    wide: u32,
    /// QR cell size in dots.
    qr_cell: u32,
    // ── text position ────────────────────────────────────────────────────────
    /// Computed text lines (auto-shrunk and/or wrapped), horizontally centred.
    text_lines: Vec<TextLine>,
    /// TSPL font ID ("1", "2" or "3").
    tspl_font: &'static str,
    /// EPL font ID (1, 3 or 4).
    epl_font: u32,
    /// ZPL / EPL font height in dots.
    font_h: u32,
}

struct FontMetrics {
    tspl: &'static str,
    epl: u32,
    h: u32,
    char_w: u32,
}

const FONTS: [FontMetrics; 3] = [
    FontMetrics { tspl: "3", epl: 4, h: 24, char_w: 16 }, // Large (TSPL Font 3 is 16x24)
    FontMetrics { tspl: "2", epl: 3, h: 20, char_w: 12 }, // Medium (TSPL Font 2 is 12x20)
    FontMetrics { tspl: "1", epl: 1, h: 12, char_w: 8 },  // Small (TSPL Font 1 is 8x12)
];

impl LabelLayout {
    /// Compute a centred layout for a label.
    ///
    /// # Arguments
    /// * `width_mm` / `height_mm` – physical label size
    /// * `dpi`                    – printer resolution
    /// * `barcode_type`           – determines module count formula
    /// * `data_len`               – number of characters in the barcode value
    /// * `text`                   – optional label text (used for centering)
    fn compute(
        width_mm: u32,
        height_mm: u32,
        dpi: u32,
        barcode_type: &BarcodeType,
        data_len: usize,
        text: Option<&str>,
    ) -> Self {
        let total_w = mm_to_dots(width_mm, dpi);
        let total_h = mm_to_dots(height_mm, dpi);

        // 3 % margin on each side, minimum 3 dots.
        let margin_x = ((total_w as f64 * 0.03).round() as u32).max(3);
        let margin_y = ((total_h as f64 * 0.03).round() as u32).max(3);

        let printable_w = total_w.saturating_sub(2 * margin_x);
        let printable_h = total_h.saturating_sub(2 * margin_y);

        // ── font selection & wrapping ─────────────────────────────────────
        let ideal_font_idx = if printable_w > mm_to_dots(35, dpi) { 0 } else { 1 };
        let mut chosen_font = &FONTS[ideal_font_idx];
        let mut lines_str = Vec::new();

        if let Some(t) = text {
            let mut current_idx = ideal_font_idx;
            
            // 1. Auto-shrink: try smaller fonts until it fits on one line.
            while current_idx < FONTS.len() {
                let f = &FONTS[current_idx];
                let width = (t.len() as u32) * f.char_w;
                if width <= printable_w || current_idx == FONTS.len() - 1 {
                    chosen_font = f;
                    break;
                }
                current_idx += 1;
            }

            // 2. Word wrap: if still too wide on the chosen (smallest) font, split into multiple lines.
            let width = (t.len() as u32) * chosen_font.char_w;
            if width > printable_w {
                let mut current_line = String::new();
                for word in t.split_whitespace() {
                    let maybe_line = if current_line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{} {}", current_line, word)
                    };
                    
                    if (maybe_line.len() as u32) * chosen_font.char_w > printable_w {
                        if !current_line.is_empty() {
                            lines_str.push(current_line);
                            current_line = word.to_string();
                        } else {
                            // Single word is too long, push it and let it clip.
                            lines_str.push(word.to_string());
                            current_line = String::new();
                        }
                    } else {
                        current_line = maybe_line;
                    }
                }
                if !current_line.is_empty() {
                    lines_str.push(current_line);
                }
            } else {
                lines_str.push(t.to_string());
            }
        }

        let line_gap = 2; // gap between text lines
        let total_text_h = if lines_str.is_empty() {
            0
        } else {
            let num_lines = lines_str.len() as u32;
            num_lines * chosen_font.h + (num_lines - 1) * line_gap
        };

        let text_gap: u32 = 4; // dots between barcode bottom and text block top
        let has_text = !lines_str.is_empty();

        // ── barcode height ────────────────────────────────────────────────
        // Cap at 55 % of printable height so bars don't dominate the label.
        let max_barcode_h = ((printable_h as f64 * 0.55).round() as u32).max(20);
        let barcode_h = if has_text {
            let available = printable_h.saturating_sub(total_text_h + text_gap);
            available.min(max_barcode_h).max(20)
        } else {
            printable_h.min(max_barcode_h).max(20)
        };

        // ── vertical centering ────────────────────────────────────────────
        // Centre the entire content block (barcode [+ text_block]) in the printable area.
        let content_h = barcode_h + if has_text { text_gap + total_text_h } else { 0 };
        let v_padding = printable_h.saturating_sub(content_h) / 2;
        let barcode_y = margin_y + v_padding;

        // ── narrow bar width ─────────────────────────────────────────────
        let modules = estimate_modules(barcode_type, data_len);
        let narrow = if modules == 0 {
            1 // QR — handled via qr_cell below
        } else {
            (printable_w / modules).max(1).min(3)
        };
        let wide = (narrow * 2).max(2);

        // ── horizontal centering ──────────────────────────────────────────
        // Barcode: centre based on estimated rendered symbol width.
        let estimated_barcode_w = if modules > 0 { modules * narrow } else { 0 };
        let barcode_x = if estimated_barcode_w > 0 && estimated_barcode_w < printable_w {
            margin_x + (printable_w - estimated_barcode_w) / 2
        } else {
            margin_x
        };

        // Text lines: horizontally centre each wrapped line.
        let mut text_lines = Vec::new();
        let mut current_y = barcode_y + barcode_h + text_gap;
        
        for line in lines_str {
            let tw = (line.len() as u32) * chosen_font.char_w;
            let x = if tw < printable_w {
                margin_x + (printable_w - tw) / 2
            } else {
                margin_x
            };
            text_lines.push(TextLine { text: line, x, y: current_y });
            current_y += chosen_font.h + line_gap;
        }

        // ── QR cell size ──────────────────────────────────────────────────
        // Assume ~29 modules per side for common short data.
        let qr_area = printable_w.min(barcode_h);
        let qr_cell = (qr_area / 29).max(1).min(10);

        LabelLayout {
            total_w,
            printable_w,
            printable_h,
            barcode_x,
            barcode_y,
            barcode_h,
            narrow,
            wide,
            qr_cell,
            text_lines,
            tspl_font: chosen_font.tspl,
            epl_font: chosen_font.epl,
            font_h: chosen_font.h,
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

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(),
        req.label_text.as_deref(),
    );

    let mut cmds = String::new();

    // Label header
    cmds.push_str(&format!("SIZE {} mm, {} mm\r\n", width, height));
    cmds.push_str("GAP 2 mm, 0 mm\r\n");
    cmds.push_str("DIRECTION 0\r\n");
    cmds.push_str("CLS\r\n");

    // Barcode / QR — centred
    match req.barcode_type {
        BarcodeType::Qr => {
            cmds.push_str(&format!(
                "QRCODE {},{},M,{},A,0,M2,S3,\"{}\"\r\n",
                l.barcode_x, l.barcode_y, l.qr_cell, req.barcode_data
            ));
        }
        _ => {
            let type_str = tspl_barcode_type(&req.barcode_type);
            // readable=0 — we emit our own TEXT command for accurate placement.
            cmds.push_str(&format!(
                "BARCODE {},{},\"{}\",{},0,0,{},{},\"{}\"\r\n",
                l.barcode_x, l.barcode_y, type_str,
                l.barcode_h, l.narrow, l.wide,
                req.barcode_data
            ));
        }
    }

    // Optional label text — loops through auto-shrunk/wrapped lines
    for line in &l.text_lines {
        cmds.push_str(&format!(
            "TEXT {},{},\"{}\",0,1,1,\"{}\"\r\n",
            line.x, line.y, l.tspl_font, line.text
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
        &BarcodeType::Code128, 12,
        Some("Test Label - OK"),
    );

    // Split barcode area: title text on top, barcode below it.
    let title_h  = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);

    // Centre the title text.
    let title_x = {
        let tw = (19_u32) * FONTS.iter().find(|f| f.tspl == l.tspl_font).unwrap().char_w;
        if tw < l.printable_w {
            let margin_x = (mm_to_dots(config.label_width_mm, config.dpi) as f64 * 0.03)
                .round() as u32;
            margin_x + (l.printable_w - tw) / 2
        } else {
            // Default to margin if it doesn't fit
            (mm_to_dots(config.label_width_mm, config.dpi) as f64 * 0.03).round() as u32
        }
    };

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
        title_x, l.barcode_y, l.tspl_font
    ));
    cmds.push_str(&format!(
        "BARCODE {},{},\"128\",{},0,0,{},{},\"123456789012\"\r\n",
        l.barcode_x, barcode_y2, barcode_h2, l.narrow, l.wide
    ));
    // Footer
    if let Some(line) = l.text_lines.first() {
        cmds.push_str(&format!(
            "TEXT {},{},\"{}\",0,1,1,\"{}\"\r\n",
            line.x, line.y, l.tspl_font, line.text
        ));
    }
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

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(),
        req.label_text.as_deref(),
    );

    let mut cmds = String::new();
    cmds.push_str("^XA\n");

    match req.barcode_type {
        BarcodeType::Qr => {
            cmds.push_str(&format!(
                "^FO{},{}^BQN,2,{}^FDMM,A{}^FS\n",
                l.barcode_x, l.barcode_y, l.qr_cell, req.barcode_data
            ));
        }
        _ => {
            // ^BY = bar width, ^BCN = barcode height, N = no printer-rendered HRI text
            cmds.push_str(&format!(
                "^FO{},{}^BY{}^BCN,{},N,N,N^FD{}^FS\n",
                l.barcode_x, l.barcode_y,
                l.narrow, l.barcode_h,
                req.barcode_data
            ));
        }
    }

    // Optional label text — looped if wrapped
    for line in &l.text_lines {
        // ^FB width, maxLines, lineSpacing, justification('C'=centre), hangingIndent
        cmds.push_str(&format!(
            "^FO0,{}^FB{},1,0,C,0^A0N,{},{}^FD{}^FS\n",
            line.y, l.total_w, l.font_h, l.font_h, line.text
        ));
    }

    cmds.push_str(&format!("^PQ{}\n", copies));
    cmds.push_str("^XZ\n");
    cmds.into_bytes()
}

fn build_test_label_zpl(config: &BarcodePrinterConfig) -> Vec<u8> {
    let l = LabelLayout::compute(
        config.label_width_mm, config.label_height_mm, config.dpi,
        &BarcodeType::Code128, 12,
        Some("Test Label - OK"),
    );

    let title_h  = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);
    let fh = l.font_h;

    let mut cmds = String::new();
    cmds.push_str("^XA\n");
    // Title — centred with ^FB
    cmds.push_str(&format!(
        "^FO0,{}^FB{},1,0,C,0^A0N,{},{}^FDNEXORA BARCODE TEST^FS\n",
        l.barcode_y, l.total_w, fh, fh
    ));
    cmds.push_str(&format!(
        "^FO{},{}^BY{}^BCN,{},N,N,N^FD123456789012^FS\n",
        l.barcode_x, barcode_y2, l.narrow, barcode_h2
    ));
    // Footer — centred with ^FB
    if let Some(line) = l.text_lines.first() {
        cmds.push_str(&format!(
            "^FO0,{}^FB{},1,0,C,0^A0N,{},{}^FDTest Label - OK^FS\n",
            line.y, l.total_w, fh, fh
        ));
    }
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

    let l = LabelLayout::compute(
        width, height, config.dpi,
        &req.barcode_type, req.barcode_data.len(),
        req.label_text.as_deref(),
    );

    let mut cmds = String::new();
    cmds.push_str("N\n");

    // B x, y, rotation, type_id, narrow, wide, height, human_readable, "data"
    cmds.push_str(&format!(
        "B{},{},0,1,{},{},{},N,\"{}\"\n",
        l.barcode_x, l.barcode_y,
        l.narrow, l.wide, l.barcode_h,
        req.barcode_data
    ));

    // Optional label text — loops through auto-shrunk/wrapped lines
    for line in &l.text_lines {
        cmds.push_str(&format!(
            "A{},{},0,{},1,1,N,\"{}\"\n",
            line.x, line.y, l.epl_font, line.text
        ));
    }

    cmds.push_str(&format!("P{}\n", copies));
    cmds.into_bytes()
}

fn build_test_label_epl(config: &BarcodePrinterConfig) -> Vec<u8> {
    let l = LabelLayout::compute(
        config.label_width_mm, config.label_height_mm, config.dpi,
        &BarcodeType::Code128, 12,
        Some("Test Label - OK"),
    );

    let title_h  = (l.barcode_h / 5).max(12);
    let barcode_y2 = l.barcode_y + title_h + 4;
    let barcode_h2 = l.barcode_h.saturating_sub(title_h + 4).max(20);

    // Centre the title text.
    let title_x = {
        let tw = (19_u32) * FONTS.iter().find(|f| f.tspl == l.tspl_font).unwrap().char_w;
        let margin_x = (mm_to_dots(config.label_width_mm, config.dpi) as f64 * 0.03)
            .round() as u32;
        if tw < l.printable_w { margin_x + (l.printable_w - tw) / 2 } else { margin_x }
    };

    let mut cmds = String::new();
    cmds.push_str("N\n");
    cmds.push_str(&format!(
        "A{},{},0,{},1,1,N,\"NEXORA BARCODE TEST\"\n",
        title_x, l.barcode_y, l.epl_font
    ));
    cmds.push_str(&format!(
        "B{},{},0,1,{},{},{},N,\"123456789012\"\n",
        l.barcode_x, barcode_y2, l.narrow, l.wide, barcode_h2
    ));
    if let Some(line) = l.text_lines.first() {
        cmds.push_str(&format!(
            "A{},{},0,{},1,1,N,\"Test Label - OK\"\n",
            line.x, line.y, l.epl_font
        ));
    }
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
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, Some("test"));
        assert_eq!(l.narrow, 1, "narrow bar must be 1 dot on 32mm label");
        assert_eq!(l.wide, 2);
    }

    #[test]
    fn test_layout_32x25_barcode_fits_width() {
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, Some("test"));
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
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, Some("Cola 330ml"));
        let total_h = mm_to_dots(25, 203);
        let text_bottom = l.text_lines.last().unwrap().y + l.font_h;
        assert!(
            text_bottom <= total_h,
            "text bottom ({} dots) must fit in label height ({} dots)",
            text_bottom, total_h
        );
    }

    #[test]
    fn test_layout_no_text_barcode_capped_at_55pct() {
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, None);
        let printable_h = mm_to_dots(25, 203).saturating_sub(2 * 6_u32.max(3));
        let max_h = ((printable_h as f64 * 0.55).round() as u32).max(20);
        assert_eq!(l.barcode_h, max_h,
            "without text, barcode height should be capped at 55% of printable height");
    }

    #[test]
    fn test_layout_content_is_vertically_centred() {
        // The content block (barcode + text) must be centred in the printable area.
        let l = LabelLayout::compute(32, 25, 203, &BarcodeType::Code128, 12, Some("test"));
        let total_h = mm_to_dots(25, 203);
        let margin_y: u32 = ((total_h as f64 * 0.03).round() as u32).max(3);
        let printable_h = total_h.saturating_sub(2 * margin_y);
        let content_h = l.barcode_h + 4 + l.font_h;
        // Content block starts at barcode_y; check it sits within the printable area.
        assert!(l.barcode_y >= margin_y, "content must start at or after top margin");
        assert!(l.text_lines.last().unwrap().y + l.font_h <= total_h, "content must end before bottom of label");
        // Padding above and below should be roughly equal (within 1 dot rounding).
        let pad_top    = l.barcode_y - margin_y;
        let pad_bottom = printable_h.saturating_sub(content_h).saturating_sub(pad_top);
        assert!(pad_top.abs_diff(pad_bottom) <= 1,
            "vertical padding top ({}) and bottom ({}) must be equal", pad_top, pad_bottom);
    }

    #[test]
    fn test_layout_barcode_is_horizontally_centred() {
        let l = LabelLayout::compute(100, 50, 203, &BarcodeType::Code128, 12, None);
        let modules = estimate_modules(&BarcodeType::Code128, 12);
        let barcode_w = modules * l.narrow;
        let total_w = mm_to_dots(100, 203);
        let margin_x: u32 = ((total_w as f64 * 0.03).round() as u32).max(3);
        let printable_w = total_w.saturating_sub(2 * margin_x);
        // The barcode should be left of centre + half barcode width.
        let expected_x = margin_x + (printable_w - barcode_w) / 2;
        assert_eq!(l.barcode_x, expected_x,
            "barcode must be horizontally centred (expected x={}, got x={})",
            expected_x, l.barcode_x);
    }

    #[test]
    fn test_layout_50x30_wider_narrow() {
        // 50 mm gives more room → narrow should be > 1
        let l = LabelLayout::compute(50, 30, 203, &BarcodeType::Code128, 12, None);
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
