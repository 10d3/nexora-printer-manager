// src/http_server.rs
// HTTP server for integration with Nexora POS web app using Axum

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

use crate::{PrinterManager, ReceiptData, ReceiptTemplate};

// ==================== Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    pub order_id: String,
    pub timestamp: String,
    pub items: Vec<PrintItem>,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub payment_method: String,
}

#[derive(Debug, Deserialize)]
pub struct PrintItem {
    pub name: String,
    pub quantity: u32,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SetTemplateRequest {
    pub template: ReceiptTemplate,
}

#[derive(Debug, Deserialize)]
pub struct PrintTemplateRequest {
    pub template_id: Option<String>,
    pub template: Option<ReceiptTemplate>,
    pub data: ReceiptData,
}

#[derive(Debug, Serialize)]
pub struct TemplateCacheResponse {
    pub templates: Vec<TemplateInfoResponse>,
    pub active_template_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplateInfoResponse {
    pub template_id: String,
    pub name: String,
    pub version: String,
    pub cached: bool,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub connected: bool,
    pub active_template: Option<String>,
    pub cached_templates: usize,
}

// ==================== App State ====================

pub struct AppState {
    pub printer_manager: Arc<Mutex<PrinterManager>>,
}

// ==================== Route Handlers ====================

/// Health check endpoint
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "healthy"}))
}

/// Get printer and server status
async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let manager = state.printer_manager.lock().unwrap();
    Json(StatusResponse {
        connected: manager.is_connected(),
        active_template: manager.active_template_id.clone(),
        cached_templates: manager.template_cache.len(),
    })
}

/// Legacy print endpoint (uses Receipt struct format)
async fn print_legacy(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PrintRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let mut manager = state.printer_manager.lock().unwrap();

    if !manager.is_connected() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "Printer not connected".to_string(),
        }));
    }

    // Convert to ReceiptData format for template printing
    let data = ReceiptData {
        store_name: None,
        store_address: None,
        store_phone: None,
        store_website: None,
        order_id: request.order_id.clone(),
        timestamp: request.timestamp,
        cashier_name: None,
        server_name: None,
        table_number: None,
        items: request
            .items
            .into_iter()
            .map(|item| crate::ReceiptItem {
                name: item.name,
                quantity: item.quantity,
                price: item.price,
                total: item.quantity as f64 * item.price,
            })
            .collect(),
        subtotal: request.subtotal,
        tax: request.tax,
        tax_rate: None,
        discount: None,
        tip: None,
        total: request.total,
        payment_method: request.payment_method,
        change: None,
        footer_message: None,
        receipt_url: None,
        custom: std::collections::HashMap::new(),
    };

    match manager.print_with_template(&data) {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: format!("Receipt printed (Order #{})", request.order_id),
        })),
        Err(e) => Ok(Json(ApiResponse {
            success: false,
            message: format!("Print failed: {}", e),
        })),
    }
}

/// Set/cache a template
async fn set_template(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SetTemplateRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let mut manager = state.printer_manager.lock().unwrap();
    let template_id = request.template.id.clone();

    match manager.set_template(request.template) {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: format!("Template '{}' cached and set as active", template_id),
        })),
        Err(e) => {
            log::error!("Failed to set template: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Print using template
async fn print_with_template(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PrintTemplateRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let mut manager = state.printer_manager.lock().unwrap();

    // Handle inline template if provided
    if let Some(template) = request.template {
        if let Err(e) = manager.set_template(template) {
            log::error!("Failed to set inline template: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    } else if let Some(template_id) = &request.template_id {
        // Verify template is cached
        if !manager.template_cache.contains_key(template_id) {
            return Ok(Json(ApiResponse {
                success: false,
                message: format!(
                    "Template '{}' not found in cache. Please set it first.",
                    template_id
                ),
            }));
        }

        // Set as active if not already
        if manager.active_template_id.as_ref() != Some(template_id) {
            manager.active_template_id = Some(template_id.clone());
        }
    } else if manager.active_template_id.is_none() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "No template specified and no active template set".to_string(),
        }));
    }

    // Check printer connection
    if !manager.is_connected() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "Printer not connected".to_string(),
        }));
    }

    // Print
    match manager.print_with_template(&request.data) {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: format!(
                "Receipt printed successfully (Order #{})",
                request.data.order_id
            ),
        })),
        Err(e) => {
            log::error!("Print failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                message: format!("Print failed: {}", e),
            }))
        }
    }
}

/// Get cached templates
async fn get_cached_templates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TemplateCacheResponse>, StatusCode> {
    let manager = state.printer_manager.lock().unwrap();

    let templates: Vec<TemplateInfoResponse> = manager
        .template_cache
        .iter()
        .map(|(id, template)| TemplateInfoResponse {
            template_id: id.clone(),
            name: template.name.clone(),
            version: template.version.clone(),
            cached: true,
        })
        .collect();

    Ok(Json(TemplateCacheResponse {
        templates,
        active_template_id: manager.active_template_id.clone(),
    }))
}

/// Get specific template
async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<String>,
) -> Result<Json<ReceiptTemplate>, StatusCode> {
    let manager = state.printer_manager.lock().unwrap();

    if let Some(template) = manager.template_cache.get(&template_id) {
        Ok(Json(template.clone()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Clear template cache
async fn clear_cache(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    let mut manager = state.printer_manager.lock().unwrap();

    manager.template_cache.clear();
    manager.active_template_id = None;

    Ok(Json(ApiResponse {
        success: true,
        message: "Template cache cleared".to_string(),
    }))
}

/// Test print with active template
async fn test_print(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    let mut manager = state.printer_manager.lock().unwrap();

    if !manager.is_connected() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "Printer not connected".to_string(),
        }));
    }

    if manager.active_template_id.is_none() {
        return Ok(Json(ApiResponse {
            success: false,
            message: "No active template set".to_string(),
        }));
    }

    // Create test data
    let test_data = ReceiptData {
        store_name: Some("Test Store".to_string()),
        store_address: Some("123 Test St".to_string()),
        store_phone: None,
        store_website: None,
        order_id: "TEST-001".to_string(),
        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        cashier_name: Some("Test User".to_string()),
        server_name: None,
        table_number: None,
        items: vec![
            crate::ReceiptItem {
                name: "Test Item 1".to_string(),
                quantity: 2,
                price: 10.00,
                total: 20.00,
            },
            crate::ReceiptItem {
                name: "Test Item 2".to_string(),
                quantity: 1,
                price: 15.50,
                total: 15.50,
            },
        ],
        subtotal: 35.50,
        tax: 2.84,
        tax_rate: Some(8.0),
        discount: None,
        tip: None,
        total: 38.34,
        payment_method: "Test Payment".to_string(),
        change: None,
        footer_message: Some("This is a test receipt".to_string()),
        receipt_url: None,
        custom: std::collections::HashMap::new(),
    };

    match manager.print_with_template(&test_data) {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: "Test receipt printed successfully".to_string(),
        })),
        Err(e) => {
            log::error!("Test print failed: {}", e);
            Ok(Json(ApiResponse {
                success: false,
                message: format!("Test print failed: {}", e),
            }))
        }
    }
}

// ==================== Server Setup ====================

/// Start HTTP server in background
pub async fn start_server(
    printer_manager: Arc<Mutex<PrinterManager>>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState { printer_manager });

    // Configure CORS for web app integration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router with all routes
    let app = Router::new()
        // Health & status
        .route("/health", get(health))
        .route("/status", get(status))
        // Legacy print
        .route("/print", post(print_legacy))
        // Template management
        .route("/template", post(set_template))
        .route("/templates", get(get_cached_templates))
        .route("/template/{id}", get(get_template))
        // Template-based printing
        .route("/print-template", post(print_with_template))
        .route("/test-print", post(test_print))
        // Cache management
        .route("/cache", delete(clear_cache))
        .layer(cors)
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    log::info!("HTTP print server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
