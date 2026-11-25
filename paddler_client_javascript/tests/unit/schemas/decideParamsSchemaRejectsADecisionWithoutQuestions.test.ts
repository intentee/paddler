import { test } from "node:test";

import { DecideParamsSchema } from "../../../src/schemas/DecideParams";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";

test("DecideParamsSchema rejects a decision without questions", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["questions"]],
    input: { questions: {}, state: { ticket: "refund" } },
    schema: DecideParamsSchema,
  });
});
