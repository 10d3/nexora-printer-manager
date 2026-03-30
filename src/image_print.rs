// src/image_print.rs

use base64::{engine::general_purpose, Engine as _};
use image::{imageops::FilterType, GenericImageView, ImageReader};
use std::io::Cursor;

/// Converts a base64-encoded PNG/JPEG into ESC/POS raster bitmap bytes (GS v 0).
///
/// # Arguments
/// * `base64_data`      – Base64 string, with or without a `data:image/...;base64,` prefix.
/// * `paper_width_dots` – Full printable width of the paper in dots (e.g. 576 for 80mm).
/// * `max_width_dots`   – Optional max image width in dots. Defaults to full paper width.
///                        Use this to print logos smaller than the full paper (e.g. 288 = half).
/// * `align`            – Horizontal position: "left" | "center" | "right".
///                        Achieved by padding empty dot columns — ESC/POS ignores text alignment
///                        commands for bitmap data, so we handle it in the row bytes directly.
///
/// # Returns
/// Raw ESC/POS bytes you can write directly to the printer.
pub fn image_to_escpos(
    base64_data: &str,
    paper_width_dots: u32,
    max_width_dots: Option<u32>,
    align: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // ── 1. Strip optional data-URI prefix ────────────────────────────────────
    let b64 = match base64_data.find(',') {
        Some(pos) => &base64_data[pos + 1..],
        None => base64_data,
    };

    // ── 2. Decode base64 → raw image bytes ───────────────────────────────────
    let img_bytes = general_purpose::STANDARD.decode(b64.trim())?;

    // ── 3. Decode image ───────────────────────────────────────────────────────
    let img = ImageReader::new(Cursor::new(img_bytes))
        .with_guessed_format()?
        .decode()?;

    // ── 4. Determine target image width ──────────────────────────────────────
    // Both paper_width and image_width must be multiples of 8.
    let paper_w = (paper_width_dots / 8) * 8;

    let image_max_w = match max_width_dots {
        Some(mw) => ((mw / 8) * 8).min(paper_w), // clamp to paper width
        None => paper_w,
    };
    let image_max_w = image_max_w.max(8);

    // ── 5. Scale image to image_max_w, preserving aspect ratio ───────────────
    let (orig_w, orig_h) = img.dimensions();
    let scale = image_max_w as f32 / orig_w as f32;
    let target_w = image_max_w;
    let target_h = (orig_h as f32 * scale).round() as u32;

    let target_w = target_w.max(8);
    let target_h = target_h.max(1);

    let gray = img
        .resize_exact(target_w, target_h, FilterType::Lanczos3)
        .to_luma8();

    let (img_w, height) = gray.dimensions();
    let img_bytes_per_row = img_w / 8;

    // ── 6. Calculate padding for alignment ───────────────────────────────────
    // Total dots per row in the ESC/POS command = paper_w (we always fill the
    // full paper width with a mix of image dots and empty padding dots).
    let total_bytes_per_row = paper_w / 8;
    let pad_total_bytes = total_bytes_per_row.saturating_sub(img_bytes_per_row);

    let (pad_left_bytes, pad_right_bytes) = match align.to_lowercase().as_str() {
        "right" => (pad_total_bytes, 0),
        "center" => {
            let left = pad_total_bytes / 2;
            let right = pad_total_bytes - left; // handles odd remainder
            (left, right)
        }
        _ => (0, pad_total_bytes), // "left" default
    };

    // ── 7. Build GS v 0 raster bitmap command ────────────────────────────────
    // Header: 1D 76 30 <mode> <xL> <xH> <yL> <yH>
    // xL/xH = bytes per row (full paper width, including padding)
    // yL/yH = number of rows (image height)
    let mut out =
        Vec::with_capacity(8 + (total_bytes_per_row * height) as usize);

    out.extend_from_slice(&[
        0x1D, 0x76, 0x30, 0x00, // GS v 0, normal density
        (total_bytes_per_row & 0xFF) as u8,
        ((total_bytes_per_row >> 8) & 0xFF) as u8,
        (height & 0xFF) as u8,
        ((height >> 8) & 0xFF) as u8,
    ]);

    for y in 0..height {
        // Left padding — empty dots (white)
        for _ in 0..pad_left_bytes {
            out.push(0x00);
        }

        // Image pixels — dark pixel (< 128) → 1 (printed dot)
        for bx in 0..img_bytes_per_row {
            let mut byte = 0u8;
            for bit in 0..8u32 {
                if gray.get_pixel(bx * 8 + bit, y).0[0] < 128 {
                    byte |= 1 << (7 - bit);
                }
            }
            out.push(byte);
        }

        // Right padding — empty dots (white)
        for _ in 0..pad_right_bytes {
            out.push(0x00);
        }
    }

    Ok(out)
}

/// Generates an ASCII art preview + real ESC/POS metadata.
///
/// Returns: (ascii_art, printed_width_dots, printed_height_dots, estimated_escpos_bytes)
pub fn generate_image_preview(
    base64_data: &str,
    paper_width_dots: u32,
    max_width_dots: Option<u32>,
    align: &str,
) -> Result<(String, u32, u32, usize), Box<dyn std::error::Error + Send + Sync>> {
    let b64 = match base64_data.find(',') {
        Some(pos) => &base64_data[pos + 1..],
        None => base64_data,
    };
    let img_bytes = general_purpose::STANDARD.decode(b64.trim())?;
    let img = ImageReader::new(Cursor::new(img_bytes))
        .with_guessed_format()?
        .decode()?;

    let paper_w = (paper_width_dots / 8) * 8;
    let image_max_w = match max_width_dots {
        Some(mw) => ((mw / 8) * 8).min(paper_w),
        None => paper_w,
    }
    .max(8);

    let (orig_w, orig_h) = img.dimensions();
    let scale = image_max_w as f32 / orig_w as f32;
    let real_w = image_max_w;
    let real_h = (orig_h as f32 * scale).round() as u32;

    let real_w = real_w.max(8);
    let real_h = real_h.max(1);
    let total_bytes_per_row = paper_w / 8;

    // Header (8) + full-width raster rows + feed/cut footer (7)
    let estimated_bytes = 8 + (total_bytes_per_row * real_h) as usize + 7;

    // ASCII preview — always 72 chars wide for terminal safety
    let preview_max_w = 72u32;
    let preview_scale = preview_max_w as f32 / orig_w as f32;
    let preview_img_w = ((orig_w as f32 * preview_scale) as u32)
        .min(preview_max_w)
        .max(1);
    // Terminal chars are ~2x taller than wide, so halve the height
    let preview_img_h = ((orig_h as f32 * preview_scale) / 2.0).round() as u32;
    let preview_img_h = preview_img_h.max(1);

    let preview_gray = img
        .resize_exact(preview_img_w, preview_img_h, FilterType::Triangle)
        .to_luma8();

    // Calculate ASCII padding for alignment
    let pad_total = preview_max_w.saturating_sub(preview_img_w);
    let pad_left = match align.to_lowercase().as_str() {
        "right" => pad_total,
        "center" => pad_total / 2,
        _ => 0,
    };
    let pad_right = pad_total - pad_left;

    let mut ascii_art =
        String::with_capacity(((preview_max_w + 1) * preview_img_h) as usize);

    for y in 0..preview_img_h {
        for _ in 0..pad_left {
            ascii_art.push(' ');
        }
        for x in 0..preview_img_w {
            if preview_gray.get_pixel(x, y).0[0] < 128 {
                ascii_art.push('█');
            } else {
                ascii_art.push(' ');
            }
        }
        for _ in 0..pad_right {
            ascii_art.push(' ');
        }
        ascii_art.push('\n');
    }

    Ok((ascii_art, real_w, real_h, estimated_bytes))
}