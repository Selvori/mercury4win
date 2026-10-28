//! Immutable published AI result scopes; each explicit continuation forks work.
//! Read/star mutations retain membership, content changes require explicit refresh.
use super::{entry_store::EntryFilter, search_store};
use crate::agent::{
    runtime::current_commit_fence,
    search_filter::FilterError,
    topic_search::{self, Candidate, Chunk, Judgment, Plan, Route},
};
use deadpool_sqlite::Pool;
use rusqlite::{params, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn migrate(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch("CREATE TABLE ai_search_revision(id INTEGER PRIMARY KEY CHECK(id=1),content INTEGER NOT NULL,flags INTEGER NOT NULL);
        INSERT INTO ai_search_revision VALUES(1,0,0);
        CREATE TABLE ai_search_session(id TEXT PRIMARY KEY,input_key TEXT NOT NULL,sentence TEXT NOT NULL,manual_json TEXT NOT NULL,plan_json TEXT NOT NULL,filter_json TEXT NOT NULL,anchor TEXT NOT NULL,revision INTEGER NOT NULL,flags INTEGER NOT NULL,checked INTEGER NOT NULL DEFAULT 0,lane_turn INTEGER NOT NULL DEFAULT 0,published INTEGER NOT NULL DEFAULT 0,created_at TEXT NOT NULL DEFAULT(datetime('now')));
        CREATE INDEX ai_search_session_key ON ai_search_session(input_key,revision,flags,checked);
        CREATE TABLE ai_search_judgment(session_id TEXT NOT NULL REFERENCES ai_search_session(id) ON DELETE CASCADE,entry_id INTEGER NOT NULL,score INTEGER NOT NULL,reason TEXT NOT NULL,evidence_key TEXT,evidence_quote TEXT,limited INTEGER NOT NULL,PRIMARY KEY(session_id,entry_id));
        CREATE INDEX ai_search_matches ON ai_search_judgment(session_id,score,entry_id);
        CREATE TRIGGER ai_search_entry_insert AFTER INSERT ON entry BEGIN UPDATE ai_search_revision SET content=content+1 WHERE id=1; END;
        CREATE TRIGGER ai_search_entry_delete AFTER DELETE ON entry BEGIN UPDATE ai_search_revision SET content=content+1 WHERE id=1; END;
        CREATE TRIGGER ai_search_entry_content AFTER UPDATE OF title,summary,feed_content,feed_content_type,published_at,feed_id,is_deleted ON entry
        WHEN old.title IS NOT new.title OR old.summary IS NOT new.summary OR old.feed_content IS NOT new.feed_content OR old.feed_content_type IS NOT new.feed_content_type OR old.published_at IS NOT new.published_at OR old.feed_id IS NOT new.feed_id OR old.is_deleted IS NOT new.is_deleted
        BEGIN UPDATE ai_search_revision SET content=content+1 WHERE id=1; END;
        CREATE TRIGGER ai_search_entry_flags AFTER UPDATE OF is_read,is_starred ON entry WHEN old.is_read IS NOT new.is_read OR old.is_starred IS NOT new.is_starred BEGIN UPDATE ai_search_revision SET flags=flags+1 WHERE id=1; END;")?;
    for (table, updates) in [
        ("content", "html,cleaned_html,markdown"),
        ("entry_note", "markdown"),
        ("entry_tag", "tag_id,entry_id"),
        ("tag", "name"),
        ("feed", "title,feed_url"),
    ] {
        for action in ["INSERT", "DELETE"] {
            c.execute_batch(&format!("CREATE TRIGGER ai_search_{table}_{} AFTER {action} ON {table} BEGIN UPDATE ai_search_revision SET content=content+1 WHERE id=1; END;",action.to_lowercase()))?;
        }
        c.execute_batch(&format!("CREATE TRIGGER ai_search_{table}_update AFTER UPDATE OF {updates} ON {table} BEGIN UPDATE ai_search_revision SET content=content+1 WHERE id=1; END;"))?;
    }
    Ok(())
}
fn storage(_: impl std::fmt::Display) -> FilterError {
    FilterError::new("storage", "")
}
pub async fn db<T: Send + 'static>(
    pool: &Pool,
    work: impl FnOnce(&mut Connection) -> Result<T, FilterError> + Send + 'static,
) -> Result<T, FilterError> {
    pool.get()
        .await
        .map_err(storage)?
        .interact(work)
        .await
        .map_err(storage)?
}
pub fn identity(sentence: &str, manual: &EntryFilter, config: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::json!([
                topic_search::VERSION,
                sentence.trim(),
                canonical(manual),
                config
            ])
            .to_string()
        )
    )
}
pub fn canonical(f: &EntryFilter) -> EntryFilter {
    let mut c = f.clone();
    c.ai_search_id = None;
    c.is_read = f.is_read.or(f.unread_only.then_some(false));
    c.unread_only = false;
    let mut ids = c.tag_ids.unwrap_or_default();
    ids.sort();
    ids.dedup();
    c.tag_ids = Some(ids);
    if c.search.as_ref().is_none_or(|s| s.is_empty()) {
        c.search = None;
    }
    c
}
fn normalized(f: &EntryFilter) -> serde_json::Value {
    serde_json::to_value(canonical(f)).unwrap()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub preliminary: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan: Option<crate::agent::staged_search::Progress>,
    pub id: String,
    pub sentence: String,
    pub input_key: String,
    pub plan: Plan,
    pub filter: EntryFilter,
    pub manual: EntryFilter,
    pub anchor: String,
    pub revision: i64,
    pub flags: i64,
    pub checked: usize,
    pub total: usize,
    pub pending: usize,
    pub matches: usize,
    pub stale: bool,
    pub lane_turn: usize,
}
pub fn revision(c: &Connection) -> Result<(i64, i64), FilterError> {
    c.query_row(
        "SELECT content,flags FROM ai_search_revision WHERE id=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .map_err(storage)
}
fn get(c: &Connection, id: &str) -> Result<Session, FilterError> {
    if id.starts_with("scan:") {
        return super::staged_search_store::session(c, id);
    }
    let row=c.query_row("SELECT sentence,input_key,plan_json,filter_json,manual_json,anchor,revision,flags,checked,lane_turn FROM ai_search_session WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)? as usize,r.get::<_,i64>(9)? as usize))).optional().map_err(storage)?.ok_or_else(||FilterError::new("expired",""))?;
    let plan: Plan = serde_json::from_str(&row.2).map_err(storage)?;
    let filter: EntryFilter = serde_json::from_str(&row.3).map_err(storage)?;
    let stale = revision(c)?.0 != row.6;
    let (total, pending) = counts(c, id, &plan, &filter, &row.5)?;
    let matches = c
        .query_row(
            "SELECT count(*) FROM ai_search_judgment WHERE session_id=?1 AND score>=60",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(storage)? as usize;
    Ok(Session {
        preliminary: 0,
        scan: None,
        id: id.into(),
        sentence: row.0,
        input_key: row.1,
        plan,
        filter,
        manual: serde_json::from_str(&row.4).map_err(storage)?,
        anchor: row.5,
        revision: row.6,
        flags: row.7,
        checked: row.8,
        lane_turn: row.9,
        total,
        pending,
        matches,
        stale,
    })
}
pub async fn load(p: &Pool, id: &str) -> Result<Session, FilterError> {
    let id = id.to_owned();
    db(p, move |c| get(c, &id)).await
}
pub async fn cached(p: &Pool, key: &str) -> Result<Option<Session>, FilterError> {
    let key = key.to_owned();
    db(p,move|c|{
    let(rev,flags)=revision(c)?;
    let id:Option<String>=c.query_row("SELECT id FROM ai_search_session WHERE input_key=?1 AND revision=?2 AND flags=?3 AND published=1 ORDER BY checked DESC,rowid DESC LIMIT 1",params![key,rev,flags],|r|r.get(0)).optional().map_err(storage)?;
    id.map(|id|get(c,&id)).transpose()
}).await
}
pub async fn ensure_current(p: &Pool, s: &Session) -> Result<(), FilterError> {
    let s = s.clone();
    db(p, move |c| {
        if revision(c)? != (s.revision, s.flags) {
            return Err(FilterError::new("stale", ""));
        }
        Ok(())
    })
    .await
}
pub async fn create(
    p: &Pool,
    key: String,
    sentence: String,
    manual: EntryFilter,
    plan: Plan,
    filter: EntryFilter,
) -> Result<Session, FilterError> {
    let fence = current_commit_fence();
    db(p,move|c|{
    // Pin a clean-index snapshot before recording its revision. A concurrent
    // body update between refresh and BEGIN must not produce a falsely complete
    // cached search using yesterday's index and today's revision.
    let mut retries=0;
    let tx=loop {
        search_store::refresh(c).map_err(storage)?;
        let tx=c.unchecked_transaction().map_err(storage)?;
        let dirty:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM search_dirty)",[],|r|r.get(0)).map_err(storage)?;
        if !dirty {break tx;}
        drop(tx);retries+=1;
        if retries>=8 {return Err(FilterError::new("stale",""));}
    };
    let(rev,flags)=revision(&tx)?;let id=uuid::Uuid::new_v4().to_string();
    // Short-lived failures must not evict the last valid visible scope. Keep
    // this week's work and the latest 20 older sessions; expire only on a new
    // explicit search, without background work or model calls.
    tx.execute("DELETE FROM ai_search_judgment WHERE session_id IN(SELECT id FROM ai_search_session WHERE created_at<datetime('now','-7 days') AND id IN(SELECT id FROM ai_search_session ORDER BY rowid DESC LIMIT -1 OFFSET 20))",[]).map_err(storage)?;
    tx.execute("DELETE FROM ai_search_session WHERE created_at<datetime('now','-7 days') AND id IN(SELECT id FROM ai_search_session ORDER BY rowid DESC LIMIT -1 OFFSET 20)",[]).map_err(storage)?;
    tx.execute("INSERT INTO ai_search_session(id,input_key,sentence,manual_json,plan_json,filter_json,anchor,revision,flags) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![id,key,sentence,serde_json::to_string(&manual).unwrap(),serde_json::to_string(&plan).unwrap(),serde_json::to_string(&filter).unwrap(),chrono::Utc::now().to_rfc3339(),rev,flags]).map_err(storage)?;
    fence.commit(tx).map_err(storage)?;get(c,&id)
}).await
}
pub async fn fork(p: &Pool, s: &Session) -> Result<Session, FilterError> {
    let s = s.clone();
    let fence = current_commit_fence();
    db(p,move|c|{
    if revision(c)?!=(s.revision,s.flags){return Err(FilterError::new("stale",""));}
    let tx=c.transaction().map_err(storage)?;let id=uuid::Uuid::new_v4().to_string();
    tx.execute("INSERT INTO ai_search_session SELECT ?1,input_key,sentence,manual_json,plan_json,filter_json,anchor,revision,flags,checked,lane_turn,0,datetime('now') FROM ai_search_session WHERE id=?2",params![id,s.id]).map_err(storage)?;
    tx.execute("INSERT INTO ai_search_judgment SELECT ?1,entry_id,score,reason,evidence_key,evidence_quote,limited FROM ai_search_judgment WHERE session_id=?2",params![id,s.id]).map_err(storage)?;
    fence.commit(tx).map_err(storage)?;get(c,&id)
}).await
}
pub async fn publish(p: &Pool, id: &str) -> Result<(), FilterError> {
    let id = id.to_owned();
    let fence = current_commit_fence();
    db(p, move |c| {
        let tx = c.transaction().map_err(storage)?;
        tx.execute("UPDATE ai_search_session SET published=1 WHERE id=?1", [id])
            .map_err(storage)?;
        fence.commit(tx).map_err(storage)
    })
    .await
}

fn bind(values: &mut Vec<Value>, value: impl Into<Value>) -> String {
    values.push(value.into());
    format!("?{}", values.len())
}
fn term_sql(term: &str, fields: &[&str], values: &mut Vec<Value>) -> String {
    if fields.is_empty() {
        return "0".into();
    }
    if term.chars().count() >= 3 {
        let query = format!(
            "{{{}}} : \"{}\"",
            fields.join(" "),
            term.replace('"', "\"\"")
        );
        let param = bind(values, query);
        format!("e.id IN(SELECT rowid FROM entry_search WHERE entry_search MATCH {param})")
    } else {
        let param = bind(values, term.to_owned());
        format!(
            "({})",
            fields
                .iter()
                .map(|f| format!("instr(s.{f},{param})>0"))
                .collect::<Vec<_>>()
                .join(" OR ")
        )
    }
}
pub(crate) fn route_sql(route: &Route, fields: &[&str], values: &mut Vec<Value>) -> String {
    format!(
        "({})",
        route
            .groups
            .iter()
            .map(|g| format!(
                "({})",
                g.iter()
                    .map(|t| term_sql(t, fields, values))
                    .collect::<Vec<_>>()
                    .join(" OR ")
            ))
            .collect::<Vec<_>>()
            .join(" AND ")
    )
}
fn fields(filter: &EntryFilter) -> Vec<&'static str> {
    use search_store::SearchField::*;
    let mut fields = match filter.search_field {
        All => vec!["title", "summary", "body"],
        Title => vec!["title"],
        Summary => vec!["summary"],
        Body => vec!["body"],
        Note => vec!["note"],
    };
    if filter.exclude_title {
        fields.retain(|f| *f != "title");
    }
    fields
}
pub(crate) fn recall(
    plan: &Plan,
    filter: &EntryFilter,
    anchor: &str,
) -> Result<(Vec<Value>, String, String), FilterError> {
    let mut hard = filter.clone();
    hard.ai_search_id = None;
    hard.search = None;
    let c = search_store::compile(&hard, anchor).map_err(storage)?;
    let mut values = c.values;
    let fields = fields(filter);
    let routes = plan
        .routes
        .iter()
        .map(|r| route_sql(r, &fields, &mut values))
        .collect::<Vec<_>>();
    let predicate = format!(
        "({}) AND ({})",
        c.conditions.join(" AND "),
        routes.join(" OR ")
    );
    let score = routes
        .iter()
        .zip(&plan.routes)
        .map(|(r, p)| format!("CASE WHEN {r} THEN {} ELSE 0 END", p.groups.len() * 10))
        .collect::<Vec<_>>()
        .join("+");
    Ok((values, predicate, score))
}
fn counts(
    c: &Connection,
    id: &str,
    plan: &Plan,
    filter: &EntryFilter,
    anchor: &str,
) -> Result<(usize, usize), FilterError> {
    let (mut values, predicate, _) = recall(plan, filter, anchor)?;
    let id = bind(&mut values, id.to_owned());
    c.query_row(&format!("SELECT count(*),COALESCE(sum(CASE WHEN NOT EXISTS(SELECT 1 FROM ai_search_judgment j WHERE j.session_id={id} AND j.entry_id=e.id) THEN 1 ELSE 0 END),0) FROM entry e JOIN entry_search s ON s.rowid=e.id WHERE {predicate}"),rusqlite::params_from_iter(values),|r|Ok((r.get::<_,i64>(0)? as usize,r.get::<_,i64>(1)? as usize))).map_err(storage)
}
pub async fn batch(p: &Pool, s: &Session) -> Result<Vec<Candidate>, FilterError> {
    let s = s.clone();
    db(p,move|c|{
    if revision(c)?!=(s.revision,s.flags){return Err(FilterError::new("stale",""));}
    let mut base=s.filter.clone();base.ai_search_id=None;base.search=None;
    let hard=search_store::compile(&base,&s.anchor).map_err(storage)?;
    let all_fields=fields(&base);
    let metadata_fields=all_fields.iter().copied().filter(|f|["title","summary"].contains(f)).collect::<Vec<_>>();
    let channels=if all_fields.contains(&"body")&&!metadata_fields.is_empty(){vec![metadata_fields,all_fields.clone()]}else{vec![all_fields.clone()]};
    // Route languages alternate before field/order lanes; later batches rotate
    // the first lane. No latest-N reservoir silently discards older/body hits.
    let mut routes=Vec::new();let mut zh=s.plan.routes.iter().filter(|r|r.language=="zh");let mut en=s.plan.routes.iter().filter(|r|r.language=="en");
    loop{let(a,b)=(zh.next(),en.next());if a.is_none()&&b.is_none(){break;}if let Some(r)=a{routes.push(r);}if let Some(r)=b{routes.push(r);}}
    routes.extend(s.plan.routes.iter().filter(|r|r.language=="neutral"));
    let mut lanes=Vec::new();
    for order in ["julianday(e.published_at) DESC,e.id DESC","julianday(e.published_at) ASC,e.id ASC"] {
        for channel in &channels {for route in &routes {
            let mut values=hard.values.clone();let predicate=route_sql(route,channel,&mut values);let id=bind(&mut values,s.id.clone());
            let sql=format!("SELECT e.id FROM entry e JOIN entry_search s ON s.rowid=e.id WHERE {} AND {predicate} AND NOT EXISTS(SELECT 1 FROM ai_search_judgment j WHERE j.session_id={id} AND j.entry_id=e.id) ORDER BY {order} LIMIT {}",hard.conditions.join(" AND "),topic_search::BATCH_SIZE);
            let ids=c.prepare(&sql).map_err(storage)?.query_map(rusqlite::params_from_iter(values),|r|r.get::<_,i64>(0)).map_err(storage)?.collect::<rusqlite::Result<Vec<_>>>().map_err(storage)?;lanes.push(ids);
        }}
    }
    let mut chosen=Vec::new();let mut seen=std::collections::HashSet::new();
    'outer:for row in 0..topic_search::BATCH_SIZE {for lane in 0..lanes.len(){let lane=&lanes[(lane+s.lane_turn)%lanes.len()];if let Some(id)=lane.get(row){if seen.insert(*id){chosen.push(*id);if chosen.len()==topic_search::BATCH_SIZE{break 'outer;}}}}}
    chosen.into_iter().map(|id|candidate(c,id,&s.plan,s.filter.search_field==search_store::SearchField::Note)).collect()
}).await
}

pub(crate) fn fragments(text: &str, plan: &Plan, max: usize) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![];
    }
    let mut folded = String::new();
    let mut map = Vec::new();
    for (i, c) in chars.iter().enumerate() {
        for ch in c.to_lowercase() {
            map.push((folded.len(), i));
            folded.push(ch);
        }
    }
    let terms = plan
        .routes
        .iter()
        .flat_map(|r| &r.groups)
        .flatten()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let mut windows = Vec::new();
    // First AND last occurrence per term, scored by independent term density
    // and specificity. A short substring such as AI inside "maintenance" must
    // not force every excerpt to the unrelated beginning of a long article.
    for term in &terms {
        for at in [folded.find(term), folded.rfind(term)]
            .into_iter()
            .flatten()
        {
            let center = map[map.partition_point(|(b, _)| *b <= at).saturating_sub(1)].1;
            let start = center.saturating_sub(160);
            let end = (start + 700).min(chars.len());
            let window = chars[start..end].iter().collect::<String>().to_lowercase();
            let score = terms
                .iter()
                .filter(|t| window.contains(**t))
                .map(|t| t.chars().count().min(24).pow(2))
                .sum::<usize>();
            windows.push((score, start, end));
        }
    }
    windows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut spans = Vec::new();
    for (_, start, end) in windows {
        if !spans.iter().any(|(a, b)| start < *b && end > *a) {
            spans.push((start, end));
            if spans.len() == max {
                break;
            }
        }
    }
    if spans.is_empty() {
        spans.push((0, 700.min(chars.len())));
    }
    spans.sort();
    spans
        .into_iter()
        .map(|(a, b)| chars[a..b].iter().collect())
        .collect()
}
fn candidate(c: &Connection, id: i64, plan: &Plan, notes: bool) -> Result<Candidate, FilterError> {
    let doc = search_store::document(c, id)
        .map_err(storage)?
        .ok_or_else(|| FilterError::new("stale", ""))?;
    let title = doc[0].chars().take(360).collect::<String>();
    let mut chunks = vec![Chunk {
        key: "title".into(),
        text: title.clone(),
    }];
    if !doc[1].is_empty() {
        chunks.push(Chunk {
            key: "summary".into(),
            text: fragments(&doc[1], plan, 1).join("\n"),
        });
    }
    let index = if notes { 3 } else { 2 };
    let snippets = fragments(&doc[index], plan, 3);
    let sent = snippets.iter().map(|s| s.chars().count()).sum::<usize>();
    for (i, text) in snippets.into_iter().enumerate() {
        chunks.push(Chunk {
            key: format!("{}.{}", if notes { "note" } else { "body" }, i + 1),
            text,
        });
    }
    let published_at = c
        .query_row("SELECT published_at FROM entry WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .map_err(storage)?;
    Ok(Candidate {
        entry_id: id,
        title,
        published_at,
        chunks,
        limited: doc[index].is_empty() || sent < doc[index].chars().count(),
    })
}
pub async fn save_batch(
    p: &Pool,
    s: &Session,
    candidates: &[Candidate],
    judgments: &[Judgment],
) -> Result<Session, FilterError> {
    let s = s.clone();
    let cs = candidates.to_vec();
    let js = judgments.to_vec();
    let fence = current_commit_fence();
    db(p, move |c| {
        let tx = c.transaction().map_err(storage)?;
        if revision(&tx)? != (s.revision, s.flags) {
            return Err(FilterError::new("stale", ""));
        }
        let published: bool = tx
            .query_row(
                "SELECT published FROM ai_search_session WHERE id=?1",
                [&s.id],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if published {
            return Err(FilterError::new("changed", ""));
        }
        for j in js {
            let candidate = cs
                .iter()
                .find(|c| c.entry_id == j.entry_id)
                .ok_or_else(|| FilterError::new("invalidJudgment", ""))?;
            tx.execute(
                "INSERT INTO ai_search_judgment VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    s.id,
                    j.entry_id,
                    j.score,
                    j.reason,
                    j.evidence.as_ref().map(|e| &e.key),
                    j.evidence.as_ref().map(|e| &e.quote),
                    candidate.limited
                ],
            )
            .map_err(storage)?;
        }
        tx.execute(
            "UPDATE ai_search_session SET checked=checked+?2,lane_turn=lane_turn+?2 WHERE id=?1",
            params![s.id, cs.len() as i64],
        )
        .map_err(storage)?;
        fence.commit(tx).map_err(storage)?;
        get(c, &s.id)
    })
    .await
}

/// Used by both pages and mutations. Never fall back to ordinary full-library
/// filtering when the token is missing, expired or paired with changed filters.
pub fn validate_scope(c: &Connection, filter: &EntryFilter, strict: bool) -> Result<bool, String> {
    let Some(id) = filter.ai_search_id.as_deref() else {
        return Ok(false);
    };
    if id.starts_with("scan:") {
        return super::staged_search_store::validate_scope(c, filter, strict);
    }
    let row: Option<(String, i64, bool)> = c
        .query_row(
            "SELECT filter_json,revision,published FROM ai_search_session WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((raw, rev, published)) = row else {
        return Err("AI_SEARCH_EXPIRED".into());
    };
    let saved: EntryFilter = serde_json::from_str(&raw).map_err(|_| "AI_SEARCH_EXPIRED")?;
    if !published || normalized(&saved) != normalized(filter) {
        return Err("AI_SEARCH_SCOPE_CHANGED".into());
    }
    let stale = revision(c).map_err(|_| "AI_SEARCH_EXPIRED")?.0 != rev;
    if stale && strict {
        return Err("AI_SEARCH_STALE".into());
    }
    Ok(stale)
}

pub fn scope_anchor(c: &Connection, filter: &EntryFilter) -> Result<Option<String>, String> {
    filter
        .ai_search_id
        .as_ref()
        .map(|id| {
            c.query_row(
                "SELECT anchor FROM ai_search_session WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| "AI_SEARCH_EXPIRED".to_owned())
        })
        .transpose()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<usize>,
    pub score: i64,
    pub reason: String,
    pub quote: String,
    pub limited: bool,
}
pub fn hit(c: &Connection, session: &str, id: i64) -> Result<Option<Hit>, String> {
    let mut hit=c.query_row("SELECT score,reason,COALESCE(evidence_quote,''),limited FROM ai_search_judgment WHERE session_id=?1 AND entry_id=?2 AND (score>=60 OR (evidence_key='keyword' AND score>0))",params![session,id],|r|Ok(Hit{score:r.get(0)?,reason:r.get(1)?,quote:r.get(2)?,limited:r.get(3)?,basis:None,parts:None})).optional().map_err(|e|e.to_string())?;
    if session.starts_with("scan:") {
        if let Some(h) = &mut hit {
            let (basis, parts) = super::staged_search_store::hit_basis(c, session, id)?;
            h.basis = Some(basis);
            h.parts = Some(parts);
        }
    }
    Ok(hit)
}

#[cfg(test)]
mod tests;
