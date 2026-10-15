import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../../src/schemas/AgentsResponse";

function agent({ id, name }: { id: string; name: string | null }): unknown {
  return {
    id,
    name,
    slots_processing: 0,
    status: {
      desired_slots_total: 1,
      download_status: "NotDownloading",
      issues: [],
      model_path: null,
      runtime: "Idle",
      state_application_status: "Applied",
      uses_chat_template_override: false,
    },
  };
}

test("orders agents by name", function () {
  const { agents } = AgentsResponseSchema.parse({
    agents: [
      agent({ id: "second", name: "second" }),
      agent({ id: "first", name: "first" }),
    ],
  });

  deepStrictEqual(
    agents.map(function ({ name }) {
      return name;
    }),
    ["first", "second"],
  );
});

test("orders an unnamed agent by its id among the named ones", function () {
  const { agents } = AgentsResponseSchema.parse({
    agents: [
      agent({ id: "bravo-id", name: "bravo" }),
      agent({ id: "alpha-id", name: null }),
      agent({ id: "charlie-id", name: "charlie" }),
    ],
  });

  deepStrictEqual(
    agents.map(function ({ id }) {
      return id;
    }),
    ["alpha-id", "bravo-id", "charlie-id"],
  );
});
