import { readFile } from "node:fs/promises";
import { build } from "esbuild";
import path from "node:path";
import { fileURLToPath } from "node:url";
export async function build_agent_test(fixture_path) {
  if (!fixture_path) throw new Error("Pass the production Rust fixture JSON as the third argument.");
  const fixture = await readFile(fixture_path, "utf8");
  const dir = path.dirname(fileURLToPath(import.meta.url));
  const result = await build({
    entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "native-memory-only", setup(builder) {
      builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
      builder.onResolve({ filter: /^production-translation-fixture$/ }, () => ({ path: "fixture", namespace: "fixture" }));
      builder.onLoad({ filter: /.*/, namespace: "fixture" }, () => ({ contents: `export default ${fixture};`, loader: "js" }));
      builder.onResolve({ filter: /^react-i18next$/ }, () => ({ path: "i18n", namespace: "mock" }));
      builder.onLoad({ filter: /.*/, namespace: "mock" }, () => ({ contents: "export const useTranslation = () => ({ t: (key) => key });", loader: "js" }));
    } }],
  });
  return result.outputFiles[0].text;
}
