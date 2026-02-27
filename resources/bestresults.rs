//! Best Results Resource
//!
//! Aggregates best benchmark throughput per test from TestRun records.
//!
//! | Method | Path                 | Description                     |
//! |--------|----------------------|---------------------------------|
//! | GET    | /admin/bestresults   | Best result per test from runs  |

use yeti_core::prelude::*;

pub type BestResults = BestResultsResource;

#[derive(Default)]
pub struct BestResultsResource;

impl Resource for BestResultsResource {
    fn name(&self) -> &str {
        "bestresults"
    }

    fn is_public(&self) -> bool { true }

    get!(_request, ctx, {
        // Query all TestRun records and find the best throughput per test
        let runs = match ctx.get_table("TestRun") {
            Ok(table) => table.scan_all().await.unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        // Group by testName, keep best throughput
        let mut best: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

        for run in &runs {
            let test_name = match run.get("testName").and_then(|v| v.as_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            // Parse the results JSON string
            let results_str = run.get("results").and_then(|v| v.as_str()).unwrap_or("{}");
            let results: serde_json::Value = serde_json::from_str(results_str).unwrap_or(json!({}));
            let throughput = results.get("throughput").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let is_better = match best.get(&test_name) {
                Some(existing) => {
                    let existing_tp = existing.get("throughput")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    throughput > existing_tp
                }
                None => true,
            };

            if is_better {
                best.insert(test_name.clone(), json!({
                    "name": test_name,
                    "throughput": throughput,
                    "run": run,
                    "results": results,
                }));
            }
        }

        let tests: Vec<serde_json::Value> = best.into_values().collect();

        reply().json(json!({
            "tests": tests,
        }))
    });
}

register_resource!(BestResultsResource);
