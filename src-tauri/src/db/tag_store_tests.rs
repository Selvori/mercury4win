use super::*;
fn db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test'); INSERT INTO entry(id,feed_id,guid,title) VALUES(1,1,'1','Intel CPUs'),(2,1,'2','Intel SQLite'),(3,1,'3','Deleted')").unwrap();
    conn
}
#[test]
fn normalization_alias_and_manual_promotion() {
    assert_eq!(normalize(" AI-._generated\t \n CPU "), "ai generated cpu");
    assert_eq!(normalize("U.S.A."), "u s a");
    assert_eq!(normalize("__ \t"), "");
    let conn = db();
    let (tag, _) = assignment(&conn, 1, "Machine-Learning", "ai_batch").unwrap();
    assert!(tag.is_provisional);
    attach_alias(&conn, tag.id, "ML").unwrap();
    let (same, inserted) = assignment(&conn, 1, " ml ", "manual").unwrap();
    assert_eq!(same.id, tag.id);
    assert_eq!(inserted, 0);
    assert!(!same.is_provisional);
    assert_eq!(same.name, "Machine-Learning");
    assert!(attach_alias(&conn, tag.id, "machine_learning").is_err());
    assert!(assignment(&conn, 1, "._ ", "manual").is_err());
}
#[test]
fn merge_deduplicates_manual_confidence_aliases_and_counts() {
    let conn = db();
    let (a, _) = assignment(&conn, 1, "ML", "manual").unwrap();
    assignment(&conn, 2, "ML", "ai_batch").unwrap();
    let (b, _) = assignment(&conn, 1, "Learning", "ai_batch").unwrap();
    attach_alias(&conn, a.id, "Machine learning").unwrap();
    conn.execute("UPDATE entry_tag SET confidence=0.8 WHERE entry_id=2", [])
        .unwrap();
    conn.execute("UPDATE entry SET is_deleted=1 WHERE id=2", [])
        .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    merge(&tx, a.id, b.id).unwrap();
    tx.commit().unwrap();
    assert!(get(&conn, a.id).is_err());
    assert_eq!(get(&conn, b.id).unwrap().usage_count, 1);
    assert_eq!(resolve(&conn, "ml").unwrap(), Some(b.id));
    assert_eq!(resolve(&conn, "machine learning").unwrap(), Some(b.id));
    assert_eq!(
        conn.query_row(
            "SELECT source FROM entry_tag WHERE entry_id=1 AND tag_id=?1",
            [b.id],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "manual"
    );
    assert_eq!(
        conn.query_row(
            "SELECT confidence FROM entry_tag WHERE entry_id=2 AND tag_id=?1",
            [b.id],
            |r| r.get::<_, f64>(0)
        )
        .unwrap(),
        0.8
    );
}
#[test]
fn merge_failure_rolls_back_every_table() {
    let conn = db();
    let (a, _) = assignment(&conn, 1, "A", "manual").unwrap();
    let (b, _) = assignment(&conn, 2, "B", "manual").unwrap();
    let (c, _) = assignment(&conn, 3, "C", "manual").unwrap();
    // Deliberately malformed legacy namespace: merge must fail, never steal C's alias.
    conn.execute(
        "INSERT INTO tag_alias(tag_id,alias_normalized,alias) VALUES(?1,'a','A')",
        [c.id],
    )
    .unwrap();
    {
        let tx = conn.unchecked_transaction().unwrap();
        assert!(merge(&tx, a.id, b.id).is_err());
    }
    assert_eq!(get(&conn, a.id).unwrap().usage_count, 1);
    assert_eq!(get(&conn, b.id).unwrap().usage_count, 1);
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM entry_tag", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert!(merge(&conn, a.id, a.id).is_err());
    assert!(merge(&conn, a.id, 999).is_err());
}
#[test]
fn v4_upgrade_preserves_canonical_collisions_and_legacy_aliases() {
    let conn = crate::db::migrations::historical_fixture(3);
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('tag_alias') WHERE name='alias'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM pragma_table_info('agent_provider_profile') WHERE name='archived_at'", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test'); INSERT INTO entry(id,feed_id,guid,title) VALUES(1,1,'1','Intel CPUs'),(2,1,'2','Intel SQLite'),(3,1,'3','Deleted'); INSERT INTO tag(id,name,normalized_name,is_provisional) VALUES(10,'Machine-Learning','machine-learning',0),(11,'machine learning','machine learning',1),(12,'LLM','llm',0); INSERT INTO tag_alias(tag_id,alias_normalized) VALUES(11,'ML'),(12,'Large_language.models'); INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,10),(1,11),(2,11),(3,12)").unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        crate::db::migrations::SCHEMA_VERSION.to_string()
    );
    crate::db::migrations::run_migrations(&conn).unwrap();
    assert_eq!(resolve(&conn, "machine learning").unwrap(), Some(10));
    assert_eq!(resolve(&conn, "ml").unwrap(), Some(10));
    assert_eq!(get(&conn, 10).unwrap().usage_count, 2);
    assert!(!get(&conn, 10).unwrap().is_provisional);
    assert_eq!(resolve(&conn, "large language models").unwrap(), Some(12));
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}
#[tokio::test]
async fn rename_alias_conflicts_maintenance_and_related() {
    let pool = crate::agent::tag_batch::tests::fixture().await;
    let a = add_tag(&pool, 1, "Intel").await.unwrap();
    let b = add_tag(&pool, 2, "Rust").await.unwrap();
    mutate(&pool, "alias", a.id, "Chip maker").await.unwrap();
    assert_eq!(
        list_tags(&pool, Some("chip_maker")).await.unwrap()[0].id,
        a.id
    );
    assert!(rename_tag(&pool, b.id, "CHIP-MAKER").await.is_err());
    rename_tag(&pool, a.id, "Intel CPUs").await.unwrap();
    assert_eq!(add_tag(&pool, 2, "Intel").await.unwrap().id, a.id);
    assert_eq!(
        crate::commands::tag_cmd::related(&pool, 1).await.unwrap()[0].id,
        2
    );
    remove_tag(&pool, 2, b.id).await.unwrap();
    assert_eq!(mutate(&pool, "cleanup", 0, "").await.unwrap(), 1);
    let local = crate::commands::tag_cmd::local_suggestions(&pool, 3)
        .await
        .unwrap();
    assert!(local.iter().any(|n| n == "Intel CPUs"));
}
