"""Loopback staged protocol fixture, no external forwarding or quality claim."""
import json,sys,time
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
sys.dont_write_bytecode=True
from fixtures import articles
root=Path(sys.argv[1]).resolve();assert root.is_dir()
rows={a['id']:a for a in articles()}
def quote(key,text): return {'key':key,'quote':text[:200]}
def generic(text): return len(text)<10 or any(w in text.lower() for w in ['weekly','roundup','digest','dispatch','bulletin','notebook','newsletter','周报','合集','简报','汇总','动态','随笔','杂谈','未命名','无标题'])
class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        request=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert 'max_tokens' not in request
        p=json.loads(request['messages'][1]['content']);stage=p['stage']
        control=json.loads((root/'control.json').read_text(encoding='utf-8'))
        with (root/'requests.jsonl').open('a',encoding='utf-8') as f:f.write(json.dumps({'at':time.time(),'payload':p,'control':control},ensure_ascii=False)+'\n')
        time.sleep(control.get('delay',0) if control.get('delay_stage',stage)==stage else 0)
        if control.get('fail')==stage or control.get('fail')=='full_after_first' and stage=='full' and any(a['index']>0 for a in p['parts']):
            self.send_response(503);self.end_headers();return
        sentence=p.get('sentence',p.get('topic',''));topic='database' if '数据库' in sentence or 'database' in sentence else 'tools' if 'tool' in sentence else 'none' if '火山' in sentence else 'models'
        positive=lambda i: topic in rows.get(i,{}).get('topics',[])
        if stage=='understanding':
            zh,en,event=(['数据库','查询','sql'],['database','sql','postgresql','query'],['performance','latency','benchmark','加速','性能']) if topic=='database' else (['开源','构建工具'],['open-source','build tool','toolforge'],['release','发布']) if topic=='tools' else (['海底火山','声学'],['subsea volcano','acoustic monitoring'],['discovery','发现']) if topic=='none' else (['人工智能','大语言模型','语言模型'],['ai','language model','foundation model','multimodal model'],['release','introduc','unveil','launch','发布','开放权重'])
            reply={'intent':'寻找直接相关事件，排除融资、教程、历史引文及其他领域混淆','entities':[],'routes':[{'language':'zh','groups':[zh,event]},{'language':'en','groups':[en,event]},{'language':'zh','groups':[zh]},{'language':'en','groups':[en]}],'filter':{},'unsupported':[]}
        elif stage=='titles':
            reply={'titles':[dict(entry_id=a['entry_id'],decision='uncertain' if generic(a['title']) else 'possible' if positive(a['entry_id']) else 'unrelated',reason='受控标题阶段') for a in p['titles']]}
            if control.get('invalid_title_batch'):
                count=sum(json.loads(line)['payload']['stage']=='titles' for line in (root/'requests.jsonl').read_text(encoding='utf-8').splitlines())
                if count==control['invalid_title_batch']:reply['titles'][0]['entry_id']=999999
        elif stage=='previews':
            reply={'previews':[dict(entry_id=a['entry_id'],decision='possible' if positive(a['entry_id']) else 'uncertain' if not a['complete'] else 'unrelated',reason='受控预览阶段',evidence=quote(next(c for c in a['chunks'] if len(c['text'])>=4)['key'],next(c for c in a['chunks'] if len(c['text'])>=4)['text'])) for a in p['previews']]}
        elif stage=='full':
            reply={'parts':[]}
            for a in p['parts']:
                opposed=a['entry_id']==105 and a['index']+1==a['total'];kind='opposes' if opposed else 'supports' if positive(a['entry_id']) or a['entry_id']==105 else 'background'
                reply['parts'].append(dict(entry_id=a['entry_id'],key=a['key'],kind=kind,reason='受控连续正文段落',evidence=quote(a['key'],a['text']) if kind!='background' else None,context='这是历史引用，实际报道仅涉及融资。' if opposed else '完整资料的主题信息累计；受控协议验收。'))
        elif stage=='final':
            reply={'articles':[dict(entry_id=a['entry_id'],decision='relevant' if positive(a['entry_id']) else 'unrelated',score=95 if positive(a['entry_id']) else 5,reason='受控整篇判断：直接相关' if positive(a['entry_id']) else '受控整篇判断：无关',evidence=a['evidence'][0] if positive(a['entry_id']) else None,context_resolution='已核对历史引用及否定上下文。' if a['signals']['opposes'] else '') for a in p['articles']]}
        else:raise ValueError(stage)
        if control.get('quote_strings') and stage=='full':
            for row in reply['parts']:
                if row['evidence']:row['evidence']=row['evidence']['quote']
        if control.get('wrong_part_echo') and stage=='full':
            for row in reply['parts']:
                row['key']='full.1'
                if isinstance(row['evidence'],dict):row['evidence']['key']='full.1'
        if control.get('null_context') and stage=='final':
            for row in reply['articles']:
                if not row['context_resolution']:row['context_resolution']=None
        if control.get('arrays') and stage!='understanding':reply=next(iter(reply.values()))
        body=json.dumps({'choices':[{'message':{'role':'assistant','content':json.dumps(reply,ensure_ascii=False)},'finish_reason':'stop'}],'usage':{'prompt_tokens':100,'completion_tokens':50,'total_tokens':150}},ensure_ascii=False).encode()
        self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(body)));self.end_headers()
        try:self.wfile.write(body)
        except (BrokenPipeError,ConnectionResetError):pass
    def log_message(self,*args):pass
ThreadingHTTPServer(('127.0.0.1',18763),Handler).serve_forever()
