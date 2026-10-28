"""Loopback-only import fixtures; writes no application data and uses no AI."""
import json
import sys
import threading
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from xml.sax.saxutils import escape

root = Path(sys.argv[1]).resolve()
root.mkdir(parents=True, exist_ok=True)
counts = Counter()
events = []
lock = threading.Lock()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        with lock:
            counts[self.path] += 1
            attempt = counts[self.path]
        status = 503 if self.path == "/fail" or (self.path == "/retry" and attempt == 1) else 200
        if status == 200:
            name, home = ("Next Feed", "https://next.fixture.test") if self.path == "/good" else ("Retry Feed", "https://retry.fixture.test")
            items = "".join(f"<item><guid>{home}/{i}</guid><link>{home}/{i}</link><title>Fixture article {i}</title><description>Fixture summary</description></item>" for i in range(3))
            body = f'<rss version="2.0"><channel><title>{name}</title><link>{home}</link><description>Fixture</description>{items}</channel></rss>'.encode()
        else:
            body = b"Fixture temporary failure"
        with lock:
            events.append({"path": self.path, "attempt": attempt, "status": status})
            (root / "requests.json").write_text(json.dumps(events, indent=2), encoding="utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/rss+xml; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


server = ThreadingHTTPServer(("127.0.0.1", 18764), Handler)
outlines = "".join(f'<outline text="{escape(title)}" type="rss" xmlUrl="http://127.0.0.1:18764/{path}"/>' for title, path in [("Retry fixture", "retry"), ("Mirror fixture", "mirror"), ("Failed fixture", "fail"), ("Next fixture", "good"), ("Repeated URL", "retry")])
(root / "import.opml").write_text(f'<?xml version="1.0" encoding="UTF-8"?><opml version="2.0"><head><title>Import regression</title></head><body>{outlines}</body></opml>', encoding="utf-8")
print("Ready http://127.0.0.1:18764", flush=True)
server.serve_forever()
