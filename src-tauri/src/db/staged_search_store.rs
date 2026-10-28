//! Durable scan work, versioned stage caches, and immutable published snapshots.
use super::{entry_store::EntryFilter, search_store, topic_search_store as old};
use crate::agent::{
    runtime::current_commit_fence,
    search_filter::FilterError,
    staged_search::{
        self, FinalReview, Item, Material, Part, PartReview, Preview, PreviewReview, Progress,
        Title, TitleReview, Versions,
    },
    topic_search::{Evidence, Plan},
};
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
mod keywords;

pub fn migrate(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch("CREATE TABLE ai_topic_entry_version(entry_id INTEGER PRIMARY KEY,title INTEGER NOT NULL DEFAULT 0,summary INTEGER NOT NULL DEFAULT 0,body INTEGER NOT NULL DEFAULT 0,note INTEGER NOT NULL DEFAULT 0);
      INSERT INTO ai_topic_entry_version(entry_id) SELECT id FROM entry;
      CREATE TABLE ai_topic_plan(plan_key TEXT PRIMARY KEY,plan_json TEXT NOT NULL,filter_json TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT(datetime('now')));
      CREATE TABLE ai_topic_scan(id TEXT PRIMARY KEY,input_key TEXT NOT NULL,semantic_key TEXT NOT NULL,configuration TEXT NOT NULL,algorithm TEXT NOT NULL,sentence TEXT NOT NULL,manual_json TEXT NOT NULL,plan_json TEXT NOT NULL,filter_json TEXT NOT NULL,anchor TEXT NOT NULL,revision INTEGER NOT NULL,status TEXT NOT NULL,request_id TEXT NOT NULL,force INTEGER NOT NULL DEFAULT 0,last_error TEXT,created_at TEXT NOT NULL DEFAULT(datetime('now')));
      CREATE INDEX ai_topic_scan_key ON ai_topic_scan(input_key,created_at);
      CREATE TABLE ai_topic_material(id TEXT PRIMARY KEY,entry_id INTEGER NOT NULL,basis TEXT NOT NULL,data_json TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT(datetime('now')));
      CREATE TABLE ai_topic_item(scan_id TEXT NOT NULL REFERENCES ai_topic_scan(id) ON DELETE CASCADE,entry_id INTEGER NOT NULL,title_json TEXT NOT NULL,versions_json TEXT NOT NULL,clue INTEGER NOT NULL,strong_clue INTEGER NOT NULL,generic INTEGER NOT NULL,stage TEXT NOT NULL DEFAULT 'title',title_done INTEGER NOT NULL DEFAULT 0,title_model INTEGER NOT NULL DEFAULT 0,preview_done INTEGER NOT NULL DEFAULT 0,material_id TEXT REFERENCES ai_topic_material(id),parts_done INTEGER NOT NULL DEFAULT 0,parts_total INTEGER NOT NULL DEFAULT 0,context TEXT NOT NULL DEFAULT '',signals_json TEXT NOT NULL DEFAULT '{}',evidence_json TEXT NOT NULL DEFAULT '[]',result_json TEXT,cache_hits INTEGER NOT NULL DEFAULT 0,error TEXT,PRIMARY KEY(scan_id,entry_id));
      CREATE INDEX ai_topic_item_stage ON ai_topic_item(scan_id,stage);
      CREATE TABLE ai_topic_cache(cache_key TEXT PRIMARY KEY,stage TEXT NOT NULL,result_json TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT(datetime('now')));
      CREATE TABLE ai_topic_part(scan_id TEXT NOT NULL,entry_id INTEGER NOT NULL,part_index INTEGER NOT NULL,cache_key TEXT NOT NULL,result_json TEXT NOT NULL,start_char INTEGER NOT NULL,end_char INTEGER NOT NULL,PRIMARY KEY(scan_id,entry_id,part_index),FOREIGN KEY(scan_id,entry_id) REFERENCES ai_topic_item(scan_id,entry_id) ON DELETE CASCADE);
      CREATE TABLE ai_topic_scope(scope_id TEXT PRIMARY KEY REFERENCES ai_search_session(id) ON DELETE CASCADE,scan_id TEXT NOT NULL REFERENCES ai_topic_scan(id),progress_json TEXT NOT NULL);
      CREATE TABLE ai_topic_scope_member(scope_id TEXT NOT NULL REFERENCES ai_topic_scope(scope_id) ON DELETE CASCADE,entry_id INTEGER NOT NULL,versions_json TEXT NOT NULL,basis TEXT NOT NULL,parts INTEGER NOT NULL,PRIMARY KEY(scope_id,entry_id));
      CREATE TRIGGER ai_topic_entry_insert AFTER INSERT ON entry BEGIN INSERT INTO ai_topic_entry_version(entry_id,title,summary,body,note) VALUES(new.id,1,1,1,1) ON CONFLICT(entry_id) DO UPDATE SET title=title+1,summary=summary+1,body=body+1,note=note+1; END;
      CREATE TRIGGER ai_topic_entry_delete AFTER DELETE ON entry BEGIN UPDATE ai_topic_entry_version SET title=title+1,summary=summary+1,body=body+1,note=note+1 WHERE entry_id=old.id; END;
      CREATE TRIGGER ai_topic_entry_title AFTER UPDATE OF title,feed_id,published_at,is_deleted ON entry WHEN old.title IS NOT new.title OR old.feed_id IS NOT new.feed_id OR old.published_at IS NOT new.published_at OR old.is_deleted IS NOT new.is_deleted BEGIN UPDATE ai_topic_entry_version SET title=title+1 WHERE entry_id=new.id; END;
      CREATE TRIGGER ai_topic_entry_summary AFTER UPDATE OF summary ON entry WHEN old.summary IS NOT new.summary BEGIN UPDATE ai_topic_entry_version SET summary=summary+1 WHERE entry_id=new.id; END;
      CREATE TRIGGER ai_topic_entry_feed_body AFTER UPDATE OF feed_content,feed_content_type ON entry WHEN old.feed_content IS NOT new.feed_content OR old.feed_content_type IS NOT new.feed_content_type BEGIN UPDATE ai_topic_entry_version SET body=body+1 WHERE entry_id=new.id; END;
      CREATE TRIGGER ai_topic_feed_name AFTER UPDATE OF title,feed_url ON feed WHEN old.title IS NOT new.title OR old.feed_url IS NOT new.feed_url BEGIN UPDATE ai_topic_entry_version SET title=title+1 WHERE entry_id IN(SELECT id FROM entry WHERE feed_id=new.id); END;")?;
    // These counters are independent: changing a body never reuses an exclusion
    // from a previous body version, while an unchanged title can reuse screening.
    for (table, column) in [("content", "body"), ("entry_note", "note")] {
        for action in ["INSERT", "UPDATE", "DELETE"] {
            let row = if action == "DELETE" { "old" } else { "new" };
            c.execute_batch(&format!("CREATE TRIGGER ai_topic_{table}_{} AFTER {action} ON {table} BEGIN UPDATE ai_topic_entry_version SET {column}={column}+1 WHERE entry_id={row}.entry_id; END;",action.to_lowercase()))?;
        }
    }
    Ok(())
}
fn err(_: impl std::fmt::Display) -> FilterError {
    FilterError::new("storage", "")
}
pub fn hash(value: impl Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(&value).unwrap()))
}
fn encode(value: impl Serialize) -> String {
    serde_json::to_string(&value).unwrap()
}
fn decode<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, FilterError> {
    serde_json::from_str(raw).map_err(err)
}
fn html_headings(raw: &str) -> Vec<String> {
    use markup5ever_rcdom::{Handle, NodeData};
    fn text(n: &Handle, out: &mut String) {
        if let NodeData::Text { contents } = &n.data {
            out.push_str(&contents.borrow());
        }
        if let NodeData::Element { name, .. } = &n.data {
            if ["script", "style"].contains(&name.local.as_ref()) {
                return;
            }
        }
        for child in n.children.borrow().iter() {
            text(child, out);
        }
    }
    fn walk(n: &Handle, out: &mut Vec<String>) {
        if let NodeData::Element { name, .. } = &n.data {
            if ["h1", "h2", "h3", "h4", "h5", "h6"].contains(&name.local.as_ref()) {
                let mut s = String::new();
                text(n, &mut s);
                if !s.trim().is_empty() {
                    out.push(s.trim().into());
                }
                return;
            }
        }
        for child in n.children.borrow().iter() {
            walk(child, out);
        }
    }
    let mut out = vec![];
    if let Ok(root) = htmd::HtmlToMarkdown::new().html_to_tree(raw) {
        walk(&root, &mut out);
    }
    out
}
pub async fn db<T: Send + 'static>(
    p: &Pool,
    work: impl FnOnce(&mut Connection) -> Result<T, FilterError> + Send + 'static,
) -> Result<T, FilterError> {
    old::db(p, work).await
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scan {
    pub id: String,
    pub input_key: String,
    pub semantic_key: String,
    pub configuration: String,
    pub sentence: String,
    pub manual: EntryFilter,
    pub plan: Plan,
    pub filter: EntryFilter,
    pub anchor: String,
    pub revision: i64,
    pub status: String,
    pub request_id: String,
    pub force: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalCache {
    pub review: FinalReview,
    pub material_id: String,
    pub parts: usize,
}

// Ordering is a local view of an immutable membership, not an AI condition.
fn conditions(filter: &EntryFilter) -> EntryFilter {
    let mut filter = old::canonical(filter);
    filter.sort = search_store::SearchSort::Relevance;
    filter
}
pub fn input_key(sentence: &str, manual: &EntryFilter, configuration: &str) -> String {
    hash(json!([
        staged_search::VERSION,
        sentence.split_whitespace().collect::<Vec<_>>().join(" "),
        conditions(manual),
        configuration
    ]))
}
pub fn stage_key(scan: &Scan, item: &Item, stage: &str) -> String {
    let v = &item.versions;
    let versions = if stage == "title" {
        json!([v.title])
    } else {
        match scan.filter.search_field {
            search_store::SearchField::Title => json!([v.title]),
            search_store::SearchField::Summary => json!([v.title, v.summary]),
            search_store::SearchField::Note => json!([v.title, v.note]),
            _ => json!([v.title, v.summary, v.body]),
        }
    };
    hash(json!([
        scan.semantic_key,
        stage,
        item.title.entry_id,
        versions
    ]))
}
fn material_key(scan: &Scan, item: &Item) -> String {
    let mut context = scan.clone();
    context.semantic_key = format!(
        "material-content-v1:{:?}:{}",
        scan.filter.search_field, scan.filter.exclude_title
    );
    stage_key(&context, item, "material")
}
pub async fn cache(p: &Pool, key: &str) -> Result<Option<Value>, FilterError> {
    let key = key.to_owned();
    db(p, move |c| {
        c.query_row(
            "SELECT result_json FROM ai_topic_cache WHERE cache_key=?1",
            [key],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(err)?
        .map(|raw| decode(&raw))
        .transpose()
    })
    .await
}
fn put_cache(
    c: &Connection,
    key: &str,
    stage: &str,
    value: impl Serialize,
) -> Result<(), FilterError> {
    c.execute("INSERT INTO ai_topic_cache(cache_key,stage,result_json) VALUES(?1,?2,?3) ON CONFLICT(cache_key) DO UPDATE SET result_json=excluded.result_json,created_at=datetime('now')",params![key,stage,encode(value)]).map_err(err)?;
    Ok(())
}
pub async fn plan_cache(p: &Pool, key: &str) -> Result<Option<Plan>, FilterError> {
    let key = key.to_owned();
    db(p, move |c| {
        c.query_row(
            "SELECT plan_json FROM ai_topic_plan WHERE plan_key=?1",
            [key],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(err)?
        .map(|raw| decode(&raw))
        .transpose()
    })
    .await
}
pub async fn save_plan(
    p: &Pool,
    key: String,
    plan: Plan,
    filter: EntryFilter,
) -> Result<(), FilterError> {
    let fence = current_commit_fence();
    db(p,move|c|{let tx=c.transaction().map_err(err)?;tx.execute("INSERT INTO ai_topic_plan(plan_key,plan_json,filter_json) VALUES(?1,?2,?3) ON CONFLICT(plan_key) DO UPDATE SET plan_json=excluded.plan_json,filter_json=excluded.filter_json",params![key,encode(plan),encode(filter)]).map_err(err)?;fence.commit(tx).map_err(err)}).await
}

fn get(c: &Connection, id: &str) -> Result<Scan, FilterError> {
    c.query_row("SELECT input_key,semantic_key,configuration,sentence,manual_json,plan_json,filter_json,anchor,revision,status,request_id,force,algorithm FROM ai_topic_scan WHERE id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?,r.get::<_,i64>(8)?,r.get::<_,String>(9)?,r.get::<_,String>(10)?,r.get::<_,bool>(11)?,r.get::<_,String>(12)?))).optional().map_err(err)?.ok_or_else(||FilterError::new("expired","")).and_then(|r|{
        if r.12!=staged_search::VERSION {return Err(FilterError::new("expired",""));}
        Ok(Scan{id:id.into(),input_key:r.0,semantic_key:r.1,configuration:r.2,sentence:r.3,manual:decode(&r.4)?,plan:decode(&r.5)?,filter:decode(&r.6)?,anchor:r.7,revision:r.8,status:r.9,request_id:r.10,force:r.11})
    })
}
pub async fn load(p: &Pool, id: &str) -> Result<Scan, FilterError> {
    let id = id.to_owned();
    db(p, move |c| get(c, &id)).await
}
pub async fn title_batch_limit(p: &Pool, id: &str) -> Result<usize, FilterError> {
    let id = id.to_owned();
    db(p, move |c| {
        let failed: i64 = c.query_row(
            "SELECT count(*) FROM ai_topic_item WHERE scan_id=?1 AND stage='title' AND error='timeout'",
            [&id], |r| r.get(0),
        ).map_err(err)?;
        Ok(if failed == 0 { staged_search::TITLE_BATCH_ITEMS } else {
            (failed as usize / 2).clamp(1, staged_search::TITLE_BATCH_ITEMS)
        })
    }).await
}
fn owned(c: &Connection, scan: &Scan) -> Result<(), FilterError> {
    let state: Option<(String, String)> = c
        .query_row(
            "SELECT request_id,status FROM ai_topic_scan WHERE id=?1",
            [&scan.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(err)?;
    if state != Some((scan.request_id.clone(), "running".into())) {
        return Err(FilterError::new("cancelled", ""));
    }
    Ok(())
}
pub async fn claim(
    p: &Pool,
    id: String,
    key: String,
    request: String,
) -> Result<Scan, FilterError> {
    let fence = current_commit_fence();
    db(p, move |c| {
        let mut s = get(c, &id)?;
        if s.input_key != key {
            return Err(FilterError::new("changed", ""));
        }
        let tx = c.transaction().map_err(err)?;
        tx.execute(
            "UPDATE ai_topic_scan SET request_id=?2,status='running',last_error=NULL WHERE id=?1",
            params![id, request],
        )
        .map_err(err)?;
        tx.execute(
            "UPDATE ai_topic_item SET error=NULL WHERE scan_id=?1",
            [&id],
        )
        .map_err(err)?;
        fence.commit(tx).map_err(err)?;
        s.request_id = request;
        s.status = "running".into();
        Ok(s)
    })
    .await
}

pub async fn create(
    p: &Pool,
    key: String,
    configuration: String,
    sentence: String,
    manual: EntryFilter,
    plan: Plan,
    filter: EntryFilter,
    request_id: String,
    force: bool,
) -> Result<Scan, FilterError> {
    let fence = current_commit_fence();
    db(p,move|c|{
    let mut attempts=0;let tx=loop{search_store::refresh(c).map_err(err)?;let tx=c.unchecked_transaction().map_err(err)?;let dirty:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM search_dirty)",[],|r|r.get(0)).map_err(err)?;if !dirty{break tx;}drop(tx);attempts+=1;if attempts==8{return Err(FilterError::new("stale",""));}};
    let revision=old::revision(&tx)?.0;let id=uuid::Uuid::new_v4().to_string();let anchor=chrono::Utc::now().to_rfc3339();
    let semantic_key=hash(json!([staged_search::VERSION,configuration,sentence.split_whitespace().collect::<Vec<_>>().join(" "),plan.intent,conditions(&filter)]));
    tx.execute("INSERT INTO ai_topic_scan(id,input_key,semantic_key,configuration,algorithm,sentence,manual_json,plan_json,filter_json,anchor,revision,status,request_id,force) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'running',?12,?13)",params![id,key,semantic_key,configuration,staged_search::VERSION,sentence,encode(&manual),encode(&plan),encode(&filter),anchor,revision,request_id,force]).map_err(err)?;
    let hard=search_store::compile(&filter,&anchor).map_err(err)?;let mut values=hard.values;
    // The keyword routes provide a rescue/priority hint ONLY. They are never in
    // the WHERE clause selecting the frozen title population.
    let clue_fields=if filter.search_field==search_store::SearchField::Note{vec!["note"]}else{vec!["body"]};
    let routes=plan.routes.iter().map(|r|old::route_sql(r,&clue_fields,&mut values)).collect::<Vec<_>>();
    let clue=format!("({})",routes.join(" OR "));
    let strong=routes.iter().zip(&plan.routes).filter(|(_,r)|r.groups.len()>1||r.groups.iter().flatten().any(|term|plan.entities.iter().any(|e|e.eq_ignore_ascii_case(term)))).map(|(sql,_)|sql.clone()).collect::<Vec<_>>();
    let strong=if strong.is_empty(){"0".into()}else{format!("({})",strong.join(" OR "))};
    values.push(rusqlite::types::Value::Text(id.clone()));let token=format!("?{}",values.len());
    // Freeze the newest allowed sources BEFORE semantic prioritization. This is
    // the ordinary newest ordering (undated articles last; ID breaks ties).
    let sql=format!("INSERT INTO ai_topic_item(scan_id,entry_id,title_json,versions_json,clue,strong_clue,generic) SELECT {token},e.id,json_object('entry_id',e.id,'title',COALESCE(e.title,''),'source',COALESCE(f.title,f.feed_url,'Unknown source'),'published_at',e.published_at),json_object('title',v.title,'summary',v.summary,'body',v.body,'note',v.note),CASE WHEN {clue} THEN 1 ELSE 0 END,CASE WHEN {strong} THEN 1 ELSE 0 END,0 FROM entry e LEFT JOIN feed f ON f.id=e.feed_id JOIN ai_topic_entry_version v ON v.entry_id=e.id LEFT JOIN entry_search s ON s.rowid=e.id WHERE {} ORDER BY julianday(e.published_at) DESC,e.id DESC LIMIT {}",hard.conditions.join(" AND "),staged_search::SCOPE_LIMIT);
    tx.execute(&sql,rusqlite::params_from_iter(values)).map_err(err)?;
    let titles=tx.prepare("SELECT entry_id,title_json FROM ai_topic_item WHERE scan_id=?1").map_err(err)?.query_map([&id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    for(entry,raw)in titles {let title:Title=decode(&raw)?;tx.execute("UPDATE ai_topic_item SET generic=?3 WHERE scan_id=?1 AND entry_id=?2",params![id,entry,staged_search::generic_title(&title.title)]).map_err(err)?;}
    keywords::seed(&tx,&get(&tx,&id)?)?;
    fence.commit(tx).map_err(err)?;get(c,&id)
}).await
}

fn items(c: &Connection, scan: &Scan, stage: &str, limit: usize) -> Result<Vec<Item>, FilterError> {
    let mut statement=c.prepare("SELECT title_json,versions_json,clue,strong_clue,generic,stage,material_id,parts_done,parts_total,context,signals_json,evidence_json FROM ai_topic_item WHERE scan_id=?1 AND stage=?2 ORDER BY strong_clue DESC,clue DESC,entry_id DESC LIMIT ?3").map_err(err)?;
    let rows = statement
        .query_map(params![scan.id, stage, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, String>(11)?,
            ))
        })
        .map_err(err)?;
    rows.map(|r| {
        let r = r.map_err(err)?;
        Ok(Item {
            title: decode(&r.0)?,
            versions: decode(&r.1)?,
            clue: r.2,
            strong_clue: r.3,
            generic: r.4,
            stage: r.5,
            material: r.6,
            parts_done: r.7 as usize,
            parts_total: r.8 as usize,
            context: r.9,
            signals: serde_json::from_str(&r.10).unwrap_or_default(),
            evidence: decode(&r.11)?,
        })
    })
    .collect()
}
pub async fn pending(
    p: &Pool,
    scan: &Scan,
    stage: &str,
    limit: usize,
) -> Result<Vec<Item>, FilterError> {
    let scan = scan.clone();
    let stage = stage.to_owned();
    db(p, move |c| {
        owned(c, &scan)?;
        items(c, &scan, &stage, limit)
    })
    .await
}
pub(crate) fn versions(c: &Connection, id: i64) -> Result<Option<Versions>, FilterError> {
    c.query_row("SELECT v.title,v.summary,v.body,v.note FROM ai_topic_entry_version v JOIN entry e ON e.id=v.entry_id WHERE v.entry_id=?1 AND e.is_deleted=0",[id],|r|Ok(Versions{title:r.get(0)?,summary:r.get(1)?,body:r.get(2)?,note:r.get(3)?})).optional().map_err(err)
}
fn same_versions(a: &Versions, b: &Versions, field: search_store::SearchField) -> bool {
    a.title == b.title
        && match field {
            search_store::SearchField::Title => true,
            search_store::SearchField::Summary => a.summary == b.summary,
            search_store::SearchField::Note => a.note == b.note,
            _ => a.summary == b.summary && a.body == b.body,
        }
}
pub async fn material(p: &Pool, scan: &Scan, item: &Item) -> Result<Option<Material>, FilterError> {
    let scan = scan.clone();
    let item = item.clone();
    let fence = current_commit_fence();
    db(p,move|c|{
    owned(c,&scan)?;let key=material_key(&scan,&item);
    let saved=c.query_row("SELECT data_json FROM ai_topic_material WHERE id=?1",[&key],|r|r.get::<_,String>(0)).optional().map_err(err)?;
    if let Some(raw)=saved{let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;tx.execute("UPDATE ai_topic_item SET material_id=?3 WHERE scan_id=?1 AND entry_id=?2",params![scan.id,item.title.entry_id,key]).map_err(err)?;fence.commit(tx).map_err(err)?;return Ok(Some(decode(&raw)?));}
    let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;
    let current=versions(&tx,item.title.entry_id)?;
    if !current.as_ref().is_some_and(|v|same_versions(v,&item.versions,scan.filter.search_field)){
        tx.execute("UPDATE ai_topic_item SET stage='changed',title_done=1,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,item.title.entry_id]).map_err(err)?;fence.commit(tx).map_err(err)?;return Ok(None);
    }
    let Some(doc)=search_store::document_for(&tx,item.title.entry_id,scan.filter.search_field==search_store::SearchField::Note).map_err(err)? else{return Err(FilterError::new("stale",""));};
    let(summary,text,basis)=match scan.filter.search_field {
        search_store::SearchField::Title=>(String::new(),if scan.filter.exclude_title {String::new()}else{item.title.title.clone()},"title_only"),
        search_store::SearchField::Summary=>(doc[1].clone(),doc[1].clone(),"summary_only"),
        search_store::SearchField::Note=>(String::new(),doc[3].clone(),"note_full"),
        search_store::SearchField::Body=>(String::new(),doc[2].clone(),"full_body"),
        _ if !doc[2].trim().is_empty()=>(doc[1].clone(),doc[2].clone(),"full_body"),
        _ if !doc[1].trim().is_empty()=>(doc[1].clone(),doc[1].clone(),"summary_only"),
        _=>(String::new(),if scan.filter.exclude_title {String::new()}else{item.title.title.clone()},"title_only"),
    };
    let raw:Option<String>=if basis=="note_full"{tx.query_row("SELECT markdown FROM entry_note WHERE entry_id=?1",[item.title.entry_id],|r|r.get(0)).optional().map_err(err)?}else if basis=="full_body"{tx.query_row("SELECT COALESCE(NULLIF(trim(markdown),''),NULLIF(trim(cleaned_html),''),html) FROM content WHERE entry_id=?1",[item.title.entry_id],|r|r.get(0)).optional().map_err(err)?.flatten().or(tx.query_row("SELECT feed_content FROM entry WHERE id=?1",[item.title.entry_id],|r|r.get(0)).optional().map_err(err)?.flatten())}else{None};
    let mut headings=raw.as_ref().map(|text|text.lines().filter(|line|line.trim_start().starts_with('#')).map(search_store::markdown_text).collect::<Vec<_>>()).unwrap_or_default();
    if headings.is_empty(){if let Some(raw)=raw {headings=html_headings(&raw);}}
    let material=Material{entry_id:item.title.entry_id,title:item.title.title.clone(),source:item.title.source.clone(),published_at:item.title.published_at.clone(),summary,text,headings,basis:basis.into(),versions:item.versions.clone()};
    tx.execute("INSERT OR IGNORE INTO ai_topic_material(id,entry_id,basis,data_json) VALUES(?1,?2,?3,?4)",params![key,item.title.entry_id,material.basis,encode(&material)]).map_err(err)?;
    tx.execute("UPDATE ai_topic_item SET material_id=?3 WHERE scan_id=?1 AND entry_id=?2",params![scan.id,item.title.entry_id,key]).map_err(err)?;
    fence.commit(tx).map_err(err)?;Ok(Some(material))
}).await
}

pub async fn prepare_full(
    p: &Pool,
    scan: &Scan,
    item: &Item,
    material: &Material,
) -> Result<(), FilterError> {
    let scan = scan.clone();
    let item = item.clone();
    let material = material.clone();
    let fence = current_commit_fence();
    db(p,move|c|{let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;let parts=material.text.chars().count().div_ceil(staged_search::PART_CHARS);tx.execute("UPDATE ai_topic_item SET stage=?3,preview_done=1,parts_total=?4,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,item.title.entry_id,if parts==0{"insufficient"}else{"full"},parts as i64]).map_err(err)?;fence.commit(tx).map_err(err)}).await
}
pub async fn finish_empty(p: &Pool, scan: &Scan, item: &Item) -> Result<(), FilterError> {
    let scan = scan.clone();
    let item = item.clone();
    let fence = current_commit_fence();
    db(p,move|c|{let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;tx.execute("UPDATE ai_topic_item SET stage='insufficient',title_done=1,preview_done=1,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,item.title.entry_id]).map_err(err)?;fence.commit(tx).map_err(err)}).await
}

// Keep both support and opposing context, rather than the first few positives.
fn retain_evidence(
    mut evidence: Vec<Evidence>,
    new: Option<Evidence>,
    opposing: bool,
) -> Vec<Evidence> {
    if let Some(e) = new {
        if !evidence
            .iter()
            .any(|v| v.key == e.key && v.quote == e.quote)
        {
            if opposing {
                evidence.insert(0, e);
            } else {
                evidence.push(e);
            }
        }
    }
    if evidence.len() > 8 {
        let last = evidence.pop().unwrap();
        evidence.truncate(7);
        evidence.push(last);
    }
    evidence
}
pub fn part_key(scan: &Scan, item: &Item, part: &Part) -> String {
    hash(json!([
        stage_key(scan, item, "part"),
        part.index,
        part.previous_context,
        part.previous_signals,
        part.previous_evidence
    ]))
}
pub async fn part_results(
    p: &Pool,
    scan: &Scan,
    input: &[Item],
    parts: &[Part],
    reviews: &[PartReview],
    cached: bool,
) -> Result<(), FilterError> {
    let scan = scan.clone();
    let input = input.to_vec();
    let parts = parts.to_vec();
    let reviews = reviews.to_vec();
    let fence = current_commit_fence();
    db(p,move|c|{
    let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;
    for review in &reviews {
        let item=input.iter().find(|i|i.title.entry_id==review.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;let part=parts.iter().find(|p|p.entry_id==review.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;
        let done:i64=tx.query_row("SELECT parts_done FROM ai_topic_item WHERE scan_id=?1 AND entry_id=?2",params![scan.id,review.entry_id],|r|r.get(0)).map_err(err)?;
        if done as usize!=part.index||part.start!=part.index*staged_search::PART_CHARS||part.end<=part.start{return Err(FilterError::new("changed",""));}
        let mut signals=item.signals.clone();match review.kind.as_str(){"supports"=>signals.supports+=1,"opposes"=>signals.opposes+=1,"background"=>signals.background+=1,_=>signals.uncertain+=1}
        let evidence=retain_evidence(item.evidence.clone(),review.evidence.clone(),review.kind=="opposes");
        let key=part_key(&scan,item,part);
        tx.execute("INSERT INTO ai_topic_part(scan_id,entry_id,part_index,cache_key,result_json,start_char,end_char) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![scan.id,review.entry_id,part.index as i64,key,encode(review),part.start as i64,part.end as i64]).map_err(err)?;
        tx.execute("UPDATE ai_topic_item SET parts_done=parts_done+1,parts_total=?3,stage=?4,context=?5,signals_json=?6,evidence_json=?7,cache_hits=cache_hits+?8,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,review.entry_id,part.total as i64,if part.index+1==part.total{"final"}else{"full"},review.context,encode(signals),encode(evidence),cached as i64]).map_err(err)?;
        put_cache(&tx,&key,"part",review)?;
    }fence.commit(tx).map_err(err)
}).await
}
pub async fn final_results(
    p: &Pool,
    scan: &Scan,
    input: &[Item],
    materials: &[Material],
    reviews: &[FinalReview],
    cached: bool,
) -> Result<(), FilterError> {
    let scan = scan.clone();
    let input = input.to_vec();
    let materials = materials.to_vec();
    let reviews = reviews.to_vec();
    let fence = current_commit_fence();
    db(p,move|c|{
    let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;
    for review in &reviews {let item=input.iter().find(|i|i.title.entry_id==review.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;let material=materials.iter().find(|m|m.entry_id==review.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;
        let total=material.text.chars().count().div_ceil(staged_search::PART_CHARS);
        if !cached {let coverage=tx.prepare("SELECT start_char,end_char FROM ai_topic_part WHERE scan_id=?1 AND entry_id=?2 ORDER BY part_index").map_err(err)?.query_map(params![scan.id,review.entry_id],|r|Ok((r.get::<_,i64>(0)? as usize,r.get::<_,i64>(1)? as usize))).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
            let mut end=0;for(a,b)in &coverage{if *a!=end{return Err(FilterError::new("invalidJudgment",""));}end=*b;}
            if coverage.len()!=total||end!=material.text.chars().count()||item.parts_done!=total{return Err(FilterError::new("invalidJudgment",""));}
        }
        if let Some(e)=&review.evidence{if !material.text.contains(&e.quote){return Err(FilterError::new("invalidJudgment",""));}}
        let stage=match review.decision.as_str(){"relevant"=>"accepted","insufficient"=>"insufficient",_=>"rejected_full"};
        let id=material_key(&scan,item);
        tx.execute("UPDATE ai_topic_item SET stage=?3,title_done=1,preview_done=1,parts_done=?4,parts_total=?4,material_id=?5,result_json=?6,cache_hits=cache_hits+?7,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,review.entry_id,stage,total as i64,id,encode(review),cached as i64]).map_err(err)?;
        put_cache(&tx,&stage_key(&scan,item,"final"),"final",FinalCache{review:review.clone(),material_id:id,parts:total})?;
    }fence.commit(tx).map_err(err)
}).await
}

fn progress(c: &Connection, scan: &Scan) -> Result<Progress, FilterError> {
    let mut p = Progress {
        scan_id: scan.id.clone(),
        started_at: scan.anchor.clone(),
        status: scan.status.clone(),
        snapshot_revision: scan.revision,
        ..Default::default()
    };
    let mut st=c.prepare("SELECT i.stage,i.title_done,i.title_model,i.preview_done,i.parts_done,i.parts_total,i.cache_hits,i.error,m.basis FROM ai_topic_item i LEFT JOIN ai_topic_material m ON m.id=i.material_id WHERE i.scan_id=?1").map_err(err)?;
    let rows = st
        .query_map([&scan.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(err)?;
    let mut title_pending = false;
    let mut preview_pending = false;
    let mut full_pending = false;
    for row in rows {
        let (stage, title_done, title_model, preview_done, done, total, hits, error, raw) =
            row.map_err(err)?;
        p.total += 1;
        p.title_screened += title_done as usize;
        p.title_model_reviewed += (title_model == 1) as usize;
        p.title_fallback += (title_model == 2) as usize;
        p.preview_screened += preview_done as usize;
        p.cache_hits += hits as usize;
        p.full_parts_done += done as usize;
        p.full_parts_total += total as usize;
        p.failed += error.is_some() as usize;
        let basis = raw.unwrap_or_default();
        if ["title", "preview", "full", "final"].contains(&stage.as_str()) {
            p.pending += 1;
            if stage != "title" {
                p.further_pending += 1;
            }
        }
        if done == total
            && total > 0
            && !["title", "preview", "full", "final"].contains(&stage.as_str())
        {
            p.full_verified += (basis == "full_body") as usize;
            p.note_verified += (basis == "note_full") as usize;
        }
        match stage.as_str() {
            "title" => title_pending = true,
            "preview" => preview_pending = true,
            "full" => full_pending = true,
            "accepted" => {
                p.matches += 1;
                p.summary_only += ["summary_only", "title_only"].contains(&basis.as_str()) as usize;
            }
            "rejected_title" => {
                p.title_excluded += 1;
                p.excluded += 1;
            }
            "rejected_preview" | "rejected_full" => p.excluded += 1,
            "insufficient" => p.insufficient += 1,
            "changed" => p.changed += 1,
            _ => {}
        }
    }
    p.phase = if title_pending {
        "titles"
    } else if preview_pending {
        "previews"
    } else if full_pending {
        "full"
    } else if p.pending > 0 {
        "finalizing"
    } else {
        "done"
    }
    .into();
    Ok(p)
}
pub async fn status(p: &Pool, scan: &Scan) -> Result<Progress, FilterError> {
    let scan = scan.clone();
    db(p, move |c| progress(c, &get(c, &scan.id)?)).await
}
pub async fn set_status(
    p: &Pool,
    scan: &Scan,
    status: &str,
    error: Option<&FilterError>,
    failed_ids: &[i64],
) -> Result<Scan, FilterError> {
    let scan = scan.clone();
    let status = status.to_owned();
    let error = error.map(|e| e.code.clone());
    let failed_ids = failed_ids.to_vec();
    let fence = current_commit_fence();
    db(p, move |c| {
        let tx = c.transaction().map_err(err)?;
        owned(&tx, &scan)?;
        tx.execute(
            "UPDATE ai_topic_scan SET status=?3,last_error=?4 WHERE id=?1 AND request_id=?2",
            params![scan.id, scan.request_id, status, error],
        )
        .map_err(err)?;
        for id in failed_ids {
            tx.execute(
                "UPDATE ai_topic_item SET error=?3 WHERE scan_id=?1 AND entry_id=?2",
                params![scan.id, id, error],
            )
            .map_err(err)?;
        }
        fence.commit(tx).map_err(err)?;
        get(c, &scan.id)
    })
    .await
}
pub async fn stop(p: &Pool, request_id: &str, mode: &str) -> Result<Option<Progress>, FilterError> {
    if !["paused", "cancelled"].contains(&mode) {
        return Err(FilterError::new("input", ""));
    }
    let request = request_id.to_owned();
    let mode = mode.to_owned();
    db(p,move|c|{let id:Option<String>=c.query_row("SELECT id FROM ai_topic_scan WHERE request_id=?1 AND status='running' ORDER BY rowid DESC LIMIT 1",[&request],|r|r.get(0)).optional().map_err(err)?;let Some(id)=id else{return Ok(None);};c.execute("UPDATE ai_topic_scan SET status=?3 WHERE id=?1 AND request_id=?2 AND status='running'",params![id,request,mode]).map_err(err)?;Ok(Some(progress(c,&get(c,&id)?)?))}).await
}

pub async fn publish(p: &Pool, scan: &Scan) -> Result<old::Session, FilterError> {
    let scan = scan.clone();
    let fence = current_commit_fence();
    db(p,move|c|{
    let current=get(c,&scan.id)?;if current.request_id!=scan.request_id{return Err(FilterError::new("cancelled",""));}
    let tx=c.transaction().map_err(err)?;let p=progress(&tx,&current)?;let scope=format!("scan:{}",uuid::Uuid::new_v4());
    let preliminary=keywords::pending(&tx,&current)?;
    tx.execute("INSERT INTO ai_search_session(id,input_key,sentence,manual_json,plan_json,filter_json,anchor,revision,flags,checked,published) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?10,?9,1)",params![scope,scan.input_key,scan.sentence,encode(&scan.manual),encode(&scan.plan),encode(&scan.filter),scan.anchor,scan.revision,(p.total-p.pending)as i64,preliminary.len() as i64]).map_err(err)?;
    tx.execute("INSERT INTO ai_topic_scope(scope_id,scan_id,progress_json) VALUES(?1,?2,?3)",params![scope,scan.id,encode(&p)]).map_err(err)?;
    let rows=tx.prepare("SELECT i.entry_id,i.versions_json,i.result_json,i.parts_total,m.basis FROM ai_topic_item i JOIN ai_topic_material m ON m.id=i.material_id WHERE i.scan_id=?1 AND i.stage='accepted'").map_err(err)?.query_map([&scan.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?))).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    for(id,versions,review,parts,basis)in rows {let r:FinalReview=decode(&review)?;if r.entry_id!=id||r.score<60{return Err(FilterError::new("invalidJudgment",""));}let e=r.evidence.as_ref().ok_or_else(||FilterError::new("invalidJudgment",""))?;
        tx.execute("INSERT INTO ai_search_judgment(session_id,entry_id,score,reason,evidence_key,evidence_quote,limited) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![scope,id,r.score,r.reason,e.key,e.quote,basis!="full_body"&&basis!="note_full"]).map_err(err)?;
        tx.execute("INSERT INTO ai_topic_scope_member(scope_id,entry_id,versions_json,basis,parts) VALUES(?1,?2,?3,?4,?5)",params![scope,id,versions,basis,parts]).map_err(err)?;
    }
    for m in preliminary {
        tx.execute("INSERT INTO ai_search_judgment(session_id,entry_id,score,reason,evidence_key,evidence_quote,limited) VALUES(?1,?2,?3,?4,'keyword',?5,1)",params![scope,m.entry_id,m.score,m.terms.into_iter().take(8).collect::<Vec<_>>().join(" / "),m.quote]).map_err(err)?;
        tx.execute("INSERT INTO ai_topic_scope_member(scope_id,entry_id,versions_json,basis,parts) VALUES(?1,?2,?3,?4,0)",params![scope,m.entry_id,m.versions,m.basis]).map_err(err)?;
    }
    fence.commit(tx).map_err(err)?;session(c,&scope)
}).await
}
pub async fn visible_key(p: &Pool, scan: &Scan) -> Result<String, FilterError> {
    let scan = scan.clone();
    db(p, move |c| {
        owned(c, &scan)?;
        keywords::visible_key(c, &scan)
    })
    .await
}
pub fn session(c: &Connection, id: &str) -> Result<old::Session, FilterError> {
    let row=c.query_row("SELECT s.input_key,s.sentence,s.manual_json,s.plan_json,s.filter_json,s.anchor,s.revision,t.progress_json,s.flags FROM ai_search_session s JOIN ai_topic_scope t ON t.scope_id=s.id WHERE s.id=?1 AND s.published=1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,i64>(8)?))).optional().map_err(err)?.ok_or_else(||FilterError::new("expired",""))?;
    let p: Progress = decode(&row.7)?;
    let filter: EntryFilter = decode(&row.4)?;
    let mut scoped = filter.clone();
    scoped.ai_search_id = Some(id.into());
    let stale = validate_scope(c, &scoped, false).map_err(err)?;
    Ok(old::Session {
        id: id.into(),
        input_key: row.0,
        sentence: row.1,
        manual: decode(&row.2)?,
        plan: decode(&row.3)?,
        filter,
        anchor: row.5,
        revision: row.6,
        flags: 0,
        checked: p.total - p.pending,
        total: p.total,
        pending: p.pending,
        matches: p.matches + row.8 as usize,
        preliminary: row.8 as usize,
        stale,
        lane_turn: 0,
        scan: Some(p),
    })
}
pub fn validate_scope(c: &Connection, filter: &EntryFilter, strict: bool) -> Result<bool, String> {
    let id = filter.ai_search_id.as_ref().ok_or("AI_SEARCH_EXPIRED")?;
    let saved:Option<String>=c.query_row("SELECT s.filter_json FROM ai_search_session s JOIN ai_topic_scope t ON t.scope_id=s.id WHERE s.id=?1 AND s.published=1",[id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let saved: EntryFilter = serde_json::from_str(&saved.ok_or("AI_SEARCH_EXPIRED")?)
        .map_err(|_| "AI_SEARCH_EXPIRED")?;
    if serde_json::to_value(conditions(&saved)).unwrap()
        != serde_json::to_value(conditions(filter)).unwrap()
    {
        return Err("AI_SEARCH_SCOPE_CHANGED".into());
    }
    let rows=c.prepare("SELECT m.entry_id,m.versions_json,m.basis FROM ai_topic_scope_member m WHERE m.scope_id=?1").map_err(|e|e.to_string())?.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
    let mut stale = false;
    for (entry, raw, basis) in rows {
        let captured: Versions = serde_json::from_str(&raw).map_err(|_| "AI_SEARCH_EXPIRED")?;
        let field = match basis.as_str() {
            "title_only" | "keyword_title" => search_store::SearchField::Title,
            "summary_only" | "keyword_summary" => search_store::SearchField::Summary,
            "note_full" | "keyword_note" => search_store::SearchField::Note,
            _ => search_store::SearchField::Body,
        };
        if !versions(c, entry)
            .map_err(|_| "AI_SEARCH_EXPIRED")?
            .is_some_and(|current| same_versions(&captured, &current, field))
        {
            stale = true;
            break;
        }
    }
    if stale && strict {
        return Err("AI_SEARCH_STALE".into());
    }
    Ok(stale)
}
pub fn hit_basis(c: &Connection, scope: &str, id: i64) -> Result<(String, usize), String> {
    c.query_row(
        "SELECT basis,parts FROM ai_topic_scope_member WHERE scope_id=?1 AND entry_id=?2",
        params![scope, id],
        |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as usize)),
    )
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ResumeState {
    pub sentence: String,
    pub manual: EntryFilter,
    pub progress: Progress,
    pub session: Option<old::Session>,
}
pub async fn recent(p: &Pool) -> Result<Option<ResumeState>, FilterError> {
    db(p,|c|{
    let id:Option<String>=c.query_row("SELECT id FROM ai_topic_scan WHERE algorithm=?1 AND status!='complete' ORDER BY rowid DESC LIMIT 1",[staged_search::VERSION],|r|r.get(0)).optional().map_err(err)?;
    let Some(id)=id else{return Ok(None);};let scan=get(c,&id)?;
    let scope:Option<String>=c.query_row("SELECT t.scope_id FROM ai_topic_scope t JOIN ai_search_session s ON s.id=t.scope_id WHERE t.scan_id=?1 ORDER BY s.rowid DESC LIMIT 1",[&id],|r|r.get(0)).optional().map_err(err)?;
    Ok(Some(ResumeState{sentence:scan.sentence.clone(),manual:scan.manual.clone(),progress:progress(c,&scan)?,session:scope.map(|id|session(c,&id)).transpose()?}))
}).await
}

pub async fn title_results(
    p: &Pool,
    scan: &Scan,
    input: &[Item],
    reviews: &[TitleReview],
    cached: bool,
) -> Result<(), FilterError> {
    let scan = scan.clone();
    let input = input.to_vec();
    let reviews = reviews.to_vec();
    let fence = current_commit_fence();
    db(p,move|c|{
    let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;
    for r in &reviews {let item=input.iter().find(|i|i.title.entry_id==r.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;
        let exclude=r.decision=="unrelated"&&!item.generic&&!item.clue&&scan.filter.search_field!=search_store::SearchField::Body&&scan.filter.search_field!=search_store::SearchField::Note&& !scan.filter.exclude_title;
        let stage=if exclude{"rejected_title"}else{"preview"};
        tx.execute("UPDATE ai_topic_item SET title_done=1,title_model=?4,stage=?3,cache_hits=cache_hits+?5,result_json=?6,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,r.entry_id,stage,!item.title.title.trim().is_empty()&&item.title.title.chars().count()<=1200,cached as i64,encode(r)]).map_err(err)?;
        put_cache(&tx,&stage_key(&scan,item,"title"),"title",r)?;
    }fence.commit(tx).map_err(err)
}).await
}
/// Title screening is only an optimization. An unusable response cannot reject
/// any source or prevent independent content verification of the frozen scope.
/// title_model=2 records this local deferral separately from valid AI screening.
pub async fn defer_titles(p: &Pool, scan: &Scan, input: &[Item]) -> Result<(), FilterError> {
    let scan = scan.clone();
    let input = input.to_vec();
    let fence = current_commit_fence();
    log::warn!(
        "[topic_search] title_fallback items={} action=retain_for_content",
        input.len()
    );
    db(p, move |c| {
        let tx = c.transaction().map_err(err)?;
        owned(&tx, &scan)?;
        for item in input {
            tx.execute("UPDATE ai_topic_item SET title_done=1,title_model=2,stage='preview',result_json=NULL,error=NULL WHERE scan_id=?1 AND entry_id=?2 AND stage='title'", params![scan.id,item.title.entry_id]).map_err(err)?;
            // Never turn unusable model output into a reusable title judgment.
            tx.execute("DELETE FROM ai_topic_cache WHERE cache_key=?1", [stage_key(&scan,&item,"title")]).map_err(err)?;
        }
        fence.commit(tx).map_err(err)
    }).await
}
pub async fn preview_results(
    p: &Pool,
    scan: &Scan,
    input: &[Item],
    previews: &[Preview],
    reviews: &[PreviewReview],
    cached: bool,
) -> Result<(), FilterError> {
    let scan = scan.clone();
    let input = input.to_vec();
    let previews = previews.to_vec();
    let reviews = reviews.to_vec();
    let fence = current_commit_fence();
    db(p,move|c|{
    let tx=c.transaction().map_err(err)?;owned(&tx,&scan)?;
    for r in &reviews {let item=input.iter().find(|i|i.title.entry_id==r.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;let preview=previews.iter().find(|p|p.entry_id==r.entry_id).ok_or_else(||FilterError::new("invalidJudgment",""))?;
        // A vague title or strong later-body clue cannot be dismissed by an
        // incomplete opening. Full review is the rescue path, not title score.
        let exclude=r.decision=="unrelated"&&(preview.complete||(!item.generic&&!item.strong_clue));
        tx.execute("UPDATE ai_topic_item SET preview_done=1,stage=?3,cache_hits=cache_hits+?4,result_json=?5,parts_total=?6,error=NULL WHERE scan_id=?1 AND entry_id=?2",params![scan.id,r.entry_id,if exclude{"rejected_preview"}else{"full"},cached as i64,encode(r),if exclude{0}else{preview.material_chars.div_ceil(staged_search::PART_CHARS)}as i64]).map_err(err)?;
        put_cache(&tx,&stage_key(&scan,item,"preview"),"preview",r)?;
    }fence.commit(tx).map_err(err)
}).await
}
