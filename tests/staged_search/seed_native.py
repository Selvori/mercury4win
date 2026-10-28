"""Create a NEW isolated fixture from a verified credential-free V5 seed."""
import sqlite3, sys, json
from pathlib import Path
sys.dont_write_bytecode=True
from fixtures import articles
source,destination=(Path(v).resolve() for v in sys.argv[1:3])
assert source.is_file() and destination.parent.is_dir() and not destination.exists()
with sqlite3.connect(source.as_uri()+'?mode=ro',uri=True) as seed:
    assert seed.execute('SELECT COUNT(*) FROM agent_provider_profile').fetchone()[0]==0
    assert seed.execute('SELECT COUNT(*) FROM llm_usage_event').fetchone()[0]==0
    with sqlite3.connect(destination) as c: seed.backup(c)
with sqlite3.connect(destination) as c:
    c.execute('PRAGMA foreign_keys=ON')
    c.executescript("""DELETE FROM entry;DELETE FROM feed;DELETE FROM tag;
    INSERT INTO feed(id,title,feed_url) VALUES(1,'技术来源','https://tech.test/rss'),(2,'新闻来源','https://news.test/rss');
    INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');
    INSERT INTO agent_provider_profile(id,name,base_url,is_enabled) VALUES('staged-native','Controlled staged search','http://127.0.0.1:18763/v1',1);
    INSERT INTO agent_model_profile(id,provider_id,name,model_name,is_enabled,supports_summary,supports_translation,supports_tagging) VALUES('staged-model','staged-native','Native fixture','fixture',1,1,0,0);
    INSERT INTO agent_profile(agent_type,primary_model_id) VALUES('summary','staged-model') ON CONFLICT(agent_type) DO UPDATE SET primary_model_id=excluded.primary_model_id,fallback_model_id=NULL;""")
    for a in articles():
        c.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_starred,is_read) VALUES(?,?,?,?,?,datetime('now',?),?,0)",(a['id'],a['feed'],f"staged-{a['id']}",a.get('title'),a.get('summary'),f"-{a['days']} days",a['starred']))
        if a.get('body') is not None:
            body=a.get('opening','')+a.get('prefix','Routine infrastructure maintenance and storage notes. ')*a.get('prefix_repeat',0)+a['body']
            c.execute('INSERT INTO content(entry_id,markdown) VALUES(?,?)',(a['id'],body))
    c.execute("INSERT INTO entry_note VALUES(1,'SECRET_NOTE synthetic private annotation',datetime('now'))")
    assert not c.execute('PRAGMA foreign_key_check').fetchall()
print(json.dumps({'synthetic_entries':71,'credentials':0}))
