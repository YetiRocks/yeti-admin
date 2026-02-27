use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use yeti_benchmarks::{cli::BenchArgs, client, reporter, runner};

#[tokio::main]
async fn main() {
    let args = BenchArgs::parse();
    let (auth_user, auth_pass) = args.auth_parts();
    let auth_user = auth_user.to_string();
    let auth_pass = auth_pass.to_string();
    let client = client::build_client();
    let duration = Duration::from_secs(args.duration);

    println!(
        "load-blob: test={}, duration={}s, vus={}, base={}",
        args.test, args.duration, args.vus, args.base_url
    );

    match args.test.as_str() {
        "blob-retrieval" => {
            // Setup: create one Article with ~150KB content
            let blob_id = Uuid::new_v4().to_string();
            let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(2700); // ~150KB
            println!("Setup: creating 150KB article (id={})...", &blob_id[..8]);

            let body = serde_json::json!({
                "id": blob_id,
                "title": "Blob Benchmark Article",
                "content": large_content,
            });
            let url = format!("{}/demo-vector/Article/", args.base_url);
            match client
                .post(&url)
                .basic_auth(&auth_user, Some(&auth_pass))
                .json(&body)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("Setup complete. Starting load test...");
                }
                Ok(resp) => {
                    eprintln!("Setup warning: POST returned {}", resp.status());
                }
                Err(e) => {
                    eprintln!("Setup error: {}", e);
                    std::process::exit(1);
                }
            }

            let blob_id = Arc::new(blob_id);
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                move |ctx| {
                    let blob_id = blob_id.clone();
                    async move {
                        let url = format!("{}/demo-vector/Article/{}", ctx.base_url, blob_id);
                        let start = std::time::Instant::now();
                        match ctx.client.get(&url)
                            .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                            .send().await {
                            Ok(resp) => {
                                let bytes =
                                    resp.bytes().await.map(|b| b.len() as u64).unwrap_or(0);
                                let latency = start.elapsed().as_micros() as u64;
                                ctx.metrics.record_success(latency, bytes);
                            }
                            Err(_) => ctx.metrics.record_error(),
                        }
                    }
                },
            )
            .await;

            let summary = metrics.summary(elapsed);
            reporter::report_results(
                &client, &args.base_url, &auth_user, &auth_pass,
                "blob-retrieval", elapsed, &summary,
            )
            .await;
        }
        other => {
            eprintln!("Unknown test for load-blob: {}", other);
            std::process::exit(1);
        }
    }
}
