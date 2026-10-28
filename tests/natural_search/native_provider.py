"""Loopback-only Provider for native natural-search acceptance; never forwards.

Usage: python tests/natural_search/native_provider.py <new evidence directory> <port>
control.json accepts mode=ok|invalid|error and delay in seconds.
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
        payload = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        data = json.loads(payload["messages"][1]["content"])
        options = json.loads(control.read_text(encoding="utf-8"))
        text = data["sentence"]
        record = {"at": time.time(), "mode": options["mode"], "input": data}
        with (root / "requests.jsonl").open("a", encoding="utf-8") as file:
            file.write(json.dumps(record, ensure_ascii=False) + "\n")
        result = {"filter": {"search": "Rust", "recent_days": 7, "starred_only": True}, "unsupported": []}
        if "上周读过" in text or "reading history" in text:
            result = {"filter": None, "unsupported": ["阅读历史未记录 / Reading history is unavailable"]}
        elif "未知" in text:
            result["filter"]["source"] = "未知来源"
        elif "重名" in text:
            result["filter"]["source"] = "重名来源"
        elif "不存在的标签" in text:
            result["filter"]["tags"] = ["不存在的标签"]
        elif "空结果" in text:
            result["filter"]["search"] = "no-such-article"
        elif "组合" in text or "combined" in text:
            result["filter"].update({"source": "技术来源", "tags": ["Rust"], "is_read": False, "has_note": True, "sort": "oldest"})
        time.sleep(options.get("delay", 0))
        content = json.dumps(result, ensure_ascii=False)
        if options["mode"] == "invalid":
            content = '{"filter":{"feed_id":999,"sql":"invalid"},"unsupported":[]}'
        response = {"choices": [{"message": {"role": "assistant", "content": content}, "finish_reason": "stop"}], "usage": {"prompt_tokens": 20, "completion_tokens": 30, "total_tokens": 50}}
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


port = int(sys.argv[2])
print(f"Natural search controlled API: http://127.0.0.1:{port}", flush=True)
ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
