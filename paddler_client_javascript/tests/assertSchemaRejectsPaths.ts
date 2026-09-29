import { deepStrictEqual, ok, throws } from "node:assert/strict";
import { z } from "zod";

export function assertSchemaRejectsPaths({
  expectedPaths,
  input,
  schema,
}: {
  expectedPaths: PropertyKey[][];
  input: unknown;
  schema: z.ZodType;
}): void {
  throws(
    function () {
      schema.parse(input);
    },
    function (error: unknown) {
      ok(error instanceof z.ZodError);
      deepStrictEqual(
        error.issues.map(function ({ path }) {
          return path;
        }),
        expectedPaths,
      );

      return true;
    },
  );
}
