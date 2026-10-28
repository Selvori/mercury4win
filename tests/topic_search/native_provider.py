"""Loopback protocol fixture, not a model quality evaluator. No forwarding."""
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
root=Path(sys.argv[1]).resolve()
assert root.is_dir()
corpus=json.loads(Path(__file__).with_name("corpus.json").read_text(encoding="utf-8"))
class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        data=json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        payload=json.loads(data["messages"][1]["content"])
        control=json.loads((root/"control.json").read_text(encoding="utf-8")) if (root/"control.json").exists() else {}
        with (root/"requests.jsonl").open("a",encoding="utf-8") as f:
            f.write(json.dumps({"time":time.time(),"payload":payload,"control":control},ensure_ascii=False)+"\n")
        time.sleep(control.get("delay",0))
        if control.get("fail"):
            self.send_response(503); self.end_headers(); return
        sentence=payload.get("sentence",payload.get("topic",""))
        topic="database" if "数据库" in sentence else "tools" if "tool" in sentence else "none" if "火山" in sentence else "entity" if "Nebula-Q47" in sentence else "models"
        if "candidates" in payload:
            positive={a["id"] for a in corpus["articles"] if topic in a["topics"]}
            judgments=[]
            for c in payload["candidates"]:
                hit=c["entry_id"] in positive
                ch=next(ch for ch in c["chunks"] if len(ch["text"])>=4)
                judgments.append({"entry_id":c["entry_id"],"score":95 if hit else 10,"reason":"本机受控协议验收：直接相关" if hit else "本机受控协议验收：不相关","evidence":{"key":ch["key"],"quote":ch["text"][:200]} if hit else None})
            reply={"judgments":judgments}
        else:
            zh,en,event=(['数据库','查询','sql'],['database','sql','postgresql','query'],['performance','latency','benchmark','fast','加速','性能']) if topic=='database' else (['开源','构建工具'],['open-source','build tool','toolforge'],['release','发布']) if topic=='tools' else (['海底火山','声学'],['subsea volcano','acoustic monitoring'],['discovery','发现']) if topic=='none' else (['人工智能','大语言模型','语言模型'],['ai','language model','foundation model','multimodal model'],['release','introduc','unveil','launch','发布','开放权重'])
            reply={"intent":"寻找主题的直接相关事件，排除融资、教程及汽车混淆","entities":[],"routes":[{"language":"zh","groups":[zh,event]},{"language":"en","groups":[en,event]},{"language":"zh","groups":[zh]},{"language":"en","groups":[en]}],"filter":{},"unsupported":[]}
        body=json.dumps({"choices":[{"message":{"role":"assistant","content":json.dumps(reply,ensure_ascii=False)},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":50,"total_tokens":150}},ensure_ascii=False).encode()
        self.send_response(200);self.send_header("Content-Type","application/json");self.send_header("Content-Length",str(len(body)));self.end_headers()
        try:self.wfile.write(body)
        except (BrokenPipeError,ConnectionResetError):pass
    def log_message(self,*args):pass
ThreadingHTTPServer(("127.0.0.1",18762),Handler).serve_forever()
