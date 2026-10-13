// Pass the path to a separately installed happy-dom/lib/index.js. This keeps
// the app dependencies and lockfile unchanged; serve.mjs needs no extra module.
import { pathToFileURL } from "node:url";
import { MessageChannel } from "node:worker_threads";
import { build_reader_test } from "./bundle.mjs";

if (!process.argv[2]) throw new Error("Usage: node tests/reader_ownership/run_dom.mjs <happy-dom/lib/index.js>");
const { Window } = await import(pathToFileURL(process.argv[2]).href);
const window = new Window({ url: "http://localhost/" });
const channels = [];
window.MessageChannel = class extends MessageChannel {
  constructor() { super(); channels.push(this); }
};
window.console = console;
window.document.body.innerHTML = '<pre id="result">Running...</pre><div id="app"></div>';
let resolve_result;
const finished = new Promise((resolve) => { resolve_result = resolve; });
window.fetch = async (url, options) => {
  if (url !== "/result") throw new Error(`Unexpected network call: ${url}`);
  resolve_result(JSON.parse(options.body));
  return { ok: true };
};
let timeout;
try {
  window.eval(await build_reader_test(process.argv[3]));
  const result = await Promise.race([
    finished,
    new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("Reader regression timed out")), 20000); }),
  ]);
  console.log(JSON.stringify(result, null, 2));
  if (result.error) process.exitCode = 1;
} finally {
  clearTimeout(timeout);
  await window.happyDOM.close();
  for (const channel of channels) { channel.port1.close(); channel.port2.close(); }
}
