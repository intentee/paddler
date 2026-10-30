import { throws } from "node:assert/strict";
import { test } from "node:test";

import { AgentSchema } from "../../../src/schemas/Agent";
import { idleAgent } from "./idleAgent";

test("AgentSchema rejects an unknown state_application_status", function () {
  const agent = idleAgent({ id: "agent-x", name: null });

  throws(function () {
    AgentSchema.parse({
      ...agent,
      status: { ...agent.status, state_application_status: "Unknown" },
    });
  });
});
