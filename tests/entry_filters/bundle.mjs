import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import path from "node:path";

export async function build_entry_test() {
  const directory = path.dirname(fileURLToPath(import.meta.url));
  const bundle = await build({
    entryPoints: [path.join(directory, "browser.tsx")], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic",
    tsconfig: path.join(directory, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "memory-transport", setup(builder) {
      // Keep the production IPC wrapper; intercept only the native transport.
      builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(directory, "mock_transport.ts") }));
      builder.onResolve({ filter: /^react-i18next$/ }, () => ({ path: "i18n", namespace: "mock" }));
      builder.onLoad({ filter: /.*/, namespace: "mock" }, () => ({
        contents: "export const useTranslation = () => ({ t: (key, params) => params?.count != null ? key + ':' + params.count : key });", loader: "js",
      }));
    } }],
  });
  return bundle.outputFiles[0].text;
}
