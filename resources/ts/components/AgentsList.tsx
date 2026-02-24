import clsx from "clsx";
import React, { CSSProperties, useState } from "react";

import { type Agent } from "../schemas/Agent";

import {
  agentRow,
  agentRowError,
  agentUsage,
  agentUsage__progress,
  agentsTable,
  sortIndicator,
  sortIndicatorAsc,
  sortIndicatorDesc,
} from "./Dashboard.module.css";

function formatTimestamp(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

type SortColumn =
  | "name"
  | "model"
  | "issues"
  | "llamacppAddr"
  | "lastUpdate"
  | "idleSlots"
  | "processingSlots";

function getSortIndicator(
  sortConfig: { key: SortColumn; direction: "ascending" | "descending" },
  currentKey: SortColumn
): React.ReactNode {
  if (sortConfig.key !== currentKey) {
    return null;
  }
  const className = clsx(sortIndicator, sortConfig.direction === "ascending" ? sortIndicatorAsc : sortIndicatorDesc);
  return (
    <span className={className}>
      {sortConfig.direction === "ascending" ? "↑" : "↓"}
    </span>
  );
}

export function AgentsList({ agents }: { agents: Array<Agent> }) {
  const [sortConfig, setSortConfig] = useState<{
    key: SortColumn;
    direction: "ascending" | "descending";
  }>({ key: "name", direction: "ascending" });

  function sortAgents(agents: Array<Agent>): Array<Agent> {
    const sortableAgents = [...agents];
    sortableAgents.sort(function (a, b) {
      const { key, direction } = sortConfig;

      // Helper function to get comparison value based on column type
      function getValue(agent: Agent, key: SortColumn): string | number {
        switch (key) {
          case "name":
            return agent.status.agent_name || "";
          case "model":
            return agent.status.model || "";
          case "llamacppAddr":
            return agent.status.external_llamacpp_addr;
          case "lastUpdate":
            return agent.last_update.secs_since_epoch;
          case "idleSlots":
            return agent.status.slots_idle;
          case "processingSlots":
            return agent.status.slots_processing;
          default:
            return "";
        }
      }

      // Special handling for issues column
      if (key === "issues") {
        const hasIssuesA = a.status.error !== null;
        const hasIssuesB = b.status.error !== null;
        if (hasIssuesA !== hasIssuesB) {
          return direction === "ascending" ? (hasIssuesA ? 1 : -1) : (hasIssuesA ? -1 : 1);
        }
        const errorA = a.status.error || "";
        const errorB = b.status.error || "";
        if (errorA < errorB) return direction === "ascending" ? -1 : 1;
        if (errorA > errorB) return direction === "ascending" ? 1 : -1;
        return 0;
      }

      const valueA = getValue(a, key);
      const valueB = getValue(b, key);

      // Handle string comparison
      if (typeof valueA === "string" && typeof valueB === "string") {
        if (valueA < valueB) return direction === "ascending" ? -1 : 1;
        if (valueA > valueB) return direction === "ascending" ? 1 : -1;
        return 0;
      }

      // Handle numeric comparison
      if (typeof valueA === "number" && typeof valueB === "number") {
        if (valueA < valueB) return direction === "ascending" ? -1 : 1;
        if (valueA > valueB) return direction === "ascending" ? 1 : -1;
        return 0;
      }

      return 0;
    });
    return sortableAgents;
  }

  function requestSort(key: SortColumn) {
    let direction: "ascending" | "descending" = "ascending";
    if (sortConfig.key === key && sortConfig.direction === "ascending") {
      direction = "descending";
    }
    setSortConfig({ key, direction });
  }

  const sortedAgents = sortAgents(agents);

  return (
    <table className={agentsTable}>
      <thead>
        <tr>
          <th onClick={function () { requestSort("name"); }}>
            Name{getSortIndicator(sortConfig, "name")}
          </th>
          <th onClick={function () { requestSort("model"); }}>
            Model{getSortIndicator(sortConfig, "model")}
          </th>
          <th onClick={function () { requestSort("issues"); }}>
            Issues{getSortIndicator(sortConfig, "issues")}
          </th>
          <th onClick={function () { requestSort("llamacppAddr"); }}>
            Llama.cpp address{getSortIndicator(sortConfig, "llamacppAddr")}
          </th>
          <th onClick={function () { requestSort("lastUpdate"); }}>
            Last update{getSortIndicator(sortConfig, "lastUpdate")}
          </th>
          <th onClick={function () { requestSort("idleSlots"); }}>
            Idle slots{getSortIndicator(sortConfig, "idleSlots")}
          </th>
          <th onClick={function () { requestSort("processingSlots"); }}>
            Processing slots{getSortIndicator(sortConfig, "processingSlots")}
          </th>
        </tr>
      </thead>
      <tbody>
        {sortedAgents.map(function ({
          agent_id,
          last_update,
          quarantined_until,
          status,
        }: Agent) {
          const hasIssues =
            status.error ||
            true !== status.is_authorized ||
            true === status.is_connect_error ||
            true === status.is_request_error ||
            true === status.is_decode_error ||
            true === status.is_deserialize_error ||
            true === status.is_unexpected_response_status ||
            true !== status.is_slots_endpoint_enabled ||
            quarantined_until;

          return (
              <tr
                className={clsx(agentRow, hasIssues ? agentRowError : undefined)}
                key={agent_id}
              >
              <td>{status.agent_name}</td>
              <td>{status.model}</td>
              <td>
                {status.error && (
                  <>
                    <p>Agent reported an Error</p>
                    <p>{status.error}</p>
                  </>
                )}
                {false === status.is_authorized && (
                  <>
                    <p>Unauthorized</p>
                    <p>
                      Probably llama.cpp API key is either invalid or not
                      present. Pass it to the agent with
                      `--llamacpp-api-key=YOURKEY` flag.
                    </p>
                  </>
                )}
                {true == status.is_connect_error && (
                  <p>Llama.cpp server is unreachable. It is likely down.</p>
                )}
                {true == status.is_decode_error && (
                  <p>
                    Llama.cpp server returned an unexpected response. Are you
                    sure that the agent is configured to monitor llama.cpp and
                    is using the correct port?
                  </p>
                )}
                {true == status.is_deserialize_error && (
                  <p>Llama.cpp server response could not be deserialized.</p>
                )}
                {true == status.is_unexpected_response_status && (
                  <p>Llama.cpp server response status is unexpected.</p>
                )}
                {false === status.is_slots_endpoint_enabled && (
                  <>
                    <p>Slots endpoint is not enabled</p>
                    <p>
                      Probably llama.cpp server is running without the `--slots`
                      flag.
                    </p>
                  </>
                )}
                {quarantined_until && (
                  <p>
                    Quarantined until{" "}
                    {formatTimestamp(quarantined_until.secs_since_epoch)}
                  </p>
                )}
                {!hasIssues && <p>None</p>}
              </td>
              <td>
                <a href={`http://${status.external_llamacpp_addr}`}>
                  {status.external_llamacpp_addr}
                </a>
              </td>
              <td>{formatTimestamp(last_update.secs_since_epoch)}</td>
              <td>{status.slots_idle}</td>
              <td>{status.slots_processing}</td>
              <td
                className={agentUsage}
                style={
                  {
                    "--slots-usage": `${(status.slots_processing / (status.slots_idle + status.slots_processing)) * 100}%`,
                  } as CSSProperties
                }
              >
                <div className={agentUsage__progress}></div>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
