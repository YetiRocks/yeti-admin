use clap::Parser;
use std::time::Duration;
use uuid::Uuid;
use yeti_benchmarks::{cli::BenchArgs, client, reporter, runner};

const SAMPLE_TOPICS: &[&str] = &[
    "technology innovation artificial intelligence",
    "climate change renewable energy sustainability",
    "space exploration mars colonization",
    "quantum computing breakthroughs",
    "biotechnology gene editing crispr",
    "ocean conservation marine biology",
    "autonomous vehicles self driving cars",
    "blockchain decentralized finance",
    "neuroscience brain computer interfaces",
    "cybersecurity threat detection",
];

#[tokio::main]
async fn main() {
    let args = BenchArgs::parse();
    let (auth_user, auth_pass) = args.auth_parts();
    let auth_user = auth_user.to_string();
    let auth_pass = auth_pass.to_string();
    let client = client::build_client();
    let duration = Duration::from_secs(args.duration);

    println!(
        "load-vector: test={}, duration={}s, vus={}, base={}",
        args.test, args.duration, args.vus, args.base_url
    );

    match args.test.as_str() {
        "vector-embed" => {
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                |ctx| async move {
                    let id = Uuid::new_v4().to_string();
                    let topic_idx = ctx.vu_id as usize % SAMPLE_TOPICS.len();
                    let body = serde_json::json!({
                        "id": id,
                        "title": format!("Vector Article {}", &id[..8]),
                        "content": format!(
                            "This article explores {}. Generated for benchmark testing with unique content to trigger embedding computation. ID: {}",
                            SAMPLE_TOPICS[topic_idx], id
                        ),
                    });
                    let url = format!("{}/demo-vector/Article/", ctx.base_url);
                    let start = std::time::Instant::now();
                    match ctx
                        .client
                        .post(&url)
                        .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                        .json(&body)
                        .send()
                        .await
                    {
                        Ok(resp) => {
                            let bytes = resp.bytes().await.map(|b| b.len() as u64).unwrap_or(0);
                            let latency = start.elapsed().as_micros() as u64;
                            ctx.metrics.record_success(latency, bytes);
                        }
                        Err(_) => ctx.metrics.record_error(),
                    }
                },
            )
            .await;

            let summary = metrics.summary(elapsed);
            reporter::report_results(
                &client, &args.base_url, &auth_user, &auth_pass,
                "vector-embed", elapsed, &summary,
            )
            .await;
        }
        "vector-search" => {
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                |ctx| async move {
                    let topic_idx = ctx.vu_id as usize % SAMPLE_TOPICS.len();
                    let query = serde_json::json!({
                        "conditions": [{
                            "field": "embedding",
                            "op": "vector",
                            "value": SAMPLE_TOPICS[topic_idx]
                        }],
                        "limit": 10
                    });
                    let url = format!(
                        "{}/demo-vector/Article/?query={}",
                        ctx.base_url,
                        urlencoding(query.to_string())
                    );
                    let start = std::time::Instant::now();
                    match ctx.client.get(&url)
                        .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                        .send().await {
                        Ok(resp) => {
                            let bytes = resp.bytes().await.map(|b| b.len() as u64).unwrap_or(0);
                            let latency = start.elapsed().as_micros() as u64;
                            ctx.metrics.record_success(latency, bytes);
                        }
                        Err(_) => ctx.metrics.record_error(),
                    }
                },
            )
            .await;

            let summary = metrics.summary(elapsed);
            reporter::report_results(
                &client, &args.base_url, &auth_user, &auth_pass,
                "vector-search", elapsed, &summary,
            )
            .await;
        }
        other => {
            eprintln!("Unknown test for load-vector: {}", other);
            std::process::exit(1);
        }
    }
}

/// Simple percent-encoding for query params.
fn urlencoding(s: String) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
