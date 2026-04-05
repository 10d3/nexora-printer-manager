// Nexora POS Printer Manager
// Complete ESC/POS printer management with USB, Network, and LPT support

#![windows_subsystem = "windows"]

use serde::{Deserialize, Serialize};
use slint::{CloseRequestResponse, Model};
use std::env;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder,
};

mod autostart;
mod http_server;
mod image_print;
mod template_render;

pub use template_render::{
    Element, ReceiptData, ReceiptItem, ReceiptTemplate, Section, TemplateLayout, TemplateRenderer,
};

slint::include_modules!();

// ==================== Configuration Models ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub connection_type: String,
    pub device_path: String,
    pub store_name: String,
    pub store_address: String,
    pub footer_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LineItem {
    pub name: String,
    pub quantity: u32,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Receipt {
    pub order_id: String,
    pub timestamp: String,
    pub items: Vec<LineItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub payment_method: String,
}

// ==================== Printer Manager ====================

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PrinterConnection {
    USB(String),     // port name (COMx or \\.\usbxxx)
    Network(String), // IP:port
    LPT(String),     // LPTx
    System(String),  // Windows Printer Name (e.g., "POS-80")
    Console,
}

pub struct PrinterManager {
    connection: Option<PrinterConnection>,
    pub config: Option<PrinterConfig>,
    pub template_cache: std::collections::HashMap<String, ReceiptTemplate>,
    pub active_template_id: Option<String>,
}

impl PrinterManager {
    pub fn new() -> Self {
        Self {
            connection: None,
            config: None,
            template_cache: std::collections::HashMap::new(),
            active_template_id: None,
        }
    }

    pub fn connect(&mut self, config: PrinterConfig) -> Result<(), String> {
        log::info!(
            "Connecting to {} printer at {}",
            config.connection_type,
            config.device_path
        );

        match config.connection_type.as_str() {
            "USB" => {
                // Check if this looks like a port or a printer name
                if config.device_path.starts_with(r"\\.\") || config.device_path.starts_with("COM")
                {
                    // It's a port path
                    #[cfg(target_os = "windows")]
                    {
                        // ... existing port opening logic ...
                        let mut wide: Vec<u16> = config.device_path.encode_utf16().collect();
                        wide.push(0);

                        const GENERIC_WRITE: u32 = 0x40000000;
                        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
                        use windows_sys::Win32::Storage::FileSystem::{
                            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
                            OPEN_EXISTING,
                        };

                        let handle = unsafe {
                            CreateFileW(
                                wide.as_ptr(),
                                GENERIC_WRITE,
                                FILE_SHARE_READ | FILE_SHARE_WRITE,
                                std::ptr::null(),
                                OPEN_EXISTING,
                                FILE_ATTRIBUTE_NORMAL,
                                std::ptr::null_mut(),
                            )
                        };

                        if handle == INVALID_HANDLE_VALUE {
                            let _err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                            // If port fails, try to see if it's actually a system printer name
                            self.connection =
                                Some(PrinterConnection::System(config.device_path.clone()));
                        } else {
                            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
                            self.connection =
                                Some(PrinterConnection::USB(config.device_path.clone()));
                        }
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        self.connection = Some(PrinterConnection::USB(config.device_path.clone()));
                    }
                } else {
                    // It's likely a Windows printer name (e.g. "POS-80")
                    self.connection = Some(PrinterConnection::System(config.device_path.clone()));
                }
            }
            "Network" => {
                // Validate IP:Port format
                if !config.device_path.contains(':') {
                    // Try to append default port if missing
                    let mut path = config.device_path.clone();
                    path.push_str(":9100");
                    self.connection = Some(PrinterConnection::Network(path));
                } else {
                    self.connection = Some(PrinterConnection::Network(config.device_path.clone()));
                }
            }
            "LPT" => {
                #[cfg(target_os = "windows")]
                {
                    self.connection = Some(PrinterConnection::LPT(config.device_path.clone()));
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

    pub fn print_raw(&mut self, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let connection = self.connection.as_ref().ok_or("Printer not connected")?;

        match connection {
            PrinterConnection::Console => {
                println!("[Image data: {} bytes]", bytes.len());
            }
            PrinterConnection::USB(path) | PrinterConnection::LPT(path) => {
                let path = path.clone();
                #[cfg(target_os = "windows")]
                self.write_to_device_windows(&path, bytes).map_err(|e| e)?;
                #[cfg(not(target_os = "windows"))]
                {
                    use std::io::Write;
                    let mut file = std::fs::File::create(&path)?;
                    file.write_all(bytes)?;
                }
            }
            PrinterConnection::Network(addr) => {
                use std::io::Write;
                let addr = addr.clone();
                let mut stream = std::net::TcpStream::connect(&addr)?;
                stream.write_all(bytes)?;
            }
            PrinterConnection::System(name) => {
                let name = name.clone();
                #[cfg(target_os = "windows")]
                self.write_to_system_printer_windows(&name, bytes)
                    .map_err(|e| e)?;
                #[cfg(not(target_os = "windows"))]
                return Err("System printer only supported on Windows".into());
            }
        }

        Ok(())
    }

    pub fn print_with_template(&mut self, data: &ReceiptData) -> Result<(), String> {
        let template_id = self
            .active_template_id
            .as_ref()
            .ok_or("No active template set")?;
        let template = self
            .template_cache
            .get(template_id)
            .ok_or("Template not found in cache")?;

        let paper_width = template.paper_width.unwrap_or(48);
        let renderer = TemplateRenderer::new(paper_width);
        let commands = renderer.render_to_commands(template, data)?;

        self.execute_commands(commands)
    }

    fn execute_commands(&self, commands: Vec<template_render::PrintCommand>) -> Result<(), String> {
        let connection = self.connection.as_ref().ok_or("Printer not connected")?;

        // Convert commands to raw ESC/POS bytes
        let mut bytes = Vec::new();
        for cmd in commands {
            match cmd {
                template_render::PrintCommand::Init => bytes.extend_from_slice(&[0x1B, 0x40]),
                template_render::PrintCommand::Write(s) => {
                    bytes.extend_from_slice(s.as_bytes());
                }
                template_render::PrintCommand::WriteLine(s) => {
                    bytes.extend_from_slice(s.as_bytes());
                    bytes.push(b'\n');
                }
                template_render::PrintCommand::Feed(n) => {
                    for _ in 0..n {
                        bytes.push(b'\n');
                    }
                }
                template_render::PrintCommand::Cut => {
                    bytes.extend_from_slice(&[0x1D, 0x56, 0x01]);
                }
                template_render::PrintCommand::Bold(on) => {
                    bytes.extend_from_slice(&[0x1B, 0x45, if on { 1 } else { 0 }]);
                }
                template_render::PrintCommand::Underline(on) => {
                    bytes.extend_from_slice(&[0x1B, 0x2D, if on { 1 } else { 0 }]);
                }
                template_render::PrintCommand::Reverse(on) => {
                    bytes.extend_from_slice(&[0x1D, 0x42, if on { 1 } else { 0 }]);
                }
                template_render::PrintCommand::Size(w, h) => {
                    let size = ((w.saturating_sub(1) & 0x07) << 4) | (h.saturating_sub(1) & 0x07);
                    bytes.extend_from_slice(&[0x1D, 0x21, size]);
                }
                template_render::PrintCommand::Align(align) => {
                    let n = match align.to_lowercase().as_str() {
                        "center" => 1,
                        "right" => 2,
                        _ => 0,
                    };
                    bytes.extend_from_slice(&[0x1B, 0x61, n]);
                }
                template_render::PrintCommand::QRCode { content, size: _ } => {
                    // Simplified QR code (requires actual implementation for different printers)
                    log::warn!("QR Code not fully implemented in raw bytes");
                    bytes.extend_from_slice(format!("[QR: {}]", content).as_bytes());
                    bytes.push(b'\n');
                }
                template_render::PrintCommand::Barcode { content, .. } => {
                    log::warn!("Barcode not fully implemented in raw bytes");
                    bytes.extend_from_slice(format!("[Barcode: {}]", content).as_bytes());
                    bytes.push(b'\n');
                }
                template_render::PrintCommand::Image(img_bytes) => {
                    bytes.extend_from_slice(&img_bytes);
                }
            }
        }

        match connection {
            PrinterConnection::Console => {
                if let Ok(s) = String::from_utf8(bytes) {
                    println!("{}", s);
                }
                Ok(())
            }
            PrinterConnection::USB(path) | PrinterConnection::LPT(path) => {
                #[cfg(target_os = "windows")]
                {
                    self.write_to_device_windows(path, &bytes)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    use std::io::Write;
                    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
                    file.write_all(&bytes).map_err(|e| e.to_string())?;
                    Ok(())
                }
            }
            PrinterConnection::Network(addr) => {
                use std::io::Write;
                let mut stream = std::net::TcpStream::connect(addr).map_err(|e| e.to_string())?;
                stream.write_all(&bytes).map_err(|e| e.to_string())?;
                Ok(())
            }
            PrinterConnection::System(name) => {
                #[cfg(target_os = "windows")]
                {
                    self.write_to_system_printer_windows(name, &bytes)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err("System printer printing is only supported on Windows.".to_string())
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn write_to_system_printer_windows(&self, name: &str, data: &[u8]) -> Result<(), String> {
        use windows_sys::Win32::Graphics::Printing::{
            ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
            StartPagePrinter, WritePrinter, DOC_INFO_1W, PRINTER_HANDLE,
        };

        let mut wide_name: Vec<u16> = name.encode_utf16().collect();
        wide_name.push(0);

        let mut h_printer: PRINTER_HANDLE = unsafe { std::mem::zeroed() };
        let success = unsafe {
            OpenPrinterW(
                wide_name.as_ptr() as *mut u16,
                &mut h_printer,
                std::ptr::null_mut(),
            )
        };

        if success == 0 {
            return Err(format!("Could not open system printer '{}'. Please check the name in Devices and Printers.", name));
        }

        let doc_name = "Nexora Receipt\0".encode_utf16().collect::<Vec<u16>>();
        let data_type = "RAW\0".encode_utf16().collect::<Vec<u16>>();

        let doc_info = DOC_INFO_1W {
            pDocName: doc_name.as_ptr() as *mut u16,
            pOutputFile: std::ptr::null_mut(),
            pDatatype: data_type.as_ptr() as *mut u16,
        };

        let job_id = unsafe { StartDocPrinterW(h_printer, 1, &doc_info as *const DOC_INFO_1W) };

        if job_id == 0 {
            unsafe { ClosePrinter(h_printer) };
            return Err("Could not start print job via Windows Spooler.".to_string());
        }

        unsafe {
            StartPagePrinter(h_printer);
            let mut written = 0;
            WritePrinter(
                h_printer,
                data.as_ptr() as *const _,
                data.len() as u32,
                &mut written,
            );
            EndPagePrinter(h_printer);
            EndDocPrinter(h_printer);
            ClosePrinter(h_printer);
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn write_to_device_windows(&self, path: &str, data: &[u8]) -> Result<(), String> {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
            OPEN_EXISTING,
        };

        // Standard generic access rights
        const GENERIC_READ: u32 = 0x80000000;
        const GENERIC_WRITE: u32 = 0x40000000;

        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        let mut handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            // Try just WRITE if BOTH fails
            handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    std::ptr::null_mut(),
                )
            };
        }

        if handle == INVALID_HANDLE_VALUE {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(format!("Cannot open {}: Windows error code {}", path, err));
        }

        let mut written: u32 = 0;
        let success = unsafe {
            WriteFile(
                handle,
                data.as_ptr(),
                data.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        };

        unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };

        if success == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            return Err(format!(
                "Failed to write to {}: Windows error code {}",
                path, err
            ));
        }

        Ok(())
    }

    pub fn print_test(&mut self) -> Result<(), String> {
        let config = self.config.as_ref().ok_or("No configuration found")?;

        let commands = vec![
            template_render::PrintCommand::Init,
            template_render::PrintCommand::Align("center".to_string()),
            template_render::PrintCommand::Size(2, 2),
            template_render::PrintCommand::Bold(true),
            template_render::PrintCommand::WriteLine("NEXORA POS".to_string()),
            template_render::PrintCommand::Size(1, 1),
            template_render::PrintCommand::Bold(false),
            template_render::PrintCommand::WriteLine("Test Print".to_string()),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::Align("left".to_string()),
            template_render::PrintCommand::WriteLine(
                "================================".to_string(),
            ),
            template_render::PrintCommand::WriteLine(format!(
                "Connection: {}",
                config.connection_type
            )),
            template_render::PrintCommand::WriteLine(format!("Device: {}", config.device_path)),
            template_render::PrintCommand::WriteLine(format!("Store: {}", config.store_name)),
            template_render::PrintCommand::WriteLine(
                "================================".to_string(),
            ),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::WriteLine("[OK] Connection Successful".to_string()),
            template_render::PrintCommand::WriteLine(format!(
                "Date: {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            )),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::WriteLine("Testing text output:".to_string()),
            template_render::PrintCommand::WriteLine("Regular Text".to_string()),
            template_render::PrintCommand::Bold(true),
            template_render::PrintCommand::WriteLine("Bold Text".to_string()),
            template_render::PrintCommand::Bold(false),
            template_render::PrintCommand::Reverse(true),
            template_render::PrintCommand::WriteLine("Inverted Text".to_string()),
            template_render::PrintCommand::Reverse(false),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::WriteLine("ESC/POS Compatible [OK]".to_string()),
            template_render::PrintCommand::Feed(3),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::Cut,
        ];

        self.execute_commands(commands)
    }

    pub fn print_receipt(&mut self, receipt: &Receipt) -> Result<(), String> {
        // Convert legacy Receipt to ReceiptData and use print_with_template if possible
        // Or just build commands manually for legacy support
        let data = ReceiptData {
            store_name: Some(
                self.config
                    .as_ref()
                    .map(|c| c.store_name.clone())
                    .unwrap_or_default(),
            ),
            store_address: Some(
                self.config
                    .as_ref()
                    .map(|c| c.store_address.clone())
                    .unwrap_or_default(),
            ),
            order_id: receipt.order_id.clone(),
            timestamp: receipt.timestamp.clone(),
            items: receipt
                .items
                .iter()
                .map(|item| ReceiptItem {
                    name: item.name.clone(),
                    quantity: item.quantity,
                    price: item.price,
                    total: item.quantity as f64 * item.price,
                    modifiers: None,
                })
                .collect(),
            subtotal: receipt.subtotal,
            tax: receipt.tax,
            total: receipt.total,
            payment_method: receipt.payment_method.clone(),
            footer_message: Some(
                self.config
                    .as_ref()
                    .map(|c| c.footer_message.clone())
                    .unwrap_or_default(),
            ),
            ..Default::default()
        };

        // If we have an active template, use it. Otherwise, build a simple receipt.
        if self.active_template_id.is_some() {
            return self.print_with_template(&data);
        }

        let mut commands = vec![
            template_render::PrintCommand::Init,
            template_render::PrintCommand::Align("center".to_string()),
            template_render::PrintCommand::Bold(true),
            template_render::PrintCommand::WriteLine(data.store_name.unwrap_or_default()),
            template_render::PrintCommand::Bold(false),
            template_render::PrintCommand::WriteLine(data.store_address.unwrap_or_default()),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::Align("left".to_string()),
            template_render::PrintCommand::WriteLine(format!("Order #{}", data.order_id)),
            template_render::PrintCommand::WriteLine(data.timestamp),
            template_render::PrintCommand::WriteLine("-".repeat(32)),
        ];

        for item in &data.items {
            commands.push(template_render::PrintCommand::WriteLine(item.name.clone()));
            let qty_price = format!("{}x ${:.2}", item.quantity, item.price);
            let total = format!("${:.2}", item.total);
            let spaces = 32_usize.saturating_sub(qty_price.len() + total.len());
            commands.push(template_render::PrintCommand::WriteLine(format!(
                "{}{}{}",
                qty_price,
                " ".repeat(spaces),
                total
            )));
        }

        commands.extend_from_slice(&[
            template_render::PrintCommand::WriteLine("-".repeat(32)),
            template_render::PrintCommand::WriteLine(format!(
                "Subtotal:                ${:.2}",
                data.subtotal
            )),
            template_render::PrintCommand::WriteLine(format!(
                "Tax:                     ${:.2}",
                data.tax
            )),
            template_render::PrintCommand::Bold(true),
            template_render::PrintCommand::WriteLine(format!(
                "TOTAL:                   ${:.2}",
                data.total
            )),
            template_render::PrintCommand::Bold(false),
            template_render::PrintCommand::WriteLine("-".repeat(32)),
            template_render::PrintCommand::WriteLine(format!("Payment: {}", data.payment_method)),
            template_render::PrintCommand::Feed(1),
            template_render::PrintCommand::Align("center".to_string()),
            template_render::PrintCommand::WriteLine(data.footer_message.unwrap_or_default()),
            template_render::PrintCommand::WriteLine("Powered by Nexora POS".to_string()),
            template_render::PrintCommand::Feed(3),
            template_render::PrintCommand::Cut,
        ]);

        self.execute_commands(commands)
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
                        format!("USB Serial (VID:{:04x} PID:{:04x})", info.vid, info.pid)
                    }
                    _ => "Serial Port".to_string(),
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

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        // 1. Find all installed printers from Registry (Most reliable for usb00X)
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(printers_key) =
            hkcu.open_subkey("Software\\Microsoft\\Windows NT\\CurrentVersion\\Devices")
        {
            for (name, value) in printers_key.enum_values().flatten() {
                let value_str = value.to_string();
                let port = value_str.split(',').nth(1).unwrap_or("").trim();

                devices.push(Device {
                    path: name.clone().into(),
                    description: format!("Printer on port: {}", port).into(),
                    r#type: "USB".into(),
                });
            }
        }

        // 2. Try to find raw ports from Registry
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(ports_key) =
            hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Ports")
        {
            for (name, _) in ports_key.enum_values().flatten() {
                if name.starts_with("usb") || name.starts_with("USB") || name.starts_with("LPT") {
                    let path = if name.starts_with(r"\\") {
                        name.clone()
                    } else {
                        format!(r"\\.\{}", name)
                    };

                    // Only add if not already added as a printer
                    if !devices.iter().any(|d| d.description.contains(&name)) {
                        devices.push(Device {
                            path: path.into(),
                            description: format!("System Port: {}", name).into(),
                            r#type: if name.starts_with("LPT") {
                                "LPT".into()
                            } else {
                                "USB".into()
                            },
                        });
                    }
                }
            }
        }

        // 3. Fallback hint
        if devices.is_empty() {
            devices.push(Device {
                path: r"\\.\usb001".into(),
                description: "Manual Entry (Check Devices & Printers)".into(),
                r#type: "USB".into(),
            });
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

fn load_tray_icon() -> tray_icon::Icon {
    let paths = ["assets/nexora.png", "assets/favicon.png"];

    for path in paths {
        let icon_path = std::path::Path::new(path);
        if icon_path.exists() {
            log::info!("Attempting to load tray icon from {:?}", path);
            match image::open(icon_path) {
                Ok(image_data) => {
                    let image_rgba = image_data.into_rgba8();
                    let (width, height) = image_rgba.dimensions();
                    let rgba = image_rgba.into_raw();
                    match tray_icon::Icon::from_rgba(rgba, width, height) {
                        Ok(icon) => return icon,
                        Err(e) => log::warn!("Failed to create icon from {:?}: {}", path, e),
                    }
                }
                Err(e) => log::warn!("Failed to decode icon image {:?}: {}", path, e),
            }
        }
    }

    log::warn!("No valid tray icon found, using fallback empty icon");
    tray_icon::Icon::from_rgba(vec![0; 4], 1, 1).unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let mut log_dir = directories::ProjectDirs::from("com", "nexora", "printer-manager")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    std::fs::create_dir_all(&log_dir).unwrap_or_default();
    let log_file = log_dir.join("nexora.log");

    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .unwrap(),
    )
    .unwrap_or_default();

    // Create printer manager
    let printer_manager = Arc::new(Mutex::new(PrinterManager::new()));

    // Keep the tray icon alive
    let mut _tray_icon_handle = None;

    let result = async {
        let args: Vec<String> = env::args().collect();
        let minimized = args.contains(&"--minimized".to_string());

        log::info!("Starting Nexora Printer Manager v1.4.0");

        // Setup Auto-launch
        let autostart = autostart::Autostart::new();
        // Setup default autostart if first time, or rely on installer
        if !autostart.is_enabled() && false {
            // We don't force it here, leave to user or installer
            let _ = autostart.enable();
        }
        let autostart_enabled = autostart.is_enabled();

        // Create UI
        let ui = MainWindow::new()?;

        // Create a second hidden window to keep the event loop alive when the main window is hidden
        let _keep_alive = MainWindow::new()?;

        // Setup System Tray
        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Show Manager", true, None);
        let autostart_item = MenuItem::new("Toggle Launch at Startup", true, None);
        let quit_item = MenuItem::new("Exit", true, None);

        tray_menu.append_items(&[
            &show_item,
            &autostart_item,
            &PredefinedMenuItem::separator(),
            &quit_item,
        ])?;

        let icon = load_tray_icon();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("Nexora Printer Manager")
            .with_icon(icon)
            .build()?;

        _tray_icon_handle = Some(tray_icon);

        // Start HTTP server
        let printer_manager_clone = Arc::clone(&printer_manager);
        tokio::spawn(async move {
            if let Err(e) = http_server::start_server(printer_manager_clone, 8080).await {
                log::error!("HTTP server error: {}", e);
            } else {
                log::info!("HTTP server started on port 8080");
            }
        });

        // Handle Tray Events
        let ui_weak = ui.as_weak();
        let show_id = show_item.id().clone();
        let autostart_id = autostart_item.id().clone();
        let quit_id = quit_item.id().clone();

        std::thread::spawn(move || {
            let menu_channel = MenuEvent::receiver();
            let autostart = autostart::Autostart::new();

            loop {
                if let Ok(event) = menu_channel.recv() {
                    if event.id == show_id {
                        let ui_weak_clone = ui_weak.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_clone.upgrade() {
                                log::info!("Restoring window from tray");
                                #[cfg(target_os = "windows")]
                                {
                                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                                        FindWindowW, ShowWindow, SW_SHOW,
                                    };
                                    let title: Vec<u16> =
                                        "Nexora Printer Manager\0".encode_utf16().collect();
                                    let hwnd =
                                        unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                                    if hwnd != std::ptr::null_mut() {
                                        unsafe { ShowWindow(hwnd, SW_SHOW) };
                                    }
                                }
                                ui.show().unwrap();
                            }
                        });
                    } else if event.id == autostart_id {
                        let _ = autostart.toggle();
                    } else if event.id == quit_id {
                        log::info!("Exiting application via tray menu");
                        let _ = slint::invoke_from_event_loop(|| {
                            slint::quit_event_loop().unwrap();
                        });
                        break;
                    }
                }
            }
        });

        // Handle Window Close (Minimize to Tray)
        {
            let ui_handle = ui.as_weak();

            ui.window().on_close_requested(move || {
                if let Some(ui) = ui_handle.upgrade() {
                    log::info!("Close requested: hiding window natively to tray");

                    #[cfg(target_os = "windows")]
                    {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            FindWindowW, ShowWindow, SW_HIDE,
                        };
                        let title: Vec<u16> = "Nexora Printer Manager\0".encode_utf16().collect();
                        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                        if hwnd != std::ptr::null_mut() {
                            unsafe { ShowWindow(hwnd, SW_HIDE) };
                        }
                    }

                    // Do NOT call `ui.hide()` here, as it triggers Slint's automatic quit
                    // when the visible window count reaches zero.
                    CloseRequestResponse::KeepWindowShown
                } else {
                    CloseRequestResponse::HideWindow
                }
            });
        }

        // Load saved configuration
        if let Ok(Some(config)) = load_config() {
            ui.set_selected_connection_type(config.connection_type.clone().into());
            ui.set_selected_device(config.device_path.clone().into());
            ui.set_status_message("Configuration loaded successfully".into());
            log::info!("Loaded saved configuration");
        }

        if !minimized {
            ui.show()?;
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

                // Load current config to keep store name, etc. if they exist
                let current_config = load_config().ok().flatten();

                let config = PrinterConfig {
                    connection_type: conn_type.to_string(),
                    device_path: device.to_string(),
                    store_name: current_config
                        .as_ref()
                        .map(|c| c.store_name.clone())
                        .unwrap_or_else(|| "Nexora POS".to_string()),
                    store_address: current_config
                        .as_ref()
                        .map(|c| c.store_address.clone())
                        .unwrap_or_else(|| "Main Branch".to_string()),
                    footer_message: current_config
                        .as_ref()
                        .map(|c| c.footer_message.clone())
                        .unwrap_or_else(|| "Thank you for your visit!".to_string()),
                };

                let mut manager = manager.lock().unwrap();

                if let Err(e) = manager.connect(config.clone()) {
                    ui.set_is_connected(false);
                    ui.set_status_message(format!("✗ Connection failed: {}", e).into());
                    log::error!("Connection failed: {}", e);
                } else {
                    ui.set_is_connected(true);
                    ui.set_status_message("✓ Printer connected successfully!".into());

                    // Save configuration
                    if let Err(e) = save_config(&config) {
                        log::warn!("Failed to save config: {}", e);
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

                if let Err(e) = manager.print_test() {
                    ui.set_status_message(format!("✗ Print failed: {}", e).into());
                    log::error!("Test print failed: {}", e);
                } else {
                    ui.set_status_message("✓ Test page printed successfully!".into());
                }

                ui.set_is_loading(false);
            });
        }

        // Save settings callback
        {
            let ui_handle = ui.as_weak();

            ui.on_save_settings(move || {
                let ui = ui_handle.unwrap();

                // Load current config to keep store name, etc. if they exist
                let current_config = load_config().ok().flatten();

                let config = PrinterConfig {
                    connection_type: ui.get_selected_connection_type().to_string(),
                    device_path: ui.get_selected_device().to_string(),
                    store_name: current_config
                        .as_ref()
                        .map(|c| c.store_name.clone())
                        .unwrap_or_else(|| "Nexora POS".to_string()),
                    store_address: current_config
                        .as_ref()
                        .map(|c| c.store_address.clone())
                        .unwrap_or_else(|| "Main Branch".to_string()),
                    footer_message: current_config
                        .as_ref()
                        .map(|c| c.footer_message.clone())
                        .unwrap_or_else(|| "Thank you for your visit!".to_string()),
                };

                if let Err(e) = save_config(&config) {
                    ui.set_status_message(format!("✗ Failed to save: {}", e).into());
                    log::error!("Save failed: {}", e);
                } else {
                    ui.set_status_message("✓ Settings saved successfully!".into());
                }
            });
        }

        // Run the application
        let _dummy_timer = slint::Timer::default();
        _dummy_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(3600),
            || {
                log::debug!("Heartbeat to keep event loop alive");
            },
        );

        slint::run_event_loop()?;
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    if let Err(e) = result {
        log::error!("Application error: {}", e);
        return Err(e);
    }

    Ok(())
}

