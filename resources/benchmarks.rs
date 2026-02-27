//! Benchmark Runner Resource
//!
//! Manages benchmark test execution and results aggregation.
//!
//! | Method | Path                        | Description                     |
//! |--------|-----------------------------|---------------------------------|
//! | GET    | /admin/runner               | Get runner state + configs      |
//! | POST   | /admin/runner               | Start a benchmark test          |
//! | GET    | /admin/best-results         | Best result per test from runs  |

use std::sync::{Arc, Mutex, OnceLock};
use yeti_core::prelude::*;

pub type Benchmarks = BenchmarksResource;

// ── Test definitions (mirrors frontend TESTS array) ──

struct TestDef {
    id: &'static str,
    name: &'static str,
    binary: &'static str,
    duration: u64,
    vus: u64,
}

const TESTS: &[TestDef] = &[
    TestDef { id: "rest-read", name: "REST Reads", binary: "load-rest", duration: 30, vus: 50 },
    TestDef { id: "rest-write", name: "REST Writes", binary: "load-rest", duration: 30, vus: 50 },
    TestDef { id: "rest-update", name: "REST Update", binary: "load-rest", duration: 30, vus: 50 },
    TestDef { id: "rest-join", name: "REST Join", binary: "load-rest", duration: 30, vus: 50 },
    TestDef { id: "graphql-read", name: "GraphQL Reads", binary: "load-graphql", duration: 30, vus: 50 },
    TestDef { id: "graphql-mutation", name: "GraphQL Mutations", binary: "load-graphql", duration: 30, vus: 50 },
    TestDef { id: "graphql-join", name: "GraphQL Join", binary: "load-graphql", duration: 30, vus: 50 },
    TestDef { id: "vector-embed", name: "Vector Embed", binary: "load-vector", duration: 30, vus: 50 },
    TestDef { id: "vector-search", name: "Vector Search", binary: "load-vector", duration: 30, vus: 50 },
    TestDef { id: "ws", name: "WebSocket", binary: "load-realtime", duration: 30, vus: 50 },
    TestDef { id: "sse", name: "SSE Streaming", binary: "load-realtime", duration: 30, vus: 50 },
    TestDef { id: "blob-retrieval", name: "150k Blob Retrieval", binary: "load-blob", duration: 30, vus: 50 },
];

// ── Runner state (in-memory, shared across requests) ──

#[derive(Clone)]
struct RunnerState {
    status: String,       // "idle", "warming", "running"
    test_name: Option<String>,
    started_at: Option<f64>,
    configured_duration: Option<u64>,
    configured_vus: Option<u64>,
    last_error: Option<String>,
    child_pid: Option<u32>,
}

impl Default for RunnerState {
    fn default() -> Self {
        Self {
            status: "idle".to_string(),
            test_name: None,
            started_at: None,
            configured_duration: None,
            configured_vus: None,
            last_error: None,
            child_pid: None,
        }
    }
}

fn runner_state() -> &'static Arc<Mutex<RunnerState>> {
    static STATE: OnceLock<Arc<Mutex<RunnerState>>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(Mutex::new(RunnerState::default())))
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

// ── Resource ──

#[derive(Default)]
pub struct BenchmarksResource;

impl Resource for BenchmarksResource {
    fn name(&self) -> &str {
        "runner"
    }

    get!(_request, ctx, {
        // Check if this is best-results or runner
        // The resource name is "runner", but we also handle "best-results"
        // via path: /admin/runner vs /admin/best-results
        // Actually, best-results is a separate resource below.
        // This handles GET /admin/runner

        let state = runner_state().lock().unwrap().clone();

        // Check if a running process has finished
        let mut current_state = state.clone();
        if current_state.status != "idle" {
            let mut should_idle = false;

            if let Some(pid) = current_state.child_pid {
                // Check if process is still alive via kill -0
                let alive = std::process::Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if !alive {
                    should_idle = true;
                }
            }

            // Duration-based timeout: if elapsed exceeds configured duration + 10s grace,
            // force transition to idle. Handles PID reuse (kill -0 sees unrelated process)
            // and zombie processes that never exit.
            if !should_idle {
                if let (Some(started), Some(duration)) = (current_state.started_at, current_state.configured_duration) {
                    let elapsed = now_secs() - started;
                    if elapsed > (duration as f64) + 10.0 {
                        should_idle = true;
                        // Kill the process in case it's actually stuck
                        if let Some(pid) = current_state.child_pid {
                            let _ = std::process::Command::new("kill")
                                .arg("-9")
                                .arg(pid.to_string())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status();
                        }
                    }
                }
            }

            if should_idle {
                let mut guard = runner_state().lock().unwrap();
                guard.status = "idle".to_string();
                guard.child_pid = None;
                current_state = guard.clone();
            }
        }

        let elapsed = current_state.started_at
            .map(|s| now_secs() - s)
            .unwrap_or(0.0);

        let warmup_secs = if current_state.status == "warming" { elapsed } else { 0.0 };
        // Cap elapsed at configured_duration so UI doesn't show e.g. 740s / 30s
        let elapsed_secs = if current_state.status == "running" {
            match current_state.configured_duration {
                Some(d) => elapsed.min(d as f64),
                None => elapsed,
            }
        } else {
            0.0
        };

        // Load configs from TestConfig table
        let configs = match ctx.get_table("TestConfig") {
            Ok(table) => {
                match table.scan_all().await {
                    Ok(records) => records,
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        };

        reply().json(json!({
            "status": current_state.status,
            "testName": current_state.test_name,
            "startedAt": current_state.started_at,
            "warmupSecs": warmup_secs,
            "elapsedSecs": elapsed_secs,
            "configuredDuration": current_state.configured_duration,
            "lastError": current_state.last_error,
            "configs": configs,
        }))
    });

    post!(request, ctx, {
        let body = request.json_value()?;
        let test_id = body.require_str("test")?;

        // Validate test exists
        let test_def = TESTS.iter().find(|t| t.id == test_id);
        if test_def.is_none() {
            return bad_request(&format!("Unknown test: {}", test_id));
        }
        let test_def = test_def.unwrap();

        // Check not already running
        {
            let state = runner_state().lock().unwrap();
            if state.status != "idle" {
                return bad_request("A test is already running");
            }
        }

        // Load config overrides from TestConfig table
        let (duration, vus) = match ctx.get_table("TestConfig") {
            Ok(table) => {
                match table.get_by_id(&test_id).await {
                    Ok(Some(cfg)) => {
                        let d = cfg.get("duration").and_then(|v| v.as_u64()).unwrap_or(test_def.duration);
                        let v = cfg.get("vus").and_then(|v| v.as_u64()).unwrap_or(test_def.vus);
                        (d, v)
                    }
                    _ => (test_def.duration, test_def.vus),
                }
            }
            Err(_) => (test_def.duration, test_def.vus),
        };

        // Find the benchmark binary
        let root = get_root_directory();
        let bin_dir = root.join("benchmarks");
        let bin_path = bin_dir.join(test_def.binary);

        if !bin_path.exists() {
            // Try in PATH as fallback
            let which_result = std::process::Command::new("which")
                .arg(test_def.binary)
                .output();
            match which_result {
                Ok(output) if output.status.success() => {
                    // Found in PATH, proceed
                }
                _ => {
                    return bad_request(&format!(
                        "Benchmark binary '{}' not found. Expected at {} or in PATH.",
                        test_def.binary,
                        bin_path.display()
                    ));
                }
            }
        }

        // Determine the actual binary path
        let actual_bin = if bin_path.exists() {
            bin_path.to_string_lossy().to_string()
        } else {
            test_def.binary.to_string()
        };

        // Start the benchmark process
        let child = std::process::Command::new(&actual_bin)
            .arg("--test")
            .arg(&test_id)
            .arg("--duration")
            .arg(duration.to_string())
            .arg("--vus")
            .arg(vus.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(child) => {
                let pid = child.id();
                let mut state = runner_state().lock().unwrap();
                state.status = "running".to_string();
                state.test_name = Some(test_id.to_string());
                state.started_at = Some(now_secs());
                state.configured_duration = Some(duration);
                state.configured_vus = Some(vus);
                state.last_error = None;
                state.child_pid = Some(pid);

                yeti_log!(info, "Benchmark started: test={}, binary={}, duration={}s, vus={}, pid={}",
                    test_id, test_def.binary, duration, vus, pid);

                reply().json(json!({
                    "status": "running",
                    "testName": test_id,
                    "pid": pid,
                }))
            }
            Err(e) => {
                let msg = format!("Failed to start benchmark '{}': {}", actual_bin, e);
                yeti_log!(error, "{}", msg);
                let mut state = runner_state().lock().unwrap();
                state.status = "idle".to_string();
                state.last_error = Some(msg.clone());
                bad_request(&msg)
            }
        }
    });
}

register_resource!(BenchmarksResource);
