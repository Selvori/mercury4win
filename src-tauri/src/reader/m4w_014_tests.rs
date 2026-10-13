//! Theme rendering and cache regressions: in-memory SQLite, no network or AI.
use super::{pipeline, theme, theme::FontConfig};
use crate::agent::translation_segments;
use crate::db::{content_store, migrations};
use deadpool_sqlite::{Config, Pool, Runtime};
use reqwest::Client;

fn luminance(hex: &str) -> f64 {
    let channels: Vec<f64> = (1..7)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f64 / 255.0)
        .map(|c| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        })
        .collect();
    channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
}

#[test]
fn paper_colors_contrast_and_explicit_fonts_are_preserved() {
    for (variant, background, foregrounds) in [
        ("light", "#f5efe6", ["#3b2f24", "#6b5b4b", "#7a4f16"]),
        ("dark", "#1f1a16", ["#e8dccf", "#bfae9a", "#d8b07a"]),
    ] {
        for foreground in foregrounds {
            let a = luminance(background);
            let b = luminance(foreground);
            let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            assert!(contrast >= 4.5, "{variant}/{foreground}: {contrast}");
        }
        for (family, expected) in [
            ("sans-serif", "Inter"),
            ("serif", "Georgia"),
            ("system", "Noto Sans"),
        ] {
            for size in [12, 16, 22, 24] {
                let font = FontConfig {
                    family: family.into(),
                    size,
                };
                let paper = theme::wrap_with_theme_and_font(
                    "<p>body</p>",
                    &format!("paper-{variant}"),
                    &font,
                );
                let classic = theme::wrap_with_theme_and_font(
                    "<p>body</p>",
                    &format!("classic-{variant}"),
                    &font,
                );
                assert!(paper.contains(background) && paper.contains(expected));
                assert!(paper.contains(&format!("font-size: {size}px")));
                assert_ne!(paper, classic);
            }
        }
    }
}

async fn pool() -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|c| {
        migrations::run_migrations(c).unwrap();
        c.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test/feed'); INSERT INTO entry(id,feed_id,guid,url,feed_content,feed_content_type) VALUES(1,1,'theme','http://127.0.0.1:1/offline','<p>Reader theme body.</p>','text/html');").unwrap();
    }).await.unwrap();
    pool
}

#[tokio::test]
async fn styles_fonts_use_independent_cache_without_rebuilding_body_and_emit_fixture() {
    let pool = pool().await;
    let client = Client::new();
    pipeline::build_reader_content(&pool, &client, 1, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    let before = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    pool.get().await.unwrap().interact(|c| {
        c.execute_batch("UPDATE entry SET feed_content=NULL; CREATE TRIGGER no_body_update BEFORE UPDATE ON content BEGIN SELECT RAISE(FAIL,'body must not rebuild'); END;").unwrap();
    }).await.unwrap();
    let mut rendered = serde_json::Map::new();
    for (style, variant) in theme::DEFAULT_THEMES {
        for family in ["sans-serif", "serif", "system"] {
            for size in [12, 16, 22, 23, 24] {
                let id = format!("{style}-{variant}");
                let font = FontConfig {
                    family: family.into(),
                    size,
                };
                let html = pipeline::build_reader_content(&pool, &client, 1, &id, &font)
                    .await
                    .unwrap();
                let key = format!("{id}:{family}:{size}");
                assert_eq!(
                    pipeline::get_cached_reader_html(&pool, 1, &key)
                        .await
                        .unwrap(),
                    Some(html.clone())
                );
                rendered.insert(key, html.into());
            }
        }
    }
    let after = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(before.markdown, after.markdown);
    assert_eq!(before.html, after.html);
    assert_eq!(before.markdown_version, after.markdown_version);
    assert_eq!(before.readability_version, after.readability_version);
    pool.get().await.unwrap().interact(|c| {
        let count: i32 = c.query_row("SELECT COUNT(*) FROM content_html_cache", [], |r| r.get(0)).unwrap();
            assert_eq!(count, 60);
        c.execute_batch("CREATE TRIGGER no_cache_write BEFORE INSERT ON content_html_cache BEGIN SELECT RAISE(FAIL,'cache hit must not write'); END;").unwrap();
    }).await.unwrap();
    let hit = pipeline::build_reader_content(
        &pool,
        &client,
        1,
        "paper-dark",
        &FontConfig {
            family: "serif".into(),
            size: 22,
        },
    )
    .await
    .unwrap();
    assert_eq!(hit, rendered["paper-dark:serif:22"].as_str().unwrap());
    if let Ok(path) = std::env::var("M4W_THEME_FIXTURE") {
        let (html, sources) = translation_segments::extract_html(&hit).unwrap();
        let fixture = serde_json::json!({"rendered": rendered, "translation": {"source_hash": "theme-source", "html": html, "source_segments": sources, "result": null}});
        std::fs::write(path, serde_json::to_vec_pretty(&fixture).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn prior_render_version_rebuilds_paper_css_without_touching_source() {
    let pool = pool().await;
    let client = Client::new();
    let font = FontConfig::default();
    pipeline::build_reader_content(&pool, &client, 1, "paper-light", &font)
        .await
        .unwrap();
    pool.get().await.unwrap().interact(|c| {
        c.execute_batch("UPDATE entry SET feed_content=NULL; UPDATE content_html_cache SET html='OLD PAPER',reader_render_version=4; CREATE TRIGGER no_body_update BEFORE UPDATE ON content BEGIN SELECT RAISE(FAIL,'source update forbidden'); END;").unwrap();
    }).await.unwrap();
    assert!(
        pipeline::get_cached_reader_html(&pool, 1, "paper-light:sans-serif:16")
            .await
            .unwrap()
            .is_none()
    );
    let html = pipeline::build_reader_content(&pool, &client, 1, "paper-light", &font)
        .await
        .unwrap();
    assert!(html.contains("#f5efe6") && !html.contains("OLD PAPER"));
    assert_eq!(pipeline::READER_RENDER_VERSION, 5);
}
