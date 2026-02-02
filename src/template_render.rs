use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Note: This module uses a PrintCommand abstraction for rendering instead of
// directly using escpos types. For direct printer integration, see main.rs.

// ==================== Template Structure ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptTemplate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: String,
    pub paper_width: Option<u32>,
    #[serde(default)]
    pub supports_logo: Option<bool>,
    #[serde(default)]
    pub supports_qr: Option<bool>,
    #[serde(default)]
    pub supports_barcode: Option<bool>,
    pub layout: TemplateLayout,
    #[serde(default)]
    pub variables: Option<HashMap<String, VariableDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDefinition {
    #[serde(rename = "type")]
    pub var_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateLayout {
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    #[serde(rename = "type")]
    pub section_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub spacing: Option<Spacing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    #[serde(default)]
    pub before: Option<u32>,
    #[serde(default)]
    pub after: Option<u32>,
}

// ==================== Element Types ====================

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
    #[serde(rename = "box")]
    Box(BoxElement),
    #[serde(rename = "grid")]
    Grid(GridElement),
    #[serde(rename = "bar_chart")]
    BarChart(BarChartElement),
    #[serde(rename = "leaderboard")]
    Leaderboard(LeaderboardElement),
}

// ==================== Text Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElement {
    pub content: String,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub font_size: Option<u8>,
    #[serde(default)]
    pub font_width: Option<u8>,
    #[serde(default)]
    pub font_weight: Option<String>,
    #[serde(default)]
    pub font_style: Option<String>,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underline: Option<bool>,
    #[serde(default)]
    pub invert: Option<bool>,
    #[serde(default)]
    pub letter_spacing: Option<u8>,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Logo Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoElement {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub max_width: Option<u32>,
    #[serde(default)]
    pub max_height: Option<u32>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Divider Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividerElement {
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub character: Option<String>,
    #[serde(default)]
    pub thickness: Option<u8>,
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub length: Option<String>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Row Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowElement {
    #[serde(default)]
    pub left: Option<String>,
    #[serde(default)]
    pub right: Option<String>,
    #[serde(default)]
    pub center: Option<String>,
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub invert: Option<bool>,
    #[serde(default)]
    pub font_size: Option<u8>,
    #[serde(default)]
    pub font_weight: Option<String>,
    #[serde(default)]
    pub font_style: Option<String>,
    #[serde(default)]
    pub letter_spacing: Option<u8>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub elements: Option<Vec<Element>>,
}

// ==================== QR Code Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRElement {
    pub content: String,
    #[serde(default)]
    pub size: Option<u8>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Barcode Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeElement {
    pub content: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub height: Option<u8>,
    #[serde(default)]
    pub width: Option<u8>,
    #[serde(default)]
    pub show_text: Option<bool>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Table Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableElement {
    pub columns: Vec<TableColumn>,
    pub data_source: String,
    #[serde(default)]
    pub show_header: Option<bool>,
    #[serde(default)]
    pub header_bold: Option<bool>,
    #[serde(default)]
    pub header_divider: Option<bool>,
    #[serde(default)]
    pub alternating_rows: Option<bool>,
    #[serde(default)]
    pub row_details: Option<Vec<RowDetail>>,
    #[serde(default)]
    pub modifiers: Option<ModifierConfig>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    #[serde(default)]
    pub header: Option<String>,
    pub field: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub font_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDetail {
    pub field: String,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub font_size: Option<u8>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierConfig {
    #[serde(default)]
    pub indent: Option<u8>,
    #[serde(default)]
    pub font_size: Option<u8>,
    #[serde(default)]
    pub prefix: Option<String>,
}

// ==================== Space Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceElement {
    #[serde(default)]
    pub lines: Option<u32>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Box Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxElement {
    pub elements: Vec<Element>,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub border: Option<u8>,
    #[serde(default)]
    pub border_position: Option<String>,
    #[serde(default)]
    pub padding: Option<u8>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Grid Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridElement {
    pub columns: u8,
    pub data: Vec<GridItem>,
    #[serde(default)]
    pub gap: Option<u8>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridItem {
    pub label: String,
    pub value: String,
}

// ==================== Bar Chart Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarChartElement {
    pub data_source: String,
    pub value_field: String,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub condition: Option<String>,
}

// ==================== Leaderboard Element ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardElement {
    pub data_source: String,
    pub fields: LeaderboardFields,
    #[serde(default)]
    pub highlight_top: Option<u8>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardFields {
    pub rank: String,
    pub name: String,
    #[serde(default)]
    pub shift: Option<String>,
    #[serde(default)]
    pub sales: Option<String>,
    #[serde(default)]
    pub transactions: Option<String>,
}

// ==================== Receipt Data ====================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReceiptData {
    // Store info
    #[serde(default)]
    pub store_name: Option<String>,
    #[serde(default)]
    pub store_address: Option<String>,
    #[serde(default)]
    pub store_phone: Option<String>,
    #[serde(default)]
    pub store_website: Option<String>,
    #[serde(default)]
    pub established_year: Option<u32>,

    // Order info
    pub order_id: String,
    pub timestamp: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub cashier_name: Option<String>,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub table_number: Option<String>,

    // Items
    #[serde(default)]
    pub items: Vec<ReceiptItem>,

    // Totals
    #[serde(default)]
    pub subtotal: f64,
    #[serde(default)]
    pub tax: f64,
    #[serde(default)]
    pub tax_rate: Option<f64>,
    #[serde(default)]
    pub discount: Option<f64>,
    #[serde(default)]
    pub tip: Option<f64>,
    #[serde(default)]
    pub service_charge: Option<f64>,
    #[serde(default)]
    pub service_rate: Option<f64>,
    #[serde(default)]
    pub total: f64,

    // Payment
    #[serde(default)]
    pub payment_method: String,
    #[serde(default)]
    pub change: Option<f64>,

    // Additional
    #[serde(default)]
    pub footer_message: Option<String>,
    #[serde(default)]
    pub farewell_message: Option<String>,
    #[serde(default)]
    pub receipt_url: Option<String>,

    // Custom fields for flexibility
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub name: String,
    #[serde(default)]
    pub quantity: u32,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub modifiers: Option<Vec<String>>,
}

impl Default for ReceiptItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            quantity: 1,
            price: 0.0,
            total: 0.0,
            modifiers: None,
        }
    }
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

    /// Render template with data to a vector of print commands (for testing/preview)
    pub fn render_to_commands(
        &self,
        template: &ReceiptTemplate,
        data: &ReceiptData,
    ) -> Result<Vec<PrintCommand>, String> {
        let mut commands = vec![PrintCommand::Init];

        // Render each section
        for section in &template.layout.sections {
            if self.should_render(&section.condition, data) {
                self.build_section_commands(&mut commands, section, data)?;
            }
        }

        // Final feed and cut
        commands.push(PrintCommand::Feed(3));
        commands.push(PrintCommand::Cut);

        Ok(commands)
    }

    /// Build commands for a section
    fn build_section_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        section: &Section,
        data: &ReceiptData,
    ) -> Result<(), String> {
        // Spacing before
        if let Some(spacing) = &section.spacing {
            if let Some(before) = spacing.before {
                commands.push(PrintCommand::Feed(before as u8));
            }
        }

        // Render elements
        for element in &section.elements {
            self.build_element_commands(commands, element, data)?;
        }

        // Spacing after
        if let Some(spacing) = &section.spacing {
            if let Some(after) = spacing.after {
                commands.push(PrintCommand::Feed(after as u8));
            }
        }

        Ok(())
    }

    /// Build commands for an element
    fn build_element_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &Element,
        data: &ReceiptData,
    ) -> Result<(), String> {
        match element {
            Element::Text(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_text_commands(commands, e, data)?;
                }
            }
            Element::Divider(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_divider_commands(commands, e)?;
                }
            }
            Element::Row(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_row_commands(commands, e, data)?;
                }
            }
            Element::QR(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_qr_commands(commands, e, data)?;
                }
            }
            Element::Barcode(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_barcode_commands(commands, e, data)?;
                }
            }
            Element::Table(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_table_commands(commands, e, data)?;
                }
            }
            Element::Space(e) => {
                if self.should_render(&e.condition, data) {
                    let lines = e.lines.unwrap_or(1);
                    commands.push(PrintCommand::Feed(lines as u8));
                }
            }
            Element::Logo(_) => {
                // Logo rendering would require image processing
                log::warn!("Logo rendering not yet implemented");
            }
            Element::Box(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_box_commands(commands, e, data)?;
                }
            }
            Element::Grid(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_grid_commands(commands, e, data)?;
                }
            }
            Element::BarChart(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_bar_chart_commands(commands, e, data)?;
                }
            }
            Element::Leaderboard(e) => {
                if self.should_render(&e.condition, data) {
                    self.build_leaderboard_commands(commands, e, data)?;
                }
            }
        }

        Ok(())
    }

    /// Build text element commands
    fn build_text_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &TextElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        // Apply styles
        if element.bold.unwrap_or(false) {
            commands.push(PrintCommand::Bold(true));
        }

        if element.underline.unwrap_or(false) {
            commands.push(PrintCommand::Underline(true));
        }

        if element.invert.unwrap_or(false) {
            commands.push(PrintCommand::Reverse(true));
        }

        // Set size
        let width = element.font_width.unwrap_or(1);
        let height = element.font_size.unwrap_or(1);
        if width > 1 || height > 1 {
            commands.push(PrintCommand::Size(width, height));
        }

        // Set alignment
        let align = element.align.as_deref().unwrap_or("left");
        commands.push(PrintCommand::Align(align.to_string()));

        // Substitute variables
        let mut content = self.substitute_variables(&element.content, data);

        // Apply letter spacing if specified
        if let Some(spacing) = element.letter_spacing {
            if spacing > 0 {
                content = self.apply_letter_spacing(&content, spacing);
            }
        }

        // Print
        commands.push(PrintCommand::WriteLine(content));

        // Reset styles
        commands.push(PrintCommand::Bold(false));
        commands.push(PrintCommand::Underline(false));
        commands.push(PrintCommand::Reverse(false));
        commands.push(PrintCommand::Size(1, 1));
        commands.push(PrintCommand::Align("left".to_string()));

        Ok(())
    }

    /// Apply letter spacing by inserting spaces between characters
    fn apply_letter_spacing(&self, text: &str, spacing: u8) -> String {
        let spacing_str = " ".repeat(spacing as usize);
        text.chars()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(&spacing_str)
    }

    /// Build divider commands
    fn build_divider_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &DividerElement,
    ) -> Result<(), String> {
        let character = if let Some(pattern) = &element.pattern {
            match pattern.as_str() {
                "diamond" => "◆ ",
                "wave" => "~",
                "dot" => "·",
                "star" => "* ",
                "elegant" | "fancy" | "line" => "─",
                _ => "-",
            }
        } else {
            match element.style.as_deref().unwrap_or("single") {
                "double" => "=",
                "dashed" => "-",
                "dotted" => ".",
                "thick" | "solid" => "━",
                "thin" | "elegant" | "gradient" => "─",
                "custom" => element.character.as_deref().unwrap_or("-"),
                _ => "-",
            }
        };

        let width = self.paper_width as usize;

        // For patterns with spaces, adjust repetition
        let divider = if character.contains(' ') {
            let pattern_len = character.len();
            let repeats = width / pattern_len;
            character.repeat(repeats)
        } else {
            character.repeat(width)
        };

        let align = element.align.as_deref().unwrap_or("left");
        commands.push(PrintCommand::Align(align.to_string()));
        commands.push(PrintCommand::WriteLine(divider));
        commands.push(PrintCommand::Align("left".to_string()));

        Ok(())
    }

    /// Build row commands
    fn build_row_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &RowElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        // Apply styles
        if element.bold.unwrap_or(false) {
            commands.push(PrintCommand::Bold(true));
        }

        if element.invert.unwrap_or(false) {
            commands.push(PrintCommand::Reverse(true));
        }

        // Set font size if specified
        let font_size = element.font_size.unwrap_or(1);
        if font_size > 1 {
            commands.push(PrintCommand::Size(font_size, font_size));
        }

        let left = element
            .left
            .as_ref()
            .map(|s| self.substitute_variables(s, data))
            .unwrap_or_default();
        let right = element
            .right
            .as_ref()
            .map(|s| self.substitute_variables(s, data))
            .unwrap_or_default();

        let width = self.paper_width as usize;
        let total_content_len = left.chars().count() + right.chars().count();

        let line = if total_content_len < width {
            let spaces = width - total_content_len;
            format!(
                "{}{:>width$}",
                left,
                right,
                width = right.chars().count() + spaces
            )
        } else {
            format!("{} {}", left, right)
        };

        commands.push(PrintCommand::WriteLine(line));

        // Reset styles
        if element.bold.unwrap_or(false) {
            commands.push(PrintCommand::Bold(false));
        }
        if element.invert.unwrap_or(false) {
            commands.push(PrintCommand::Reverse(false));
        }
        if font_size > 1 {
            commands.push(PrintCommand::Size(1, 1));
        }

        Ok(())
    }

    /// Build QR code commands
    fn build_qr_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &QRElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let content = self.substitute_variables(&element.content, data);
        let size = element.size.unwrap_or(6);
        let align = element.align.as_deref().unwrap_or("center");

        commands.push(PrintCommand::Align(align.to_string()));
        commands.push(PrintCommand::QRCode { content, size });
        commands.push(PrintCommand::Align("left".to_string()));

        Ok(())
    }

    /// Build barcode commands
    fn build_barcode_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &BarcodeElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let content = self.substitute_variables(&element.content, data);
        let height = element.height.unwrap_or(100);
        let width = element.width.unwrap_or(3);
        let format = element
            .format
            .clone()
            .unwrap_or_else(|| "CODE128".to_string());
        let show_text = element.show_text.unwrap_or(true);
        let align = element.align.as_deref().unwrap_or("center");

        commands.push(PrintCommand::Align(align.to_string()));
        commands.push(PrintCommand::Barcode {
            content,
            format,
            height,
            width,
            show_text,
        });
        commands.push(PrintCommand::Align("left".to_string()));

        Ok(())
    }

    /// Build table commands
    fn build_table_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &TableElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        // Print header if enabled
        if element.show_header.unwrap_or(false) {
            if element.header_bold.unwrap_or(true) {
                commands.push(PrintCommand::Bold(true));
            }

            let header_line = self.format_table_row(&element.columns, None);
            commands.push(PrintCommand::WriteLine(header_line));

            if element.header_bold.unwrap_or(true) {
                commands.push(PrintCommand::Bold(false));
            }

            if element.header_divider.unwrap_or(true) {
                let divider = "-".repeat(self.paper_width as usize);
                commands.push(PrintCommand::WriteLine(divider));
            }
        }

        // Print rows from data source
        let rows = self.get_data_source_items(&element.data_source, data);

        for (index, row) in rows.iter().enumerate() {
            // Alternating row background
            if element.alternating_rows.unwrap_or(false) && index % 2 == 1 {
                commands.push(PrintCommand::Reverse(true));
            }

            let row_line = self.format_table_row(&element.columns, Some(row));
            commands.push(PrintCommand::WriteLine(row_line));

            if element.alternating_rows.unwrap_or(false) && index % 2 == 1 {
                commands.push(PrintCommand::Reverse(false));
            }

            // Print row details if configured
            if let Some(details) = &element.row_details {
                for detail in details {
                    if let Some(value) = row.get(&detail.field) {
                        // Check condition
                        if detail.condition.is_some() {
                            // Skip if condition not met (simplified)
                            if value.is_empty() {
                                continue;
                            }
                        }

                        let prefix = detail.prefix.as_deref().unwrap_or("");
                        let suffix = detail.suffix.as_deref().unwrap_or("");
                        let detail_line = format!("  {}{}{}", prefix, value, suffix);

                        if let Some(font_size) = detail.font_size {
                            if font_size != 1 {
                                commands.push(PrintCommand::Size(font_size, font_size));
                            }
                        }

                        commands.push(PrintCommand::WriteLine(detail_line));

                        if detail.font_size.is_some() {
                            commands.push(PrintCommand::Size(1, 1));
                        }
                    }
                }
            }

            // Print modifiers if configured
            if let Some(modifier_config) = &element.modifiers {
                if let Some(modifiers_value) = row.get("modifiers") {
                    let modifiers: Vec<&str> = modifiers_value.split(',').collect();
                    for modifier in modifiers {
                        let modifier = modifier.trim();
                        if !modifier.is_empty() {
                            self.build_modifier_command(commands, modifier, modifier_config)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Build modifier command
    fn build_modifier_command(
        &self,
        commands: &mut Vec<PrintCommand>,
        modifier: &str,
        config: &ModifierConfig,
    ) -> Result<(), String> {
        let indent = " ".repeat(config.indent.unwrap_or(2) as usize);
        let prefix = config.prefix.as_deref().unwrap_or("");

        let font_size = config.font_size.unwrap_or(1);
        if font_size > 1 {
            commands.push(PrintCommand::Size(font_size, font_size));
        }

        commands.push(PrintCommand::WriteLine(format!(
            "{}{}{}",
            indent, prefix, modifier
        )));

        if font_size > 1 {
            commands.push(PrintCommand::Size(1, 1));
        }

        Ok(())
    }

    /// Build box element commands
    fn build_box_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &BoxElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let style = element.style.as_deref().unwrap_or("default");
        let border = element.border.unwrap_or(0);
        let padding = element.padding.unwrap_or(0);

        // Handle different box styles
        match style {
            "filled" => {
                commands.push(PrintCommand::Reverse(true));
            }
            "shaded" => {
                // Shaded background - use reverse for thermal printers
                commands.push(PrintCommand::Reverse(true));
            }
            _ => {}
        }

        // Top border
        if border > 0 {
            let border_positions = element.border_position.as_deref().unwrap_or("all");
            if border_positions.contains("top") || border_positions == "all" {
                let border_line = "━".repeat(self.paper_width as usize);
                commands.push(PrintCommand::WriteLine(border_line));
            }
        }

        // Top padding
        for _ in 0..padding {
            commands.push(PrintCommand::Feed(1));
        }

        // Render inner elements
        for inner_elem in &element.elements {
            self.build_element_commands(commands, inner_elem, data)?;
        }

        // Bottom padding
        for _ in 0..padding {
            commands.push(PrintCommand::Feed(1));
        }

        // Bottom border
        if border > 0 {
            let border_positions = element.border_position.as_deref().unwrap_or("all");
            if border_positions.contains("bottom")
                || border_positions == "all"
                || border_positions == "top-bottom"
            {
                let border_line = "━".repeat(self.paper_width as usize);
                commands.push(PrintCommand::WriteLine(border_line));
            }
        }

        // Reset reverse mode
        if style == "filled" || style == "shaded" {
            commands.push(PrintCommand::Reverse(false));
        }

        Ok(())
    }

    /// Build grid element commands
    fn build_grid_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &GridElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let col_count = element.columns as usize;
        let gap = element.gap.unwrap_or(0) as usize;
        let col_width = (self.paper_width as usize - (col_count - 1) * gap) / col_count;

        // Process items in pairs based on column count
        for chunk in element.data.chunks(col_count) {
            let mut line = String::new();

            for (i, item) in chunk.iter().enumerate() {
                let label_value = format!(
                    "{}: {}",
                    item.label,
                    self.substitute_variables(&item.value, data)
                );

                let formatted = if label_value.len() > col_width {
                    label_value[..col_width].to_string()
                } else {
                    format!("{:<width$}", label_value, width = col_width)
                };

                line.push_str(&formatted);

                if i < chunk.len() - 1 {
                    line.push_str(&" ".repeat(gap));
                }
            }

            commands.push(PrintCommand::WriteLine(line));
        }

        Ok(())
    }

    /// Build bar chart commands (ASCII representation)
    fn build_bar_chart_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &BarChartElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let rows = self.get_data_source_items(&element.data_source, data);

        if rows.is_empty() {
            return Ok(());
        }

        // Find max value
        let max_value: f64 = rows
            .iter()
            .filter_map(|row| row.get(&element.value_field))
            .filter_map(|v| v.parse::<f64>().ok())
            .fold(0.0, f64::max);

        if max_value == 0.0 {
            return Ok(());
        }

        let chart_width = (self.paper_width - 10) as usize; // Leave room for labels

        for row in &rows {
            if let Some(value_str) = row.get(&element.value_field) {
                if let Ok(value) = value_str.parse::<f64>() {
                    let bar_length = ((value / max_value) * chart_width as f64) as usize;
                    let bar = "█".repeat(bar_length);

                    // Get label (try hour field for hourly data)
                    let label = row
                        .get("hour")
                        .or_else(|| row.get("label"))
                        .cloned()
                        .unwrap_or_default();

                    let line = format!("{:>5} │{}", label, bar);
                    commands.push(PrintCommand::WriteLine(line));
                }
            }
        }

        Ok(())
    }

    /// Build leaderboard commands
    fn build_leaderboard_commands(
        &self,
        commands: &mut Vec<PrintCommand>,
        element: &LeaderboardElement,
        data: &ReceiptData,
    ) -> Result<(), String> {
        let rows = self.get_data_source_items(&element.data_source, data);
        let highlight_top = element.highlight_top.unwrap_or(0);

        for (index, row) in rows.iter().enumerate() {
            let rank = row.get(&element.fields.rank).cloned().unwrap_or_default();
            let name = row.get(&element.fields.name).cloned().unwrap_or_default();

            let shift = element
                .fields
                .shift
                .as_ref()
                .and_then(|f| row.get(f))
                .cloned()
                .unwrap_or_default();

            let sales = element
                .fields
                .sales
                .as_ref()
                .and_then(|f| row.get(f))
                .cloned()
                .unwrap_or_default();

            // Highlight top performers
            if index < highlight_top as usize {
                commands.push(PrintCommand::Bold(true));
                commands.push(PrintCommand::Reverse(true));
            }

            // Format leaderboard entry
            let entry = if shift.is_empty() {
                format!("{:>2}. {:<20} ${}", rank, name, sales)
            } else {
                format!("{:>2}. {:<15} {:>8} ${}", rank, name, shift, sales)
            };

            commands.push(PrintCommand::WriteLine(entry));

            if index < highlight_top as usize {
                commands.push(PrintCommand::Bold(false));
                commands.push(PrintCommand::Reverse(false));
            }
        }

        Ok(())
    }

    /// Format a table row
    fn format_table_row(
        &self,
        columns: &[TableColumn],
        data: Option<&HashMap<String, String>>,
    ) -> String {
        let mut line = String::new();
        let _total_width = self.paper_width as usize;

        let specified_width: u32 = columns.iter().filter_map(|c| c.width).sum();

        let remaining = if specified_width < self.paper_width {
            self.paper_width - specified_width
        } else {
            0
        };

        let unspecified_count = columns.iter().filter(|c| c.width.is_none()).count() as u32;

        for (i, col) in columns.iter().enumerate() {
            let width = col.width.unwrap_or_else(|| {
                if unspecified_count > 0 {
                    remaining / unspecified_count
                } else {
                    10
                }
            }) as usize;

            let content = if let Some(data) = data {
                let raw = data.get(&col.field).cloned().unwrap_or_default();
                // Apply format
                if let Some(format) = &col.format {
                    match format.as_str() {
                        "currency" => {
                            if let Ok(num) = raw.parse::<f64>() {
                                format!("${:.2}", num)
                            } else {
                                raw
                            }
                        }
                        _ => raw,
                    }
                } else {
                    raw
                }
            } else {
                col.header.clone().unwrap_or_else(|| col.field.clone())
            };

            // Truncate if too long
            let content = if content.len() > width {
                content[..width].to_string()
            } else {
                content
            };

            let aligned_content = match col.align.as_deref().unwrap_or("left") {
                "right" => format!("{:>width$}", content, width = width),
                "center" => format!("{:^width$}", content, width = width),
                _ => format!("{:<width$}", content, width = width),
            };

            line.push_str(&aligned_content);

            if i < columns.len() - 1 {
                line.push(' ');
            }
        }

        line
    }

    /// Get items from a data source
    fn get_data_source_items(
        &self,
        source: &str,
        data: &ReceiptData,
    ) -> Vec<HashMap<String, String>> {
        match source {
            "items" => data
                .items
                .iter()
                .map(|item| self.item_to_map(item))
                .collect(),
            _ => {
                // Try to get from custom fields
                if let Some(value) = data.custom.get(source) {
                    if let Some(arr) = value.as_array() {
                        arr.iter()
                            .filter_map(|v| {
                                if let Some(obj) = v.as_object() {
                                    let mut map = HashMap::new();
                                    for (k, v) in obj {
                                        let str_value = match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            serde_json::Value::Number(n) => n.to_string(),
                                            serde_json::Value::Bool(b) => b.to_string(),
                                            _ => v.to_string(),
                                        };
                                        map.insert(k.clone(), str_value);
                                    }
                                    Some(map)
                                } else {
                                    None
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Convert item to hashmap for table rendering
    fn item_to_map(&self, item: &ReceiptItem) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), item.name.clone());
        map.insert("quantity".to_string(), item.quantity.to_string());
        map.insert("price".to_string(), format!("{:.2}", item.price));
        map.insert("total".to_string(), format!("{:.2}", item.total));
        if let Some(modifiers) = &item.modifiers {
            map.insert("modifiers".to_string(), modifiers.join(","));
        }
        map
    }

    /// Substitute variables in text
    fn substitute_variables(&self, text: &str, data: &ReceiptData) -> String {
        let re = Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}").unwrap();

        re.replace_all(text, |caps: &regex::Captures| {
            let var_name = &caps[1];
            self.get_variable_value(var_name, data)
        })
        .to_string()
    }

    /// Get variable value from data
    fn get_variable_value(&self, name: &str, data: &ReceiptData) -> String {
        match name {
            "store_name" => data.store_name.clone().unwrap_or_default(),
            "store_address" => data.store_address.clone().unwrap_or_default(),
            "store_phone" => data.store_phone.clone().unwrap_or_default(),
            "store_website" => data.store_website.clone().unwrap_or_default(),
            "established_year" => data
                .established_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
            "order_id" => data.order_id.clone(),
            "timestamp" => data.timestamp.clone(),
            "date" => data.date.clone().unwrap_or_else(|| {
                // Fallback: extract date from timestamp
                data.timestamp
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string()
            }),
            "time" => data.time.clone().unwrap_or_else(|| {
                // Fallback: extract time from timestamp
                data.timestamp
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string()
            }),
            "cashier_name" => data.cashier_name.clone().unwrap_or_default(),
            "server_name" => data.server_name.clone().unwrap_or_default(),
            "table_number" => data.table_number.clone().unwrap_or_default(),
            "subtotal" => format!("{:.2}", data.subtotal),
            "tax" => format!("{:.2}", data.tax),
            "tax_rate" => data
                .tax_rate
                .map(|r| format!("{:.1}", r))
                .unwrap_or_default(),
            "discount" => data
                .discount
                .map(|d| format!("{:.2}", d))
                .unwrap_or_else(|| "0.00".to_string()),
            "tip" => data
                .tip
                .map(|t| format!("{:.2}", t))
                .unwrap_or_else(|| "0.00".to_string()),
            "service_charge" => data
                .service_charge
                .map(|s| format!("{:.2}", s))
                .unwrap_or_else(|| "0.00".to_string()),
            "service_rate" => data
                .service_rate
                .map(|r| format!("{:.0}", r))
                .unwrap_or_else(|| "0".to_string()),
            "total" => format!("{:.2}", data.total),
            "payment_method" => data.payment_method.clone(),
            "change" => data
                .change
                .map(|c| format!("{:.2}", c))
                .unwrap_or_else(|| "0.00".to_string()),
            "footer_message" => data.footer_message.clone().unwrap_or_default(),
            "farewell_message" => data.farewell_message.clone().unwrap_or_default(),
            "receipt_url" => data.receipt_url.clone().unwrap_or_default(),
            _ => {
                // Try custom fields
                if let Some(value) = data.custom.get(name) {
                    match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Null => String::new(),
                        _ => value.to_string().trim_matches('"').to_string(),
                    }
                } else {
                    String::new()
                }
            }
        }
    }

    /// Evaluate simple conditions
    fn should_render(&self, condition: &Option<String>, data: &ReceiptData) -> bool {
        if let Some(cond) = condition {
            self.evaluate_condition(cond, data)
        } else {
            true
        }
    }

    /// Simple condition evaluator
    fn evaluate_condition(&self, condition: &str, data: &ReceiptData) -> bool {
        // Handle comparison operators
        if condition.contains(">") {
            let parts: Vec<&str> = condition.split(">").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let var_value = self.get_variable_value(parts[0], data);
                if let Ok(num) = var_value.parse::<f64>() {
                    if let Ok(threshold) = parts[1].parse::<f64>() {
                        return num > threshold;
                    }
                }
            }
        } else if condition.contains("!=") {
            let parts: Vec<&str> = condition.split("!=").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let var_value = self.get_variable_value(parts[0], data);
                let compare_value = parts[1].trim_matches('"').trim_matches('\'');
                if compare_value == "null" {
                    return !var_value.is_empty();
                }
                return var_value != compare_value;
            }
        } else if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let var_value = self.get_variable_value(parts[0], data);
                let compare_value = parts[1].trim_matches('"').trim_matches('\'');
                if compare_value == "true" {
                    return var_value == "true" || var_value == "1";
                } else if compare_value == "false" {
                    return var_value == "false" || var_value == "0" || var_value.is_empty();
                }
                return var_value == compare_value;
            }
        } else if condition.contains(".length") {
            // Handle array length conditions like "items.length > 0"
            let parts: Vec<&str> = condition.split(">").map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let array_name = parts[0].trim_end_matches(".length");
                let items = self.get_data_source_items(array_name, data);
                if let Ok(threshold) = parts[1].parse::<usize>() {
                    return items.len() > threshold;
                }
            }
        }

        true // Default to showing if condition can't be evaluated
    }
}

// ==================== Print Commands ====================

/// Print commands for building output without direct printer access
#[derive(Debug, Clone)]
pub enum PrintCommand {
    Init,
    WriteLine(String),
    Feed(u8),
    Cut,
    Bold(bool),
    Underline(bool),
    Reverse(bool),
    Size(u8, u8),
    Align(String),
    QRCode {
        content: String,
        size: u8,
    },
    Barcode {
        content: String,
        format: String,
        height: u8,
        width: u8,
        show_text: bool,
    },
}

// ==================== Template Loading ====================

/// Load and parse a template from JSON
#[allow(dead_code)]
pub fn load_template(json: &str) -> Result<ReceiptTemplate, serde_json::Error> {
    serde_json::from_str(json)
}

/// Convert TypeScript template exports to JSON format for parsing
#[allow(dead_code)]
pub fn parse_template_export(content: &str, template_id: &str) -> Option<String> {
    // Find the template object in the export
    let pattern = format!(r#""{}":\s*\{{"#, template_id);
    let re = regex::Regex::new(&pattern).ok()?;

    if let Some(start_match) = re.find(content) {
        let start_idx = start_match.start() + template_id.len() + 4; // Skip '"id": {'

        // Count braces to find the end
        let mut brace_count = 1;
        let mut end_idx = start_idx;

        for (i, c) in content[start_idx..].chars().enumerate() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_idx = start_idx + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        if brace_count == 0 {
            let template_content = &content[start_idx - 1..end_idx];
            // This would need more processing to convert TS to valid JSON
            // For now, return as-is (requires proper JS/TS to JSON conversion)
            return Some(template_content.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let renderer = TemplateRenderer::new(48);
        let data = ReceiptData {
            order_id: "12345".to_string(),
            timestamp: "2024-01-15 14:30:00".to_string(),
            store_name: Some("Test Store".to_string()),
            total: 99.99,
            payment_method: "VISA".to_string(),
            ..Default::default()
        };

        let result =
            renderer.substitute_variables("Order #{{order_id}} - Total: ${{total}}", &data);
        assert_eq!(result, "Order #12345 - Total: $99.99");
    }

    #[test]
    fn test_letter_spacing() {
        let renderer = TemplateRenderer::new(48);
        let result = renderer.apply_letter_spacing("TEST", 2);
        assert_eq!(result, "T  E  S  T");
    }

    #[test]
    fn test_condition_evaluation() {
        let renderer = TemplateRenderer::new(48);
        let data = ReceiptData {
            order_id: "12345".to_string(),
            timestamp: "2024-01-15".to_string(),
            discount: Some(10.0),
            payment_method: "CASH".to_string(),
            ..Default::default()
        };

        assert!(renderer.evaluate_condition("discount > 0", &data));
        assert!(!renderer.evaluate_condition("discount > 100", &data));
    }

    #[test]
    fn test_template_parsing() {
        let json = r#"{
            "id": "test",
            "name": "Test Template",
            "version": "1.0.0",
            "layout": {
                "sections": [
                    {
                        "type": "header",
                        "elements": [
                            {"type": "text", "content": "Hello World", "align": "center"}
                        ]
                    }
                ]
            }
        }"#;

        let template = load_template(json).expect("Failed to parse template");
        assert_eq!(template.id, "test");
        assert_eq!(template.name, "Test Template");
        assert_eq!(template.layout.sections.len(), 1);
    }
}
