import React, {
  ReactNode,
  useCallback,
  useState,
  type MouseEvent,
} from "react";

import { ExplainButtonTooltip } from "./ExplainButtonTooltip";

export function ExplainButton({
  children,
  inferenceAddr,
  prompt,
  systemPrompt,
}: {
  children: ReactNode;
  inferenceAddr: string;
  prompt: string;
  systemPrompt: string;
}) {
  const [isTooltipOpen, setIsTooltipOpen] = useState(false);

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setIsTooltipOpen(true);
    },
    [setIsTooltipOpen],
  );

  return (
    <div>
      <button onClick={onClick}>{children}</button>
      {isTooltipOpen && (
        <ExplainButtonTooltip
          inferenceAddr={inferenceAddr}
          prompt={prompt}
          systemPrompt={systemPrompt}
        />
      )}
    </div>
  );
}
