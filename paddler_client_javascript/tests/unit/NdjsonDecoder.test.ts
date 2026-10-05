import { deepStrictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { JsonError } from "../../src/JsonError";
import { NdjsonDecoder } from "../../src/NdjsonDecoder";

test("joins a value split across pushed chunks", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('{"index":'), []);
  deepStrictEqual(ndjsonDecoder.push('0}\n{"index":1}\n'), [
    { index: 0 },
    { index: 1 },
  ]);
});

test("skips blank lines", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('\n  \r\n{"index":0}\n'), [{ index: 0 }]);
});

test("decodes a trailing value without a newline when finished", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  deepStrictEqual(ndjsonDecoder.push('{"index":0}'), []);
  deepStrictEqual(ndjsonDecoder.finish(), [{ index: 0 }]);
});

test("rejects a line that is not JSON with the raw line attached", function () {
  const ndjsonDecoder = new NdjsonDecoder();

  throws(
    function () {
      ndjsonDecoder.push("not json\n");
    },
    function (error: unknown) {
      return error instanceof JsonError && error.raw === "not json";
    },
  );
});
