//! Git Repository Management Resource
//!
//! REST API for cloning repos and managing git operations on applications.
//!
//! | Method | Path                                          | Description              |
//! |--------|-----------------------------------------------|--------------------------|
//! | POST   | /yeti-applications/repos/check                | Check repo accessibility |
//! | POST   | /yeti-applications/repos/clone                | Clone repo into apps/    |
//! | POST   | /yeti-applications/repos/pull/{app_id}        | Pull latest for an app   |
//! | GET    | /yeti-applications/repos/status/{app_id}      | Git status for an app    |

use std::path::PathBuf;
use yeti_core::prelude::*;

pub type Repos = ReposResource;

#[derive(Default)]
pub struct ReposResource;

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

/// Get the keys directory path
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

/// Validate app_id format: [a-z0-9][a-z0-9-]*[a-z0-9]
fn validate_app_id(id: &str) -> std::result::Result<(), String> {
    if id.len() < 2 {
        return Err("app_id must be at least 2 characters".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("app_id must contain only lowercase letters, digits, and hyphens".to_string());
    }
    if id.starts_with('-') || id.ends_with('-') {
        return Err("app_id must not start or end with a hyphen".to_string());
    }
    Ok(())
}

/// Validate git URL format (must start with git@ or https://)
fn validate_git_url(url: &str) -> std::result::Result<(), String> {
    if url.starts_with("git@") || url.starts_with("https://") {
        Ok(())
    } else {
        Err("URL must start with 'git@' or 'https://'".to_string())
    }
}

/// Extract repo name from git URL
/// e.g. git@github.com:org/my-app.git -> my-app
/// e.g. https://github.com/org/my-app.git -> my-app
/// e.g. git@github.com:my-app.git -> my-app
fn extract_repo_name(url: &str) -> Option<String> {
    // Try splitting by '/' first, then by ':'
    let segment = url.rsplit('/').next()?;
    // If the segment contains ':', it means there was no '/' after the host
    // e.g. "git@github.com:my-app.git" → segment is full URL
    let name = if segment.contains(':') {
        segment.rsplit(':').next()?
    } else {
        segment
    };
    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty() {
        None
    } else {
        Some(name.to_lowercase())
    }
}

/// Build GIT_SSH_COMMAND for a named key
fn git_ssh_command(key_name: &str) -> std::result::Result<String, String> {
    let key_path = keys_dir().join(key_name);
    if !key_path.exists() {
        return Err(format!("SSH key '{}' not found", key_name));
    }
    Ok(format!(
        "ssh -i {} -o StrictHostKeyChecking=accept-new -o IdentitiesOnly=yes",
        key_path.to_string_lossy()
    ))
}

/// Run a git command, optionally with SSH key
fn run_git(args: &[&str], cwd: Option<&std::path::Path>, key: Option<&str>) -> std::result::Result<String, String> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    if let Some(key_name) = key {
        let ssh_cmd = git_ssh_command(key_name)?;
        cmd.env("GIT_SSH_COMMAND", &ssh_cmd);
    }

    let output = cmd.output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("git failed: {}", if stderr.is_empty() { &stdout } else { &stderr }));
    }

    Ok(stdout)
}

impl Resource for ReposResource {
    fn name(&self) -> &str {
        "repos"
    }

    get!(request, _ctx, {
        // GET /repos/status/{app_id}
        let uri_path = request.uri().path();
        let app_id = if uri_path.contains("/repos/status/") {
            uri_path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| YetiError::Validation("App ID required (use /repos/status/{app_id})".to_string()))?
                .to_string()
        } else {
            return bad_request("Use /repos/status/{app_id}");
        };

        validate_app_id(&app_id)
            .map_err(|e| YetiError::Validation(e))?;

        let app_path = apps_dir().join(&app_id);
        if !app_path.is_dir() {
            return not_found(&format!("Application '{}' not found", app_id));
        }

        let is_git = app_path.join(".git").is_dir();
        if !is_git {
            return reply().json(json!({
                "app_id": app_id,
                "is_git": false,
            }));
        }

        // Get branch name
        let branch = run_git(&["-C", &app_path.to_string_lossy(), "branch", "--show-current"], None, None)
            .unwrap_or_default()
            .trim()
            .to_string();

        // Get remote URL
        let remote_url = run_git(&["-C", &app_path.to_string_lossy(), "remote", "get-url", "origin"], None, None)
            .unwrap_or_default()
            .trim()
            .to_string();

        // Check if dirty
        let status_output = run_git(&["-C", &app_path.to_string_lossy(), "status", "--porcelain"], None, None)
            .unwrap_or_default();
        let dirty = !status_output.trim().is_empty();

        reply().json(json!({
            "app_id": app_id,
            "is_git": true,
            "branch": branch,
            "remote_url": remote_url,
            "dirty": dirty,
        }))
    });

    post!(request, _ctx, {
        let body = request.json_value()?;

        // Parse the request URI to determine the operation
        let uri_path = request.uri().path();

        if uri_path.contains("/repos/check") {
            // --- Check repo accessibility ---
            let url = body.require_str("url")?;

            validate_git_url(&url)
                .map_err(|e| YetiError::Validation(e))?;

            // Run git ls-remote with timeout to check public accessibility
            let mut cmd = std::process::Command::new("git");
            cmd.args(["ls-remote", "--exit-code", &url]);
            cmd.env("GIT_TERMINAL_PROMPT", "0");
            cmd.env("GIT_SSH_COMMAND", "ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new -o BatchMode=yes");
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());

            let mut child = cmd.spawn()
                .map_err(|e| YetiError::Internal(format!("Failed to run git: {}", e)))?;

            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(10);
            let is_public = loop {
                match child.try_wait() {
                    Ok(Some(status)) => break status.success(),
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            break false;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(_) => break false,
                }
            };

            reply().json(json!({
                "url": url,
                "public": is_public,
            }))

        } else if uri_path.contains("/repos/clone") {
            // --- Clone operation ---
            let url = body.require_str("url")?;

            validate_git_url(&url)
                .map_err(|e| YetiError::Validation(e))?;

            let app_id = body.get("app_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| extract_repo_name(&url))
                .ok_or_else(|| YetiError::Validation("Cannot determine app_id from URL, please provide 'app_id'".to_string()))?;

            validate_app_id(&app_id)
                .map_err(|e| YetiError::Validation(e))?;

            let app_path = apps_dir().join(&app_id);
            if app_path.exists() {
                return bad_request(&format!("Application '{}' already exists", app_id));
            }

            let key = body.get("key").and_then(|v| v.as_str());

            // Run git clone
            let app_path_str = app_path.to_string_lossy().to_string();
            let args = vec!["clone", &url, &app_path_str];

            let output = run_git(&args, None, key)
                .map_err(|e| {
                    // Clean up partial clone if it exists
                    let _ = std::fs::remove_dir_all(&app_path);
                    YetiError::Internal(e)
                })?;

            reply().code(201).json(json!({
                "app_id": app_id,
                "cloned": true,
                "output": output.trim(),
            }))

        } else if uri_path.contains("/repos/pull/") {
            // --- Pull operation ---
            // Extract app_id from the URI path after /repos/pull/
            let app_id = uri_path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| YetiError::Validation("App ID required in path (use /repos/pull/{app_id})".to_string()))?
                .to_string();

            validate_app_id(&app_id)
                .map_err(|e| YetiError::Validation(e))?;

            let app_path = apps_dir().join(&app_id);
            if !app_path.is_dir() {
                return not_found(&format!("Application '{}' not found", app_id));
            }
            if !app_path.join(".git").is_dir() {
                return bad_request(&format!("Application '{}' is not a git repository", app_id));
            }

            let key = body.get("key").and_then(|v| v.as_str());
            let app_path_str = app_path.to_string_lossy().to_string();
            let args = vec!["-C", &app_path_str, "pull"];

            let output = run_git(&args, None, key)
                .map_err(|e| YetiError::Internal(e))?;

            reply().json(json!({
                "app_id": app_id,
                "pulled": true,
                "output": output.trim(),
            }))

        } else {
            bad_request("Unknown repos operation. Use /repos/clone or /repos/pull/{app_id}")
        }
    });
}

register_resource!(ReposResource);
