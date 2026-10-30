import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { NdjsonDecoder } from "../../src/NdjsonDecoder";

test("NdjsonDecoder decodes a trailing value without a newline when finished", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('{"index":0}'), []);
  deepStrictEqual(ndjsonDecoder.finish(), [{ index: 0 }]);
});
