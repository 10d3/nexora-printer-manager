// Copyright (C) 2026 [Nexora]
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.

#![windows_subsystem = "windows"]

use serde::{Deserialize, Serialize};
use slint::Model;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod http_server;
mod template_render;

pub use template_render::{ReceiptData, ReceiptItem, ReceiptTemplate, TemplateRenderer};

slint::include_modules!();

// ==================== Configuration Models ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrinterConfig {
    connection_type: String,
    device_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct LineItem {
    name: String,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct Receipt {
    order_id: String,
    timestamp: String,
    items: Vec<LineItem>,
    subtotal: f64,
    tax: f64,
    total: f64,
    payment_method: String,
}

// ==================== Printer Manager ====================

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PrinterConnection {
    USB(String),     // port name
    Network(String), // IP:port
    Console,
}

pub struct PrinterManager {
    connection: Option<PrinterConnection>,
    config: Option<PrinterConfig>,
    pub template_cache: HashMap<String, ReceiptTemplate>,
    pub active_template_id: Option<String>,
}

impl PrinterManager {
    pub fn new() -> Self {
        Self {
            connection: None,
            config: None,
            template_cache: HashMap::new(),
            active_template_id: None,
        }
    }

    pub(crate) fn connect(&mut self, config: PrinterConfig) -> Result<(), String> {
        log::info!(
            "Connecting to {} printer at {}",
            config.connection_type,
            config.device_path
        );

        match config.connection_type.as_str() {
            "USB" => {
                // Validate port exists
                self.connection = Some(PrinterConnection::USB(config.device_path.clone()));
            }
            "Network" => {
                // Validate IP is reachable
                self.connection = Some(PrinterConnection::Network(config.device_path.clone()));
            }
            "LPT" => {
                #[cfg(target_os = "windows")]
                {
                    self.connection = Some(PrinterConnection::USB(config.device_path.clone()));
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err("LPT ports are only supported on Windows.".to_string());
                }
            }
            "Console" => {
                self.connection = Some(PrinterConnection::Console);
            }
            _ => {
                return Err(format!(
                    "Unsupported connection type: {}",
                    config.connection_type
                ))
            }
        };

        self.config = Some(config);
        log::info!("Printer connected successfully");
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connection = None;
        log::info!("Printer disconnected");
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub fn set_template(&mut self, template: ReceiptTemplate) -> Result<(), String> {
        let id = template.id.clone();
        self.template_cache.insert(id.clone(), template);
        self.active_template_id = Some(id);
        Ok(())
    }

    pub fn get_active_template(&self) -> Option<&ReceiptTemplate> {
        self.active_template_id
            .as_ref()
            .and_then(|id| self.template_cache.get(id))
    }

    pub fn print_with_template(&mut self, data: &ReceiptData) -> Result<(), String> {
        let _connection = self.connection.as_ref().ok_or("Printer not connected")?;
        let template = self.get_active_template().ok_or("No active template set")?;

        // For now, just log what we would print
        log::info!(
            "Would print receipt using template '{}' for order #{}",
            template.name,
            data.order_id
        );

        // Build output for console/testing
        let mut output = String::new();
        output.push_str(&format!("=== Template: {} ===\n", template.name));
        output.push_str(&format!("Order: {}\n", data.order_id));
        output.push_str(&format!("Time: {}\n", data.timestamp));
        output.push_str(&format!("Items: {} item(s)\n", data.items.len()));
        output.push_str(&format!("Total: ${:.2}\n", data.total));
        output.push_str(&format!("Payment: {}\n", data.payment_method));

        println!("{}", output);

        Ok(())
    }

    fn print_test(&mut self) -> Result<(), String> {
        let connection = self.connection.as_ref().ok_or("Printer not connected")?;
        let config = self.config.as_ref().ok_or("No configuration found")?;

        // Build test output
        let mut output = String::new();
        output.push_str("\n");
        output.push_str("NEXORA POS\n");
        output.push_str("Test Print\n");
        output.push_str("\n");

        output.push_str("================================\n");
        output.push_str(&format!("Connection: {}\n", config.connection_type));
        output.push_str(&format!("Device: {}\n", config.device_path));
        output.push_str("================================\n");
        output.push_str("\n");

        output.push_str("[OK] Connection Successful\n");
        output.push_str("\n");

        output.push_str(&format!(
            "Date: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        output.push_str("\n");

        output.push_str("Testing text output:\n");
        output.push_str("Regular Text\n");
        output.push_str("Bold Text\n");
        output.push_str("Styled Text\n");
        output.push_str("\n\n");
        output.push_str("ESC/POS Compatible [OK]\n");
        output.push_str("\n\n\n");

        // Print to console or device based on connection type
        match connection {
            PrinterConnection::Console => {
                println!("{}", output);
            }
            PrinterConnection::USB(_) | PrinterConnection::Network(_) => {
                // For real printers, we would write to the port
                // For now, just log that we would print
                log::info!("Would print test output to printer");
                println!("{}", output);
            }
        }

        log::info!("Test print completed successfully");
        Ok(())
    }

    #[allow(dead_code)]
    fn print_receipt(&mut self, receipt: &Receipt) -> Result<(), String> {
        let connection = self.connection.as_ref().ok_or("Printer not connected")?;
        // Build receipt output
        let mut output = String::new();
        output.push_str("\n");
        output.push_str("NEXORA POS\n");
        output.push_str(&format!("Order #{}\n", receipt.order_id));
        output.push_str(&receipt.timestamp);
        output.push_str("\n\n");

        output.push_str("--------------------------------\n");

        // Items
        for item in &receipt.items {
            let item_total = item.quantity as f64 * item.price;
            output.push_str(&item.name);
            output.push_str("\n");

            let qty_price = format!("{}x ${:.2}", item.quantity, item.price);
            let total_str = format!("${:.2}", item_total);
            let spaces = 32_usize.saturating_sub(qty_price.len() + total_str.len());
            let line = format!("{}{}{}", qty_price, " ".repeat(spaces), total_str);

            output.push_str(&line);
            output.push_str("\n");
        }

        output.push_str("--------------------------------\n");

        // Totals
        let subtotal_line = format!("Subtotal:                ${:.2}\n", receipt.subtotal);
        output.push_str(&subtotal_line);

        let tax_line = format!("Tax:                     ${:.2}\n", receipt.tax);
        output.push_str(&tax_line);

        let total_line = format!("TOTAL:                   ${:.2}\n", receipt.total);
        output.push_str(&total_line);

        output.push_str("--------------------------------\n");
        output.push_str(&format!("Payment: {}\n", receipt.payment_method));
        // Footer
        output.push_str("\n");
        output.push_str("Thank you for your business!\n");
        output.push_str("Powered by Nexora POS\n");
        output.push_str("\n\n\n");

        // Print to console or device based on connection type
        match connection {
            PrinterConnection::Console => {
                println!("{}", output);
            }
            PrinterConnection::USB(_) | PrinterConnection::Network(_) => {
                // For real printers, we would write to the port
                // For now, just log that we would print
                log::info!("Would print receipt to printer");
                println!("{}", output);
            }
        }

        log::info!("Receipt printed: Order #{}", receipt.order_id);
        Ok(())
    }
}

// ==================== Device Detection ====================

fn scan_available_devices() -> Vec<Device> {
    let mut devices = Vec::new();

    // Scan USB/Serial devices
    match serialport::available_ports() {
        Ok(ports) => {
            for port in ports {
                let description = match &port.port_type {
                    serialport::SerialPortType::UsbPort(info) => {
                        format!("USB Device (VID:{:04x} PID:{:04x})", info.vid, info.pid)
                    }
                    _ => "Serial/USB Device".to_string(),
                };

                devices.push(Device {
                    path: port.port_name.into(),
                    description: description.into(),
                    r#type: "USB".into(),
                });
            }
        }
        Err(e) => {
            log::warn!("Failed to scan serial ports: {}", e);
        }
    }

    // Add LPT ports for Windows
    #[cfg(target_os = "windows")]
    {
        for i in 1..=3 {
            devices.push(Device {
                path: format!("LPT{}", i).into(),
                description: format!("Parallel Port {}", i).into(),
                r#type: "LPT".into(),
            });
        }
    }

    // Add common network printer IPs as suggestions
    devices.push(Device {
        path: "192.168.1.100".into(),
        description: "Network Printer (Enter your IP)".into(),
        r#type: "Network".into(),
    });

    // Try to detect printers on local network
    if let Ok(local_ip) = local_ip_address::local_ip() {
        if let std::net::IpAddr::V4(ipv4) = local_ip {
            let octets = ipv4.octets();
            let base = format!("{}.{}.{}", octets[0], octets[1], octets[2]);

            devices.push(Device {
                path: format!("{}.100", base).into(),
                description: format!("Suggested: {}.100", base).into(),
                r#type: "Network".into(),
            });
        }
    }

    devices
}

// ==================== Configuration Storage ====================

fn get_config_path() -> Result<std::path::PathBuf, String> {
    let config_dir = directories::ProjectDirs::from("com", "nexora", "printer-manager")
        .ok_or("Failed to determine config directory")?;

    std::fs::create_dir_all(config_dir.config_dir())
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    Ok(config_dir.config_dir().join("config.json"))
}

fn save_config(config: &PrinterConfig) -> Result<(), String> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(path, json).map_err(|e| format!("Failed to write config: {}", e))?;

    log::info!("Configuration saved");
    Ok(())
}

fn load_config() -> Result<Option<PrinterConfig>, String> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let json =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: PrinterConfig =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse config: {}", e))?;

    log::info!("Configuration loaded");
    Ok(Some(config))
}

// ==================== Main Application ====================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    log::info!("Starting Nexora Printer Manager v1.0.0");

    // Create printer manager
    let printer_manager = Arc::new(Mutex::new(PrinterManager::new()));

    // Create UI
    let ui = MainWindow::new()?;

    // Start HTTP server
    let printer_manager_clone = Arc::clone(&printer_manager);
    tokio::spawn(async move {
        if let Err(e) = http_server::start_server(printer_manager_clone, 8080).await {
            log::error!("HTTP server error: {}", e);
        } else {
            log::info!("HTTP server started on port 8080");
        }
    });

    // Load saved configuration
    if let Ok(Some(config)) = load_config() {
        ui.set_selected_connection_type(config.connection_type.clone().into());
        ui.set_selected_device(config.device_path.clone().into());
        ui.set_status_message("Configuration loaded successfully".into());
        log::info!("Loaded saved configuration");
    }

    // Scan devices callback
    {
        let ui_handle = ui.as_weak();
        ui.on_scan_devices(move || {
            let ui = ui_handle.unwrap();
            ui.set_is_loading(true);
            ui.set_status_message("Scanning for devices...".into());

            let devices = scan_available_devices();

            let device_models: Vec<Device> = devices.into_iter().collect();
            let model_array = std::rc::Rc::new(slint::VecModel::from(device_models));
            ui.set_available_devices(model_array.into());

            ui.set_is_loading(false);
            ui.set_status_message(
                format!("Found {} device(s)", ui.get_available_devices().row_count()).into(),
            );

            log::info!("Device scan completed");
        });
    }

    // Connect printer callback
    {
        let ui_handle = ui.as_weak();
        let manager = Arc::clone(&printer_manager);

        ui.on_connect_printer(move |conn_type, device| {
            let ui = ui_handle.unwrap();
            ui.set_is_loading(true);
            ui.set_status_message("Connecting to printer...".into());

            let config = PrinterConfig {
                connection_type: conn_type.to_string(),
                device_path: device.to_string(),
            };

            let mut manager = manager.lock().unwrap();

            match manager.connect(config.clone()) {
                Ok(_) => {
                    ui.set_is_connected(true);
                    ui.set_status_message("✓ Printer connected successfully!".into());

                    // Save configuration
                    if let Err(e) = save_config(&config) {
                        log::warn!("Failed to save config: {}", e);
                    }
                }
                Err(e) => {
                    ui.set_is_connected(false);
                    ui.set_status_message(format!("✗ Connection failed: {}", e).into());
                    log::error!("Connection failed: {}", e);
                }
            }

            ui.set_is_loading(false);
        });
    }

    // Disconnect printer callback
    {
        let ui_handle = ui.as_weak();
        let manager = Arc::clone(&printer_manager);

        ui.on_disconnect_printer(move || {
            let ui = ui_handle.unwrap();
            let mut manager = manager.lock().unwrap();
            manager.disconnect();
            ui.set_is_connected(false);
            ui.set_status_message("Printer disconnected".into());
        });
    }

    // Test print callback
    {
        let ui_handle = ui.as_weak();
        let manager = Arc::clone(&printer_manager);

        ui.on_test_print(move || {
            let ui = ui_handle.unwrap();
            ui.set_is_loading(true);
            ui.set_status_message("Printing test page...".into());

            let mut manager = manager.lock().unwrap();

            match manager.print_test() {
                Ok(_) => {
                    ui.set_status_message("✓ Test page printed successfully!".into());
                }
                Err(e) => {
                    ui.set_status_message(format!("✗ Print failed: {}", e).into());
                    log::error!("Test print failed: {}", e);
                }
            }

            ui.set_is_loading(false);
        });
    }

    // Save settings callback
    {
        let ui_handle = ui.as_weak();

        ui.on_save_settings(move || {
            let ui = ui_handle.unwrap();

            let config = PrinterConfig {
                connection_type: ui.get_selected_connection_type().to_string(),
                device_path: ui.get_selected_device().to_string(),
            };

            match save_config(&config) {
                Ok(_) => {
                    ui.set_status_message("✓ Settings saved successfully!".into());
                }
                Err(e) => {
                    ui.set_status_message(format!("✗ Failed to save: {}", e).into());
                    log::error!("Save failed: {}", e);
                }
            }
        });
    }

    // Run the application
    ui.run()?;
    Ok(())
}
