import { build } from "esbuild";
import { pathToFileURL, fileURLToPath } from "node:url";
import path from "node:path";
import { MessageChannel } from "node:worker_threads";
if (!process.argv[2]) throw new Error("Pass separately installed happy-dom/lib/index.js");
const { Window } = await import(pathToFileURL(process.argv[2]).href);
const window = new Window({ url: "http://localhost/" });
const channels = [];
window.MessageChannel = class extends MessageChannel { constructor() { super(); channels.push(this); } };
window.console = console;
window.document.body.innerHTML = '<div id="app"></div>';
const directory = path.dirname(fileURLToPath(import.meta.url));
const bundle = await build({
  entryPoints: [path.join(directory, "dom.tsx")], bundle: true, write: false, platform: "browser", format: "iife", jsx: "automatic",
  tsconfig: path.join(directory, "../../tsconfig.app.json"), define: { "process.env.NODE_ENV": '"development"' },
  plugins: [{ name: "controlled-native-transport", setup(b) { b.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(directory, "mock_transport.ts") })); } }],
});
let resolve;
const completed = new Promise((r) => { resolve = r; });
window.fetch = async (url, options) => { if (url !== "/result") throw new Error(`Unexpected network ${url}`); resolve(JSON.parse(options.body)); return { ok: true }; };
let timer;
try {
  window.eval(bundle.outputFiles[0].text);
  const result = await Promise.race([completed, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("DOM regression timeout")), 20000); })]);
  console.log(JSON.stringify(result, null, 2)); if (result.error) process.exitCode = 1;
} finally { clearTimeout(timer); await window.happyDOM.close(); for (const c of channels) { c.port1.close(); c.port2.close(); } }
