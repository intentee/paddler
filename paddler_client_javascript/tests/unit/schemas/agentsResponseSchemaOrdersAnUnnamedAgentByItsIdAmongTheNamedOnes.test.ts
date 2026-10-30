import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../../src/schemas/AgentsResponse";
import { idleAgent } from "./idleAgent";

test("AgentsResponseSchema orders an unnamed agent by its id among the named ones", function () {
  const { agents } = AgentsResponseSchema.parse({
    agents: [
      idleAgent({ id: "bravo-id", name: "bravo" }),
      idleAgent({ id: "alpha-id", name: null }),
      idleAgent({ id: "charlie-id", name: "charlie" }),
    ],
  });

  deepStrictEqual(
    agents.map(function ({ id }) {
      return id;
    }),
    ["alpha-id", "bravo-id", "charlie-id"],
  );
});
