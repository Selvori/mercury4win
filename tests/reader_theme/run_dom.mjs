import { pathToFileURL } from "node:url";
import { MessageChannel } from "node:worker_threads";
import { build_theme_test } from "./bundle.mjs";
const { Window } = await import(pathToFileURL(process.argv[2]).href);
const bundle = await build_theme_test(process.argv[3]);
for (const initial of ["saved-auto", "saved-dark", "invalid"]) {
  const window = new Window({ url: "http://localhost/" });
  const channels = [];
  window.MessageChannel = class extends MessageChannel { constructor() { super(); channels.push(this); } };
  window.console = console;
  window.document.body.innerHTML = '<div id="app"></div>';
  window.initial_case = initial;
  const listener_set = new Set();
  const media = { matches: true, addEventListener: (_, callback) => listener_set.add(callback), removeEventListener: (_, callback) => listener_set.delete(callback) };
  window.matchMedia = () => media;
  window.theme_media = { count: () => listener_set.size, silently: (matches) => { media.matches = matches; }, change: (matches) => { media.matches = matches; for (const callback of listener_set) callback(); } };
  const saved = initial === "saved-auto" ? ["auto", "paper", "serif", "22"] : initial === "saved-dark" ? ["dark", "classic", "system", "16"] : ["unknown", "unknown", "unknown", "99"];
  for (const [index, key] of ["mercury_theme", "mercury_reader_style", "mercury_font_family", "mercury_font_size"].entries()) window.localStorage.setItem(key, saved[index]);
  let resolve_result;
  const finished = new Promise((resolve) => { resolve_result = resolve; });
  window.fetch = async (url, options) => { if (url !== "/result") throw new Error(`Unexpected network: ${url}`); resolve_result(JSON.parse(options.body)); return { ok: true }; };
  let timeout;
  try {
    window.eval(bundle);
    const result = await Promise.race([finished, new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error("Theme regression timeout")), 20000); })]);
    console.log(JSON.stringify({ initial, ...result }, null, 2));
    if (result.error) process.exitCode = 1;
  } finally { clearTimeout(timeout); await window.happyDOM.close(); for (const c of channels) { c.port1.close(); c.port2.close(); } }
}
