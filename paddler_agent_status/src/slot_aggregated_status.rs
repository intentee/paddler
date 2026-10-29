use std::collections::BTreeSet;
use std::num::NonZeroU16;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;

use dashmap::DashSet;
use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::sync::watch;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::agent_issue_fix::AgentIssueFix;
use crate::dispenses_slots::DispensesSlots;
use crate::serving_report::ServingReport;

pub struct SlotAggregatedStatus {
    pub desired_slots_total: u16,
    desired_state_issues: DashSet<AgentIssue>,
    download_status: RwLock<ModelDownloadStatus>,
    model_path: RwLock<Option<String>>,
    pipeline_issues: DashSet<AgentIssue>,
    runtime: RwLock<AgentRuntimeStatus>,
    serving_issues: RwLock<BTreeSet<AgentIssue>>,
    slots_processing: RwLock<u64>,
    slots_released: Notify,
    state_application_status: RwLock<AgentStateApplicationStatus>,
    update_tx: watch::Sender<()>,
    uses_chat_template_override: AtomicValue<AtomicBool>,
    version: AtomicValue<AtomicU64>,
}

impl SlotAggregatedStatus {
    #[must_use]
    pub fn new(desired_slots_total: u16) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            desired_slots_total,
            desired_state_issues: DashSet::new(),
            download_status: RwLock::new(ModelDownloadStatus::NotDownloading),
            model_path: RwLock::new(None),
            pipeline_issues: DashSet::new(),
            runtime: RwLock::new(AgentRuntimeStatus::Idle),
            serving_issues: RwLock::new(BTreeSet::new()),
            state_application_status: RwLock::new(AgentStateApplicationStatus::Fresh),
            slots_processing: RwLock::new(0),
            slots_released: Notify::new(),
            update_tx,
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
            version: AtomicValue::<AtomicU64>::new(0),
        }
    }

    fn announce_change(&self) {
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn get_state_application_status(&self) -> AgentStateApplicationStatus {
        *self.state_application_status.read()
    }

    pub fn has_issue(&self, issue: &AgentIssue) -> bool {
        self.desired_state_issues.contains(issue)
            || self.pipeline_issues.contains(issue)
            || self.serving_issues.read().contains(issue)
    }

    pub fn has_issue_like<TFunction>(&self, issue_like: TFunction) -> bool
    where
        TFunction: Fn(&AgentIssue) -> bool,
    {
        self.desired_state_issues
            .iter()
            .chain(self.pipeline_issues.iter())
            .any(|ref_multi| issue_like(ref_multi.key()))
            || self.serving_issues.read().iter().any(issue_like)
    }

    pub fn add_downloaded_bytes(&self, added_bytes: u64) {
        self.download_status
            .write()
            .add_downloaded_bytes(added_bytes);
        self.announce_change();
    }

    pub fn set_downloaded_bytes(&self, downloaded_bytes: u64) {
        self.download_status
            .write()
            .set_downloaded_bytes(downloaded_bytes);
        self.announce_change();
    }

    pub fn begin_desired_state(&self) {
        self.desired_state_issues.clear();
        *self.download_status.write() = ModelDownloadStatus::NotDownloading;
        self.announce_change();
    }

    pub fn begin_pipeline(&self) {
        self.pipeline_issues.clear();
        *self.state_application_status.write() = AgentStateApplicationStatus::Fresh;
        self.announce_change();
    }

    pub fn retry_pipeline(&self) {
        {
            let mut state_application_status = self.state_application_status.write();

            *state_application_status = match *state_application_status {
                AgentStateApplicationStatus::AttemptedAndRetrying
                | AgentStateApplicationStatus::Stuck => AgentStateApplicationStatus::Stuck,
                AgentStateApplicationStatus::Applied | AgentStateApplicationStatus::Fresh => {
                    AgentStateApplicationStatus::AttemptedAndRetrying
                }
            };
        }

        self.announce_change();
    }

    pub fn fail_download(&self, issue: AgentIssue) {
        *self.download_status.write() = ModelDownloadStatus::NotDownloading;
        self.desired_state_issues.insert(issue);
        self.announce_change();
    }

    pub fn register_desired_state_issue(&self, issue: AgentIssue) {
        if self.desired_state_issues.insert(issue) {
            self.announce_change();
        }
    }

    pub fn register_pipeline_issue(&self, issue: AgentIssue) {
        if self.pipeline_issues.insert(issue) {
            self.announce_change();
        }
    }

    pub fn register_fix(&self, fix: &AgentIssueFix) {
        let issues_before = self.desired_state_issues.len() + self.pipeline_issues.len();

        self.desired_state_issues
            .retain(|issue| !fix.can_fix(issue));
        self.pipeline_issues.retain(|issue| !fix.can_fix(issue));

        if self.desired_state_issues.len() + self.pipeline_issues.len() < issues_before {
            self.announce_change();
        }
    }

    pub fn serve_nothing(&self, uses_chat_template_override: bool) {
        *self.model_path.write() = None;
        *self.runtime.write() = AgentRuntimeStatus::Idle;
        self.serving_issues.write().clear();
        *self.state_application_status.write() = AgentStateApplicationStatus::Applied;
        self.uses_chat_template_override
            .set(uses_chat_template_override);
        self.announce_change();
    }

    pub fn withdraw_serving(&self) {
        *self.model_path.write() = None;
        *self.runtime.write() = AgentRuntimeStatus::Idle;
        self.serving_issues.write().clear();
        self.announce_change();
    }

    pub fn set_download_status(&self, download_status: ModelDownloadStatus) {
        *self.download_status.write() = download_status;
        self.announce_change();
    }

    pub fn start_serving(
        &self,
        ServingReport {
            decisions,
            model_path,
            serving_issues,
            uses_chat_template_override,
        }: ServingReport,
    ) {
        for slot_index in 0..u32::from(self.desired_slots_total) {
            self.register_fix(&AgentIssueFix::SlotStarted(slot_index));
        }

        *self.model_path.write() = Some(model_path);
        *self.runtime.write() = AgentRuntimeStatus::Serving {
            decisions,
            slots_total: u64::from(self.desired_slots_total),
        };
        *self.serving_issues.write() = serving_issues;
        *self.state_application_status.write() = AgentStateApplicationStatus::Applied;
        self.uses_chat_template_override
            .set(uses_chat_template_override);
        self.announce_change();
    }

    pub fn slots_processing_count(&self) -> u64 {
        *self.slots_processing.read()
    }

    pub async fn wait_until_no_slots_are_processing(&self) {
        loop {
            let slots_released = self.slots_released.notified();

            if self.slots_processing_count() == 0 {
                return;
            }

            slots_released.await;
        }
    }
}

impl DispensesSlots for SlotAggregatedStatus {
    fn release_slots(&self, slots: NonZeroU16) {
        *self.slots_processing.write() -= u64::from(slots.get());
        self.slots_released.notify_waiters();
    }

    fn take_slots(&self, slots: NonZeroU16) {
        *self.slots_processing.write() += u64::from(slots.get());
    }
}

impl SubscribesToUpdates for SlotAggregatedStatus {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

impl ProducesSnapshot for SlotAggregatedStatus {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> Self::Snapshot {
        SlotAggregatedStatusSnapshot {
            status: AgentStatus {
                desired_slots_total: self.desired_slots_total,
                download_status: self.download_status.read().clone(),
                issues: self
                    .desired_state_issues
                    .iter()
                    .chain(self.pipeline_issues.iter())
                    .map(|issue| issue.clone())
                    .chain(self.serving_issues.read().iter().cloned())
                    .collect(),
                model_path: self.model_path.read().clone(),
                runtime: *self.runtime.read(),
                state_application_status: self.get_state_application_status(),
                uses_chat_template_override: self.uses_chat_template_override.get(),
            },
            version: self.version.get(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::num::NonZeroU16;
    use std::task::Poll;

    use tokio_test::task::spawn;

    use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::inference_capability::InferenceCapability;
    use paddler_messaging::model_download_status::ModelDownloadStatus;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

    use super::SlotAggregatedStatus;
    use crate::agent_issue_fix::AgentIssueFix;
    use crate::dispenses_slots::DispensesSlots;
    use crate::serving_report::ServingReport;

    fn serving_report(decisions: AgentDecisionsServing) -> ServingReport {
        ServingReport {
            decisions,
            model_path: "model".to_owned(),
            serving_issues: BTreeSet::new(),
            uses_chat_template_override: false,
        }
    }

    fn serving_without_a_chat_template() -> ServingReport {
        ServingReport {
            serving_issues: BTreeSet::from([AgentIssue::UnableToFindChatTemplate(model_path(
                "model",
            ))]),
            ..serving_report(AgentDecisionsServing::NotServed)
        }
    }

    fn model_path(path: &str) -> ModelPath {
        ModelPath {
            model_path: path.to_owned(),
        }
    }

    fn is_slot_cannot_start(agent_issue: &AgentIssue) -> bool {
        matches!(agent_issue, AgentIssue::SlotCannotStart(_))
    }

    #[test]
    fn waiting_for_idle_slots_completes_once_the_last_slot_is_released() {
        let status = SlotAggregatedStatus::new(1);

        status.take_slots(NonZeroU16::MIN);

        let mut idle_wait = spawn(status.wait_until_no_slots_are_processing());

        assert!(idle_wait.poll().is_pending());

        status.release_slots(NonZeroU16::MIN);

        assert!(idle_wait.is_woken());
        assert_eq!(idle_wait.poll(), Poll::Ready(()));
    }

    #[test]
    fn taking_a_slot_does_not_announce_a_status_change() {
        let status = SlotAggregatedStatus::new(2);
        let update_rx = status.subscribe_to_updates();

        status.take_slots(NonZeroU16::MIN);

        assert!(!update_rx.has_changed().unwrap());
    }

    #[test]
    fn register_issue_and_has_issue_round_trip() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        assert!(!status.has_issue(&issue));

        status.register_desired_state_issue(issue.clone());

        assert!(status.has_issue(&issue));
    }

    #[test]
    fn register_fix_removes_matching_issues() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_desired_state_issue(issue.clone());
        status.register_fix(&AgentIssueFix::ModelFileExists(model_path("model_test")));

        assert!(!status.has_issue(&issue));
    }

    #[test]
    fn has_issue_like_matches_with_predicate() {
        let status = SlotAggregatedStatus::new(2);

        status.register_desired_state_issue(AgentIssue::ModelFileDoesNotExist(model_path(
            "model_test",
        )));

        assert!(!status.has_issue_like(is_slot_cannot_start));

        status.register_pipeline_issue(AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            inference_capability: InferenceCapability::Embeddings,
            slot_index: 3,
        }));

        assert!(status.has_issue_like(is_slot_cannot_start));

        assert!(!status.has_issue_like(|agent_issue| {
            matches!(agent_issue, AgentIssue::ModelCannotBeLoaded(_))
        }));
    }

    #[test]
    fn serving_fixes_the_slot_start_issues_of_every_desired_slot() {
        let status = SlotAggregatedStatus::new(2);

        for slot_index in 0..2 {
            status.register_pipeline_issue(AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: "context creation failed".to_owned(),
                inference_capability: InferenceCapability::TextGeneration,
                slot_index,
            }));
        }

        status.start_serving(serving_report(AgentDecisionsServing::NotServed));

        assert!(!status.has_issue_like(is_slot_cannot_start));
    }

    #[test]
    fn serving_nothing_makes_the_agent_idle() {
        let status = SlotAggregatedStatus::new(2);

        status.start_serving(serving_report(AgentDecisionsServing::NotServed));
        status.serve_nothing(false);

        assert_eq!(
            status.make_snapshot().status.runtime,
            AgentRuntimeStatus::Idle
        );
    }

    #[test]
    fn starting_to_serve_reports_the_model_its_decisions_and_the_applied_state() {
        let status = SlotAggregatedStatus::new(4);

        status.start_serving(serving_report(AgentDecisionsServing::Served));

        let snapshot = status.make_snapshot();

        assert_eq!(snapshot.status.desired_slots_total, 4);
        assert_eq!(snapshot.status.model_path, Some("model".to_owned()));
        assert_eq!(
            snapshot.status.runtime,
            AgentRuntimeStatus::Serving {
                decisions: AgentDecisionsServing::Served,
                slots_total: 4,
            }
        );
        assert_eq!(
            snapshot.status.state_application_status,
            AgentStateApplicationStatus::Applied
        );
    }

    #[test]
    fn a_retried_pipeline_reports_retrying_then_stuck_until_the_next_pipeline() {
        let status = SlotAggregatedStatus::new(2);
        let mut reported_statuses = vec![status.get_state_application_status()];

        status.retry_pipeline();
        reported_statuses.push(status.get_state_application_status());
        status.retry_pipeline();
        reported_statuses.push(status.get_state_application_status());
        status.retry_pipeline();
        reported_statuses.push(status.get_state_application_status());
        status.begin_pipeline();
        reported_statuses.push(status.make_snapshot().status.state_application_status);

        assert_eq!(
            reported_statuses,
            [
                AgentStateApplicationStatus::Fresh,
                AgentStateApplicationStatus::AttemptedAndRetrying,
                AgentStateApplicationStatus::Stuck,
                AgentStateApplicationStatus::Stuck,
                AgentStateApplicationStatus::Fresh,
            ]
        );
    }

    #[test]
    fn register_issue_twice_keeps_single_entry() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_desired_state_issue(issue.clone());
        status.register_desired_state_issue(issue);

        assert_eq!(status.make_snapshot().status.issues.len(), 1);
    }

    #[test]
    fn register_fix_without_matching_issue_keeps_issues() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_desired_state_issue(issue.clone());
        status.register_fix(&AgentIssueFix::ModelFileExists(model_path("other_model")));

        assert!(status.has_issue(&issue));
    }

    #[test]
    fn slots_processing_count_tracks_taken_slots() {
        let status = SlotAggregatedStatus::new(2);

        assert_eq!(status.slots_processing_count(), 0);

        status.take_slots(NonZeroU16::MIN);

        assert_eq!(status.slots_processing_count(), 1);

        status.release_slots(NonZeroU16::MIN);

        assert_eq!(status.slots_processing_count(), 0);
    }

    #[test]
    fn serving_nothing_forgets_the_model_but_keeps_requests_in_flight() {
        let status = SlotAggregatedStatus::new(2);

        status.start_serving(serving_report(AgentDecisionsServing::NotServed));
        status.take_slots(NonZeroU16::MIN);

        status.serve_nothing(false);

        let snapshot = status.make_snapshot();

        assert_eq!(snapshot.status.runtime, AgentRuntimeStatus::Idle);
        assert_eq!(status.slots_processing_count(), 1);
        assert_eq!(snapshot.status.model_path, None);
    }

    #[test]
    fn beginning_a_desired_state_clears_only_the_desired_state_issues() {
        let status = SlotAggregatedStatus::new(2);
        let pipeline_issue = AgentIssue::ModelCannotBeLoaded(model_path("serving_model"));

        status.set_download_status(ModelDownloadStatus::Downloading {
            downloaded_bytes: 500,
            model_path: "https://example.com/model.gguf".to_owned(),
            total_bytes: 1000,
        });
        status.register_desired_state_issue(AgentIssue::ModelFileDoesNotExist(model_path(
            "replaced_model",
        )));
        status.register_pipeline_issue(pipeline_issue.clone());

        status.begin_desired_state();

        let snapshot = status.make_snapshot();

        assert_eq!(snapshot.status.issues, BTreeSet::from([pipeline_issue]));
        assert_eq!(
            snapshot.status.download_status,
            ModelDownloadStatus::NotDownloading
        );
    }

    #[test]
    fn beginning_a_pipeline_clears_only_the_pipeline_issues() {
        let status = SlotAggregatedStatus::new(2);
        let desired_state_issue = AgentIssue::ModelFileDoesNotExist(model_path("desired_model"));

        status.register_desired_state_issue(desired_state_issue.clone());
        status.register_pipeline_issue(AgentIssue::ModelCannotBeLoaded(model_path(
            "replaced_model",
        )));

        status.begin_pipeline();

        assert_eq!(
            status.make_snapshot().status.issues,
            BTreeSet::from([desired_state_issue])
        );
    }

    #[test]
    fn failing_a_download_stops_it_and_registers_the_issue() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::DownloadInterrupted(model_path("https://example.com/model.gguf"));

        status.set_download_status(ModelDownloadStatus::Downloading {
            downloaded_bytes: 500,
            model_path: "https://example.com/model.gguf".to_owned(),
            total_bytes: 1000,
        });
        status.fail_download(issue.clone());

        let snapshot = status.make_snapshot();

        assert_eq!(
            snapshot.status.download_status,
            ModelDownloadStatus::NotDownloading
        );
        assert_eq!(snapshot.status.issues, BTreeSet::from([issue]));
    }

    #[test]
    fn serving_reports_whether_the_served_state_overrides_the_chat_template() {
        let status = SlotAggregatedStatus::new(2);

        assert!(!status.make_snapshot().status.uses_chat_template_override);

        status.start_serving(ServingReport {
            uses_chat_template_override: true,
            ..serving_report(AgentDecisionsServing::NotServed)
        });

        assert!(status.make_snapshot().status.uses_chat_template_override);

        status.serve_nothing(false);

        assert!(!status.make_snapshot().status.uses_chat_template_override);
    }

    #[test]
    fn serving_issues_stay_reported_while_a_new_pipeline_begins() {
        let status = SlotAggregatedStatus::new(2);
        let serving_issue = AgentIssue::UnableToFindChatTemplate(model_path("model"));

        status.start_serving(serving_without_a_chat_template());
        status.begin_pipeline();

        assert!(status.has_issue(&serving_issue));
        assert!(status.has_issue_like(|agent_issue| *agent_issue == serving_issue));
        assert_eq!(
            status.make_snapshot().status.issues,
            BTreeSet::from([serving_issue])
        );
    }

    #[test]
    fn serving_again_replaces_the_previous_serving_issues() {
        let status = SlotAggregatedStatus::new(2);

        status.start_serving(serving_without_a_chat_template());
        status.start_serving(serving_report(AgentDecisionsServing::NotServed));

        assert_eq!(status.make_snapshot().status.issues, BTreeSet::new());
    }

    #[test]
    fn serving_nothing_clears_the_serving_issues() {
        let status = SlotAggregatedStatus::new(2);

        status.start_serving(serving_without_a_chat_template());
        status.serve_nothing(false);

        assert_eq!(status.make_snapshot().status.issues, BTreeSet::new());
    }

    #[test]
    fn released_slots_return_exactly_the_slots_that_were_taken() {
        let status = SlotAggregatedStatus::new(4);
        let two_slots = NonZeroU16::new(2).unwrap();

        status.take_slots(two_slots);
        status.take_slots(NonZeroU16::MIN);
        status.release_slots(two_slots);

        assert_eq!(status.slots_processing_count(), 1);
    }
}
