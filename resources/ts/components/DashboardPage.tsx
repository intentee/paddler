import React, { useContext } from "react";

import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { AgentListStream } from "./AgentListStream";
import { BufferedRequestsStream } from "./BufferedRequestsStream";
import { DashboardPageAddAgent } from "./DashboardPageAddAgent";

import {
  dashboardPage,
  dashboardPage__addrOverview,
  dashboardPage__blocks,
  dashboardPage__compatibilityServiceAddr,
  dashboardPage__genericAddr,
  dashboardPage__inferenceAddr,
  dashboardPage__inferenceAddrList,
  dashboardPage__inferenceServiceAddr,
  dashboardPage__managementAddr,
  dashboardPage__serviceBlock,
  dashboardPage__statsdAddr,
} from "./DashboardPage.module.css";

export function DashboardPage() {
  const {
    compatOpenAIAddr,
    inferenceAddr,
    managementAddr,
    statsdAddr,
    statsdPrefix,
    statsdReportingIntervalMillis,
  } = useContext(PaddlerConfigurationContext);

  return (
    <div className={dashboardPage}>
      <div className={dashboardPage__blocks}>
        <div className={dashboardPage__serviceBlock}></div>
        <div className={dashboardPage__serviceBlock}></div>
        <div className={dashboardPage__serviceBlock}>
          <div
            className={`${dashboardPage__genericAddr} ${dashboardPage__managementAddr}`}
          >
            <p>Management addr: {managementAddr}</p>
          </div>
        </div>
      </div>
      <div className={dashboardPage__blocks}>
        <div className={dashboardPage__serviceBlock}>
          <div className={dashboardPage__addrOverview}>
            <div className={dashboardPage__inferenceAddrList}>
              <div
                className={`${dashboardPage__genericAddr} ${dashboardPage__inferenceAddr} ${dashboardPage__inferenceServiceAddr}`}
              >
                <p>Inference addr:</p>
                <p>{inferenceAddr}</p>
              </div>
              {compatOpenAIAddr && (
                <div
                  className={`${dashboardPage__genericAddr} ${dashboardPage__inferenceAddr} ${dashboardPage__compatibilityServiceAddr}`}
                >
                  <p>
                    OpenAI <abbr title="compatibility service">compat</abbr>{" "}
                    addr:
                  </p>
                  <p>{compatOpenAIAddr}</p>
                </div>
              )}
            </div>
            {statsdAddr && (
              <div
                className={`${dashboardPage__genericAddr} ${dashboardPage__statsdAddr}`}
              >
                <p>StatsD addr: {statsdAddr}</p>
                <p>StatsD prefix: {statsdPrefix}</p>
                <p>
                  StatsD reporting interval:{" "}
                  {statsdReportingIntervalMillis / 1000}s
                </p>
              </div>
            )}
          </div>
        </div>
        <div className={dashboardPage__serviceBlock}>
          <BufferedRequestsStream />
        </div>
        <div className={dashboardPage__serviceBlock}>
          <AgentListStream />
          <DashboardPageAddAgent />
        </div>
      </div>
    </div>
  );
}
