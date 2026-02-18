//! SSH Deploy Key Management Resource
//!
//! REST API for managing ED25519 SSH keypairs for git authentication.
//!
//! | Method | Path                                | Description              |
//! |--------|-------------------------------------|--------------------------|
//! | GET    | /yeti-applications/keys             | List all keys            |
//! | GET    | /yeti-applications/keys/{name}      | Get single key (pub)     |
//! | POST   | /yeti-applications/keys             | Generate new keypair     |
//! | DELETE | /yeti-applications/keys/{name}      | Remove keypair           |

use std::path::PathBuf;
use yeti_core::prelude::*;

pub type Keys = KeysResource;

#[derive(Default)]
pub struct KeysResource;

/// Get the keys directory path: ~/yeti/keys/
fn keys_dir() -> PathBuf {
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
    root_path.join("keys")
}

/// Validate key name format: [a-z0-9][a-z0-9-]*[a-z0-9]
fn validate_key_name(name: &str) -> std::result::Result<(), String> {
    if name.len() < 2 {
        return Err("key name must be at least 2 characters".to_string());
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("key name must contain only lowercase letters, digits, and hyphens".to_string());
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err("key name must not start or end with a hyphen".to_string());
    }
    Ok(())
}

/// Ensure keys directory exists with 0700 permissions
fn ensure_keys_dir() -> std::result::Result<PathBuf, String> {
    let dir = keys_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create keys directory: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("Failed to set keys directory permissions: {}", e))?;
        }
    }
    Ok(dir)
}

/// Read a public key file and return its contents
fn read_pub_key(dir: &std::path::Path, name: &str) -> std::result::Result<String, String> {
    let pub_path = dir.join(format!("{}.pub", name));
    std::fs::read_to_string(&pub_path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Failed to read public key: {}", e))
}

impl Resource for KeysResource {
    fn name(&self) -> &str {
        "keys"
    }

    get!(_request, ctx, {
        let dir = keys_dir();

        // Single key by path ID
        if let Some(key_name) = ctx.path_id() {
            validate_key_name(key_name)
                .map_err(|e| YetiError::Validation(e))?;

            let pub_path = dir.join(format!("{}.pub", key_name));
            if !pub_path.exists() {
                return not_found(&format!("Key '{}' not found", key_name));
            }

            let public_key = read_pub_key(&dir, key_name)
                .map_err(|e| YetiError::Internal(e))?;

            return reply().json(json!({
                "name": key_name,
                "public_key": public_key,
            }));
        }

        // List all keys
        let mut keys = Vec::new();
        if dir.is_dir() {
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| YetiError::Internal(format!("Cannot read keys dir: {}", e)))?;

            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if !file_name.ends_with(".pub") {
                    continue;
                }
                let name = file_name.strip_suffix(".pub").unwrap().to_string();

                let public_key = read_pub_key(&dir, &name).unwrap_or_default();

                let created = entry.metadata()
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                keys.push(json!({
                    "name": name,
                    "public_key": public_key,
                    "created": created,
                }));
            }
        }

        keys.sort_by(|a, b| {
            let a_name = a["name"].as_str().unwrap_or("");
            let b_name = b["name"].as_str().unwrap_or("");
            a_name.cmp(b_name)
        });

        reply().json(json!(keys))
    });

    post!(request, _ctx, {
        let body = request.json_value()?;
        let name = body.require_str("name")?;

        validate_key_name(&name)
            .map_err(|e| YetiError::Validation(e))?;

        let dir = ensure_keys_dir()
            .map_err(|e| YetiError::Internal(e))?;

        let key_path = dir.join(&name);
        let pub_path = dir.join(format!("{}.pub", name));

        if key_path.exists() || pub_path.exists() {
            return bad_request(&format!("Key '{}' already exists", name));
        }

        // Generate ED25519 keypair via ssh-keygen
        let output = std::process::Command::new("ssh-keygen")
            .args([
                "-t", "ed25519",
                "-f", &key_path.to_string_lossy(),
                "-N", "",
                "-C", &format!("yeti-deploy-key-{}", name),
            ])
            .output()
            .map_err(|e| YetiError::Internal(format!("Failed to run ssh-keygen: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(YetiError::Internal(format!("ssh-keygen failed: {}", stderr)));
        }

        // Set private key to 0600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        }

        let public_key = read_pub_key(&dir, &name)
            .map_err(|e| YetiError::Internal(e))?;

        reply().code(201).json(json!({
            "name": name,
            "public_key": public_key,
            "created": true,
        }))
    });

    delete!(_request, ctx, {
        let key_name = ctx.require_id()?.to_string();

        validate_key_name(&key_name)
            .map_err(|e| YetiError::Validation(e))?;

        let dir = keys_dir();
        let key_path = dir.join(&key_name);
        let pub_path = dir.join(format!("{}.pub", &key_name));

        if !key_path.exists() && !pub_path.exists() {
            return not_found(&format!("Key '{}' not found", key_name));
        }

        // Remove both private and public key files
        if key_path.exists() {
            std::fs::remove_file(&key_path)
                .map_err(|e| YetiError::Internal(format!("Failed to remove private key: {}", e)))?;
        }
        if pub_path.exists() {
            std::fs::remove_file(&pub_path)
                .map_err(|e| YetiError::Internal(format!("Failed to remove public key: {}", e)))?;
        }

        reply().json(json!({"deleted": true, "name": key_name}))
    });
}

register_resource!(KeysResource);
