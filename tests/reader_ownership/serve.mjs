// node tests/reader_ownership/serve.mjs
// Open the printed loopback URL in a browser. Results also print in this shell.
import { createServer } from "node:http";
import { build_reader_test } from "./bundle.mjs";

const bundle = await build_reader_test();
const server = createServer(async (request, response) => {
  if (request.url === "/bundle.js") {
    response.setHeader("Content-Type", "text/javascript; charset=utf-8");
    response.end(bundle);
  } else if (request.url === "/result" && request.method === "POST") {
    let body = "";
    for await (const chunk of request) body += chunk;
    const result = JSON.parse(body);
    console.log(JSON.stringify(result, null, 2));
    response.end("received");
  } else {
    response.setHeader("Content-Type", "text/html; charset=utf-8");
    response.end('<!doctype html><meta charset="utf-8"><title>M4W-001 isolated React regression</title><pre id="result">Running...</pre><div id="app"></div><script type="module" src="/bundle.js"></script>');
  }
});
server.listen(0, "127.0.0.1", () => console.log(`Reader ownership regression: http://127.0.0.1:${server.address().port}`));
