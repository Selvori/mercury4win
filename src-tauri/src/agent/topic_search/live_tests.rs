//! Explicit paid synthetic evaluation. No user article/content/note tables read.
use crate::agent::{
    configuration,
    topic_search::{self, tests},
    usage,
};
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "Paid API: explicit authorization and M4W_TOPIC_LIVE_APP_DIR / M4W_TOPIC_LIVE_REPORT required"]
async fn approved_live_topic_search() {
    let app = std::path::PathBuf::from(
        std::env::var_os("M4W_TOPIC_LIVE_APP_DIR").expect("Explicit routing directory"),
    );
    let report = std::path::PathBuf::from(
        std::env::var_os("M4W_TOPIC_LIVE_REPORT").expect("Explicit new report"),
    );
    assert!(!report.exists());
    assert!(report.parent().unwrap().is_dir());
    let p = super::live_search_filter_tests::fixture(&app).await;
    p.get().await.unwrap().interact(tests::seed).await.unwrap();
    // Validate the report query before any paid request.
    p.get().await.unwrap().interact(|c| { c.prepare("SELECT request_status,prompt_tokens,completion_tokens,total_tokens FROM llm_usage_event ORDER BY id").unwrap(); }).await.unwrap();
    let mut cfg = configuration::load(&p, &app.join("prompts"), "summary")
        .await
        .unwrap();
    cfg.routes.truncate(1);
    let route = cfg.routes.first_mut().expect("Configured summary route");
    assert!(route
        .provider_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    let encrypted = std::fs::read_to_string(
        app.join("credentials")
            .join(format!("{}.key", route.provider_id)),
    )
    .unwrap();
    route.api_key = super::simple_decrypt(&encrypted);
    assert!(!route.api_key.is_empty());
    let model = route.model_name.clone();
    let mut results = vec![];
    let client = reqwest::Client::new();
    // All seven pre-labelled cases, same final prompts, no retries or fallback.
    // Production run is capped at 1 plan + 3 judgments = 28 requests at most.
    let selected = std::env::var("M4W_TOPIC_LIVE_CASE").ok();
    let corpus = tests::corpus();
    let cases = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| selected.as_ref().is_none_or(|key| case["key"] == *key))
        .collect::<Vec<_>>();
    assert!(!cases.is_empty());
    let count = cases.len();
    for case in cases {
        let start = std::time::Instant::now();
        let diagnostics = std::sync::Arc::new(std::sync::Mutex::new(vec![]));
        let outcome = topic_search::DIAGNOSTICS
            .scope(
                diagnostics.clone(),
                usage::scope(
                    &p,
                    0,
                    "search_filter",
                    topic_search::run(
                        &p,
                        &client,
                        cfg.clone(),
                        case["query"].as_str().unwrap().into(),
                        tests::manual(),
                        None,
                        true,
                        &tauri::ipc::Channel::new(|_| Ok(())),
                    ),
                ),
            )
            .await;
        let (passed, detail) = match outcome {
            Ok(run) => {
                let page = tests::scope(&p, &run.session, 100, None).await;
                let mut actual = page.entries.iter().map(|e| e.id).collect::<Vec<_>>();
                actual.sort();
                let expected = case["expected"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| id.as_i64().unwrap())
                    .collect::<Vec<_>>();
                let missed = expected
                    .iter()
                    .filter(|id| !actual.contains(id))
                    .collect::<Vec<_>>();
                let false_positive = actual
                    .iter()
                    .filter(|id| !expected.contains(id))
                    .collect::<Vec<_>>();
                let passed = actual == expected && run.error.is_none() && run.session.pending == 0;
                (
                    passed,
                    json!({"session":run.session,"error":run.error,"actual":actual,"missed":missed,"false_positive":false_positive,"evidence":page.search_info}),
                )
            }
            Err(error) => (false, json!({"error":error})),
        };
        println!(
            "topic live {}: {}",
            case["key"],
            if passed { "PASS" } else { "FAIL" }
        );
        if !passed {
            println!("sanitized failure: {detail}");
        }
        let transport_failed = detail["error"]["code"] == "provider";
        results.push(json!({"case":case,"passed":passed,"elapsed_ms":start.elapsed().as_millis(),"result":detail,"exchanges":diagnostics.lock().unwrap().clone()}));
        // Preserve each completed case even if later accounting or transport fails.
        std::fs::write(
            report.with_extension("progress.json"),
            serde_json::to_vec_pretty(&results).unwrap(),
        )
        .unwrap();
        if transport_failed {
            break;
        }
    }
    usage::flush().await;
    let metrics=p.get().await.unwrap().interact(|c|{
        let mut s=c.prepare("SELECT request_status,prompt_tokens,completion_tokens,total_tokens FROM llm_usage_event WHERE agent_type='search_filter' ORDER BY id").unwrap();
        s.query_map([],|r|Ok(json!({"status":r.get::<_,String>(0)?,"prompt_tokens":r.get::<_,Option<i64>>(1)?,"completion_tokens":r.get::<_,Option<i64>>(2)?,"total_tokens":r.get::<_,Option<i64>>(3)?}))).unwrap().collect::<rusqlite::Result<Vec<Value>>>().unwrap()
    }).await.unwrap();
    let passed = results.len() == count && results.iter().all(|v| v["passed"] == true);
    let evidence = json!({"date":chrono::Local::now().to_rfc3339(),"version":topic_search::VERSION,"model":model,"synthetic_only":true,"no_retry":true,"primary_only":true,"max_requests":28,"request_count":metrics.len(),"total_tokens":metrics.iter().filter_map(|v|v["total_tokens"].as_i64()).sum::<i64>(),"requests":metrics,"passed":passed,"cases":results});
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(report)
        .unwrap()
        .write_all(serde_json::to_string_pretty(&evidence).unwrap().as_bytes())
        .unwrap();
    assert!(
        passed,
        "Inspect synthetic evaluation report, including failures"
    );
}
