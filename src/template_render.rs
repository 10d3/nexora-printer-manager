// src/template_render.rs
// Template data types for Nexora POS
// Defines JSON template structure and receipt data format

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== Template Structure ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptTemplate {
    pub id: String,
    pub name: String,
    pub version: String,
    pub paper_width: Option<u32>,
    pub layout: TemplateLayout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLayout {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub r#type: String,
    pub name: Option<String>,
    pub condition: Option<String>,
    pub elements: Vec<Element>,
    pub spacing: Option<Spacing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub before: Option<u32>,
    pub after: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Element {
    #[serde(rename = "text")]
    Text(TextElement),
    #[serde(rename = "logo")]
    Logo(LogoElement),
    #[serde(rename = "divider")]
    Divider(DividerElement),
    #[serde(rename = "row")]
    Row(RowElement),
    #[serde(rename = "qr")]
    QR(QRElement),
    #[serde(rename = "barcode")]
    Barcode(BarcodeElement),
    #[serde(rename = "table")]
    Table(TableElement),
    #[serde(rename = "space")]
    Space(SpaceElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElement {
    pub content: String,
    pub align: Option<String>,
    pub font_size: Option<u8>,
    pub font_width: Option<u8>,
    pub bold: Option<bool>,
    pub underline: Option<bool>,
    pub invert: Option<bool>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoElement {
    pub source: Option<String>,
    pub align: Option<String>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividerElement {
    pub style: Option<String>,
    pub character: Option<String>,
    pub length: Option<String>,
    pub align: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowElement {
    pub left: Option<String>,
    pub right: Option<String>,
    pub center: Option<String>,
    pub bold: Option<bool>,
    pub separator: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRElement {
    pub content: String,
    pub size: Option<u8>,
    pub align: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeElement {
    pub content: String,
    pub format: Option<String>,
    pub height: Option<u8>,
    pub width: Option<u8>,
    pub show_text: Option<bool>,
    pub align: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableElement {
    pub columns: Vec<TableColumn>,
    pub data_source: String,
    pub show_header: Option<bool>,
    pub header_bold: Option<bool>,
    pub header_divider: Option<bool>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub header: String,
    pub field: String,
    pub width: Option<u32>,
    pub align: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceElement {
    pub lines: Option<u32>,
    pub condition: Option<String>,
}

// ==================== Receipt Data ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    // Store info
    pub store_name: Option<String>,
    pub store_address: Option<String>,
    pub store_phone: Option<String>,
    pub store_website: Option<String>,

    // Order info
    pub order_id: String,
    pub timestamp: String,
    pub cashier_name: Option<String>,
    pub server_name: Option<String>,
    pub table_number: Option<String>,

    // Items
    pub items: Vec<ReceiptItem>,

    // Totals
    pub subtotal: f64,
    pub tax: f64,
    pub tax_rate: Option<f64>,
    pub discount: Option<f64>,
    pub tip: Option<f64>,
    pub total: f64,

    // Payment
    pub payment_method: String,
    pub change: Option<f64>,

    // Additional
    pub footer_message: Option<String>,
    pub receipt_url: Option<String>,

    // Custom fields
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub name: String,
    pub quantity: u32,
    pub price: f64,
    pub total: f64,
}

// ==================== Template Renderer ====================

pub struct TemplateRenderer {
    paper_width: u32,
}

impl TemplateRenderer {
    pub fn new(paper_width: u32) -> Self {
        Self { paper_width }
    }

    /// Get paper width
    pub fn paper_width(&self) -> u32 {
        self.paper_width
    }
}
