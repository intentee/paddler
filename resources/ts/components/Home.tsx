import React from "react";
import { Route, Router, Switch } from "wouter";

import { ChangeModelPage } from "./ChangeModelPage";
import { DashboardPage } from "./DashboardPage";
import { PromptContextProvider } from "./PromptContextProvider";
import { PromptImageContextProvider } from "./PromptImageContextProvider";
import { PromptPage } from "./PromptPage";
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
              <PromptImageContextProvider>
                <PromptThinkingContextProvider>
                  <PromptPage />
                </PromptThinkingContextProvider>
              </PromptImageContextProvider>
            </PromptContextProvider>
          </Route>
          <Route>404 :(</Route>
        </Switch>
      </WorkbenchLayout>
    </Router>
  );
}
