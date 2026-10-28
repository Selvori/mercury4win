"""Seed an isolated native DB from the existing synthetic V5 search fixture."""
import json
import sqlite3
import sys
from pathlib import Path

source, destination = map(lambda v: Path(v).resolve(), sys.argv[1:3])
assert source.is_file() and not destination.exists()
assert destination.parent.is_dir()
with sqlite3.connect(source.as_uri() + "?mode=ro", uri=True) as seed:
    assert seed.execute("SELECT COUNT(*) FROM agent_provider_profile").fetchone()[0] == 0
    assert seed.execute("SELECT COUNT(*) FROM llm_usage_event").fetchone()[0] == 0
    with sqlite3.connect(destination) as conn:
        seed.backup(conn)
with sqlite3.connect(destination) as conn:
    conn.execute("PRAGMA foreign_keys=ON")
    conn.executescript("""
        DELETE FROM entry;
        DELETE FROM feed;
        DELETE FROM tag;
        INSERT INTO feed(id,title,feed_url) VALUES
            (1,'技术来源','https://a.invalid/rss'),(2,'新闻来源','https://b.invalid/rss'),
            (3,'重名来源','https://c.invalid/rss'),(4,'重名来源','https://d.invalid/rss');
        INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');
        INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_starred,is_read) VALUES
            (1,1,'nl-1','Rust 最近收藏 A','受控测试文章 A',datetime('now','-1 day'),1,0),
            (2,1,'nl-2','Rust 最近收藏 B','受控测试文章 B',datetime('now','-2 days'),1,1),
            (3,2,'nl-3','Rust 旧收藏','受控旧文章',datetime('now','-20 days'),1,0),
            (4,2,'nl-4','Python 未收藏','受控普通文章',datetime('now','-1 day'),0,0);
        INSERT INTO content(entry_id,markdown) VALUES(1,'Only synthetic local body, not for AI.'),(2,'Synthetic Rust body B.');
        INSERT INTO entry_note(entry_id,markdown,updated_at) VALUES(1,'Synthetic note, not for AI.',datetime('now'));
        INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1),(1,2),(2,1);
        INSERT INTO agent_provider_profile(id,name,base_url,is_enabled) VALUES('nl-native','Controlled Natural Search','http://127.0.0.1:18761/v1',1);
        INSERT INTO agent_model_profile(id,provider_id,name,model_name,is_enabled,supports_summary,supports_translation,supports_tagging)
            VALUES('nl-native-model','nl-native','Native fixture','fixture',1,1,0,0);
        INSERT INTO agent_profile(agent_type,primary_model_id) VALUES('summary','nl-native-model') ON CONFLICT(agent_type) DO UPDATE SET primary_model_id=excluded.primary_model_id,fallback_model_id=NULL;
    """)
    assert conn.execute("PRAGMA foreign_key_check").fetchall() == []
print(json.dumps({"synthetic_entries": 4, "sources": 4, "tags": 2, "real_credentials": False}))
