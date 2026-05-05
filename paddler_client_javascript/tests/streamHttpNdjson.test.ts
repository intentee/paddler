import test from "ava";
import { createServer, type RequestListener, type Server } from "node:http";
import { firstValueFrom, lastValueFrom, toArray } from "rxjs";
import { z } from "zod";

import { HttpError } from "../src/HttpError";
import { streamHttpNdjson } from "../src/streamHttpNdjson";

const Schema = z.object({ index: z.number() });

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
        url: new URL(`http://127.0.0.1:${address.port}/stream`),
      });
    });
  });
}

test("yields parsed messages from an NDJSON stream", async function (t) {
  const { server, url } = await listenOnce(function (_req, res) {
    res.writeHead(200, { "Content-Type": "application/x-ndjson" });
    res.write(`${JSON.stringify({ index: 0 })}\n`);
    res.write(`${JSON.stringify({ index: 1 })}\n`);
    res.write(`${JSON.stringify({ index: 2 })}\n`);
    res.end();
  });

  try {
    const messages = await lastValueFrom(
      streamHttpNdjson({
        url,
        body: {},
        signal: new AbortController().signal,
        schema: Schema,
      }).pipe(toArray()),
    );

    t.deepEqual(messages, [
      { index: 0 },
      { index: 1 },
      { index: 2 },
    ]);
  } finally {
    server.close();
  }
});

test("non-2xx response throws HttpError", async function (t) {
  const { server, url } = await listenOnce(function (_req, res) {
    res.writeHead(503);
    res.end();
  });

  try {
    await t.throwsAsync(
      async function () {
        await firstValueFrom(
          streamHttpNdjson({
            url,
            body: {},
            signal: new AbortController().signal,
            schema: Schema,
          }),
        );
      },
      { instanceOf: HttpError },
    );
  } finally {
    server.close();
  }
});
