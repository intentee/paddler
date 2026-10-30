import { throws } from "node:assert/strict";
import { test } from "node:test";

import { JsonError } from "../../src/JsonError";
import { NdjsonDecoder } from "../../src/NdjsonDecoder";

test("NdjsonDecoder rejects a line that is not JSON with the raw line attached", function () {
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
