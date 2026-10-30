import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../../src/schemas/AgentsResponse";
import { idleAgent } from "./idleAgent";

test("AgentsResponseSchema orders agents by name", function () {
  const { agents } = AgentsResponseSchema.parse({
    agents: [
      idleAgent({ id: "second", name: "second" }),
      idleAgent({ id: "first", name: "first" }),
    ],
  });

  deepStrictEqual(
    agents.map(function ({ name }) {
      return name;
    }),
    ["first", "second"],
  );
});
