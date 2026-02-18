//! File Browser/Editor Resource
//!
//! REST API for browsing and editing application files.
//!
//! | Method | Path                                           | Description        |
//! |--------|------------------------------------------------|--------------------|
//! | GET    | /yeti-applications/files?app={id}&path=/       | List directory      |
//! | GET    | /yeti-applications/files?app={id}&path=/f.rs   | Read file as text   |
//! | PUT    | /yeti-applications/files                       | Update file         |
//! | POST   | /yeti-applications/files                       | Create file         |
//! | DELETE | /yeti-applications/files?app={id}&path=/file   | Delete file         |

use std::path::PathBuf;
use yeti_core::prelude::*;

pub type Files = FilesResource;

#[derive(Default)]
pub struct FilesResource;

/// Get the applications directory path
fn apps_dir() -> PathBuf {
    let root = std::env::var("ROOT_DIRECTORY").unwrap_or_else(|_| "~/yeti".to_string());
    let root_path = if root.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(root.strip_prefix("~/").unwrap())
        } else {
            PathBuf::from(&root)
        }
    } else {
        PathBuf::from(&root)
    };
    root_path.join("applications")
}

/// Validate and resolve a file path within an app directory.
/// Returns the canonical path if safe, or an error if path traversal is detected.
fn resolve_safe_path(app_id: &str, rel_path: &str) -> std::result::Result<PathBuf, String> {
    // Reject obvious traversal patterns
    if rel_path.contains("..") {
        return Err("Path traversal not allowed".to_string());
    }

    let apps_path = apps_dir();
    let app_path = apps_path.join(app_id);

    if !app_path.is_dir() {
        return Err(format!("Application '{}' not found", app_id));
    }

    // Strip leading slash from relative path
    let clean_path = rel_path.strip_prefix('/').unwrap_or(rel_path);

    // If path is empty or "/", return the app directory itself
    if clean_path.is_empty() {
        let canonical = app_path.canonicalize()
            .map_err(|e| format!("Cannot resolve path: {}", e))?;
        return Ok(canonical);
    }

    let full_path = app_path.join(clean_path);

    // Canonicalize the base app path
    let canonical_base = app_path.canonicalize()
        .map_err(|e| format!("Cannot resolve app path: {}", e))?;

    // For existing paths, canonicalize and verify
    if full_path.exists() {
        let canonical = full_path.canonicalize()
            .map_err(|e| format!("Cannot resolve path: {}", e))?;
        if !canonical.starts_with(&canonical_base) {
            return Err("Path traversal not allowed".to_string());
        }
        return Ok(canonical);
    }

    // For non-existing paths (create), verify parent exists and is within base
    if let Some(parent) = full_path.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize()
                .map_err(|e| format!("Cannot resolve parent: {}", e))?;
            if !canonical_parent.starts_with(&canonical_base) {
                return Err("Path traversal not allowed".to_string());
            }
            return Ok(full_path);
        }
    }

    Err("Parent directory does not exist".to_string())
}

/// Extract query params from request URI
fn get_query_param(request: &Request<Vec<u8>>, param: &str) -> Option<String> {
    let query = request.uri().query()?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next().unwrap_or("");
        if key == param {
            return Some(urlencoding::decode(value).unwrap_or_default().into_owned());
        }
    }
    None
}

impl Resource for FilesResource {
    fn name(&self) -> &str {
        "files"
    }

    get!(request, _ctx, {
        let app_id = get_query_param(&request, "app")
            .ok_or_else(|| YetiError::Validation("Missing 'app' query parameter".to_string()))?;
        let rel_path = get_query_param(&request, "path")
            .unwrap_or_else(|| "/".to_string());

        let safe_path = resolve_safe_path(&app_id, &rel_path)
            .map_err(|e| YetiError::Validation(e))?;

        // Directory listing
        if safe_path.is_dir() {
            let entries = std::fs::read_dir(&safe_path)
                .map_err(|e| YetiError::Internal(format!("Cannot read directory: {}", e)))?;

            let mut items: Vec<serde_json::Value> = Vec::new();
            for entry in entries.flatten() {
                let meta = entry.metadata().ok();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = meta.as_ref().map_or(false, |m| m.is_dir());
                let size = meta.as_ref().map_or(0, |m| m.len());

                items.push(json!({
                    "name": name,
                    "type": if is_dir { "directory" } else { "file" },
                    "size": size,
                }));
            }

            items.sort_by(|a, b| {
                let a_type = a["type"].as_str().unwrap_or("");
                let b_type = b["type"].as_str().unwrap_or("");
                let a_name = a["name"].as_str().unwrap_or("");
                let b_name = b["name"].as_str().unwrap_or("");
                // Directories first, then alphabetical
                b_type.cmp(a_type).then(a_name.cmp(b_name))
            });

            return reply().json(json!({
                "app": app_id,
                "path": rel_path,
                "type": "directory",
                "entries": items,
            }));
        }

        // File read
        if safe_path.is_file() {
            let content = std::fs::read(&safe_path)
                .map_err(|e| YetiError::Internal(format!("Cannot read file: {}", e)))?;

            // Check if content is valid UTF-8
            match String::from_utf8(content) {
                Ok(text) => {
                    let size = safe_path.metadata().map(|m| m.len()).unwrap_or(0);
                    return reply().json(json!({
                        "app": app_id,
                        "path": rel_path,
                        "type": "file",
                        "content": text,
                        "size": size,
                    }));
                }
                Err(_) => {
                    return bad_request("File is not valid UTF-8 text");
                }
            }
        }

        not_found(&format!("Path '{}' not found in app '{}'", rel_path, app_id))
    });

    post!(request, _ctx, {
        let body = request.json_value()?;
        let app_id = body.require_str("app")?;
        let rel_path = body.require_str("path")?;
        let content = body.require_str("content")?;

        let safe_path = resolve_safe_path(&app_id, &rel_path)
            .map_err(|e| YetiError::Validation(e))?;

        if safe_path.exists() {
            return bad_request(&format!("File '{}' already exists, use PUT to update", rel_path));
        }

        // Create parent directories if needed
        if let Some(parent) = safe_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| YetiError::Internal(format!("Failed to create directories: {}", e)))?;
        }

        std::fs::write(&safe_path, &content)
            .map_err(|e| YetiError::Internal(format!("Failed to write file: {}", e)))?;

        reply().code(201).json(json!({
            "app": app_id,
            "path": rel_path,
            "created": true,
            "size": content.len(),
        }))
    });

    put!(request, _ctx, {
        let body = request.json_value()?;
        let app_id = body.require_str("app")?;
        let rel_path = body.require_str("path")?;
        let content = body.require_str("content")?;

        let safe_path = resolve_safe_path(&app_id, &rel_path)
            .map_err(|e| YetiError::Validation(e))?;

        if !safe_path.exists() {
            return not_found(&format!("File '{}' not found in app '{}'", rel_path, app_id));
        }

        std::fs::write(&safe_path, &content)
            .map_err(|e| YetiError::Internal(format!("Failed to write file: {}", e)))?;

        reply().json(json!({
            "app": app_id,
            "path": rel_path,
            "updated": true,
            "size": content.len(),
        }))
    });

    delete!(request, _ctx, {
        let app_id = get_query_param(&request, "app")
            .ok_or_else(|| YetiError::Validation("Missing 'app' query parameter".to_string()))?;
        let rel_path = get_query_param(&request, "path")
            .ok_or_else(|| YetiError::Validation("Missing 'path' query parameter".to_string()))?;

        let safe_path = resolve_safe_path(&app_id, &rel_path)
            .map_err(|e| YetiError::Validation(e))?;

        if !safe_path.exists() {
            return not_found(&format!("Path '{}' not found in app '{}'", rel_path, app_id));
        }

        if safe_path.is_dir() {
            std::fs::remove_dir_all(&safe_path)
                .map_err(|e| YetiError::Internal(format!("Failed to remove directory: {}", e)))?;
        } else {
            std::fs::remove_file(&safe_path)
                .map_err(|e| YetiError::Internal(format!("Failed to remove file: {}", e)))?;
        }

        reply().json(json!({
            "app": app_id,
            "path": rel_path,
            "deleted": true,
        }))
    });
}

register_resource!(FilesResource);
