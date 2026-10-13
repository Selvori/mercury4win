import { build } from "esbuild";
import path from "node:path";
import { fileURLToPath } from "node:url";
export async function build_agent_test() {
  const dir = path.dirname(fileURLToPath(import.meta.url));
  const result = await build({ entryPoints: [path.join(dir, "browser.tsx")], bundle: true, write: false,
    platform: "browser", format: "iife", jsx: "automatic", tsconfig: path.join(dir, "../../tsconfig.app.json"),
    define: { "process.env.NODE_ENV": '"development"' },
    plugins: [{ name: "native-memory-only", setup(builder) {
      builder.onResolve({ filter: /^@tauri-apps\/api\/core$|^@tauri-apps\/plugin-dialog$/ }, () => ({ path: path.join(dir, "mock_transport.ts") }));
    } }],
  });
  return result.outputFiles[0].text;
}
