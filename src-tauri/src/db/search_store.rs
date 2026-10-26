//! Local lexical search. FTS is a disposable derivative, never the source of truth.
use super::entry_store::{EntryFilter, EntryQueryParams, TagMatchMode};
use super::models::{Entry, EntryCursor, EntryPage};
use chrono::{DateTime, NaiveDate, Utc};
use deadpool_sqlite::Pool;
use rusqlite::{params, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SearchField {
    #[default]
    All,
    Title,
    Summary,
    Body,
    Note,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Newest,
    Oldest,
    Relevance,
}

// Changes on every source path (including cascade deletes and reader rebuilds)
// are durable even if the process exits before refreshing the index.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE entry_search USING fts5(title,summary,body,note,tokenize='trigram');
        CREATE TABLE search_dirty(entry_id INTEGER PRIMARY KEY);
        CREATE TABLE smart_view(id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL COLLATE NOCASE UNIQUE, filter_json TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')));
        INSERT INTO search_dirty SELECT id FROM entry;
        CREATE INDEX idx_entry_published ON entry(published_at,id);
        CREATE INDEX idx_entry_tag_tag ON entry_tag(tag_id,entry_id);",
    )?;
    for (table, key, updates) in [
        (
            "entry",
            "id",
            "title,summary,feed_content,feed_content_type,is_deleted",
        ),
        ("content", "entry_id", "html,cleaned_html,markdown"),
        ("entry_note", "entry_id", "markdown"),
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER search_{table}_insert AFTER INSERT ON {table} BEGIN
                INSERT INTO search_dirty SELECT new.{key} WHERE NOT EXISTS(SELECT 1 FROM search_dirty WHERE entry_id=new.{key}); END;
             CREATE TRIGGER search_{table}_update AFTER UPDATE OF {updates} ON {table} BEGIN
                INSERT INTO search_dirty SELECT new.{key} WHERE NOT EXISTS(SELECT 1 FROM search_dirty WHERE entry_id=new.{key}); END;
             CREATE TRIGGER search_{table}_delete AFTER DELETE ON {table} BEGIN
                INSERT INTO search_dirty SELECT old.{key} WHERE NOT EXISTS(SELECT 1 FROM search_dirty WHERE entry_id=old.{key}); END;"
        ))?;
    }
    Ok(())
}

pub(crate) fn html_text(html: &str) -> String {
    fn walk(node: &std::rc::Rc<htmd::Node>, out: &mut String) {
        use markup5ever_rcdom::NodeData;
        match &node.data {
            NodeData::Text { contents } => out.push_str(&contents.borrow()),
            NodeData::Element { name, .. } => {
                if matches!(
                    name.local.as_ref(),
                    "script" | "style" | "head" | "template"
                ) {
                    return;
                }
                let block = matches!(
                    name.local.as_ref(),
                    "p" | "div"
                        | "article"
                        | "section"
                        | "br"
                        | "li"
                        | "pre"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "td"
                        | "tr"
                        | "blockquote"
                );
                if block {
                    out.push(' ');
                }
                for child in node.children.borrow().iter() {
                    walk(child, out);
                }
                if block {
                    out.push(' ');
                }
                return;
            }
            _ => {}
        }
        for child in node.children.borrow().iter() {
            walk(child, out);
        }
    }
    let mut out = String::new();
    if let Ok(tree) = htmd::HtmlToMarkdown::new().html_to_tree(html) {
        walk(&tree, &mut out);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn markdown_text(md: &str) -> String {
    use comrak::nodes::NodeValue;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, md, &comrak::Options::default());
    let mut out = String::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::CodeBlock(c) => {
                out.push(' ');
                out.push_str(&c.literal);
                out.push(' ');
            }
            NodeValue::SoftBreak
            | NodeValue::LineBreak
            | NodeValue::Paragraph
            | NodeValue::Heading(_) => out.push(' '),
            NodeValue::HtmlInline(t) => {
                out.push_str(&html_text(t));
            }
            NodeValue::HtmlBlock(t) => {
                out.push_str(&html_text(&t.literal));
            }
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Original plain text for snippets; folded copies are stored in FTS.
fn document(conn: &Connection, id: i64) -> rusqlite::Result<Option<[String; 4]>> {
    conn.query_row(
        "SELECT e.title,e.summary,c.markdown,c.cleaned_html,c.html,
        e.feed_content,e.feed_content_type,n.markdown FROM entry e
        LEFT JOIN content c ON c.entry_id=e.id LEFT JOIN entry_note n ON n.entry_id=e.id
        WHERE e.id=?1 AND e.is_deleted=0",
        [id],
        |r| {
            let get = |i| r.get::<_, Option<String>>(i).map(|s| s.unwrap_or_default());
            let md = get(2)?;
            let cleaned = get(3)?;
            let html = get(4)?;
            let feed = get(5)?;
            let kind = get(6)?;
            let body = if !md.trim().is_empty() {
                markdown_text(&md)
            } else if !cleaned.trim().is_empty() {
                html_text(&cleaned)
            } else if !html.trim().is_empty() {
                html_text(&html)
            } else if kind == "text" {
                feed
            } else {
                html_text(&feed)
            };
            Ok([get(0)?, html_text(&get(1)?), body, markdown_text(&get(7)?)])
        },
    )
    .optional()
}

/// Bounded transactions keep normal readers/writers responsive during backfill.
/// Refresh and removal are atomic; another search cannot observe a half-written row.
pub fn refresh(conn: &Connection) -> Result<(), String> {
    loop {
        let tx =
            rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| e.to_string())?;
        let changed = refresh_batch(&tx)?;
        tx.commit().map_err(|e| e.to_string())?;
        if !changed {
            return Ok(());
        }
    }
}

/// Caller holds the write transaction: used by scope-wide mutations so source,
/// index and update predicates cannot race with another writer.
pub fn refresh_locked(conn: &Connection) -> Result<(), String> {
    while refresh_batch(conn)? {}
    Ok(())
}

fn refresh_batch(conn: &Connection) -> Result<bool, String> {
    let ids: Vec<i64> = conn
        .prepare("SELECT entry_id FROM search_dirty LIMIT 100")
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    let changed = !ids.is_empty();
    for id in ids {
        let doc = document(conn, id).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM entry_search WHERE rowid=?1", [id])
            .map_err(|e| e.to_string())?;
        if let Some(fields) = doc {
            let f = fields.map(|s| s.to_lowercase());
            conn.execute(
                "INSERT INTO entry_search(rowid,title,summary,body,note) VALUES(?1,?2,?3,?4,?5)",
                params![id, f[0], f[1], f[2], f[3]],
            )
            .map_err(|e| e.to_string())?;
        }
        conn.execute("DELETE FROM search_dirty WHERE entry_id=?1", [id])
            .map_err(|e| e.to_string())?;
    }
    Ok(changed)
}

pub fn terms(filter: &EntryFilter) -> Vec<String> {
    filter
        .search
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_lowercase)
        .collect()
}

pub fn validate(filter: &EntryFilter) -> Result<(), String> {
    if filter
        .search
        .as_ref()
        .is_some_and(|s| s.chars().count() > 1000)
        || terms(filter).len() > 24
    {
        return Err("Search is limited to 1000 characters and 24 terms".into());
    }
    if filter.tag_ids.as_ref().is_some_and(|ids| ids.len() > 100) {
        return Err("Too many tags".into());
    }
    if filter
        .recent_days
        .is_some_and(|d| !(1..=36500).contains(&d))
    {
        return Err("Invalid recent days".into());
    }
    for date in [&filter.date_from, &filter.date_to].into_iter().flatten() {
        if date.len() != 10 || NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err("Invalid date".into());
        }
    }
    if let (Some(from), Some(to)) = (&filter.date_from, &filter.date_to) {
        if from > to {
            return Err("Date range is reversed".into());
        }
    }
    Ok(())
}

pub struct Compiled {
    pub conditions: Vec<String>,
    pub values: Vec<Value>,
    pub score: String,
}
fn bind(values: &mut Vec<Value>, value: impl Into<Value>) -> String {
    values.push(value.into());
    format!("?{}", values.len())
}

pub fn compile(filter: &EntryFilter, anchor: &str) -> Result<Compiled, String> {
    validate(filter)?;
    let mut c = Compiled {
        conditions: vec!["e.is_deleted=0".into()],
        values: vec![],
        score: "0".into(),
    };
    if let Some(id) = filter.feed_id {
        c.conditions
            .push(format!("e.feed_id={}", bind(&mut c.values, id)));
    }
    if let Some(read) = filter.is_read.or(filter.unread_only.then_some(false)) {
        c.conditions
            .push(format!("e.is_read={}", if read { 1 } else { 0 }));
    }
    if filter.starred_only {
        c.conditions.push("e.is_starred=1".into());
    }
    if let Some(has) = filter.has_note {
        c.conditions.push(format!(
            "{} EXISTS(SELECT 1 FROM entry_note n WHERE n.entry_id=e.id AND trim(n.markdown)<>'')",
            if has { "" } else { "NOT" }
        ));
    }
    if let Some(date) = &filter.date_from {
        c.conditions.push(format!(
            "julianday(COALESCE(e.published_at,e.created_at))>=julianday({})",
            bind(&mut c.values, date.clone())
        ));
    }
    if let Some(date) = &filter.date_to {
        c.conditions.push(format!(
            "julianday(COALESCE(e.published_at,e.created_at))<julianday({},'+1 day')",
            bind(&mut c.values, date.clone())
        ));
    }
    if let Some(days) = filter.recent_days {
        let now = bind(&mut c.values, anchor.to_string());
        let delta = bind(&mut c.values, format!("-{days} days"));
        c.conditions.push(format!("julianday(COALESCE(e.published_at,e.created_at)) BETWEEN julianday({now},{delta}) AND julianday({now})"));
    }
    let mut ids = filter.tag_ids.clone().unwrap_or_default();
    ids.sort_unstable();
    ids.dedup();
    if !ids.is_empty() {
        let count = ids.len();
        let placeholders = ids
            .into_iter()
            .map(|id| bind(&mut c.values, id))
            .collect::<Vec<_>>()
            .join(",");
        c.conditions.push(format!("e.id IN(SELECT entry_id FROM entry_tag WHERE tag_id IN({placeholders}) GROUP BY entry_id HAVING COUNT(DISTINCT tag_id)>={})", if matches!(filter.tag_mode,TagMatchMode::All) {count} else {1}));
    }
    let fields: &[(&str, i64)] = match filter.search_field {
        SearchField::All => &[("title", 12), ("summary", 4), ("body", 2), ("note", 8)],
        SearchField::Title => &[("title", 12)],
        SearchField::Summary => &[("summary", 4)],
        SearchField::Body => &[("body", 2)],
        SearchField::Note => &[("note", 8)],
    };
    let mut scores = vec![];
    for term in terms(filter) {
        let needle = bind(&mut c.values, term.clone());
        let literal = fields
            .iter()
            .map(|(field, weight)| {
                scores.push(format!(
                    "CASE WHEN instr(s.{field},{needle})>0 THEN {weight} ELSE 0 END"
                ));
                format!("instr(s.{field},{needle})>0")
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        // Trigram MATCH returns no rows for two-character terms. Literal short
        // matching also treats %, _, quotes and FTS operators as ordinary text.
        if term.chars().count() >= 3 {
            let quoted = term.replace('"', "\"\"");
            let columns = fields.iter().map(|(f, _)| *f).collect::<Vec<_>>().join(" ");
            let query = bind(&mut c.values, format!("{{{columns}}} : \"{quoted}\""));
            c.conditions.push(format!(
                "e.id IN(SELECT rowid FROM entry_search WHERE entry_search MATCH {query})"
            ));
        } else {
            c.conditions.push(format!(
                "EXISTS(SELECT 1 FROM entry_search s WHERE s.rowid=e.id AND ({literal}))"
            ));
        }
        if filter.exclude_title {
            c.conditions.push(format!("NOT EXISTS(SELECT 1 FROM entry_search s WHERE s.rowid=e.id AND instr(s.title,{needle})>0)"));
        }
    }
    if !scores.is_empty() {
        c.score = format!(
            "COALESCE((SELECT {} FROM entry_search s WHERE s.rowid=e.id),0)",
            scores.join("+")
        );
    }
    Ok(c)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPart {
    pub text: String,
    pub matched: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub field: String,
    pub parts: Vec<SearchPart>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInfo {
    pub preview: String,
    pub entry_id: i64,
    pub feed_title: String,
    pub body_available: bool,
    pub hits: Vec<SearchHit>,
}

// Byte boundaries are mapped through Unicode lowercase expansion. No HTML is
// returned: clients render text nodes and <mark>, including hostile source text.
pub fn snippet(text: &str, terms: &[String]) -> Vec<SearchPart> {
    let mut folded = String::new();
    let mut bounds = vec![];
    for (start, ch) in text.char_indices() {
        for lower in ch.to_lowercase() {
            for _ in 0..lower.len_utf8() {
                bounds.push((start, start + ch.len_utf8()));
            }
            folded.push(lower);
        }
    }
    let mut ranges = vec![];
    for term in terms {
        if term.is_empty() {
            continue;
        }
        for (start, _) in folded.match_indices(term) {
            ranges.push((bounds[start].0, bounds[start + term.len() - 1].1));
        }
    }
    if ranges.is_empty() {
        return vec![];
    }
    ranges.sort_unstable();
    let first = ranges[0].0;
    let starts: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect();
    let at = starts.binary_search(&first).unwrap_or(0);
    let start = starts[at.saturating_sub(48)];
    let end = starts[(at.saturating_sub(48) + 200).min(starts.len() - 1)];
    let mut merged: Vec<(usize, usize)> = vec![];
    for (a, b) in ranges {
        if a >= end || b <= start {
            continue;
        }
        let (a, b) = (a.max(start), b.min(end));
        if let Some(last) = merged.last_mut().filter(|last| a <= last.1) {
            last.1 = last.1.max(b);
        } else {
            merged.push((a, b));
        }
    }
    let mut parts = vec![];
    if start > 0 {
        parts.push(SearchPart {
            text: "…".into(),
            matched: false,
        });
    }
    let mut pos = start;
    for (a, b) in merged {
        if a > pos {
            parts.push(SearchPart {
                text: text[pos..a].into(),
                matched: false,
            });
        }
        parts.push(SearchPart {
            text: text[a..b].into(),
            matched: true,
        });
        pos = b;
    }
    if pos < end {
        parts.push(SearchPart {
            text: text[pos..end].into(),
            matched: false,
        });
    }
    if end < text.len() {
        parts.push(SearchPart {
            text: "…".into(),
            matched: false,
        });
    }
    parts
}

pub async fn query_entries(pool: &Pool, params: &EntryQueryParams) -> Result<EntryPage, String> {
    let filter = params.filter.clone();
    let cursor = params.cursor.clone();
    let limit = params.limit.clamp(1, 500);
    validate(&filter)?;
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| query(conn, &filter, cursor.as_ref(), limit))
        .await
        .map_err(|e| e.to_string())?
}

pub fn query(
    conn: &Connection,
    filter: &EntryFilter,
    cursor: Option<&EntryCursor>,
    limit: u32,
) -> Result<EntryPage, String> {
    let words = terms(filter);
    if !words.is_empty() {
        refresh(conn)?;
    }
    let anchor = cursor
        .and_then(|c| c.anchor.clone())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    if DateTime::parse_from_rfc3339(&anchor).is_err() {
        return Err("Invalid cursor anchor".into());
    }
    let mut c = compile(filter, &anchor)?;
    // Count, rows and snippets share one read snapshot.
    let tx = loop {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        // Pin a read snapshot before checking the queue. A source update racing
        // refresh must not be paired with yesterday's index in this response.
        let dirty: bool = tx
            .query_row("SELECT EXISTS(SELECT 1 FROM search_dirty)", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        if words.is_empty() || !dirty {
            break tx;
        }
        drop(tx);
        refresh(conn)?;
    };
    let total: i64 = tx
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM entry e WHERE {}",
                c.conditions.join(" AND ")
            ),
            rusqlite::params_from_iter(c.values.iter()),
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let ranked = filter.sort == SearchSort::Relevance && !words.is_empty();
    let ascending = filter.sort == SearchSort::Oldest;
    if let Some(cursor) = cursor {
        let id = bind(&mut c.values, cursor.id);
        let op = if ascending { ">" } else { "<" };
        let cursor_date: Option<f64> = tx
            .query_row("SELECT julianday(?1)", params![cursor.published_at], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let date = if let Some(date) = cursor_date {
            let date = bind(&mut c.values, date);
            format!("(julianday(e.published_at) {op} {date} OR (julianday(e.published_at)={date} AND e.id {op} {id}) OR julianday(e.published_at) IS NULL)")
        } else {
            format!("(julianday(e.published_at) IS NULL AND e.id {op} {id})")
        };
        if ranked {
            let score = bind(&mut c.values, cursor.score.unwrap_or(0));
            c.conditions.push(format!(
                "({0}<{score} OR ({0}={score} AND {date}))",
                c.score
            ));
        } else {
            c.conditions.push(date);
        }
    }
    let order = if ascending {
        "julianday(e.published_at) IS NULL, julianday(e.published_at) ASC,e.id ASC"
    } else {
        "julianday(e.published_at) DESC,e.id DESC"
    };
    let order = if ranked {
        format!("{} DESC,{order}", c.score)
    } else {
        order.to_string()
    };
    let page_limit = bind(&mut c.values, i64::from(limit.clamp(1, 500)) + 1);
    let sql=format!("SELECT e.id,e.feed_id,e.guid,e.url,e.title,e.author,e.published_at,e.summary,
        e.is_read,e.is_starred,e.is_deleted,e.created_at,e.feed_content,e.feed_content_type,{} AS score
        FROM entry e WHERE {} ORDER BY {order} LIMIT {page_limit}",c.score,c.conditions.join(" AND "));
    let mut rows: Vec<(Entry, i64)> = tx
        .prepare(&sql)
        .map_err(|e| e.to_string())?
        .query_map(rusqlite::params_from_iter(c.values.iter()), |r| {
            Ok((
                Entry {
                    id: r.get(0)?,
                    feed_id: r.get(1)?,
                    guid: r.get(2)?,
                    url: r.get(3)?,
                    title: r.get(4)?,
                    author: r.get(5)?,
                    published_at: r.get(6)?,
                    summary: r.get(7)?,
                    is_read: r.get(8)?,
                    is_starred: r.get(9)?,
                    is_deleted: r.get(10)?,
                    created_at: r.get(11)?,
                    feed_content: r.get(12)?,
                    feed_content_type: r.get(13)?,
                },
                r.get(14)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    let has_more = rows.len() > limit.clamp(1, 500) as usize;
    rows.truncate(limit.clamp(1, 500) as usize);
    let next_cursor = if has_more {
        rows.last().map(|(entry, score)| EntryCursor {
            id: entry.id,
            published_at: entry.published_at.clone(),
            score: Some(*score),
            anchor: Some(anchor),
        })
    } else {
        None
    };
    let mut search_info = vec![];
    for (entry, _) in &rows {
        let fields = if words.is_empty() {
            [
                entry.title.clone().unwrap_or_default(),
                html_text(entry.summary.as_deref().unwrap_or("")),
                String::new(),
                String::new(),
            ]
        } else {
            document(&tx, entry.id)
                .map_err(|e| e.to_string())?
                .unwrap_or_default()
        };
        let hits = ["title", "summary", "body", "note"]
            .into_iter()
            .zip(fields.iter())
            .filter_map(|(field, text)| {
                let parts = snippet(text, &words);
                (!parts.is_empty()).then_some(SearchHit {
                    field: field.into(),
                    parts,
                })
            })
            .collect();
        let feed_title = tx
            .query_row(
                "SELECT COALESCE(title,feed_url) FROM feed WHERE id=?1",
                [entry.feed_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let body_available = if !words.is_empty() {
            !fields[2].is_empty()
        } else {
            entry.feed_content.as_deref().is_some_and(|s|!s.trim().is_empty()) || tx.query_row("SELECT EXISTS(SELECT 1 FROM content WHERE entry_id=?1 AND (length(trim(COALESCE(markdown,'')))>0 OR length(trim(COALESCE(cleaned_html,'')))>0 OR length(trim(COALESCE(html,'')))>0))",[entry.id],|r|r.get(0)).map_err(|e|e.to_string())?
        };
        search_info.push(SearchInfo {
            entry_id: entry.id,
            feed_title,
            body_available,
            hits,
            preview: fields[1].chars().take(240).collect(),
        });
    }
    Ok(EntryPage {
        entries: rows.into_iter().map(|(entry, _)| entry).collect(),
        next_cursor,
        total,
        search_info,
    })
}

pub async fn rebuild(pool: &Pool) -> Result<(), String> {
    pool.get().await.map_err(|e|e.to_string())?.interact(|conn| {
        let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
        // Drop/recreate recovers damaged FTS internals too. Source tables and
        // smart views are untouched. Transaction rollback keeps the old index.
        tx.execute_batch("DROP TABLE IF EXISTS entry_search;
            CREATE VIRTUAL TABLE entry_search USING fts5(title,summary,body,note,tokenize='trigram');
            INSERT OR IGNORE INTO search_dirty SELECT id FROM entry;").map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?;
        refresh(conn)
    }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]
mod tests;
