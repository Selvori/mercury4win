//! Opted-in finite synthetic experiment. Routing tables and one key are read-only.
use crate::agent::{
    configuration, provider,
    staged_search::{self, tests},
    topic_search, usage,
};
use serde_json::{json, Value};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

#[path = "live_replay.rs"]
mod replay;

#[tokio::test]
#[ignore = "Paid synthetic experiment: explicit M4W_STAGED_APP_DIR and fresh M4W_STAGED_REPORT required"]
async fn approved_live_staged_search() {
    let app = std::path::PathBuf::from(
        std::env::var_os("M4W_STAGED_APP_DIR").expect("Explicit routing directory"),
    );
    let report =
        std::path::PathBuf::from(std::env::var_os("M4W_STAGED_REPORT").expect("Fresh report path"));
    assert!(!report.exists());
    assert!(report.parent().unwrap().is_dir());
    let p = super::live_search_filter_tests::fixture(&app).await;
    p.get().await.unwrap().interact(tests::seed).await.unwrap();
    let large_scope = std::env::var("M4W_STAGED_LARGE_SCOPE").as_deref() == Ok("609");
    let synthetic_articles = if large_scope { 609 } else { 71 };
    if large_scope {
        assert_eq!(std::env::var("M4W_STAGED_CASE").as_deref(), Ok("models_zh"));
        p.get().await.unwrap().interact(|c| {
            for id in 1000..1538 {
                c.execute("INSERT INTO entry(id,feed_id,title,summary) VALUES(?1,1,?2,'Three goals decided the match.')",rusqlite::params![id,format!("Home club wins another football match {id}")]).unwrap();
            }
        }).await.unwrap();
    }
    p.get().await.unwrap().interact(|c|{c.prepare("SELECT request_phase,request_status,prompt_tokens,completion_tokens,total_tokens FROM llm_usage_event WHERE agent_type='search_filter' ORDER BY id").unwrap();}).await.unwrap();
    let mut cfg = configuration::load(&p, &app.join("prompts"), "summary")
        .await
        .unwrap();
    cfg.routes.truncate(1);
    let route = cfg.routes.first_mut().unwrap();
    assert!(route
        .provider_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    route.api_key = super::simple_decrypt(
        &std::fs::read_to_string(
            app.join("credentials")
                .join(format!("{}.key", route.provider_id)),
        )
        .unwrap(),
    );
    assert!(!route.api_key.is_empty());
    let model = route.model_name.clone();
    let replay = replay::Replay::load(&p, &model).await;
    let allowance = std::env::var("M4W_STAGED_ALLOWANCE")
        .ok()
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(160);
    assert!((1..=160).contains(&allowance));
    let ledger = report.with_extension("requests.jsonl");
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&ledger)
        .unwrap();
    writeln!(f,"{}",json!({"event":"preflight","synthetic_articles":synthetic_articles,"max_requests":allowance,"model":model,"version":staged_search::VERSION})).unwrap();
    f.sync_all().unwrap();
    drop(f);
    let count = Arc::new(Mutex::new(0usize));
    let n = count.clone();
    let audit: Arc<dyn Fn(Value) -> Result<(), String> + Send + Sync> = Arc::new(move |mut v| {
        let mut count = n.lock().unwrap();
        if v["event"] == "started" {
            if *count >= allowance {
                return Err("Synthetic experiment request allowance reached".into());
            }
            *count += 1;
        }
        v["request_number"] = json!(*count);
        v["at"] = json!(chrono::Utc::now().to_rfc3339());
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&ledger)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{}", v).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())
    });
    let client = reqwest::Client::new();
    let mut results = vec![];
    let selected = std::env::var("M4W_STAGED_CASE").ok();
    let cases = tests::extra()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| selected.as_ref().is_none_or(|key| c["key"] == *key))
        .cloned()
        .collect::<Vec<_>>();
    assert!(!cases.is_empty());
    let expected_cases = cases.len();
    let skip_baseline = std::env::var("M4W_STAGED_SKIP_BASELINE").as_deref() == Ok("1");
    let resume_once = std::env::var("M4W_STAGED_RESUME_ONCE").as_deref() == Ok("1");
    // Old baseline runs to its finite lexical scope exhaustion, same 71 hard-eligible articles.
    for (algorithm, case) in cases.iter().map(|c| ("staged", c)).chain(
        cases
            .iter()
            .filter(|c| {
                !skip_baseline && ["models_zh", "database_zh"].contains(&c["key"].as_str().unwrap())
            })
            .map(|c| ("old_v4", c)),
    ) {
        let begin = std::time::Instant::now();
        let first = Arc::new(Mutex::new(None));
        let first_sink = first.clone();
        let first_quick = Arc::new(Mutex::new(None));
        let quick_sink = first_quick.clone();
        let first_verified = Arc::new(Mutex::new(None));
        let verified_sink = first_verified.clone();
        let channel = tauri::ipc::Channel::<String>::new(move |raw| {
            let raw = raw.deserialize::<String>().unwrap();
            let event: Value = serde_json::from_str(&raw).unwrap();
            if event["type"] == "search_snapshot"
                && event["session"]["matches"].as_u64().unwrap_or(0) > 0
            {
                let mut f = first_sink.lock().unwrap();
                if f.is_none() {
                    *f = Some(begin.elapsed().as_millis());
                }
                if event["session"]["preliminary"].as_u64().unwrap_or(0) > 0 {
                    let mut q = quick_sink.lock().unwrap();
                    if q.is_none() {
                        *q = Some(begin.elapsed().as_millis());
                    }
                }
                if event["session"]["scan"]["matches"].as_u64().unwrap_or(0) > 0 {
                    let mut v = verified_sink.lock().unwrap();
                    if v.is_none() {
                        *v = Some(begin.elapsed().as_millis());
                    }
                }
            }
            Ok(())
        });
        let query = case["query"].as_str().unwrap().to_owned();
        let start_requests = *count.lock().unwrap();
        let replay_count = replay
            .as_ref()
            .map(|r| r.replayed.load(std::sync::atomic::Ordering::SeqCst))
            .unwrap_or(0);
        let hook: replay::Hook = replay
            .as_ref()
            .map(|r| r.hook(case["key"].as_str().unwrap()))
            .unwrap_or_else(|| Arc::new(|_| Ok(None)));
        let task = provider::TEST_AUDIT.scope(
            audit.clone(),
            usage::scope(&p, 0, "search_filter", async {
                if algorithm == "staged" {
                    let mut r = staged_search::run(
                        &p,
                        &client,
                        cfg.clone(),
                        query,
                        tests::manual(),
                        None,
                        true,
                        format!("live-{}", case["key"]),
                        &channel,
                    )
                    .await?;
                    if resume_once && r.error.is_some() {
                        let saved_error = r.error.as_ref().unwrap().code.clone();
                        let saved = r.session.scan.as_ref().expect("Saved checkpoint").clone();
                        assert!(saved.pending>0);
                        std::fs::write(report.with_extension("resume.json"),serde_json::to_vec_pretty(&json!({"reconstructed":replay.is_some(),"error":saved_error,"checkpoint":saved,"explicit_test_resume":true})).unwrap()).unwrap();
                        r = staged_search::run(&p,&client,cfg.clone(),case["query"].as_str().unwrap().into(),tests::manual(),Some(saved.scan_id),true,format!("live-resume-{}",case["key"]),&channel).await?;
                    }
                    Ok((r.session, r.error))
                } else {
                    let mut resume = None;
                    loop {
                        let r = topic_search::run(
                            &p,
                            &client,
                            cfg.clone(),
                            query.clone(),
                            tests::manual(),
                            resume,
                            false,
                            &channel,
                        )
                        .await?;
                        if r.session.matches > 0 && first.lock().unwrap().is_none() {
                            *first.lock().unwrap() = Some(begin.elapsed().as_millis());
                        }
                        if r.session.pending == 0 || r.error.is_some() {
                            break Ok((r.session, r.error));
                        }
                        resume = Some(r.session.id);
                    }
                }
            }),
        );
        let outcome: Result<_, crate::agent::search_filter::FilterError> =
            provider::TEST_REPLAY.scope(hook, task).await;
        let detail = match outcome {
            Ok((session, error)) => {
                let actual = tests::result_ids(&p, &session, 100).await;
                let expected = case["expected"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap())
                    .collect::<Vec<_>>();
                let missed = expected
                    .iter()
                    .filter(|v| !actual.contains(v))
                    .collect::<Vec<_>>();
                let false_positive = actual
                    .iter()
                    .filter(|v| !expected.contains(v))
                    .collect::<Vec<_>>();
                json!({"passed":actual==expected&&error.is_none()&&session.pending==0,"actual":actual,"missed":missed,"false_positive":false_positive,"error":error,"session":session})
            }
            Err(e) => json!({"passed":false,"error":e}),
        };
        println!(
            "staged live {algorithm} {}: {}",
            case["key"],
            if detail["passed"] == true {
                "PASS"
            } else {
                "FAIL"
            }
        );
        results.push(json!({"algorithm":algorithm,"case":case,"result":detail,"first_result_ms":*first.lock().unwrap(),"first_quick_ms":*first_quick.lock().unwrap(),"first_verified_ms":*first_verified.lock().unwrap(),"elapsed_ms":begin.elapsed().as_millis(),"requests":*count.lock().unwrap()-start_requests,"timing_includes_replay":replay.is_some(),"replayed_requests":replay.as_ref().map(|r|r.replayed.load(std::sync::atomic::Ordering::SeqCst)-replay_count).unwrap_or(0)}));
        std::fs::write(
            report.with_extension("progress.json"),
            serde_json::to_vec_pretty(&results).unwrap(),
        )
        .unwrap();
        if detail["error"]["code"] == "provider" || *count.lock().unwrap() >= allowance {
            break;
        }
    }
    usage::flush().await;
    let metrics=p.get().await.unwrap().interact(|c|{c.prepare("SELECT request_phase,request_status,prompt_tokens,completion_tokens,total_tokens FROM llm_usage_event WHERE agent_type='search_filter' ORDER BY id").unwrap().query_map([],|r|Ok(json!({"phase":r.get::<_,Option<String>>(0)?,"status":r.get::<_,String>(1)?,"prompt_tokens":r.get::<_,Option<i64>>(2)?,"completion_tokens":r.get::<_,Option<i64>>(3)?,"total_tokens":r.get::<_,Option<i64>>(4)?}))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()}).await.unwrap();
    let passed = results
        .iter()
        .filter(|r| r["algorithm"] == "staged")
        .count()
        == expected_cases
        && results
            .iter()
            .filter(|r| r["algorithm"] == "staged")
            .all(|r| r["result"]["passed"] == true);
    let evidence = json!({"date":chrono::Local::now().to_rfc3339(),"version":staged_search::VERSION,"baseline":"81f7b2e plus retained uncommitted work","platform":"Windows","model":model,"synthetic_only":true,"primary_only":true,"no_automatic_retry":true,"selected_repair_case":selected,"reconstructed_from":replay.as_ref().map(|r|&r.path),"replayed_requests":replay.as_ref().map(|r|r.replayed.load(std::sync::atomic::Ordering::SeqCst)).unwrap_or(0),"replayed_failures":replay.as_ref().map(|r|r.replayed_failures.load(std::sync::atomic::Ordering::SeqCst)).unwrap_or(0),"max_experiment_requests":allowance,"request_count":metrics.len(),"total_tokens":metrics.iter().filter_map(|v|v["total_tokens"].as_i64()).sum::<i64>(),"requests":metrics,"cases":results,"passed":passed});
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(report)
        .unwrap();
    f.write_all(serde_json::to_vec_pretty(&evidence).unwrap().as_slice())
        .unwrap();
    f.sync_all().unwrap();
    assert!(passed, "Inspect retained failures; no automatic reruns");
}
