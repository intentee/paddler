import React from "react";

export function AgentIssueRepeatedServingFailure({
  failingPart,
}: {
  failingPart: string;
}) {
  return (
    <p>
      The agent does not serve the model. It keeps retrying every few seconds,
      but the same {failingPart} will keep failing the same way.
    </p>
  );
}
