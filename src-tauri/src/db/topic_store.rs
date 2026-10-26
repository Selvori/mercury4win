//! Persistent research topics. Successful briefings own immutable source snapshots.
use super::{entry_store::EntryFilter, search_store};
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MIGRATION_V7: &str = "
CREATE TABLE topic(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', archived INTEGER NOT NULL DEFAULT 0, rule_json TEXT, rule_name TEXT, notes TEXT NOT NULL DEFAULT '', revision INTEGER NOT NULL DEFAULT 1, updated_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE TABLE topic_member(topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE, entry_id INTEGER NOT NULL, group_name TEXT NOT NULL DEFAULT '', PRIMARY KEY(topic_id,entry_id));
CREATE TABLE topic_run(request_id TEXT PRIMARY KEY, topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE, status TEXT NOT NULL, input_json TEXT NOT NULL, configuration_json TEXT NOT NULL, error TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), finished_at TEXT);
CREATE TABLE topic_brief(id INTEGER PRIMARY KEY AUTOINCREMENT, topic_id INTEGER NOT NULL REFERENCES topic(id) ON DELETE CASCADE, request_id TEXT NOT NULL UNIQUE, baseline_id INTEGER, input_hash TEXT NOT NULL, input_json TEXT NOT NULL, result_json TEXT NOT NULL, model_json TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('now')));
CREATE INDEX topic_brief_history ON topic_brief(topic_id,id);
";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub archived: bool,
    pub rule: Option<EntryFilter>,
    pub rule_name: Option<String>,
    pub notes: String,
    pub revision: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub entry_id: i64,
    pub title: String,
    pub url: Option<String>,
    pub feed: String,
    pub body: String,
    pub content_version: String,
    pub group_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub topic_id: i64,
    pub name: String,
    pub description: String,
    pub revision: i64,
    #[serde(default)]
    pub rule: Option<EntryFilter>,
    #[serde(default)]
    pub rule_name: Option<String>,
    pub sources: Vec<Source>,
    pub baseline_id: Option<i64>,
    pub hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Citation {
    pub entry_id: i64,
    pub quote: String,
    pub content_version: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claim {
    pub kind: String,
    pub text: String,
    pub citations: Vec<Citation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BriefResult {
    pub claims: Vec<Claim>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Brief {
    pub id: i64,
    pub baseline_id: Option<i64>,
    pub input: Input,
    pub result: BriefResult,
    pub model: serde_json::Value,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub request_id: String,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}
#[derive(Serialize)]
pub struct Detail {
    pub topic: Topic,
    pub input: Input,
    pub history: Vec<Brief>,
    pub runs: Vec<Run>,
    pub changed: bool,
}

pub async fn db<T: Send + 'static>(
    pool: &Pool,
    work: impl FnOnce(&mut Connection) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(work)
        .await
        .map_err(|e| e.to_string())?
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn topic_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Topic> {
    let json: Option<String> = r.get(4)?;
    let rule = json
        .map(|s| {
            serde_json::from_str(&s).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .transpose()?;
    Ok(Topic {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        archived: r.get(3)?,
        rule,
        rule_name: r.get(5)?,
        notes: r.get(6)?,
        revision: r.get(7)?,
    })
}
const TOPIC_SELECT: &str =
    "SELECT id,name,description,archived,rule_json,rule_name,notes,revision FROM topic";
pub fn get(conn: &Connection, id: i64) -> Result<Topic, String> {
    conn.query_row(&format!("{TOPIC_SELECT} WHERE id=?1"), [id], topic_row)
        .map_err(err)
}
pub async fn list(pool: &Pool) -> Result<Vec<Topic>, String> {
    db(pool, |c| {
        let mut s = c
            .prepare(&format!("{TOPIC_SELECT} ORDER BY archived,name,id"))
            .map_err(err)?;
        let rows = s.query_map([], topic_row).map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    })
    .await
}
pub async fn save(
    pool: &Pool,
    id: Option<i64>,
    name: String,
    description: String,
    archived: bool,
    rule: Option<EntryFilter>,
    rule_name: Option<String>,
) -> Result<i64, String> {
    if name.trim().is_empty() || name.chars().count() > 80 || description.chars().count() > 4000 {
        return Err("Invalid topic name or description".into());
    }
    if let Some(f) = &rule {
        search_store::validate(f)?;
    }
    let json = rule.map(|f| serde_json::to_string(&f).unwrap());
    db(pool,move |c| {
        if let Some(id)=id {let n=c.execute("UPDATE topic SET name=?1,description=?2,archived=?3,rule_json=?4,rule_name=?5,revision=revision+1,updated_at=datetime('now') WHERE id=?6",params![name.trim(),description,archived,json,rule_name,id]).map_err(err)?; if n==0 {return Err("Topic no longer exists".into());} Ok(id)}
        else {c.execute("INSERT INTO topic(name,description,archived,rule_json,rule_name) VALUES(?1,?2,?3,?4,?5)",params![name.trim(),description,archived,json,rule_name]).map_err(err)?; Ok(c.last_insert_rowid())}
    }).await
}
pub async fn delete(pool: &Pool, id: i64) -> Result<(), String> {
    db(pool, move |c| {
        let tx = c.transaction().map_err(err)?;
        tx.execute("DELETE FROM topic_member WHERE topic_id=?1", [id])
            .map_err(err)?;
        tx.execute("DELETE FROM topic_run WHERE topic_id=?1", [id])
            .map_err(err)?;
        tx.execute("DELETE FROM topic_brief WHERE topic_id=?1", [id])
            .map_err(err)?;
        tx.execute("DELETE FROM topic WHERE id=?1", [id])
            .map_err(err)?;
        tx.commit().map_err(err)
    })
    .await
}
pub async fn notes(pool: &Pool, id: i64, notes: String) -> Result<(), String> {
    if notes.len() > 200_000 {
        return Err("Notes too long".into());
    }
    db(pool, move |c| {
        c.execute("UPDATE topic SET notes=?1 WHERE id=?2", params![notes, id])
            .map(|_| ())
            .map_err(err)
    })
    .await
}
pub async fn members(
    pool: &Pool,
    id: i64,
    ids: Vec<i64>,
    remove: bool,
    group_name: Option<String>,
) -> Result<(), String> {
    if ids.len() > 500 || group_name.as_ref().is_some_and(|g| g.chars().count() > 100) {
        return Err("Too many members or group label too long".into());
    }
    db(pool,move|c| {let tx=c.transaction().map_err(err)?;get(&tx,id)?;
        for entry in ids {if remove {tx.execute("DELETE FROM topic_member WHERE topic_id=?1 AND entry_id=?2",params![id,entry]).map_err(err)?;}
        else {let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM entry WHERE id=?1 AND is_deleted=0)",[entry],|r|r.get(0)).map_err(err)?;if !exists {return Err("Article no longer exists".into());}
            tx.execute("INSERT INTO topic_member(topic_id,entry_id,group_name) VALUES(?1,?2,?3) ON CONFLICT(topic_id,entry_id) DO UPDATE SET group_name=COALESCE(?4,group_name)",params![id,entry,group_name.clone().unwrap_or_default(),group_name]).map_err(err)?;}}
        tx.execute("UPDATE topic SET revision=revision+1 WHERE id=?1",[id]).map_err(err)?;tx.commit().map_err(err)
    }).await
}
fn source(conn: &Connection, id: i64, group: String) -> Result<Source, String> {
    conn.query_row("SELECT e.title,e.url,COALESCE(f.title,f.feed_url),c.markdown,c.cleaned_html,c.html,e.feed_content,e.feed_content_type,e.is_deleted FROM entry e JOIN feed f ON f.id=e.feed_id LEFT JOIN content c ON c.entry_id=e.id WHERE e.id=?1",[id],|r| {
        let title:Option<String>=r.get(0)?; let markdown:Option<String>=r.get(3)?;
        let html:Option<String>=r.get(4)?;
        let feed:Option<String>=r.get(6)?;let kind:Option<String>=r.get(7)?;let deleted:bool=r.get(8)?;
        let body=if deleted {String::new()} else if let Some(md)=markdown.filter(|m|!m.trim().is_empty()) {search_store::markdown_text(&md)} else if let Some(html)=html {search_store::html_text(&html)} else if kind.as_deref()==Some("text/plain") {feed.unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ")} else if feed.as_ref().is_some_and(|s|!s.trim().is_empty()) {search_store::html_text(&feed.unwrap_or_default())} else {String::new()};
        let url:Option<String>=r.get(1)?;let title=title.unwrap_or_default();let feed:String=r.get(2)?;
        let version=hash(&serde_json::json!([title,url,feed,body]).to_string());
        Ok(Source{entry_id:id,title,url,feed,body,content_version:version,group_name:group})
    }).optional().map_err(err)?.ok_or_else(||"Source article was removed".into())
}
pub fn input(conn: &Connection, id: i64) -> Result<Input, String> {
    let topic = get(conn, id)?;
    let mut stmt = conn
        .prepare("SELECT entry_id,group_name FROM topic_member WHERE topic_id=?1 ORDER BY entry_id")
        .map_err(err)?;
    let members = stmt
        .query_map([id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    let mut sources = Vec::new();
    for (entry, group) in members {
        let mut s = source(conn, entry, group.clone()).unwrap_or_else(|_| Source {
            entry_id: entry,
            title: format!("#{entry}"),
            url: None,
            feed: String::new(),
            body: String::new(),
            content_version: "unavailable".into(),
            group_name: group,
        });
        if s.group_name.is_empty() {
            s.group_name = format!("auto:{}", &hash(&s.body)[..16]);
        }
        sources.push(s);
    }
    let baseline_id = conn
        .query_row(
            "SELECT MAX(id) FROM topic_brief WHERE topic_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(err)?;
    let fingerprint =
        hash(&serde_json::json!([topic.name, topic.description, sources]).to_string());
    Ok(Input {
        topic_id: id,
        name: topic.name,
        description: topic.description,
        revision: topic.revision,
        rule: topic.rule,
        rule_name: topic.rule_name,
        sources,
        baseline_id,
        hash: fingerprint,
    })
}
pub fn history(conn: &Connection, id: i64) -> Result<Vec<Brief>, String> {
    let mut s=conn.prepare("SELECT id,baseline_id,input_json,result_json,model_json,created_at FROM topic_brief WHERE topic_id=?1 ORDER BY id DESC").map_err(err)?;
    let rows = s
        .query_map([id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get(5)?,
            ))
        })
        .map_err(err)?;
    rows.map(|r| {
        let (id, baseline_id, i, result, m, created_at) = r.map_err(err)?;
        Ok(Brief {
            id,
            baseline_id,
            input: serde_json::from_str(&i).map_err(err)?,
            result: serde_json::from_str(&result).map_err(err)?,
            model: serde_json::from_str(&m).map_err(err)?,
            created_at,
        })
    })
    .collect()
}
pub async fn detail(pool: &Pool, id: i64) -> Result<Detail, String> {
    db(pool,move|c|{let tx=c.unchecked_transaction().map_err(err)?;let topic=get(&tx,id)?;let input=input(&tx,id)?;let history=history(&tx,id)?;let changed=history.first().is_none_or(|b|b.input.hash!=input.hash);let mut s=tx.prepare("SELECT request_id,status,error,created_at FROM topic_run WHERE topic_id=?1 ORDER BY created_at DESC,rowid DESC LIMIT 20").map_err(err)?;let runs=s.query_map([id],|r|Ok(Run{request_id:r.get(0)?,status:r.get(1)?,error:r.get(2)?,created_at:r.get(3)?})).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;Ok(Detail{topic,input,history,runs,changed})}).await
}
pub async fn start_run(
    pool: &Pool,
    request: String,
    input: Input,
    config: serde_json::Value,
) -> Result<(), String> {
    db(pool,move|c|{c.execute("INSERT INTO topic_run(request_id,topic_id,status,input_json,configuration_json) VALUES(?1,?2,'running',?3,?4)",params![request,input.topic_id,serde_json::to_string(&input).unwrap(),config.to_string()]).map_err(err)?;Ok(())}).await
}
pub async fn finish_run(
    pool: &Pool,
    request: String,
    status: String,
    error: Option<String>,
) -> Result<(), String> {
    db(pool,move|c| {c.execute("UPDATE topic_run SET status=?1,error=?2,finished_at=datetime('now') WHERE request_id=?3 AND status='running'",params![status,error,request]).map_err(err)?;Ok(())}).await
}
pub async fn commit(
    pool: &Pool,
    request: String,
    frozen: Input,
    result: BriefResult,
    model: serde_json::Value,
) -> Result<i64, String> {
    let fence = crate::agent::runtime::current_commit_fence();
    db(pool,move|c|{let tx=c.transaction().map_err(err)?;let now=input(&tx,frozen.topic_id)?;
        if now.hash!=frozen.hash || now.baseline_id!=frozen.baseline_id || now.revision!=frozen.revision {return Err("Topic or sources changed during generation; retry with the current scope".into());}
        tx.execute("INSERT INTO topic_brief(topic_id,request_id,baseline_id,input_hash,input_json,result_json,model_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![frozen.topic_id,request,frozen.baseline_id,frozen.hash,serde_json::to_string(&frozen).unwrap(),serde_json::to_string(&result).unwrap(),model.to_string()]).map_err(err)?;
        let id=tx.last_insert_rowid();tx.execute("UPDATE topic_run SET status='succeeded',finished_at=datetime('now') WHERE request_id=?1",[request]).map_err(err)?;fence.commit(tx)?;Ok(id)
    }).await
}

#[cfg(test)]
mod tests;
