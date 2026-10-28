"""Create a new isolated synthetic native DB from a verified V5 fixture."""
import json
import sqlite3
import sys
from pathlib import Path
source, destination = (Path(v).resolve() for v in sys.argv[1:3])
assert source.is_file() and destination.parent.is_dir() and not destination.exists()
with sqlite3.connect(source.as_uri() + "?mode=ro", uri=True) as seed:
    assert seed.execute("SELECT COUNT(*) FROM agent_provider_profile").fetchone()[0] == 0
    assert seed.execute("SELECT COUNT(*) FROM llm_usage_event").fetchone()[0] == 0
    with sqlite3.connect(destination) as conn:
        seed.backup(conn)
corpus = json.loads(Path(__file__).with_name("corpus.json").read_text(encoding="utf-8"))
with sqlite3.connect(destination) as conn:
    conn.execute("PRAGMA foreign_keys=ON")
    conn.executescript("""
      DELETE FROM entry; DELETE FROM feed; DELETE FROM tag;
      INSERT INTO feed(id,title,feed_url) VALUES(1,'技术来源','https://tech.test/rss'),(2,'新闻来源','https://news.test/rss');
      INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');
      INSERT INTO agent_provider_profile(id,name,base_url,is_enabled) VALUES('topic-native','Controlled Topic Search','http://127.0.0.1:18762/v1',1);
      INSERT INTO agent_model_profile(id,provider_id,name,model_name,is_enabled,supports_summary,supports_translation,supports_tagging) VALUES('topic-native-model','topic-native','Native fixture','fixture',1,1,0,0);
      INSERT INTO agent_profile(agent_type,primary_model_id) VALUES('summary','topic-native-model') ON CONFLICT(agent_type) DO UPDATE SET primary_model_id=excluded.primary_model_id,fallback_model_id=NULL;
    """)
    for a in corpus["articles"]:
        conn.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_starred,is_read) VALUES(?,?,?,?,?,datetime('now',?),?,0)", (a["id"],a["feed"],f'topic-{a["id"]}',a["title"],a["summary"],f'-{a["days"]} days',a["starred"]))
        body = "Routine infrastructure maintenance and storage notes. " * a.get("prefix_repeat",0) + a["body"]
        conn.execute("INSERT INTO content(entry_id,markdown) VALUES(?,?)",(a["id"],body))
    for i in range(100,160):
        conn.execute("INSERT INTO entry(id,feed_id,title,summary,published_at) VALUES(?,1,'AI model launch commentary','Financing-only AI company news',datetime('now'))",(i,))
    conn.execute("INSERT INTO entry_note VALUES(1,'SECRET_NOTE synthetic private annotation',datetime('now'))")
    assert not conn.execute("PRAGMA foreign_key_check").fetchall()
print(json.dumps({"synthetic_entries":80,"real_credentials":False}))
