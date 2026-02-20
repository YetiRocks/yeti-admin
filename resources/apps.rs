//! Application CRUD Resource
//!
//! REST API for managing Yeti applications.
//!
//! | Method | Path                             | Description                    |
//! |--------|----------------------------------|--------------------------------|
//! | GET    | /yeti-applications/apps          | List all apps                  |
//! | GET    | /yeti-applications/apps/{id}     | Get single app detail          |
//! | POST   | /yeti-applications/apps          | Create new app from template   |
//! | PUT    | /yeti-applications/apps/{id}     | Update app config.yaml         |
//! | DELETE | /yeti-applications/apps/{id}     | Remove app directory           |

use std::path::Path;
use yeti_core::prelude::*;

pub type Apps = AppsResource;

#[derive(Default)]
pub struct AppsResource;

/// Recursively copy a template directory, skipping build artifacts
fn copy_template(src: &Path, dst: &Path) -> std::io::Result<()> {
    const SKIP_DIRS: &[&str] = &["source", "node_modules", ".git", "target", "test"];
    const SKIP_FILES: &[&str] = &["Cargo.toml", "build.rs", ".gitignore"];

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            if SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            copy_template(&src_path, &dst_path)?;
        } else {
            if SKIP_FILES.contains(&name.as_ref()) {
                continue;
            }
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Read and parse an app's config.yaml
fn read_app_config(app_path: &Path) -> Option<serde_json::Value> {
    let config_path = app_path.join("config.yaml");
    let content = std::fs::read_to_string(&config_path).ok()?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    let json_str = serde_json::to_string(&yaml).ok()?;
    serde_json::from_str(&json_str).ok()
}

/// List files in an app directory (non-recursive, just top-level entries)
fn list_app_files(app_path: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(app_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let file_type = entry.file_type().ok();
                let suffix = if file_type.map_or(false, |ft| ft.is_dir()) { "/" } else { "" };
                files.push(format!("{}{}", name, suffix));
            }
        }
    }
    files.sort();
    files
}

/// Check if app has a schema.graphql
fn has_schema(app_path: &Path) -> bool {
    app_path.join("schema.graphql").exists()
}

/// Count resource files
fn count_resources(app_path: &Path) -> usize {
    let resources_dir = app_path.join("resources");
    if !resources_dir.is_dir() {
        return 0;
    }
    std::fs::read_dir(&resources_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .map_or(false, |ext| ext == "rs")
                })
                .count()
        })
        .unwrap_or(0)
}

/// Count @table types across all schema files referenced in config.yaml
fn count_tables(app_path: &Path) -> usize {
    // Read config to find schema files
    let config = read_app_config(app_path);
    let schema_paths: Vec<String> = config
        .as_ref()
        .and_then(|c| c.get("schemas"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    // If no schemas in config, check for schema.graphql at root
    let paths_to_check: Vec<std::path::PathBuf> = if schema_paths.is_empty() {
        let default = app_path.join("schema.graphql");
        if default.exists() { vec![default] } else { vec![] }
    } else {
        schema_paths.iter().map(|p| app_path.join(p)).collect()
    };

    let mut count = 0;
    for schema_path in paths_to_check {
        if let Ok(content) = std::fs::read_to_string(&schema_path) {
            count += content.matches("@table").count();
        }
    }
    count
}

impl Resource for AppsResource {
    fn name(&self) -> &str {
        "apps"
    }

    get!(_request, ctx, {
        let apps_path = get_apps_directory();

        // Single app by path ID
        if let Some(app_id) = ctx.path_id() {
            let app_path = apps_path.join(app_id);
            if !app_path.is_dir() {
                return not_found(&format!("Application '{}' not found", app_id));
            }

            let config = read_app_config(&app_path);
            let files = list_app_files(&app_path);
            let has_schema = has_schema(&app_path);
            let resource_count = count_resources(&app_path);
            let table_count = count_tables(&app_path);

            return reply().json(json!({
                "app_id": app_id,
                "config": config,
                "files": files,
                "has_schema": has_schema,
                "resource_count": resource_count,
                "table_count": table_count,
            }));
        }

        // List all apps
        let entries = std::fs::read_dir(&apps_path)
            .map_err(|e| YetiError::Internal(format!("Cannot read applications dir: {}", e)))?;

        let mut apps = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = entry.file_name();
            let Some(id) = dir_name.to_str() else { continue };

            // Skip hidden directories and non-app directories
            if id.starts_with('.') {
                continue;
            }

            let config = read_app_config(&path);
            let name = config
                .as_ref()
                .and_then(|c| c.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string();
            let enabled = config
                .as_ref()
                .and_then(|c| c.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let has_schema = has_schema(&path);
            let resource_count = count_resources(&path);
            let table_count = count_tables(&path);
            let is_extension = config
                .as_ref()
                .and_then(|c| c.get("extension"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            apps.push(json!({
                "app_id": id,
                "name": name,
                "enabled": enabled,
                "has_schema": has_schema,
                "resource_count": resource_count,
                "table_count": table_count,
                "is_extension": is_extension,
            }));
        }

        apps.sort_by(|a, b| {
            let a_id = a["app_id"].as_str().unwrap_or("");
            let b_id = b["app_id"].as_str().unwrap_or("");
            a_id.cmp(b_id)
        });

        reply().json(json!(apps))
    });

    post!(request, _ctx, {
        let body = request.json_value()?;
        let app_id = body.require_str("id")?;

        validate_identifier(&app_id, "app_id")?;

        let name = body.get("name").and_then(|v| v.as_str()).unwrap_or(&app_id);
        let description = body.get("description").and_then(|v| v.as_str()).unwrap_or("A new Yeti application");

        let apps_path = get_apps_directory();
        let app_path = apps_path.join(&app_id);

        if app_path.exists() {
            return bad_request(&format!("Application '{}' already exists", app_id));
        }

        let template = body.get("template").and_then(|v| v.as_str());

        if template == Some("application-template") {
            // Copy from application-template
            let template_path = apps_path.join("application-template");
            if !template_path.is_dir() {
                return Err(YetiError::Internal("Application template not found".to_string()));
            }

            copy_template(&template_path, &app_path)
                .map_err(|e| YetiError::Internal(format!("Failed to copy template: {}", e)))?;

            // Update config.yaml with new app_id/name/description
            let config_path = app_path.join("config.yaml");
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| YetiError::Internal(format!("Failed to read template config: {}", e)))?;
            let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| YetiError::Internal(format!("Failed to parse template config: {}", e)))?;

            if let Some(map) = yaml.as_mapping_mut() {
                map.insert(
                    serde_yaml::Value::String("app_id".to_string()),
                    serde_yaml::Value::String(app_id.to_string()),
                );
                map.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(name.to_string()),
                );
                map.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String(description.to_string()),
                );
            }

            let new_content = serde_yaml::to_string(&yaml)
                .map_err(|e| YetiError::Internal(format!("Failed to serialize config: {}", e)))?;
            std::fs::write(&config_path, &new_content)
                .map_err(|e| YetiError::Internal(format!("Failed to write config: {}", e)))?;

            reply().code(201).json(json!({
                "app_id": app_id,
                "name": name,
                "description": description,
                "template": "application-template",
                "created": true,
            }))
        } else {
            // Create blank app from inline template
            std::fs::create_dir_all(app_path.join("resources"))
                .map_err(|e| YetiError::Internal(format!("Failed to create directories: {}", e)))?;
            std::fs::create_dir_all(app_path.join("web"))
                .map_err(|e| YetiError::Internal(format!("Failed to create web dir: {}", e)))?;

            let config_content = format!(
                r#"# Application metadata
name: "{}"
app_id: "{}"
version: "1.0.0"
description: "{}"

# Application state
enabled: true

# Custom resources (Rust plugins)
resources:
  - resources/*.rs

# Static file serving
static_files:
  path: web
  route: /
"#,
                name, app_id, description
            );

            std::fs::write(app_path.join("config.yaml"), &config_content)
                .map_err(|e| YetiError::Internal(format!("Failed to write config: {}", e)))?;

            let index_html = format!(
                r#"<!DOCTYPE html>
<html><head><title>{}</title></head>
<body><h1>{}</h1><p>{}</p></body>
</html>"#,
                name, name, description
            );
            std::fs::write(app_path.join("web").join("index.html"), &index_html)
                .map_err(|e| YetiError::Internal(format!("Failed to write index.html: {}", e)))?;

            reply().code(201).json(json!({
                "app_id": app_id,
                "name": name,
                "description": description,
                "created": true,
            }))
        }
    });

    put!(request, ctx, {
        let app_id = ctx.require_id()?.to_string();
        let body = request.json_value()?;

        let apps_path = get_apps_directory();
        let app_path = apps_path.join(&app_id);
        let config_path = app_path.join("config.yaml");

        if !config_path.exists() {
            return not_found(&format!("Application '{}' not found", app_id));
        }

        // Read existing config
        let existing_content = std::fs::read_to_string(&config_path)
            .map_err(|e| YetiError::Internal(format!("Failed to read config: {}", e)))?;
        let mut existing: serde_yaml::Value = serde_yaml::from_str(&existing_content)
            .map_err(|e| YetiError::Internal(format!("Failed to parse config: {}", e)))?;

        // Convert body to serde_yaml::Value for merging
        let body_str = serde_json::to_string(&body)
            .map_err(|e| YetiError::Internal(format!("JSON serialize failed: {}", e)))?;
        let updates: serde_yaml::Value = serde_yaml::from_str(&body_str)
            .map_err(|e| YetiError::Internal(format!("YAML parse failed: {}", e)))?;

        // Merge updates into existing (top-level keys only)
        if let (Some(existing_map), Some(updates_map)) = (existing.as_mapping_mut(), updates.as_mapping()) {
            for (key, value) in updates_map {
                existing_map.insert(key.clone(), value.clone());
            }
        }

        // Write back
        let new_content = serde_yaml::to_string(&existing)
            .map_err(|e| YetiError::Internal(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&config_path, &new_content)
            .map_err(|e| YetiError::Internal(format!("Failed to write config: {}", e)))?;

        // Return updated config as JSON
        let json_str = serde_json::to_string(&existing)
            .map_err(|e| YetiError::Internal(format!("JSON serialize failed: {}", e)))?;
        let json_val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| YetiError::Internal(format!("JSON parse failed: {}", e)))?;

        reply().json(json!({
            "app_id": app_id,
            "config": json_val,
            "updated": true,
        }))
    });

    delete!(_request, ctx, {
        let app_id = ctx.require_id()?.to_string();

        // Cannot delete self
        if app_id == "yeti-applications" {
            return bad_request("Cannot delete the yeti-applications app");
        }

        let apps_path = get_apps_directory();
        let app_path = apps_path.join(&app_id);

        if !app_path.is_dir() {
            return not_found(&format!("Application '{}' not found", app_id));
        }

        // Remove app directory
        std::fs::remove_dir_all(&app_path)
            .map_err(|e| YetiError::Internal(format!("Failed to remove app directory: {}", e)))?;

        // Also remove cache directory if it exists
        let cache_path = get_cache_directory().join(&app_id);
        if cache_path.is_dir() {
            let _ = std::fs::remove_dir_all(&cache_path);
        }

        reply().json(json!({"deleted": true, "app_id": app_id}))
    });
}

register_resource!(AppsResource);
