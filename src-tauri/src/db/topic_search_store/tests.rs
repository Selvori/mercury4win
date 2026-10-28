use super::*;
use crate::agent::topic_search::tests::{manual, plan, pool};

#[tokio::test]
async fn every_recall_lane_obeys_manual_field_and_title_exclusion() {
    let p = pool().await;
    p.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "INSERT INTO entry(id,feed_id,title) VALUES(200,1,'AQUILA title-only subject')",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO content(entry_id,markdown) VALUES(200,'Unrelated local body')",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    let plan:Plan=serde_json::from_value(serde_json::json!({"intent":"aquila","entities":[],"routes":[{"language":"zh","groups":[["aquila"]]},{"language":"en","groups":[["aquila"]]}],"filter":{},"unsupported":[]})).unwrap();
    for (field, exclude) in [
        (search_store::SearchField::Body, false),
        (search_store::SearchField::Summary, false),
        (search_store::SearchField::All, true),
        (search_store::SearchField::Title, true),
    ] {
        let mut filter = manual();
        filter.search_field = field;
        filter.exclude_title = exclude;
        let s = create(
            &p,
            format!("field-{field:?}-{exclude}"),
            "aquila".into(),
            filter.clone(),
            plan.clone(),
            filter,
        )
        .await
        .unwrap();
        assert_eq!(s.total, 0);
        assert!(
            batch(&p, &s).await.unwrap().is_empty(),
            "metadata shortcut must not escape selected fields"
        );
    }
}

#[test]
fn specific_late_body_hits_survive_short_substrings_in_routine_prefixes() {
    let plan:Plan=serde_json::from_value(serde_json::json!({"intent":"model releases","entities":[],"routes":[{"language":"zh","groups":[["ai","大语言模型"]]},{"language":"en","groups":[["ai","foundation model","language model"],["announced","launch","new model","new release","released"]]}],"filter":{},"unsupported":[]})).unwrap();
    let text = format!(
        "{}Today we are releasing a new AI foundation model, Harbor-11.",
        "Routine infrastructure maintenance and storage notes. ".repeat(500)
    );
    let chunks = fragments(&text, &plan, 3);
    assert!(
        chunks
            .iter()
            .any(|s| s.contains("releasing a new AI foundation model")),
        "'ai' inside 'maintenance' must not hide the specific late release"
    );
    assert!(chunks.iter().all(|s| s.chars().count() <= 700));
}

#[test]
fn v8_upgrade_is_idempotent_and_preserves_business_data() {
    let mut c = crate::db::migrations::historical_fixture(8);
    c.execute_batch("INSERT INTO feed(id,title,feed_url) VALUES(1,'keep','https://keep.test'); INSERT INTO entry(id,feed_id,title) VALUES(1,1,'keep'); INSERT INTO entry_note(entry_id,markdown,updated_at) VALUES(1,'keep note',datetime('now'));").unwrap();
    for _ in 0..2 {
        crate::db::migrations::run_migrations(&mut c).unwrap();
    }
    assert_eq!(
        c.query_row(
            "SELECT markdown FROM entry_note WHERE entry_id=1",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "keep note"
    );
    assert_eq!(
        c.query_row(
            "SELECT value FROM settings WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        crate::db::migrations::SCHEMA_VERSION.to_string()
    );
    assert_eq!(revision(&c).unwrap(), (0, 0));
}

#[tokio::test]
async fn scope_staleness_flags_notes_and_unpublished_cache() {
    let p = pool().await;
    let metadata = crate::agent::search_filter::metadata(&p).await.unwrap();
    let (plan, filter) = topic_search::parse_plan(
        &plan("models").to_string(),
        "新的AI模型",
        &metadata,
        &manual(),
    )
    .unwrap();
    let s = create(
        &p,
        "key".into(),
        "新的AI模型".into(),
        manual(),
        plan,
        filter,
    )
    .await
    .unwrap();
    assert!(
        cached(&p, "key").await.unwrap().is_none(),
        "cancelled unpublished work cannot become an active cached scope"
    );
    let cs = batch(&p, &s).await.unwrap();
    assert!(cs
        .iter()
        .all(|c| c.chunks.iter().all(|ch| !ch.text.contains("SECRET_NOTE"))));
    let js = cs
        .iter()
        .map(|c| Judgment {
            entry_id: c.entry_id,
            score: 0,
            reason: "not selected".into(),
            evidence: None,
        })
        .collect::<Vec<_>>();
    let s = save_batch(&p, &s, &cs, &js).await.unwrap();
    publish(&p, &s.id).await.unwrap();
    for i in 0..22 {
        create(
            &p,
            format!("failed-attempt-{i}"),
            "retry".into(),
            manual(),
            s.plan.clone(),
            s.filter.clone(),
        )
        .await
        .unwrap();
    }
    assert!(
        load(&p, &s.id).await.is_ok(),
        "repeated replacement attempts must not expire this week's valid scope"
    );
    let mut f = s.filter.clone();
    f.ai_search_id = Some(s.id.clone());
    let check = f.clone();
    db(&p, move |c| {
        assert!(!validate_scope(c, &check, true).unwrap());
        c.execute("UPDATE entry SET is_read=1,is_starred=1 WHERE id=2", [])
            .unwrap();
        assert!(!validate_scope(c, &check, true).unwrap());
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        ensure_current(&p, &s).await.is_err(),
        "continuation must acknowledge changed manual flags"
    );
    assert!(cached(&p, "key").await.unwrap().is_none());
    assert!(save_batch(&p, &s, &[], &[]).await.is_err());
    let check = f.clone();
    db(&p, move |c| {
        c.execute(
            "UPDATE content SET markdown='new content' WHERE entry_id=2",
            [],
        )
        .unwrap();
        assert!(validate_scope(c, &check, false).unwrap());
        assert_eq!(
            validate_scope(c, &check, true).unwrap_err(),
            "AI_SEARCH_STALE"
        );
        Ok(())
    })
    .await
    .unwrap();
    f.ai_search_id = Some("' OR 1=1 --".into());
    db(&p, move |c| {
        assert_eq!(
            validate_scope(c, &f, false).unwrap_err(),
            "AI_SEARCH_EXPIRED"
        );
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn literal_terms_are_parameterized_and_notes_require_explicit_field() {
    let p = pool().await;
    let metadata = crate::agent::search_filter::metadata(&p).await.unwrap();
    let mut raw = plan("models");
    raw["routes"] = serde_json::json!([{"language":"zh","groups":[["' OR 1=1 --"]]},{"language":"en","groups":[["\" OR *"]]}]);
    let (plan, filter) =
        topic_search::parse_plan(&raw.to_string(), "query", &metadata, &manual()).unwrap();
    let s = create(&p, "sql".into(), "query".into(), manual(), plan, filter)
        .await
        .unwrap();
    assert_eq!(s.total, 0);
    assert!(batch(&p, &s).await.unwrap().is_empty());
    let mut raw=serde_json::from_value::<serde_json::Value>(serde_json::json!({"intent":"notes","entities":[],"routes":[{"language":"zh","groups":[["private annotation"]]},{"language":"en","groups":[["private annotation"]]}],"filter":{"search_field":"note"},"unsupported":[]})).unwrap();
    let (plan, filter) =
        topic_search::parse_plan(&raw.to_string(), "in my notes", &metadata, &manual()).unwrap();
    let s = create(
        &p,
        "notes".into(),
        "in my notes".into(),
        manual(),
        plan,
        filter,
    )
    .await
    .unwrap();
    let cs = batch(&p, &s).await.unwrap();
    assert_eq!(cs.len(), 1);
    assert!(cs[0]
        .chunks
        .iter()
        .any(|ch| ch.text.contains("SECRET_NOTE")));
    raw["filter"] = serde_json::json!({"has_note":true});
    let (plan, filter) =
        topic_search::parse_plan(&raw.to_string(), "has notes", &metadata, &manual()).unwrap();
    let s = create(
        &p,
        "presence".into(),
        "has notes".into(),
        manual(),
        plan,
        filter,
    )
    .await
    .unwrap();
    assert_eq!(s.total, 0, "has_note must not opt in to note text recall");
}
