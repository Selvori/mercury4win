"""Loopback-only, synthetic two-stage Provider. Never forwards requests.

python tests/topics/auto_provider.py <output directory> [port=18757]
control.json: mode=ok|no_information|invalid|invalid_quote|summary_error|summary_empty,
delay (seconds), delay_stage=selection|summary|all.
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
        selecting = "candidates" in payload
        stage = "selection" if selecting else "summary"
        options = json.loads(control.read_text(encoding="utf-8"))
        with (root / "requests.jsonl").open("a", encoding="utf-8") as file:
            file.write(json.dumps({"stage": stage, "mode": options["mode"], "payload": payload}, ensure_ascii=False) + "\n")
        if options.get("delay_stage", "all") in (stage, "all"):
            time.sleep(options.get("delay", 0))
        if selecting:
            selected = [{"entry_id": s["entry_id"], "group": "合成事件 " + str(s["entry_id"])}
                        for s in payload["candidates"] if s["entry_id"] in (1, 2, 7, 8)]
            if options["mode"] == "no_information":
                selected = []
            elif options["mode"] == "invalid":
                selected = [{"entry_id": 999, "group": "invalid"}]
            result = {"selected": selected, "no_information": not selected}
        elif options["mode"] == "summary_empty":
            result = {"claims": [], "no_information": True}
        elif options["mode"] == "invalid_quote":
            result = {"claims": [{"kind": "highlights", "text": "Synthetic invalid quote fixture",
                                   "citations": [{"entry_id": payload["current"]["sources"][0]["entry_id"],
                                                  "quote": "This quotation does not exist in the frozen source."}]}]}
        else:
            result = {"claims": [{"kind": ("highlights" if i == 0 else "details" if i == 1 else "reading"),
                                  "text": "受控验收：" + s["title"] + ("（仅依据标题与 Feed 摘要）" if s["basis"] == "metadata" else "，这是一条合成信息，验证流程而非模型质量。"),
                                  "citations": [{"entry_id": s["entry_id"], "passage_id": s["passages"][0]["passage_id"]}]}
                                 for i, s in enumerate(payload["current"]["sources"])]}
        body = json.dumps({"choices": [{"message": {"role": "assistant", "content": json.dumps(result, ensure_ascii=False)}, "finish_reason": "stop"}],
                           "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150}}, ensure_ascii=False).encode("utf-8")
        self.send_response(503 if not selecting and options["mode"] == "summary_error" else 200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            pass

    def log_message(self, *_):
        pass


port = int(sys.argv[2]) if len(sys.argv) > 2 else 18757
print(f"Controlled two-stage Provider on 127.0.0.1:{port}", flush=True)
ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
