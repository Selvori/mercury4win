import { createServer } from "node:http";
import { build_markdown_test } from "./bundle.mjs";

const bundle = await build_markdown_test();
const server = createServer(async (request, response) => {
  if (request.url === "/bundle.js") {
    response.setHeader("Content-Type", "text/javascript; charset=utf-8");
    response.end(bundle);
  } else if (request.url === "/result" && request.method === "POST") {
    let body = "";
    for await (const chunk of request) body += chunk;
    console.log(JSON.stringify(JSON.parse(body), null, 2));
    response.end("received");
  } else {
    response.setHeader("Content-Type", "text/html; charset=utf-8");
    response.end('<!doctype html><meta charset="utf-8"><title>M4W-016 Markdown safety regression</title><pre id="result">Running...</pre><div id="app"></div><script type="module" src="/bundle.js"></script>');
  }
});
server.listen(0, "127.0.0.1", () => console.log(`Markdown safety regression: http://127.0.0.1:${server.address().port}`));
