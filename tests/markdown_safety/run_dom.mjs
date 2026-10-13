// jsdom is a temporary test dependency, never an app runtime dependency.
import { pathToFileURL } from "node:url";
import { MessageChannel } from "node:worker_threads";
import { build_markdown_test } from "./bundle.mjs";

if (!process.argv[2]) throw new Error("Usage: node tests/markdown_safety/run_dom.mjs <jsdom/lib/api.js>");
const { JSDOM } = await import(pathToFileURL(process.argv[2]).href);
const { window } = new JSDOM('<pre id="result">Running...</pre><div id="app"></div>', {
  url: "http://localhost/", runScripts: "outside-only",
});
const channels = [];
window.MessageChannel = class extends MessageChannel {
  constructor() { super(); channels.push(this); }
};
window.console = console;
let resolve_result;
const finished = new Promise((resolve) => { resolve_result = resolve; });
window.fetch = async (url, options) => {
  if (url !== "/result") throw new Error(`Unexpected network call: ${url}`);
  resolve_result(JSON.parse(options.body));
  return { ok: true };
};
let timeout;
try {
  window.eval(await build_markdown_test());
  const result = await Promise.race([
    finished,
    new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("Markdown regression timed out")), 20000); }),
  ]);
  console.log(JSON.stringify(result, null, 2));
  if (result.error) process.exitCode = 1;
} finally {
  clearTimeout(timeout);
  window.close();
  for (const channel of channels) { channel.port1.close(); channel.port2.close(); }
}
