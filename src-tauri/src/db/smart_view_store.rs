use super::{entry_store::EntryFilter, search_store};
use deadpool_sqlite::Pool;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartView {
    pub id: i64,
    pub name: String,
    pub filter: EntryFilter,
    pub updated_at: String,
}

pub async fn list(pool: &Pool) -> Result<Vec<SmartView>, String> {
    pool.get().await.map_err(|e| e.to_string())?.interact(|conn| {
        let mut stmt=conn.prepare("SELECT id,name,filter_json,updated_at FROM smart_view ORDER BY name COLLATE NOCASE,id").map_err(|e|e.to_string())?;
        let rows=stmt.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|e|e.to_string())?;
        rows.map(|r| {
            let (id,name,json,updated_at)=r.map_err(|e|e.to_string())?;
            let filter=serde_json::from_str(&json).map_err(|e|format!("Invalid smart view {id}: {e}"))?;
            Ok(SmartView{id,name,filter,updated_at})
        }).collect()
    }).await.map_err(|e|e.to_string())?
}

pub async fn save(
    pool: &Pool,
    id: Option<i64>,
    name: String,
    filter: EntryFilter,
) -> Result<i64, String> {
    search_store::validate(&filter)?;
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("View name must contain 1–80 characters".into());
    }
    let json = serde_json::to_string(&filter).map_err(|e| e.to_string())?;
    pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
        if let Some(id)=id {
            let count=conn.execute("UPDATE smart_view SET name=?1,filter_json=?2,updated_at=datetime('now') WHERE id=?3",params![name,json,id]).map_err(|e|e.to_string())?;
            if count==0 {return Err("View no longer exists".into());}
            Ok(id)
        } else {
            conn.execute("INSERT INTO smart_view(name,filter_json) VALUES(?1,?2)",params![name,json]).map_err(|e|e.to_string())?;
            Ok(conn.last_insert_rowid())
        }
    }).await.map_err(|e|e.to_string())?
}

pub async fn delete(pool: &Pool, id: i64) -> Result<(), String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute("DELETE FROM smart_view WHERE id=?1", [id])
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}
