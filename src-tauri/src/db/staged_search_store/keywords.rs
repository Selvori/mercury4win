//! Fast OR matching inside the already-frozen latest-100 population.
use super::*;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct KeywordMatch {
    pub entry_id: i64,
    pub versions: String,
    pub score: i64,
    pub terms: Vec<String>,
    pub quote: String,
    pub basis: String,
}

fn key(scan: &Scan) -> String {
    hash(json!(["keyword-preview", scan.id]))
}

fn contains(text: &str, term: &str) -> bool {
    if term.len() <= 3 && term.chars().all(|c| c.is_ascii_alphanumeric()) {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == term)
    } else {
        text.contains(term)
    }
}

pub(super) fn seed(c: &Connection, scan: &Scan) -> Result<(), FilterError> {
    let terms = scan
        .plan
        .routes
        .iter()
        .flat_map(|r| &r.groups)
        .flatten()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect::<BTreeSet<_>>();
    let (fields, basis): (&[(usize, i64)], &str) = match scan.filter.search_field {
        search_store::SearchField::Title => (&[(0, 12)], "keyword_title"),
        search_store::SearchField::Summary => (&[(1, 4)], "keyword_summary"),
        search_store::SearchField::Body => (&[(2, 2)], "keyword_body"),
        search_store::SearchField::Note => (&[(3, 8)], "keyword_note"),
        _ => (&[(0, 12), (1, 4), (2, 2)], "keyword_body"),
    };
    let mut statement=c.prepare("SELECT i.entry_id,i.versions_json,COALESCE(s.title,''),COALESCE(s.summary,''),COALESCE(s.body,''),CASE WHEN ?2 THEN COALESCE(s.note,'') ELSE '' END FROM ai_topic_item i LEFT JOIN entry_search s ON s.rowid=i.entry_id WHERE i.scan_id=?1").map_err(err)?;
    let rows = statement
        .query_map(
            params![
                scan.id,
                scan.filter.search_field == search_store::SearchField::Note
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    [
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                    ],
                ))
            },
        )
        .map_err(err)?;
    let mut matches = vec![];
    for row in rows {
        let (entry_id, versions, texts) = row.map_err(err)?;
        let mut score = 0;
        let mut matched = vec![];
        let mut snippet = String::new();
        for term in &terms {
            let hit = fields
                .iter()
                .filter(|(i, _)| !(*i == 0 && scan.filter.exclude_title))
                .find(|(i, _)| contains(&texts[*i], term));
            if let Some((index, weight)) = hit {
                score += weight;
                matched.push(term.clone());
                if snippet.is_empty() {
                    let text = &texts[*index];
                    let at = text.find(term).unwrap_or(0);
                    let before = text[..at].chars().count().saturating_sub(70);
                    snippet = text.chars().skip(before).take(240).collect();
                }
            }
        }
        if score > 0 {
            matches.push(KeywordMatch {
                entry_id,
                versions,
                score,
                terms: matched,
                quote: snippet,
                basis: basis.into(),
            });
        }
    }
    put_cache(c, &key(scan), "keyword-preview", matches)
}

pub(super) fn pending(c: &Connection, scan: &Scan) -> Result<Vec<KeywordMatch>, FilterError> {
    let raw: Option<String> = c
        .query_row(
            "SELECT result_json FROM ai_topic_cache WHERE cache_key=?1",
            [key(scan)],
            |r| r.get(0),
        )
        .optional()
        .map_err(err)?;
    let matches: Vec<KeywordMatch> = raw.map(|raw| decode(&raw)).transpose()?.unwrap_or_default();
    let stages = c
        .prepare("SELECT entry_id,stage FROM ai_topic_item WHERE scan_id=?1")
        .map_err(err)?
        .query_map([&scan.id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(err)?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(err)?;
    Ok(matches
        .into_iter()
        .filter(|m| {
            stages
                .get(&m.entry_id)
                .is_some_and(|s| ["title", "preview", "full", "final"].contains(&s.as_str()))
        })
        .collect())
}

pub(super) fn visible_key(c: &Connection, scan: &Scan) -> Result<String, FilterError> {
    let confirmed=c.prepare("SELECT entry_id,result_json FROM ai_topic_item WHERE scan_id=?1 AND stage='accepted' ORDER BY entry_id").map_err(err)?.query_map([&scan.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    Ok(hash(json!([
        confirmed,
        pending(c, scan)?
            .iter()
            .map(|m| m.entry_id)
            .collect::<Vec<_>>()
    ])))
}
