export function generatedToken(generatedTokenResult: unknown): unknown {
  return {
    Response: {
      generated_by: null,
      request_id: "req-1",
      response: { GeneratedToken: generatedTokenResult },
    },
  };
}
