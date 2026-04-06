// src/logo_cache.rs
// Logo caching functionality with in-memory and disk persistence

use crate::{LogoCacheEntry, LogoMetadata, PrinterManager};
use chrono::Utc;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Compute SHA256 hash of base64 string
pub fn compute_content_hash(base64_data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(base64_data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate auto ID based on content hash (first 12 chars of SHA256)
pub fn generate_auto_id(content_hash: &str) -> String {
    format!("logo-{}", &content_hash[..12.min(content_hash.len())])
}

/// Extract dimensions from base64 image data
/// Returns (width, height) if successful, otherwise returns (0, 0)
pub fn get_image_dimensions(base64_data: &str) -> (u32, u32) {
    use base64::{engine::general_purpose, Engine as _};
    use image::ImageReader;
    use std::io::Cursor;

    // Strip optional data-URI prefix
    let b64 = match base64_data.find(',') {
        Some(pos) => &base64_data[pos + 1..],
        None => base64_data,
    };

    // Decode base64
    let bytes = match general_purpose::STANDARD.decode(b64) {
        Ok(b) => b,
        Err(_) => return (0, 0),
    };

    // Try to load and get dimensions
    match ImageReader::new(Cursor::new(bytes)).with_guessed_format() {
        Ok(reader) => {
            match reader.into_dimensions() {
                Ok((width, height)) => (width, height),
                Err(_) => (0, 0),
            }
        }
        Err(_) => (0, 0),
    }
}

/// Initialize logo cache from disk (called on startup)
pub fn load_logos_from_disk(manager: &mut PrinterManager) -> Result<(), String> {
    let cache_path = &manager.logo_cache_path.clone();

    // Create cache directory if it doesn't exist
    if !Path::new(cache_path).exists() {
        if let Err(e) = fs::create_dir_all(cache_path) {
            log::warn!("Failed to create logo cache directory: {}", e);
            // Not fatal - continue with empty cache
            return Ok(());
        }
    }

    // Try to load index file
    let index_path = format!("{}/.index.json", cache_path);
    if !Path::new(&index_path).exists() {
        // No index file yet, start with empty cache
        return Ok(());
    }

    let index_data = match fs::read_to_string(&index_path) {
        Ok(data) => data,
        Err(e) => {
            log::warn!("Failed to read logo index: {}", e);
            return Ok(());
        }
    };

    // Parse index JSON
    let index: HashMap<String, serde_json::Value> = match serde_json::from_str(&index_data) {
        Ok(idx) => idx,
        Err(e) => {
            log::warn!("Failed to parse logo index JSON: {}", e);
            return Ok(());
        }
    };

    // Load each entry
    for (id, entry_value) in index.iter() {
        if let Ok(entry) = serde_json::from_value::<LogoCacheEntry>(entry_value.clone()) {
            // Verify file exists
            if let Some(file_path) = &entry.file_path {
                if Path::new(file_path).exists() {
                    manager.logo_cache.insert(id.clone(), entry);
                    log::debug!("Loaded logo from cache: {}", id);
                } else {
                    log::warn!("Logo file not found, skipping: {}", file_path);
                }
            }
        }
    }

    Ok(())
}

/// Save logo to disk and update index
pub fn save_logo_to_disk(manager: &mut PrinterManager, logo: &LogoCacheEntry) -> Result<(), String> {
    let cache_path = &manager.logo_cache_path.clone();

    // Ensure directory exists
    if !Path::new(cache_path).exists() {
        if let Err(e) = fs::create_dir_all(cache_path) {
            return Err(format!("Failed to create cache directory: {}", e));
        }
    }

    // Determine filename: use user-provided ID if not auto-generated, else use hash
    let filename = if logo.id.starts_with("logo-") {
        format!("{}.b64", &logo.id[6..]) // Remove "logo-" prefix for auto IDs
    } else {
        format!("{}.b64", logo.id)
    };

    let file_path = format!("{}/{}", cache_path, filename);

    // Write base64 data to file
    if let Err(e) = fs::write(&file_path, &logo.base64_data) {
        return Err(format!("Failed to write logo file: {}", e));
    }

    // Update index file
    let index_path = format!("{}/.index.json", cache_path);

    // Load existing index or create new
    let mut index: HashMap<String, LogoCacheEntry> = if Path::new(&index_path).exists() {
        let data = match fs::read_to_string(&index_path) {
            Ok(d) => d,
            Err(e) => return Err(format!("Failed to read index: {}", e)),
        };
        match serde_json::from_str(&data) {
            Ok(idx) => idx,
            Err(e) => return Err(format!("Failed to parse index: {}", e)),
        }
    } else {
        HashMap::new()
    };

    // Add/update entry
    let mut entry = logo.clone();
    entry.file_path = Some(file_path);
    index.insert(logo.id.clone(), entry);

    // Write updated index
    let index_json = match serde_json::to_string_pretty(&index) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to serialize index: {}", e)),
    };

    if let Err(e) = fs::write(&index_path, index_json) {
        return Err(format!("Failed to write index file: {}", e));
    }

    Ok(())
}

/// Cache a logo (new or existing)
pub fn cache_logo(
    manager: &mut PrinterManager,
    id: Option<String>,
    base64_data: &str,
) -> Result<(String, String, bool), String> {
    // Compute content hash
    let content_hash = compute_content_hash(base64_data);

    // Check if logo with same hash already exists
    for (existing_id, existing_entry) in &manager.logo_cache {
        if existing_entry.content_hash == content_hash {
            // Logo already cached
            log::debug!("Logo already cached with ID: {}", existing_id);
            return Ok((existing_id.clone(), content_hash, false)); // cached = false (reused)
        }
    }

    // Determine ID
    let final_id = if let Some(user_id) = id {
        // Check if named ID already exists
        if manager.logo_cache.contains_key(&user_id) {
            return Err(format!("Logo ID already exists: {}", user_id));
        }
        user_id
    } else {
        // Generate auto ID from hash
        generate_auto_id(&content_hash)
    };

    // Get image dimensions
    let (width, height) = get_image_dimensions(base64_data);

    // Create cache entry
    let now = Utc::now().to_rfc3339();
    let entry = LogoCacheEntry {
        id: final_id.clone(),
        content_hash: content_hash.clone(),
        base64_data: base64_data.to_string(),
        file_path: None,
        metadata: LogoMetadata {
            file_size_bytes: base64_data.len(),
            original_width: width,
            original_height: height,
            mime_type: None,
            usage_count: 0,
            cached_dimensions: None,
        },
        created_at: now,
        last_used: None,
    };

    // Save to disk
    save_logo_to_disk(manager, &entry)?;

    // Add to in-memory cache
    manager.logo_cache.insert(final_id.clone(), entry);

    log::info!("Logo cached with ID: {}", final_id);
    Ok((final_id, content_hash, true)) // cached = true (newly cached)
}

/// Get logo from cache by ID or hash
pub fn get_logo(manager: &PrinterManager, id_or_hash: &str) -> Option<LogoCacheEntry> {
    // Try exact ID match
    if let Some(entry) = manager.logo_cache.get(id_or_hash) {
        return Some(entry.clone());
    }

    // Try hash match (if parameter looks like hash - 64 chars hex)
    if id_or_hash.len() == 64 && id_or_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        for entry in manager.logo_cache.values() {
            if entry.content_hash == id_or_hash {
                return Some(entry.clone());
            }
        }
    }

    // Try treating as auto-generated ID (logo-XXXXXX format)
    let try_id = format!("logo-{}", id_or_hash);
    if let Some(entry) = manager.logo_cache.get(&try_id) {
        return Some(entry.clone());
    }

    None
}

/// Update usage statistics for a logo
pub fn update_logo_usage(manager: &mut PrinterManager, logo_id: &str) -> Result<(), String> {
    if let Some(entry) = manager.logo_cache.get_mut(logo_id) {
        entry.metadata.usage_count += 1;
        entry.last_used = Some(Utc::now().to_rfc3339());
        let entry_clone = entry.clone();
        
        // Update on disk
        save_logo_to_disk(manager, &entry_clone)?;
        Ok(())
    } else {
        Err(format!("Logo not found: {}", logo_id))
    }
}

/// Delete logo from cache and disk
pub fn delete_logo(manager: &mut PrinterManager, logo_id: &str) -> Result<(), String> {
    if let Some(entry) = manager.logo_cache.remove(logo_id) {
        // Delete from disk if file exists
        if let Some(file_path) = entry.file_path {
            if Path::new(&file_path).exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    log::warn!("Failed to delete logo file: {}", e);
                }
            }
        }

        // Update index file
        let cache_path = &manager.logo_cache_path.clone();
        let index_path = format!("{}/.index.json", cache_path);

        if Path::new(&index_path).exists() {
            if let Ok(data) = fs::read_to_string(&index_path) {
                if let Ok(mut index) = serde_json::from_str::<HashMap<String, LogoCacheEntry>>(&data) {
                    index.remove(logo_id);

                    if let Ok(json) = serde_json::to_string_pretty(&index) {
                        let _ = fs::write(&index_path, json);
                    }
                }
            }
        }

        log::info!("Logo deleted: {}", logo_id);
        Ok(())
    } else {
        Err(format!("Logo not found: {}", logo_id))
    }
}

/// Clear all logos from cache and disk
pub fn clear_logo_cache(manager: &mut PrinterManager) -> Result<(), String> {
    let _cache_path = &manager.logo_cache_path.clone();

    // Get list of IDs to delete (to avoid borrow checker issues)
    let logo_ids: Vec<String> = manager.logo_cache.keys().cloned().collect();

    // Delete each logo
    for id in logo_ids {
        delete_logo(manager, &id)?;
    }

    log::info!("Logo cache cleared");
    Ok(())
}

/// Get all cached logos
pub fn get_all_logos(manager: &PrinterManager) -> Vec<LogoCacheEntry> {
    manager
        .logo_cache
        .values()
        .cloned()
        .collect()
}

/// Get cache statistics
pub fn get_cache_stats(manager: &PrinterManager) -> (usize, u64, u64) {
    let count = manager.logo_cache.len();
    let total_size: u64 = manager
        .logo_cache
        .values()
        .map(|e| e.metadata.file_size_bytes as u64)
        .sum();

    let disk_usage: u64 = if Path::new(&manager.logo_cache_path).exists() {
        if let Ok(entries) = fs::read_dir(&manager.logo_cache_path) {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.metadata().ok())
                .map(|metadata| metadata.len())
                .sum()
        } else {
            0
        }
    } else {
        0
    };

    (count, total_size, disk_usage)
}

/// Auto-cache logos from a template's inline base64 sources
/// Returns the number of logos that were auto-cached (newly cached, not reused)
pub fn auto_cache_template_logos(manager: &mut PrinterManager, template: &mut crate::ReceiptTemplate) -> Result<usize, String> {
    let mut auto_cached_count = 0;

    // Scan layout sections for logo elements
    for section in &mut template.layout.sections {
        for element in &mut section.elements {
            if let crate::Element::Logo(logo_elem) = element {
                // Check if source is set and logo_id is not set
                if logo_elem.logo_id.is_none() {
                    if let Some(source) = &logo_elem.source {
                        // Check if source looks like base64 (contains base64 markers or is long data-uri)
                        if source.contains("base64,") || source.len() > 100 || source.contains("/") || source.contains("\\") {
                            // This looks like base64, cache it
                            match cache_logo(manager, None, source) {
                                Ok((logo_id, _, newly_cached)) => {
                                    logo_elem.logo_id = Some(logo_id);
                                    if newly_cached {
                                        auto_cached_count += 1;
                                    }
                                    log::debug!("Auto-cached logo for template element");
                                }
                                Err(e) => {
                                    log::warn!("Failed to auto-cache logo: {}", e);
                                    // Continue with other logos instead of failing
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(auto_cached_count)
}

/// Resolve all logo references in a template for rendering
/// Replaces logo_id and smart-detected source strings with actual base64 data
/// Also updates usage statistics for resolved logos
pub fn resolve_template_logos(manager: &mut PrinterManager, template: &mut crate::ReceiptTemplate) -> Result<(), String> {
    for section in &mut template.layout.sections {
        for element in &mut section.elements {
            if let crate::Element::Logo(logo_elem) = element {
                if !logo_elem.source.is_some() || logo_elem.logo_id.is_some() {
                    // Try to resolve logo_id first
                    if let Some(logo_id) = &logo_elem.logo_id.clone() {
                        if let Some(entry) = get_logo(manager, logo_id) {
                            logo_elem.source = Some(entry.base64_data);
                            // Update usage stats
                            let _ = update_logo_usage(manager, logo_id);
                            log::debug!("Resolved logo_id to base64: {}", logo_id);
                        } else {
                            log::warn!("Logo not found in cache: {}", logo_id);
                        }
                    } else if let Some(source) = logo_elem.source.clone() {
                        // Check if source might be a logo ID (not base64-looking)
                        // Base64 typically has: data-uri prefix, long length, or specific markers
                        if !source.contains("base64,") && !source.contains("/") && !source.contains("\\") && source.len() < 100 {
                            // Looks like it might be an ID, try to resolve it
                            if let Some(entry) = get_logo(manager, &source) {
                                logo_elem.source = Some(entry.base64_data);
                                // Update usage stats
                                let _ = update_logo_usage(manager, &source);
                                log::debug!("Resolved source string to cached logo: {}", source);
                            }
                            // If not found in cache, keep original source (might fail during rendering, which is OK)
                        }
                        // Otherwise keep source as-is (it's likely base64)
                    }
                }
            }
        }
    }
    Ok(())
}


