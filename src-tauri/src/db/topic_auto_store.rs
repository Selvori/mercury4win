//! Automatic curation state is independent of successful briefing and read position.
use super::{
    search_store,
    topic_store::{self as store, Input, Source, Topic},
};
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub const MIGRATION_V8: &str = "
ALTER TABLE topic ADD COLUMN mode TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE topic ADD COLUMN read_brief_id INTEGER NOT NULL DEFAULT 0;
ALTER TABLE topic ADD COLUMN last_checked_at TEXT;
ALTER TABLE topic ADD COLUMN input_budget INTEGER NOT NULL DEFAULT 24000;
CREATE TABLE topic_checked(topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE,scope TEXT NOT NULL,entry_id INTEGER NOT NULL,version TEXT NOT NULL,PRIMARY KEY(topic_id,scope,entry_id));
CREATE TABLE topic_feedback(topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE,entry_id INTEGER NOT NULL,reason TEXT NOT NULL,PRIMARY KEY(topic_id,entry_id));
CREATE TABLE topic_selection(topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE,cache_key TEXT NOT NULL,selection_json TEXT NOT NULL,PRIMARY KEY(topic_id,cache_key));
";
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Guard {
    pub scope: String,
    pub versions: Vec<(i64, String)>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Scan {
    pub total: usize,
    pub scanned: usize,
    pub pending: usize,
    pub excluded: Vec<i64>,
    #[serde(skip_serializing)]
    pub candidates: Vec<Source>,
    #[serde(skip_serializing)]
    pub guard: Guard,
}
#[derive(Serialize)]
pub struct Status {
    pub topic: Topic,
    pub automatic: Option<Scan>,
    pub changed: bool,
}
pub async fn status(pool: &Pool, id: i64) -> Result<Status, String> {
    store::db(pool, move |c| {
        let topic = store::get(c, id)?;
        if topic.mode == "automatic" {
            search_store::refresh(c)?;
        }
        let tx = c.unchecked_transaction().map_err(err)?;
        let topic = store::get(&tx, id)?;
        let automatic = if topic.mode == "automatic" {
            Some(scan(&tx, &topic)?)
        } else {
            None
        };
        let changed = automatic.as_ref().is_some_and(|s| s.pending > 0);
        Ok(Status {
            topic,
            automatic,
            changed,
        })
    })
    .await
}
pub fn scope(topic: &Topic) -> String {
    store::hash(
        &serde_json::json!([
            "auto-information-v1",
            topic.name,
            topic.description,
            topic.rule
        ])
        .to_string(),
    )
}
pub fn source(c: &Connection, id: i64) -> Result<Source, String> {
    let available: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM entry WHERE id=?1 AND is_deleted=0)",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    if !available {
        return Err("Source article was removed".into());
    }
    let mut s = store::source(c, id, String::new())?;
    s.basis = "body".into();
    if s.body.trim().is_empty() {
        let summary: Option<String> = c
            .query_row("SELECT summary FROM entry WHERE id=?1", [id], |r| r.get(0))
            .map_err(err)?;
        s.body = format!(
            "{}\n{}",
            s.title,
            search_store::html_text(&summary.unwrap_or_default())
        );
        s.basis = "metadata".into();
        s.content_version =
            store::hash(&serde_json::json!([s.title, s.url, s.feed, s.body, s.basis]).to_string());
    }
    s.group_name = format!("auto:{}", &store::hash(&s.body)[..16]);
    Ok(s)
}
fn discovery_terms(topic: &Topic) -> Vec<String> {
    let name = format!("{} {}", topic.name, topic.description).to_lowercase();
    let mut terms = Vec::new();
    let aliases: &[&str] = if (name.contains("ai") || name.contains("智能"))
        && (name.contains("编程") || name.contains("coding") || name.contains("code"))
    {
        &[
            "coding agent",
            "coding assistant",
            "copilot",
            "codex",
            "claude code",
            "cursor",
            "编程助手",
            "代码助手",
        ]
    } else if name.contains("rust") {
        &["rust", "cargo", "rustc", "crates.io"]
    } else if name.contains("新能源车")
        || name.contains("电动汽车")
        || name.contains("electric vehicle")
    {
        &[
            "electric vehicle",
            "battery electric",
            "tesla",
            "byd",
            "电动汽车",
            "新能源车",
        ]
    } else if name.contains("网络安全")
        || name.contains("信息安全")
        || name.contains("cyber")
        || name.contains("security")
    {
        &[
            "cybersecurity",
            "cyber security",
            "security",
            "vulnerability",
            "vulnerabilities",
            "ransomware",
            "malware",
            "breach",
            "cve-",
            "网络安全",
            "信息安全",
            "漏洞",
            "勒索软件",
        ]
    } else {
        &[]
    };
    terms.extend(aliases.iter().map(|s| s.to_string()));
    let words = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .collect::<Vec<_>>();
    terms.extend(
        words
            .iter()
            .filter(|w| w.chars().count() <= 80)
            .map(|w| w.to_string()),
    );
    // Chinese sentences are not whitespace-separated words. Small literal
    // fragments widen discovery; only the AI may decide actual relevance.
    for word in words {
        let chars = word.chars().collect::<Vec<_>>();
        for pair in chars.windows(2).filter(|p| p.iter().all(|c| !c.is_ascii())) {
            terms.push(pair.iter().collect());
        }
    }
    let mut seen = std::collections::HashSet::new();
    terms.retain(|s| seen.insert(s.clone()));
    terms.truncate(32);
    terms
}

pub(crate) fn selection_excerpt(source: &Source, topic: &Topic) -> String {
    if source.body.chars().count() <= 1000 {
        return source.body.clone();
    }
    let opening = source.body.chars().take(600).collect::<String>();
    let rest = source.body.chars().skip(600).collect::<String>();
    let matched = search_store::snippet(&rest, &discovery_terms(topic));
    let context = if matched.is_empty() {
        source
            .body
            .chars()
            .rev()
            .take(390)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>()
    } else {
        matched.into_iter().map(|p| p.text).collect::<String>()
    };
    format!("{opening}\n[…]\n{context}")
}
// Refresh outside a read transaction. A bounded, stable local window is explicit
// in the UI; publication timestamps are never used as a processing cursor.
pub fn scan(c: &Connection, topic: &Topic) -> Result<Scan, String> {
    let filter = topic.rule.clone().unwrap_or_default();
    let compiled = search_store::compile(&filter, &chrono::Utc::now().to_rfc3339())?;
    let where_sql = compiled.conditions.join(" AND ");
    let total = c
        .query_row(
            &format!("SELECT count(*) FROM entry e WHERE {where_sql}"),
            rusqlite::params_from_iter(compiled.values.iter()),
            |r| r.get::<_, i64>(0),
        )
        .map_err(err)? as usize;
    let mut stmt = c
        .prepare(&format!(
            "SELECT e.id FROM entry e WHERE {where_sql} ORDER BY e.id DESC LIMIT 1000"
        ))
        .map_err(err)?;
    let mut ids = stmt
        .query_map(rusqlite::params_from_iter(compiled.values.iter()), |r| {
            r.get::<_, i64>(0)
        })
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let scope = scope(topic);
    let terms = discovery_terms(topic);
    // Search the whole allowed local corpus, not just the newest import window.
    // Bounded results and parameterized literal matching preserve scope and cost.
    let mut values = compiled.values.clone();
    let mut matches = Vec::new();
    let long = terms
        .iter()
        .filter(|s| s.chars().count() >= 3)
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>();
    if !long.is_empty() {
        values.push(format!("{{title summary body}} : ({})", long.join(" OR ")).into());
        matches.push(format!(
            "e.id IN (SELECT rowid FROM entry_search WHERE entry_search MATCH ?{})",
            values.len()
        ));
    }
    let mut short = Vec::new();
    for term in terms.iter().filter(|s| s.chars().count() < 3) {
        values.push(term.clone().into());
        let n = values.len();
        short.push(format!(
            "instr(s.title,?{n})>0 OR instr(s.summary,?{n})>0 OR instr(s.body,?{n})>0"
        ));
    }
    if !short.is_empty() {
        matches.push(format!(
            "e.id IN (SELECT rowid FROM entry_search s WHERE {})",
            short.join(" OR ")
        ));
    }
    if !matches.is_empty() {
        values.push(topic.id.into());
        let topic_param = values.len();
        values.push(scope.clone().into());
        let scope_param = values.len();
        let mut matching = c.prepare(&format!(
            "SELECT e.id FROM entry e WHERE {where_sql} AND ({})
             AND NOT EXISTS(SELECT 1 FROM topic_feedback f WHERE f.topic_id=?{topic_param} AND f.entry_id=e.id)
             ORDER BY EXISTS(SELECT 1 FROM topic_checked k WHERE k.topic_id=?{topic_param} AND k.scope=?{scope_param} AND k.entry_id=e.id), e.id DESC LIMIT 1000", matches.join(" OR "))).map_err(err)?;
        for id in matching
            .query_map(rusqlite::params_from_iter(values.iter()), |r| {
                r.get::<_, i64>(0)
            })
            .map_err(err)?
        {
            let id = id.map_err(err)?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    // Always revisit manually retained sources, including old revised articles.
    let mut members = c
        .prepare("SELECT entry_id FROM topic_member WHERE topic_id=?1")
        .map_err(err)?;
    for id in members
        .query_map([topic.id], |r| r.get::<_, i64>(0))
        .map_err(err)?
    {
        let id = id.map_err(err)?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let mut excluded_stmt = c
        .prepare("SELECT entry_id FROM topic_feedback WHERE topic_id=?1 ORDER BY entry_id")
        .map_err(err)?;
    let excluded = excluded_stmt
        .query_map([topic.id], |r| r.get::<_, i64>(0))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let mut candidates = Vec::new();
    let mut pending = 0;
    for id in &ids {
        if excluded.contains(id) {
            continue;
        }
        let Ok(mut s) = source(c, *id) else { continue };
        let retained: Option<String> = c
            .query_row(
                "SELECT group_name FROM topic_member WHERE topic_id=?1 AND entry_id=?2",
                params![topic.id, id],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)?;
        if let Some(group) = retained.filter(|g| !g.is_empty()) {
            s.group_name = group;
        }
        let checked: Option<String> = c
            .query_row(
                "SELECT version FROM topic_checked WHERE topic_id=?1 AND scope=?2 AND entry_id=?3",
                params![topic.id, scope, id],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)?;
        if checked.as_deref() != Some(&s.content_version) {
            pending += 1;
            // Rank local previews, while retaining a recent discovery lane for
            // aliases/languages absent from the small lexical hint set. AI still
            // decides relevance; these scores never become confirmed news.
            let title = s.title.to_lowercase();
            let preview = s.body.to_lowercase();
            let score = terms
                .iter()
                .map(|word| {
                    if title.contains(word) {
                        4
                    } else if preview.contains(word) {
                        1
                    } else {
                        0
                    }
                })
                .sum::<usize>();
            candidates.push((score, s));
        }
    }
    let recent = candidates
        .iter()
        .take(12)
        .map(|(_, s)| s.entry_id)
        .collect::<Vec<_>>();
    candidates.sort_by(|(a, x), (b, y)| b.cmp(a).then_with(|| y.entry_id.cmp(&x.entry_id)));
    let mut selected = Vec::new();
    let mut chosen = std::collections::HashSet::new();
    // Interleave recent previews with lexical matches throughout each batch.
    let mut ranked = candidates.iter();
    let mut recent = recent.iter();
    while selected.len() < 60 {
        let next = if selected.len() % 5 == 4 {
            recent
                .next()
                .and_then(|id| candidates.iter().find(|(_, s)| s.entry_id == *id))
                .or_else(|| ranked.next())
        } else {
            ranked.next().or_else(|| {
                recent
                    .next()
                    .and_then(|id| candidates.iter().find(|(_, s)| s.entry_id == *id))
            })
        };
        let Some((_, s)) = next else { break };
        if chosen.insert(s.entry_id) {
            selected.push(s.clone());
        }
    }
    let candidates = selected;
    let guard = Guard {
        scope,
        versions: candidates
            .iter()
            .map(|s| (s.entry_id, s.content_version.clone()))
            .collect(),
    };
    Ok(Scan {
        total,
        scanned: ids.len(),
        pending,
        excluded,
        candidates,
        guard,
    })
}

pub async fn continue_run(
    pool: &Pool,
    request: String,
    input: Input,
    configuration: serde_json::Value,
) -> Result<(), String> {
    let fence = crate::agent::runtime::current_commit_fence();
    store::db(pool, move |c| {
        let tx = c.transaction().map_err(err)?;
        let n = tx.execute("UPDATE topic_run SET input_json=?1,configuration_json=?2 WHERE request_id=?3 AND topic_id=?4 AND status='running'", params![serde_json::to_string(&input).map_err(err)?,configuration.to_string(),request,input.topic_id]).map_err(err)?;
        if n != 1 { return Err("Briefing run changed; retry".into()); }
        fence.commit(tx)
    }).await
}
pub fn verify(c: &Connection, input: &Input, guard: &Guard) -> Result<(), String> {
    let topic = store::get(c, input.topic_id)?;
    let now = store::input(c, input.topic_id)?;
    if topic.mode != "automatic"
        || scope(&topic) != guard.scope
        || topic.revision != input.revision
        || now.baseline_id != input.baseline_id
    {
        return Err("Topic or successful baseline changed; retry".into());
    }
    for (id, version) in &guard.versions {
        if source(c, *id)?.content_version != *version {
            return Err("Article changed during selection; retry".into());
        }
    }
    for s in &input.sources {
        if source(c, s.entry_id)?.content_version != s.content_version {
            return Err("Selected source changed; retry".into());
        }
    }
    Ok(())
}
pub fn acknowledge(c: &Connection, id: i64, guard: &Guard) -> Result<(), String> {
    for (entry, version) in &guard.versions {
        c.execute("INSERT INTO topic_checked(topic_id,scope,entry_id,version) VALUES(?1,?2,?3,?4) ON CONFLICT(topic_id,scope,entry_id) DO UPDATE SET version=excluded.version",params![id,guard.scope,entry,version]).map_err(err)?;
    }
    c.execute(
        "UPDATE topic SET last_checked_at=datetime('now') WHERE id=?1",
        [id],
    )
    .map_err(err)?;
    Ok(())
}
pub async fn check_batch(pool: &Pool, input: Input) -> Result<(), String> {
    let fence = crate::agent::runtime::current_commit_fence();
    store::db(pool, move |c| {
        let tx = c.transaction().map_err(err)?;
        let guard = input.auto_guard.as_ref().ok_or("Missing selection guard")?;
        verify(&tx, &input, guard)?;
        acknowledge(&tx, input.topic_id, guard)?;
        fence.commit(tx)
    })
    .await
}
pub async fn finish_check(pool: &Pool, request: String) -> Result<(), String> {
    let fence = crate::agent::runtime::current_commit_fence();
    store::db(pool, move |c| {
        let tx = c.transaction().map_err(err)?;
        tx.execute("UPDATE topic_run SET status='no_information',finished_at=datetime('now') WHERE request_id=?1 AND status='running'", [request]).map_err(err)?;
        fence.commit(tx)
    }).await
}

/// Explicit retry of a possibly over-conservative no-information decision.
/// Preserve versions actually used by any successful briefing.
pub async fn recheck(pool: &Pool, id: i64) -> Result<(), String> {
    store::db(pool, move |c| {
        let tx = c.transaction().map_err(err)?;
        let topic = store::get(&tx, id)?;
        if topic.mode != "automatic" || topic.archived {
            return Err("Restore an automatic topic before rechecking".into());
        }
        let used = store::history(&tx, id)?
            .into_iter()
            .flat_map(|b| b.input.sources)
            .map(|s| (s.entry_id, s.content_version))
            .collect::<std::collections::HashSet<_>>();
        let scope = scope(&topic);
        let checked = tx
            .prepare("SELECT entry_id,version FROM topic_checked WHERE topic_id=?1 AND scope=?2")
            .map_err(err)?
            .query_map(params![id, scope], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        for (entry, version) in checked {
            if !used.contains(&(entry, version)) {
                tx.execute(
                    "DELETE FROM topic_checked WHERE topic_id=?1 AND scope=?2 AND entry_id=?3",
                    params![id, scope, entry],
                )
                .map_err(err)?;
            }
        }
        tx.execute("DELETE FROM topic_selection WHERE topic_id=?1", [id])
            .map_err(err)?;
        tx.commit().map_err(err)
    })
    .await
}
pub async fn cache(
    pool: &Pool,
    id: i64,
    key: String,
    selection: Option<String>,
) -> Result<Option<String>, String> {
    let fence = crate::agent::runtime::current_commit_fence();
    store::db(pool, move |c| {
        if let Some(selection) = selection {
            let tx = c.transaction().map_err(err)?;
            // Only the latest selection needs caching; old successful history lives separately.
            tx.execute("DELETE FROM topic_selection WHERE topic_id=?1", [id])
                .map_err(err)?;
            tx.execute(
                "INSERT INTO topic_selection(topic_id,cache_key,selection_json) VALUES(?1,?2,?3)",
                params![id, key, selection],
            )
            .map_err(err)?;
            fence.commit(tx)?;
            Ok(None)
        } else {
            c.query_row(
                "SELECT selection_json FROM topic_selection WHERE topic_id=?1 AND cache_key=?2",
                params![id, key],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)
        }
    })
    .await
}
#[cfg(test)]
pub async fn options(pool: &Pool, id: i64, mode: String, budget: usize) -> Result<(), String> {
    if !["manual", "automatic"].contains(&mode.as_str()) {
        return Err("Invalid curation mode".into());
    }
    store::db(pool, move |c| {
        c.execute(
            "UPDATE topic SET mode=?1,input_budget=?2,revision=revision+1 WHERE id=?3",
            params![mode, budget as i64, id],
        )
        .map_err(err)?;
        Ok(())
    })
    .await
}
pub async fn feedback(
    pool: &Pool,
    id: i64,
    entry: i64,
    reason: Option<String>,
) -> Result<(), String> {
    if reason
        .as_ref()
        .is_some_and(|s| !["unrelated", "skip"].contains(&s.as_str()))
    {
        return Err("Invalid feedback".into());
    }
    store::db(pool,move|c| {let tx=c.transaction().map_err(err)?;store::get(&tx,id)?;
        if let Some(reason)=reason {tx.execute("INSERT INTO topic_feedback(topic_id,entry_id,reason) VALUES(?1,?2,?3) ON CONFLICT(topic_id,entry_id) DO UPDATE SET reason=excluded.reason",params![id,entry,reason]).map_err(err)?;
            tx.execute("DELETE FROM topic_member WHERE topic_id=?1 AND entry_id=?2",params![id,entry]).map_err(err)?;
        } else {tx.execute("DELETE FROM topic_feedback WHERE topic_id=?1 AND entry_id=?2",params![id,entry]).map_err(err)?;tx.execute("DELETE FROM topic_checked WHERE topic_id=?1 AND entry_id=?2",params![id,entry]).map_err(err)?;}
        tx.execute("UPDATE topic SET revision=revision+1 WHERE id=?1",[id]).map_err(err)?;tx.execute("DELETE FROM topic_selection WHERE topic_id=?1",[id]).map_err(err)?;tx.commit().map_err(err)
    }).await
}
pub async fn read(pool: &Pool, id: i64, brief: i64) -> Result<(), String> {
    store::db(pool,move|c| {c.execute("UPDATE topic SET read_brief_id=MAX(read_brief_id,?2) WHERE id=?1 AND EXISTS(SELECT 1 FROM topic_brief WHERE topic_id=?1 AND id=?2)",params![id,brief]).map_err(err)?;Ok(())}).await
}

#[cfg(test)]
mod tests;
