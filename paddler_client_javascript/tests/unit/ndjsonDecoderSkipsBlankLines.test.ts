import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { NdjsonDecoder } from "../../src/NdjsonDecoder";

test("NdjsonDecoder skips blank lines", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('\n  \r\n{"index":0}\n'), [{ index: 0 }]);
});
