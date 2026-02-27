use clap::Parser;
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
        "load-graphql: test={}, duration={}s, vus={}, base={}",
        args.test, args.duration, args.vus, args.base_url
    );

    match args.test.as_str() {
        "graphql-read" => {
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                |ctx| async move {
                    let query = serde_json::json!({
                        "query": "{ Book { id title isbn genre price } }"
                    });
                    let url = format!("{}/demo-graphql/graphql", ctx.base_url);
                    let start = std::time::Instant::now();
                    match ctx.client.post(&url)
                        .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                        .json(&query).send().await {
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
                "graphql-read", elapsed, &summary,
            )
            .await;
        }
        "graphql-mutation" => {
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                |ctx| async move {
                    let id = Uuid::new_v4().to_string();
                    let mutation = format!(
                        r#"mutation {{ createBook(input: {{ id: "{}", title: "GQL Bench {}", isbn: "978-{}", genre: "benchmark", price: 9.99 }}) {{ id }} }}"#,
                        id, &id[..8], &id[..10]
                    );
                    let query = serde_json::json!({ "query": mutation });
                    let url = format!("{}/demo-graphql/graphql", ctx.base_url);
                    let start = std::time::Instant::now();
                    match ctx
                        .client
                        .post(&url)
                        .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                        .json(&query)
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
                "graphql-mutation", elapsed, &summary,
            )
            .await;
        }
        "graphql-join" => {
            let (metrics, elapsed) = runner::run_load_test(
                args.vus,
                duration,
                client.clone(),
                args.base_url.clone(),
                auth_user.clone(),
                auth_pass.clone(),
                |ctx| async move {
                    let id = (ctx.vu_id % 100) + 1;
                    let query_str = format!(
                        r#"{{ Book(id: "{}") {{ id title author {{ name }} }} }}"#,
                        id
                    );
                    let query = serde_json::json!({ "query": query_str });
                    let url = format!("{}/demo-graphql/graphql", ctx.base_url);
                    let start = std::time::Instant::now();
                    match ctx.client.post(&url)
                        .basic_auth(&ctx.auth_user, Some(&ctx.auth_pass))
                        .json(&query).send().await {
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
                "graphql-join", elapsed, &summary,
            )
            .await;
        }
        other => {
            eprintln!("Unknown test for load-graphql: {}", other);
            std::process::exit(1);
        }
    }
}
