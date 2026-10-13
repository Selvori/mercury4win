import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import path from "node:path";

export async function build_markdown_test() {
  const directory = path.dirname(fileURLToPath(import.meta.url));
  const bundle = await build({
    entryPoints: [path.join(directory, "browser.tsx")], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic", loader: { ".md": "text" },
    tsconfig: path.join(directory, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "memory-ipc", setup(builder) {
      builder.onResolve({ filter: /^@\/lib\/tauri_bindings$/ }, () => ({ path: path.join(directory, "../reader_ownership/mock_bindings.ts") }));
      builder.onResolve({ filter: /^react-i18next$/ }, () => ({ path: "i18n", namespace: "mock" }));
      builder.onLoad({ filter: /.*/, namespace: "mock" }, () => ({ contents: "export const useTranslation = () => ({ t: (key) => key });", loader: "js" }));
    } }],
  });
  return bundle.outputFiles[0].text;
}
