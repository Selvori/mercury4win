//! Reconstruct synthetic evaluation checkpoints without paying for prior work.
//! Every recorded prompt/input must match; only unfinished cases may issue new HTTP.
use super::*;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
};
pub type Hook = Arc<
    dyn Fn(
            &provider::ChatCompletionRequest,
        ) -> Result<Option<provider::ChatCompletionResponse>, String>
        + Send
        + Sync,
>;
pub struct Replay {
    pub path: String,
    pub replayed: Arc<AtomicUsize>,
    pub replayed_failures: Arc<AtomicUsize>,
    cases: BTreeMap<String, (Vec<Value>, bool)>,
}
fn normalized(request: &Value) -> Value {
    let mut value = request.clone();
    let mut data: Value =
        serde_json::from_str(value["messages"][1]["content"].as_str().unwrap()).unwrap();
    if data["stage"] == "understanding" {
        data.as_object_mut().unwrap().remove("now_local");
    }
    value["messages"][1]["content"] = data;
    value
}
impl Replay {
    pub async fn load(p: &deadpool_sqlite::Pool, model: &str) -> Option<Self> {
        let path = std::env::var_os("M4W_STAGED_REPLAY_REPORT")?;
        let path = std::path::PathBuf::from(path);
        let report: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(report["version"], staged_search::VERSION);
        assert_eq!(report["model"], model);
        assert_eq!(report["synthetic_only"], true);
        let rows = std::fs::read_to_string(path.with_extension("requests.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        let mut pairs = vec![];
        let mut dates = BTreeMap::<i64, String>::new();
        for row in rows.iter().filter(|r| r["event"] == "started") {
            let n = row["request_number"].as_u64().unwrap();
            let finished = rows
                .iter()
                .find(|r| r["request_number"] == n && r["event"] == "finished");
            let failed = rows
                .iter()
                .any(|r| r["request_number"] == n && r["event"] == "failed");
            assert!(
                finished.is_some() || failed,
                "Interrupted request without terminal fact cannot be reconstructed"
            );
            let data: Value =
                serde_json::from_str(row["request"]["messages"][1]["content"].as_str().unwrap())
                    .unwrap();
            for field in ["titles", "previews", "parts", "articles"] {
                if let Some(items) = data[field].as_array() {
                    for item in items {
                        if let Some(date) = item["published_at"].as_str() {
                            dates.insert(item["entry_id"].as_i64().unwrap(), date.into());
                        }
                    }
                }
            }
            pairs.push(json!({"request":row["request"],"response":finished.map(|f|f["response"].clone()),"failed":failed}));
        }
        p.get()
            .await
            .unwrap()
            .interact(move |c| {
                for (id, date) in dates {
                    c.execute(
                        "UPDATE entry SET published_at=?2 WHERE id=?1",
                        rusqlite::params![id, date],
                    )
                    .unwrap();
                }
            })
            .await
            .unwrap();
        let mut offset = 0;
        let mut cases = BTreeMap::new();
        for case in report["cases"].as_array().unwrap() {
            assert_eq!(case["algorithm"], "staged");
            let count = case["requests"].as_u64().unwrap() as usize;
            cases.insert(
                case["case"]["key"].as_str().unwrap().into(),
                (
                    pairs[offset..offset + count].to_vec(),
                    case["result"]["error"].is_object(),
                ),
            );
            offset += count;
        }
        assert_eq!(offset, pairs.len());
        Some(Self {
            path: path.to_string_lossy().into_owned(),
            replayed: Arc::new(AtomicUsize::new(0)),
            replayed_failures: Arc::new(AtomicUsize::new(0)),
            cases,
        })
    }
    pub fn hook(&self, key: &str) -> Hook {
        let (pairs, allow_new) = self
            .cases
            .get(key)
            .expect("Every case has a recorded prefix")
            .clone();
        let queue = Arc::new(Mutex::new(VecDeque::from(pairs)));
        let replayed = self.replayed.clone();
        let replayed_failures = self.replayed_failures.clone();
        Arc::new(move |request| {
            let mut queue = queue.lock().unwrap();
            let Some(pair) = queue.front() else {
                return if allow_new {
                    Ok(None)
                } else {
                    Err("Unexpected new HTTP after a complete recorded case".into())
                };
            };
            let actual = serde_json::to_value(request).unwrap();
            if normalized(&actual) != normalized(&pair["request"]) {
                return Err("Recorded synthetic input mismatch; refusing additional HTTP".into());
            }
            let pair = queue.pop_front().unwrap();
            if pair["failed"] == true {
                // Only the captured failure category is known, never invent its
                // HTTP status/body. Reconstruct the saved provider-error state;
                // a separately explicit resume may perform new real HTTP.
                replayed_failures.fetch_add(1, Ordering::SeqCst);
                return Err("Recorded provider failure".into());
            }
            replayed.fetch_add(1, Ordering::SeqCst);
            serde_json::from_value(json!({"choices":[{"message":{"role":"assistant","content":pair["response"]},"finish_reason":"stop"}],"usage":null})).map(Some).map_err(|_|"Invalid recorded response".into())
        })
    }
}
