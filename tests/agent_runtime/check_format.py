"""Check changed Rust; leave unrelated historical command formatting intact."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
files = [
    "src-tauri/src/agent/runtime.rs",
    "src-tauri/src/agent/runtime_tests.rs",
    "src-tauri/src/agent/summary.rs",
    "src-tauri/src/agent/m4w_017_tests.rs",
    "src-tauri/src/db/models.rs",
]
subprocess.run(["rustfmt", "--edition", "2021", "--check", *files], cwd=root, check=True)
source = (root / "src-tauri/src/commands/agent_cmd.rs").read_text(encoding="utf-8")
commands = source.split("pub(crate) async fn resolve_agent_route", 1)[1].split(
    "// ── Custom prompt templates ──", 1
)[0].strip()
commands = "pub(crate) async fn resolve_agent_route" + commands
status = "#[tauri::command]\n" + source.split("#[tauri::command]\npub async fn get_agent_status", 1)[1].split(
    "// ── Model management ──", 1
)[0]
status = status.replace("#[tauri::command]\n(", "#[tauri::command]\npub async fn get_agent_status(", 1)
subprocess.run(
    ["rustfmt", "--edition", "2021", "--check"],
    input=commands + "\n\n" + status.strip() + "\n", text=True, encoding="utf-8", cwd=root, check=True,
)
print("M4W-003 changed Rust format passed")
