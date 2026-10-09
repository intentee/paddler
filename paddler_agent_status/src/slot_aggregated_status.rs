use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;

use dashmap::DashSet;
use parking_lot::RwLock;
use tokio::sync::watch;
use tokio::sync::watch::error::RecvError;

use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

use crate::agent_issue_fix::AgentIssueFix;
use crate::dispenses_slots::DispensesSlots;

pub struct SlotAggregatedStatus {
    pub desired_slots_total: u16,
    download_status: RwLock<ModelDownloadStatus>,
    issues: DashSet<AgentIssue>,
    model_path: RwLock<Option<String>>,
    runtime: RwLock<AgentRuntimeStatus>,
    slots_processing_tx: watch::Sender<u64>,
    state_application_status: RwLock<AgentStateApplicationStatus>,
    update_tx: watch::Sender<()>,
    uses_chat_template_override: AtomicValue<AtomicBool>,
    version: AtomicValue<AtomicU64>,
}

impl SlotAggregatedStatus {
    #[must_use]
    pub fn new(desired_slots_total: u16) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());
        let (slots_processing_tx, _initial_slots_processing_rx) = watch::channel(0);

        Self {
            desired_slots_total,
            download_status: RwLock::new(ModelDownloadStatus::NotDownloading),
            issues: DashSet::new(),
            model_path: RwLock::new(None),
            runtime: RwLock::new(AgentRuntimeStatus::Idle),
            state_application_status: RwLock::new(AgentStateApplicationStatus::Fresh),
            slots_processing_tx,
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
        self.issues.contains(issue)
    }

    pub fn has_issue_like<TFunction>(&self, issue_like: TFunction) -> bool
    where
        TFunction: Fn(&AgentIssue) -> bool,
    {
        self.issues
            .iter()
            .any(|ref_multi| issue_like(ref_multi.key()))
    }

    pub fn add_downloaded_bytes(&self, added_bytes: u64) {
        self.download_status
            .write()
            .add_downloaded_bytes(added_bytes);
        self.announce_change();
    }

    pub fn fail_download(&self, issue: AgentIssue) {
        *self.download_status.write() = ModelDownloadStatus::NotDownloading;
        self.issues.insert(issue);
        self.announce_change();
    }

    pub fn register_issue(&self, issue: AgentIssue) {
        if self.issues.insert(issue) {
            self.announce_change();
        }
    }

    pub fn register_fix(&self, fix: &AgentIssueFix) {
        let size_before = self.issues.len();

        self.issues.retain(|issue| !fix.can_fix(issue));

        if self.issues.len() < size_before {
            self.announce_change();
        }
    }

    pub fn reset(&self) {
        self.issues.clear();
        self.set_model_path(None);
        *self.runtime.write() = AgentRuntimeStatus::Idle;
        self.announce_change();
    }

    pub fn set_download_status(&self, download_status: ModelDownloadStatus) {
        *self.download_status.write() = download_status;
        self.announce_change();
    }

    pub fn set_model_path(&self, model_path: Option<String>) {
        {
            let mut path_lock = self.model_path.write();

            *path_lock = model_path;
        }

        self.announce_change();
    }

    pub fn set_state_application_status(&self, status: AgentStateApplicationStatus) {
        *self.state_application_status.write() = status;
        self.announce_change();
    }

    pub fn set_uses_chat_template_override(&self, uses: bool) {
        self.uses_chat_template_override.set(uses);
        self.announce_change();
    }

    pub fn start_serving(&self, inference_mode: InferenceMode) {
        for slot_index in 0..u32::from(self.desired_slots_total) {
            self.register_fix(&AgentIssueFix::SlotStarted(slot_index));
        }

        *self.runtime.write() = AgentRuntimeStatus::Serving {
            inference_mode,
            slots_total: u64::from(self.desired_slots_total),
        };
        self.announce_change();
    }

    pub fn slots_processing_count(&self) -> u64 {
        *self.slots_processing_tx.borrow()
    }

    pub async fn wait_until_no_slots_are_processing(&self) -> Result<(), RecvError> {
        self.slots_processing_tx
            .subscribe()
            .wait_for(|slots_processing| *slots_processing == 0)
            .await
            .map(drop)
    }
}

impl DispensesSlots for SlotAggregatedStatus {
    fn release_slot(&self) {
        self.slots_processing_tx
            .send_modify(|slots_processing| *slots_processing -= 1);
    }

    fn take_slot(&self) {
        self.slots_processing_tx
            .send_modify(|slots_processing| *slots_processing += 1);
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
                issues: self.issues.iter().map(|item| item.clone()).collect(),
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
    use std::task::Poll;

    use tokio_test::task::spawn;

    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
    use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_messaging::model_download_status::ModelDownloadStatus;
    use paddler_messaging::produces_snapshot::ProducesSnapshot;
    use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

    use super::SlotAggregatedStatus;
    use crate::agent_issue_fix::AgentIssueFix;
    use crate::dispenses_slots::DispensesSlots;

    #[test]
    fn waiting_for_idle_slots_completes_once_the_last_slot_is_released() {
        let status = SlotAggregatedStatus::new(1);

        status.take_slot();

        let mut idle_wait = spawn(status.wait_until_no_slots_are_processing());

        assert!(idle_wait.poll().is_pending());

        status.release_slot();

        assert!(idle_wait.is_woken());
        assert!(matches!(idle_wait.poll(), Poll::Ready(wait_result) if wait_result.is_ok()));
    }

    #[test]
    fn taking_a_slot_does_not_announce_a_status_change() {
        let status = SlotAggregatedStatus::new(2);
        let update_rx = status.subscribe_to_updates();

        status.take_slot();

        assert!(!update_rx.has_changed().unwrap());
    }

    fn model_path(path: &str) -> ModelPath {
        ModelPath {
            model_path: path.to_owned(),
        }
    }

    #[test]
    fn register_issue_and_has_issue_round_trip() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        assert!(!status.has_issue(&issue));

        status.register_issue(issue.clone());

        assert!(status.has_issue(&issue));
    }

    #[test]
    fn register_fix_removes_matching_issues() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_issue(issue.clone());
        status.register_fix(&AgentIssueFix::ModelFileExists(model_path("model_test")));

        assert!(!status.has_issue(&issue));
    }

    fn is_slot_cannot_start(agent_issue: &AgentIssue) -> bool {
        matches!(agent_issue, AgentIssue::SlotCannotStart(_))
    }

    #[test]
    fn has_issue_like_matches_with_predicate() {
        let status = SlotAggregatedStatus::new(2);

        status.register_issue(AgentIssue::ModelFileDoesNotExist(model_path("model_test")));

        assert!(!status.has_issue_like(is_slot_cannot_start));

        status.register_issue(AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
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
            status.register_issue(AgentIssue::SlotCannotStart(SlotCannotStartParams {
                error: "context creation failed".to_owned(),
                slot_index,
            }));
        }

        status.start_serving(InferenceMode::TextGeneration);

        assert!(!status.has_issue_like(is_slot_cannot_start));
    }

    #[test]
    fn resetting_a_serving_agent_makes_it_idle() {
        let status = SlotAggregatedStatus::new(2);

        status.start_serving(InferenceMode::Embeddings);
        status.reset();

        assert_eq!(
            status.make_snapshot().status.runtime,
            AgentRuntimeStatus::Idle
        );
    }

    #[test]
    fn snapshot_reports_the_desired_slots_and_the_model_path() {
        let status = SlotAggregatedStatus::new(4);

        status.set_model_path(Some("test_model".to_owned()));
        status.start_serving(InferenceMode::Embeddings);

        let snapshot = status.make_snapshot();

        assert_eq!(snapshot.status.desired_slots_total, 4);
        assert_eq!(snapshot.status.model_path, Some("test_model".to_owned()));
        assert_eq!(
            snapshot.status.runtime,
            AgentRuntimeStatus::Serving {
                inference_mode: InferenceMode::Embeddings,
                slots_total: 4,
            }
        );
        assert_eq!(
            snapshot.status.state_application_status,
            AgentStateApplicationStatus::Fresh
        );
    }

    #[test]
    fn get_state_application_status_reflects_set_value() {
        let status = SlotAggregatedStatus::new(2);

        assert_eq!(
            status.get_state_application_status(),
            AgentStateApplicationStatus::Fresh
        );

        status.set_state_application_status(AgentStateApplicationStatus::Applied);

        assert_eq!(
            status.get_state_application_status(),
            AgentStateApplicationStatus::Applied
        );

        assert_eq!(
            status.make_snapshot().status.state_application_status,
            AgentStateApplicationStatus::Applied
        );
    }

    #[test]
    fn register_issue_twice_keeps_single_entry() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_issue(issue.clone());
        status.register_issue(issue);

        assert_eq!(status.make_snapshot().status.issues.len(), 1);
    }

    #[test]
    fn register_fix_without_matching_issue_keeps_issues() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_issue(issue.clone());
        status.register_fix(&AgentIssueFix::ModelFileExists(model_path("other_model")));

        assert!(status.has_issue(&issue));
    }

    #[test]
    fn slots_processing_count_tracks_taken_slots() {
        let status = SlotAggregatedStatus::new(2);

        assert_eq!(status.slots_processing_count(), 0);

        status.take_slot();

        assert_eq!(status.slots_processing_count(), 1);

        status.release_slot();

        assert_eq!(status.slots_processing_count(), 0);
    }

    #[test]
    fn reset_clears_state_but_keeps_requests_in_flight() {
        let status = SlotAggregatedStatus::new(2);

        status.set_model_path(Some("test_model".to_owned()));
        status.start_serving(InferenceMode::TextGeneration);
        status.take_slot();
        status.register_issue(AgentIssue::ModelFileDoesNotExist(model_path("model_test")));

        status.reset();

        let snapshot = status.make_snapshot();

        assert_eq!(snapshot.status.runtime, AgentRuntimeStatus::Idle);
        assert_eq!(status.slots_processing_count(), 1);
        assert_eq!(snapshot.status.model_path, None);
        assert!(snapshot.status.issues.is_empty());
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
    fn set_uses_chat_template_override() {
        let status = SlotAggregatedStatus::new(2);

        assert!(!status.make_snapshot().status.uses_chat_template_override);

        status.set_uses_chat_template_override(true);

        assert!(status.make_snapshot().status.uses_chat_template_override);

        status.set_uses_chat_template_override(false);

        assert!(!status.make_snapshot().status.uses_chat_template_override);
    }
}
