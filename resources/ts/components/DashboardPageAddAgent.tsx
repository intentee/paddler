import React, {
  useCallback,
  useContext,
  useState,
  type MouseEvent,
} from "react";

import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { CodeEditor } from "./CodeEditor";
import { ModalWindow } from "./ModalWindow";

import {
  dashboardPageAddAgent,
  dashboardPageAddAgent__button,
  dashboardPageAddAgent__instructions,
} from "./DashboardPageAddAgent.module.css";

export function DashboardPageAddAgent() {
  const { managementAddr } = useContext(PaddlerConfigurationContext);
  const [isPreviewVisible, setIsPreviewVisible] = useState(false);

  const onClick = useCallback(
    function (evt: MouseEvent<HTMLButtonElement>) {
      evt.preventDefault();

      setIsPreviewVisible(true);
    },
    [setIsPreviewVisible],
  );

  const onClose = useCallback(
    function () {
      setIsPreviewVisible(false);
    },
    [setIsPreviewVisible],
  );

  return (
    <>
      <div className={dashboardPageAddAgent}>
        <button className={dashboardPageAddAgent__button} onClick={onClick}>
          How to add an agent?
        </button>
      </div>
      {isPreviewVisible && (
        <ModalWindow onClose={onClose} title="How to add an agent?">
          <div className={dashboardPageAddAgent__instructions}>
            <p>
              <strong>TL;DR</strong> For example, if you want to start an agent
              with 4 slots, name it "my-agent", you can run the following
              command on the agent's server:
            </p>
            <CodeEditor
              editable={false}
              value={`paddler agent --management-addr ${managementAddr} --name my-agent --slots 4`}
            />
          </div>
          <div className={dashboardPageAddAgent__instructions}>
            <p>
              Agents need to be able to reach the management service in the
              balancer (in this setup available at <code>{managementAddr}</code>
              ).
            </p>
            <p>
              Agent name is arbitrary. It is not used for any internal
              algorithm; it should only have a meaning to you, for example, to
              be able to more easily identify the agent's server on the list.
            </p>
            <p>
              Ideally, it is best not to start more than one agent on the same
              device. They should work just fine, but instead it is a much
              better idea to just give the agent more slots to work with.
            </p>
            <p>
              The number of slots is the number of concurrent requests that the
              agent can handle. You can do some benchmarking to determine how
              many you need. Start with a small number, like 4 or 8.
            </p>
          </div>
        </ModalWindow>
      )}
    </>
  );
}
