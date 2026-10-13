import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import path from "node:path";

export async function build_reader_test(entry_point = "browser.tsx") {
  const directory = path.dirname(fileURLToPath(import.meta.url));
  const bundle = await build({
    entryPoints: [path.resolve(directory, entry_point)], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic",
    tsconfig: path.join(directory, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "memory-ipc", setup(builder) {
      builder.onResolve({ filter: /^@\/lib\/tauri_bindings$/ }, () => ({ path: path.join(directory, "mock_bindings.ts") }));
      builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: "channel", namespace: "mock" }));
      builder.onResolve({ filter: /^react-i18next$/ }, () => ({ path: "i18n", namespace: "mock" }));
      builder.onResolve({ filter: /^\.\/font_settings_dialog$/ }, () => ({ path: "fonts", namespace: "mock" }));
      builder.onLoad({ filter: /.*/, namespace: "mock" }, ({ path: name }) => ({ contents: {
        channel: "export class Channel { onmessage = () => {}; } export function invoke() { throw new Error('Unexpected real IPC in isolated test'); }",
        i18n: "export const useTranslation = () => ({ t: (key) => key });",
        fonts: "export const FontSettingsDialog = () => null;",
      }[name], loader: "js" }));
    } }],
  });
  return bundle.outputFiles[0].text;
}
