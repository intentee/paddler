import React from "react";
import { Route, Router, Switch } from "wouter";

import { ChangeModelPage } from "./ChangeModelPage";
import { DashboardPage } from "./DashboardPage";
import { PromptContextProvider } from "./PromptContextProvider";
import { PromptMediaContextProvider } from "./PromptMediaContextProvider";
import { PromptPage } from "./PromptPage";
import { PromptSystemMessageContextProvider } from "./PromptSystemMessageContextProvider";
import { PromptThinkingContextProvider } from "./PromptThinkingContextProvider";
import { WorkbenchLayout } from "./WorkbenchLayout";

export function Home() {
  return (
    <Router>
      <WorkbenchLayout>
        <Switch>
          <Route path="/">
            <DashboardPage />
          </Route>
          <Route path="/model">
            <ChangeModelPage />
          </Route>
          <Route path="/prompt">
            <PromptContextProvider>
              <PromptMediaContextProvider>
                <PromptThinkingContextProvider>
                  <PromptSystemMessageContextProvider>
                    <PromptPage />
                  </PromptSystemMessageContextProvider>
                </PromptThinkingContextProvider>
              </PromptMediaContextProvider>
            </PromptContextProvider>
          </Route>
          <Route>404 :(</Route>
        </Switch>
      </WorkbenchLayout>
    </Router>
  );
}
