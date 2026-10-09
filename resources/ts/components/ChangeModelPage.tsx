import React, { useContext } from "react";

import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useBalancerDesiredState } from "../hooks/useBalancerDesiredState";
import { matchFetchJsonState } from "../matchFetchJsonState";
import { BalancerDesiredStateContextProvider } from "./BalancerDesiredStateContextProvider";
import { ChangeModelForm } from "./ChangeModelForm";
import { ChatTemplateContextProvider } from "./ChatTemplateContextProvider";
import { FloatingStatus } from "./FloatingStatus";

export function ChangeModelPage() {
  const { managementAddr } = useContext(PaddlerConfigurationContext);
  const loadingState = useBalancerDesiredState({ managementAddr });

  return matchFetchJsonState(loadingState, {
    empty() {
      return (
        <FloatingStatus>Unable to pick the desired state source</FloatingStatus>
      );
    },
    error({ error }) {
      return (
        <FloatingStatus>Error loading desired state: {error}</FloatingStatus>
      );
    },
    loading() {
      return <FloatingStatus>Loading desired state...</FloatingStatus>;
    },
    ok({ response: balancerDesiredState }) {
      return (
        <BalancerDesiredStateContextProvider
          defaultBalancerDesiredState={balancerDesiredState}
        >
          <ChatTemplateContextProvider>
            <ChangeModelForm />
          </ChatTemplateContextProvider>
        </BalancerDesiredStateContextProvider>
      );
    },
  });
}
