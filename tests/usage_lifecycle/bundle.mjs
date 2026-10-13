import { build } from "esbuild";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
export async function build_agent_test(fixture_path) {
  const fixture = fixture_path ? JSON.parse(await readFile(fixture_path, "utf8")) : null;
  const dir = path.dirname(fileURLToPath(import.meta.url));
  const result = await build({
    entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "native-memory-only", setup(builder) {
      if (fixture) {
        builder.onResolve({ filter: /^\.\/fixture$/ }, () => ({ path: "fixture", namespace: "production-fixture" }));
        builder.onLoad({ filter: /.*/, namespace: "production-fixture" }, () => ({ contents: `export const fixture = ${JSON.stringify(fixture)};`, loader: "js" }));
      }
      builder.onResolve({ filter: /^@tauri-apps\/api\/core$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
      builder.onResolve({ filter: /^react-i18next$/ }, () => ({ path: "i18n", namespace: "mock" }));
      builder.onLoad({ filter: /.*/, namespace: "mock" }, () => ({ contents: `export { useTranslation } from ${JSON.stringify(path.join(dir, "i18n.ts"))};`, loader: "js", resolveDir: dir }));
    } }],
  });
  return result.outputFiles[0].text;
}
