export function decisionResponse(decisionResult: unknown): unknown {
  return {
    Response: {
      generated_by: "agent-1",
      request_id: "req-decision",
      response: { Decision: decisionResult },
    },
  };
}
