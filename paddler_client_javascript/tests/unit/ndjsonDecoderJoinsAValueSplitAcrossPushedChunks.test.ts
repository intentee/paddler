import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { NdjsonDecoder } from "../../src/NdjsonDecoder";

test("NdjsonDecoder joins a value split across pushed chunks", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('{"index":'), []);
  deepStrictEqual(ndjsonDecoder.push('0}\n{"index":1}\n'), [
    { index: 0 },
    { index: 1 },
  ]);
});
