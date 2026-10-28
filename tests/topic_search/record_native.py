"""Record only the verified synthetic native fixture after app shutdown."""
import json
import re
import sqlite3
import sys
from pathlib import Path

app, capture, report = (Path(p).resolve() for p in sys.argv[1:4])
output = report.with_suffix('.json')
ui_output = report.with_name(report.name.removesuffix('-evidence') + '-ui').with_suffix('.txt')
assert not output.exists() and not ui_output.exists()
with sqlite3.connect((app/'mercury.db').as_uri()+'?mode=ro', uri=True) as c:
    assert {r[0] for r in c.execute('SELECT feed_url FROM feed')} == {'https://tech.test/rss','https://news.test/rss'}
    assert c.execute('SELECT COUNT(*) FROM entry').fetchone()[0] == 80
    assert c.execute("SELECT base_url FROM agent_provider_profile WHERE id='topic-native'").fetchone()[0] == 'http://127.0.0.1:18762/v1'
    rows = [json.loads(line) for line in (capture/'requests.jsonl').read_text(encoding='utf-8').splitlines()]
    assert 'SECRET_NOTE' not in json.dumps(rows,ensure_ascii=False)
    facts = {
        'date':'2026-10-28','baseline':'81f7b2e + all preserved uncommitted changes',
        'synthetic_only':True,'identifier':app.name,'entries':80,
        'schema':c.execute("SELECT value FROM settings WHERE key='schema_version'").fetchone()[0],
        'foreign_key_errors':c.execute('PRAGMA foreign_key_check').fetchall(),
        'read_ids':[r[0] for r in c.execute('SELECT id FROM entry WHERE is_read=1 ORDER BY id')],
        'note_count':c.execute('SELECT COUNT(*) FROM entry_note').fetchone()[0],
        'smart_views':c.execute('SELECT name,filter_json FROM smart_view ORDER BY id').fetchall(),
        'requests':len(rows),
        'http_status_counts':dict(c.execute('SELECT request_status,COUNT(*) FROM llm_usage_event GROUP BY request_status')),
        'no_note_outbound':True,
        'late_body_seen':any('Harbor-11' in ch['text'] for r in rows for a in r['payload'].get('candidates',[]) if a['entry_id']==5 for ch in a['chunks']),
        'credentials_files':len(list((app/'credentials').glob('*.key'))) if (app/'credentials').exists() else 0,
        'published_scopes':[
            {'id':s[0],'sentence':s[1],'checked':s[2],
             'matches':[r[0] for r in c.execute('SELECT entry_id FROM ai_search_judgment WHERE session_id=? AND score>=60 ORDER BY entry_id',(s[0],))]}
            for s in c.execute('SELECT id,sentence,checked FROM ai_search_session WHERE published=1 ORDER BY rowid')
        ],
        'request_sequence':[{'topic':r['payload'].get('sentence',r['payload'].get('topic')),
            'phase':'judging' if 'candidates' in r['payload'] else 'understanding',
            'candidate_ids':[a['entry_id'] for a in r['payload'].get('candidates',[])],
            'control':r['control']} for r in rows],
    }
assert facts['read_ids']==[1,2,3,4,5,6] and not facts['foreign_key_errors']
assert facts['credentials_files']==0 and facts['late_body_seen']
assert sum(facts['http_status_counts'].values())==facts['requests']
output.write_text(json.dumps(facts,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

trace=json.loads((capture/'ui-trace.json').read_text(encoding='utf-8'))
patterns=r'Current AI results|Checked |Relevance|Sort Value|当前 AI|已检查|正在|AI 请求|已取消请求|普通视图兼容验收|disabled.*smart|disabled.*智能视图|Engineering dispatch|星河团队|Terra-8B|AI 主题搜索|AI topic search|80 entries|80 篇|不使用 AI|部分检查|partialFailure'
seen=set()
lines=['Actual synthetic UIA excerpts, numeric accessibility indexes/indentation removed; deduplicated in observation order.']
for state in trace:
    for line in state.splitlines():
        if re.search(patterns,line):
            line=re.sub(r'^\s*\d+\s+','',line).strip()
            if line not in seen:seen.add(line);lines.append(line)
ui_output.write_text('\n'.join(lines)+'\n',encoding='utf-8')
print(json.dumps({k:facts[k] for k in ['entries','schema','requests','http_status_counts','read_ids','late_body_seen','no_note_outbound']},ensure_ascii=False))
