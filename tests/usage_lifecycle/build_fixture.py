"""Convert only the synthetic Rust report test's output to a TS fixture."""
import json
import sys
from pathlib import Path

report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
Path(__file__).with_name("fixture.ts").write_text(
    "export const fixture = " + json.dumps(report, ensure_ascii=False, indent=2) + ";\n",
    encoding="utf-8",
)
