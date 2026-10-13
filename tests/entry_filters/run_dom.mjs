import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { MessageChannel } from "node:worker_threads";
import { build_entry_test } from "./bundle.mjs";

if (!process.argv[2]) throw new Error("Pass separately installed happy-dom/lib/index.js");
const { Window } = await import(pathToFileURL(process.argv[2]).href);
const window = new Window({ url: "http://localhost/" });
const channels = [];
window.MessageChannel = class extends MessageChannel {
  constructor() { super(); channels.push(this); }
};
window.console = console;
window.detail_wire = JSON.parse(await readFile(process.argv[3], "utf8"));
window.document.body.innerHTML = '<div id="app"></div>';
let resolve_result;
const finished = new Promise((resolve) => { resolve_result = resolve; });
window.fetch = async (url, options) => {
  if (url !== "/result") throw new Error(`Unexpected network: ${url}`);
  resolve_result(JSON.parse(options.body));
  return { ok: true };
};
let timeout;
try {
  window.eval(await build_entry_test());
  const result = await Promise.race([finished, new Promise((_, reject) => {
    timeout = setTimeout(() => reject(new Error("Pagination regression timed out")), 30000);
  })]);
  console.log(JSON.stringify(result, null, 2));
  if (result.error) process.exitCode = 1;
} finally {
  clearTimeout(timeout);
  await window.happyDOM.close();
  for (const channel of channels) { channel.port1.close(); channel.port2.close(); }
}
