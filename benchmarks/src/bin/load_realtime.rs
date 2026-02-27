use clap::Parser;
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;
use yeti_benchmarks::{cli::BenchArgs, client, metrics::Metrics, reporter};

#[tokio::main]
async fn main() {
    let args = BenchArgs::parse();
    let (auth_user, auth_pass) = args.auth_parts();
    let auth_user = auth_user.to_string();
    let auth_pass = auth_pass.to_string();
    let client = client::build_client();
    let duration = Duration::from_secs(args.duration);

    println!(
        "load-realtime: test={}, duration={}s, vus={}, base={}",
        args.test, args.duration, args.vus, args.base_url
    );

    match args.test.as_str() {
        "ws" => {
            run_ws_test(&args, &auth_user, &auth_pass, &client, duration).await;
        }
        "sse" => {
            run_sse_test(&args, &auth_user, &auth_pass, &client, duration).await;
        }
        other => {
            eprintln!("Unknown test for load-realtime: {}", other);
            std::process::exit(1);
        }
    }
}

async fn run_ws_test(
    args: &BenchArgs,
    auth_user: &str,
    auth_pass: &str,
    client: &reqwest::Client,
    duration: Duration,
) {
    let metrics = Arc::new(Metrics::new());
    let deadline = Instant::now() + duration;

    // Build TLS connector that accepts invalid certs
    let tls = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed to build TLS connector");
    let connector = tokio_tungstenite::Connector::NativeTls(tls);

    // Spawn subscriber VUs
    let mut handles = Vec::new();
    for _vu in 0..args.vus {
        let ws_url = format!(
            "{}/demo-realtime/message?stream=ws",
            args.base_url.replace("https://", "wss://").replace("http://", "ws://")
        );
        let m = metrics.clone();
        let conn = connector.clone();

        handles.push(tokio::spawn(async move {
            let Ok((mut ws, _)) = tokio_tungstenite::connect_async_tls_with_config(
                &ws_url,
                None,
                false,
                Some(conn),
            )
            .await
            else {
                return;
            };

            while Instant::now() < deadline {
                match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
                    Ok(Some(Ok(msg))) => {
                        let bytes = msg.into_data().len() as u64;
                        m.record_success(0, bytes);
                    }
                    Ok(Some(Err(_))) => {
                        m.record_error();
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => continue, // timeout, keep waiting
                }
            }

            let _ = ws.close(None).await;
        }));
    }

    // Give subscribers a moment to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publisher task: POST messages as fast as possible
    let pub_url = format!("{}/demo-realtime/message", args.base_url);
    let pub_client = client.clone();
    let pub_user = auth_user.to_string();
    let pub_pass = auth_pass.to_string();
    let pub_handle = tokio::spawn(async move {
        while Instant::now() < deadline {
            let body = serde_json::json!({
                "id": Uuid::new_v4().to_string(),
                "content": "benchmark message",
                "channel": "bench",
            });
            let _ = pub_client
                .post(&pub_url)
                .basic_auth(&pub_user, Some(&pub_pass))
                .json(&body)
                .send()
                .await;
        }
    });

    // Wait for all tasks
    pub_handle.await.ok();
    for h in handles {
        h.await.ok();
    }

    let elapsed = duration.as_secs_f64();
    let summary = metrics.summary(elapsed);
    reporter::report_results(client, &args.base_url, auth_user, auth_pass, "ws", elapsed, &summary)
        .await;
}

async fn run_sse_test(
    args: &BenchArgs,
    auth_user: &str,
    auth_pass: &str,
    client: &reqwest::Client,
    duration: Duration,
) {
    let metrics = Arc::new(Metrics::new());
    let deadline = Instant::now() + duration;

    // Build a client for SSE subscribers
    let sse_client = client::build_client();

    // Spawn subscriber VUs
    let mut handles = Vec::new();
    for _vu in 0..args.vus {
        let sse_url = format!("{}/demo-realtime/message?stream=sse", args.base_url);
        let m = metrics.clone();
        let c = sse_client.clone();
        let sse_user = auth_user.to_string();
        let sse_pass = auth_pass.to_string();

        handles.push(tokio::spawn(async move {
            let Ok(resp) = c.get(&sse_url)
                .basic_auth(&sse_user, Some(&sse_pass))
                .send().await else {
                return;
            };
            let mut stream = resp.bytes_stream();

            while Instant::now() < deadline {
                match tokio::time::timeout(Duration::from_secs(5), stream.next()).await {
                    Ok(Some(Ok(chunk))) => {
                        // Count data lines as received messages
                        let text = String::from_utf8_lossy(&chunk);
                        let msg_count = text.lines().filter(|l| l.starts_with("data:")).count();
                        for _ in 0..msg_count.max(1) {
                            m.record_success(0, chunk.len() as u64);
                        }
                    }
                    Ok(Some(Err(_))) => {
                        m.record_error();
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => continue,
                }
            }
        }));
    }

    // Give subscribers a moment to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publisher task
    let pub_url = format!("{}/demo-realtime/message", args.base_url);
    let pub_client = client.clone();
    let pub_user = auth_user.to_string();
    let pub_pass = auth_pass.to_string();
    let pub_handle = tokio::spawn(async move {
        while Instant::now() < deadline {
            let body = serde_json::json!({
                "id": Uuid::new_v4().to_string(),
                "content": "benchmark sse message",
                "channel": "bench",
            });
            let _ = pub_client
                .post(&pub_url)
                .basic_auth(&pub_user, Some(&pub_pass))
                .json(&body)
                .send()
                .await;
        }
    });

    pub_handle.await.ok();
    for h in handles {
        h.await.ok();
    }

    let elapsed = duration.as_secs_f64();
    let summary = metrics.summary(elapsed);
    reporter::report_results(client, &args.base_url, auth_user, auth_pass, "sse", elapsed, &summary)
        .await;
}
