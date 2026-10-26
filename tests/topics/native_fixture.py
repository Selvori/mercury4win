"""Prepare / inspect only the synthetic topic acceptance DB (never user data)."""
import json
import sqlite3
import sys
from pathlib import Path

path = Path(sys.argv[1]).resolve()
if "com.mercury4win.topics20261026" not in path.parts or path.name != "mercury.db":
    raise SystemExit("Refuse database outside named isolated acceptance directory")
connection = sqlite3.connect(f"file:{path.as_posix()}?mode=rw", uri=True)
assert connection.execute("SELECT title FROM entry WHERE id=1").fetchone()[0] == "Release"
action = sys.argv[2]
if action == "configure":
    connection.execute("INSERT INTO agent_provider_profile(id,name,base_url,is_enabled) VALUES(?,?,?,1)",
                       ("controlled", "Controlled loopback", "http://127.0.0.1:18756"))
    connection.execute("INSERT INTO agent_model_profile(id,provider_id,name,model_name,supports_summary,is_enabled) VALUES(?,?,?,?,1,1)",
                       ("controlled", "controlled", "Controlled fixture", "controlled-fixture"))
    connection.execute("UPDATE smart_view SET name=?,filter_json=? WHERE id=1",
                       ("Changed upstream view", json.dumps({"search": "NO_MATCH_AFTER_EDIT"})))
elif action == "revise":
    connection.execute("UPDATE entry SET feed_content=? WHERE id=3", ("<p>Revision: independent test on GPU A measured 37 tokens per second after driver correction.</p>",))
    connection.execute("DELETE FROM content WHERE entry_id=3")
    connection.execute("DELETE FROM content_html_cache WHERE entry_id=3")
elif action == "stalled":
    frozen = connection.execute("SELECT input_json FROM topic_brief ORDER BY id DESC LIMIT 1").fetchone()[0]
    connection.execute("INSERT INTO topic_run(request_id,topic_id,status,input_json,configuration_json) VALUES('synthetic-interrupted',1,'running',?1,'{}')", (frozen,))
elif action != "inspect":
    raise SystemExit("Unknown action")
connection.commit()
result = {"counts": {table: connection.execute(f"SELECT count(*) FROM {table}").fetchone()[0]
                     for table in ["entry", "entry_note", "smart_view", "topic", "topic_member", "topic_run", "topic_brief", "llm_usage_event"]},
          "topics": connection.execute("SELECT id,name,notes,rule_json FROM topic").fetchall(),
          "versions": connection.execute("SELECT id,topic_id,baseline_id,input_hash FROM topic_brief ORDER BY id").fetchall(),
          "runs": connection.execute("SELECT request_id,status,error FROM topic_run ORDER BY rowid").fetchall(),
          "usage": connection.execute("SELECT agent_type,request_status,total_tokens FROM llm_usage_event ORDER BY id").fetchall(),
          "foreign_keys": connection.execute("PRAGMA foreign_key_check").fetchall()}
print(json.dumps(result, ensure_ascii=False, indent=2))
