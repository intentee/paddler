import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../../src/schemas/AgentsResponse";

function agentNamed(name: string): unknown {
  return {
    desired_slots_total: 1,
    download_current: 0,
    download_filename: null,
    download_indeterminate: false,
    download_total: 0,
    id: name,
    issues: [],
    model_path: null,
    name,
    slots_processing: 0,
    slots_total: 0,
    state_application_status: "Applied",
    uses_chat_template_override: false,
  };
}

test("orders agents by name", function () {
  const { agents } = AgentsResponseSchema.parse({
    agents: [agentNamed("second"), agentNamed("first")],
  });

  deepStrictEqual(
    agents.map(function ({ name }) {
      return name;
    }),
    ["first", "second"],
  );
});
