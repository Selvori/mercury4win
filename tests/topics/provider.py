"""Loopback-only controlled Provider for native acceptance; never calls a model.

python tests/topics/provider.py <new output directory> [port]
Edit control.json: {"mode":"ok|invalid|error", "delay":0}. Request data is synthetic.
"""
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

root = Path(sys.argv[1]).resolve()
root.mkdir(parents=True, exist_ok=True)
control = root / "control.json"
if not control.exists():
    control.write_text('{"mode":"ok","delay":0}', encoding="utf-8")


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        payload = json.loads(request["messages"][1]["content"])
        current, previous = payload["current"], payload["previous"]
        options = json.loads(control.read_text(encoding="utf-8"))
        record = {"at": time.time(), "mode": options["mode"], "input": payload, "model": request["model"]}
        with (root / "requests.jsonl").open("a", encoding="utf-8") as file:
            file.write(json.dumps(record, ensure_ascii=False) + "\n")
        time.sleep(options.get("delay", 0))
        sources = [s for s in current["sources"] if s["body"].strip()]
        first = sources[0]

        def claim(kind, text, source=first):
            return {"kind": kind, "text": text, "citations": [{"entry_id": source["entry_id"], "quote": source["body"]}]}

        claims = [claim("overview", "受控验收：这些来源讨论本地推理性能，以下判断仅用于验证应用流程。"),
                  claim("common", "发布与转载描述同一发布，不能计算为两份独立证据。"),
                  {"kind": "open_questions", "text": "受控样例不评价真实模型的分析质量；需要核对独立复现。", "citations": []}]
        for source in sources:
            if source["entry_id"] == 3:
                claims.append(claim("evidence", "独立测试采用长上下文，测试条件需要随结论一起阅读。", source))
            if source["entry_id"] == 4:
                claims.append(claim("conditions", "CPU 与 GPU 条件不同，不能直接视为矛盾。", source))
            if source["entry_id"] == 5:
                claims.append(claim("contradictions", "同配置的无法复现报告与宣传存在明确分歧。", source))
        if previous:
            old = {s["entry_id"]: s for s in previous["input"]["sources"]}
            for source in sources:
                prior = old.get(source["entry_id"])
                if not prior:
                    claims.append(claim("new_evidence", "新增资料：" + source["title"], source))
                elif prior["content_version"] != source["content_version"]:
                    claims.append(claim("corrections", "正文已修订：" + source["title"], source))
            claims.append(claim("disagreement_changes", "增量比较基于上一个成功版本的冻结正文。"))
        content = json.dumps({"claims": claims}, ensure_ascii=False)
        if options["mode"] == "invalid":
            content = '{"claims":[{"kind":"overview","text":"fake","citations":[{"entry_id":999,"quote":"fabricated evidence"}]}]}'
        response = {"choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 100, "completion_tokens": 200, "total_tokens": 300}}
        body = json.dumps(response, ensure_ascii=False).encode("utf-8")
        self.send_response(503 if options["mode"] == "error" else 200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass

    def log_message(self, *_):
        pass


port = int(sys.argv[2]) if len(sys.argv) > 2 else 18756
print(f"Controlled Provider: http://127.0.0.1:{port}", flush=True)
ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
