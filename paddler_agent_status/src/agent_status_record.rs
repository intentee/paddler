use std::collections::BTreeSet;
use std::mem::replace;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::desired_state_version::DesiredStateVersion;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

use crate::agent_issue_fix::AgentIssueFix;
use crate::serving_report::ServingReport;

#[derive(Default)]
pub struct AgentStatusRecord {
    desired_state_issues: BTreeSet<AgentIssue>,
    desired_state_version: DesiredStateVersion,
    download_status: ModelDownloadStatus,
    model_path: Option<String>,
    pipeline_issues: BTreeSet<AgentIssue>,
    runtime: AgentRuntimeStatus,
    state_application_status: AgentStateApplicationStatus,
    uses_chat_template_override: bool,
    version: u64,
}

impl AgentStatusRecord {
    pub fn acknowledge_desired_state_version(
        &mut self,
        desired_state_version: DesiredStateVersion,
    ) -> bool {
        replace(&mut self.desired_state_version, desired_state_version) != desired_state_version
    }

    pub fn begin_desired_state(&mut self) {
        self.desired_state_issues.clear();
        self.download_status = ModelDownloadStatus::NotDownloading;
    }

    pub fn begin_pipeline(&mut self) {
        self.pipeline_issues.clear();
        self.state_application_status = AgentStateApplicationStatus::Fresh;
    }

    pub const fn count_change(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    pub fn fail_download(&mut self, issue: AgentIssue) {
        self.download_status = ModelDownloadStatus::NotDownloading;
        self.desired_state_issues.insert(issue);
    }

    pub fn fix_issues(&mut self, fix: &AgentIssueFix) -> bool {
        let issue_count_before = self.issues().count();

        self.desired_state_issues
            .retain(|issue| !fix.can_fix(issue));
        self.pipeline_issues.retain(|issue| !fix.can_fix(issue));

        self.issues().count() < issue_count_before
    }

    #[must_use]
    pub fn has_issue(&self, issue: &AgentIssue) -> bool {
        self.desired_state_issues.contains(issue) || self.pipeline_issues.contains(issue)
    }

    pub fn issues(&self) -> impl Iterator<Item = &AgentIssue> {
        self.desired_state_issues
            .iter()
            .chain(self.pipeline_issues.iter())
    }

    pub fn register_desired_state_issue(&mut self, issue: AgentIssue) -> bool {
        self.desired_state_issues.insert(issue)
    }

    pub fn register_pipeline_issue(&mut self, issue: AgentIssue) -> bool {
        self.pipeline_issues.insert(issue)
    }

    pub const fn retry_pipeline(&mut self) {
        self.state_application_status = match self.state_application_status {
            AgentStateApplicationStatus::AttemptedAndRetrying
            | AgentStateApplicationStatus::Stuck => AgentStateApplicationStatus::Stuck,
            AgentStateApplicationStatus::Applied | AgentStateApplicationStatus::Fresh => {
                AgentStateApplicationStatus::AttemptedAndRetrying
            }
        };
    }

    pub fn serve_nothing(&mut self, uses_chat_template_override: bool) {
        self.model_path = None;
        self.runtime = AgentRuntimeStatus::Idle;
        self.state_application_status = AgentStateApplicationStatus::Applied;
        self.uses_chat_template_override = uses_chat_template_override;
    }

    pub fn set_download_status(&mut self, download_status: ModelDownloadStatus) {
        self.download_status = download_status;
    }

    pub const fn set_downloaded_bytes(&mut self, downloaded_bytes: u64) {
        self.download_status.set_downloaded_bytes(downloaded_bytes);
    }

    #[must_use]
    pub fn snapshot(&self, desired_slots_total: u16) -> SlotAggregatedStatusSnapshot {
        SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                desired_slots_total,
                desired_state_version: self.desired_state_version,
                download_status: self.download_status.clone(),
                issues: self.issues().cloned().collect(),
                model_path: self.model_path.clone(),
                runtime: self.runtime,
                state_application_status: self.state_application_status,
                uses_chat_template_override: self.uses_chat_template_override,
            },
            version: self.version,
        }
    }

    pub fn start_serving(
        &mut self,
        ServingReport {
            capabilities,
            model_path,
            uses_chat_template_override,
        }: ServingReport,
        desired_slots_total: u16,
    ) {
        for slot_index in 0..u32::from(desired_slots_total) {
            self.fix_issues(&AgentIssueFix::SlotStarted(slot_index));
        }

        self.model_path = Some(model_path);
        self.runtime = AgentRuntimeStatus::Serving {
            capabilities,
            slots_total: u64::from(desired_slots_total),
        };
        self.state_application_status = AgentStateApplicationStatus::Applied;
        self.uses_chat_template_override = uses_chat_template_override;
    }

    #[must_use]
    pub const fn state_application_status(&self) -> AgentStateApplicationStatus {
        self.state_application_status
    }

    pub fn withdraw_serving(&mut self) {
        self.model_path = None;
        self.runtime = AgentRuntimeStatus::Idle;
    }
}
