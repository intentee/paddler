import test from "ava";
import { createServer, type RequestListener, type Server } from "node:http";
import { z } from "zod";

import { fetchJson } from "../src/fetchJson";
import { HttpError } from "../src/HttpError";

const Schema = z.object({ ok: z.boolean(), count: z.number() });

function listenOnce(handler: RequestListener): Promise<{
  server: Server;
  url: URL;
}> {
  return new Promise(function (resolve) {
    const server = createServer(handler);

    server.listen(0, "127.0.0.1", function () {
      const address = server.address();

      if (typeof address !== "object" || address === null) {
        throw new Error("server did not bind");
      }

      resolve({
        server,
        url: new URL(`http://127.0.0.1:${address.port}/json`),
      });
    });
  });
}

test("parses JSON body against the schema", async function (t) {
  const { server, url } = await listenOnce(function (_req, res) {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ ok: true, count: 7 }));
  });

  try {
    const result = await fetchJson({
      url,
      signal: new AbortController().signal,
      schema: Schema,
    });

    t.deepEqual(result, { ok: true, count: 7 });
  } finally {
    server.close();
  }
});

test("non-2xx status throws HttpError", async function (t) {
  const { server, url } = await listenOnce(function (_req, res) {
    res.writeHead(404);
    res.end();
  });

  try {
    await t.throwsAsync(
      async function () {
        return fetchJson({
          url,
          signal: new AbortController().signal,
          schema: Schema,
        });
      },
      { instanceOf: HttpError },
    );
  } finally {
    server.close();
  }
});
