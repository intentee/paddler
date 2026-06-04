use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicU64;

use anyhow::Result;
use dashmap::DashSet;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use parking_lot::RwLock;
use tokio::sync::watch;

use crate::agent_issue_fix::AgentIssueFix;
use crate::dispenses_slots::DispensesSlots;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::produces_snapshot::ProducesSnapshot;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates;

pub struct SlotAggregatedStatus {
    desired_slots_total: i32,
    download_current: AtomicValue<AtomicU64>,
    download_filename: RwLock<Option<String>>,
    download_indeterminate: AtomicValue<AtomicBool>,
    download_total: AtomicValue<AtomicU64>,
    issues: DashSet<AgentIssue>,
    model_path: RwLock<Option<String>>,
    slots_processing: AtomicValue<AtomicI32>,
    slots_total: AtomicValue<AtomicI32>,
    state_application_status_code: AtomicValue<AtomicI32>,
    update_tx: watch::Sender<()>,
    uses_chat_template_override: AtomicValue<AtomicBool>,
    version: AtomicValue<AtomicI32>,
}

impl SlotAggregatedStatus {
    #[must_use]
    pub fn new(desired_slots_total: i32) -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            desired_slots_total,
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            issues: DashSet::new(),
            model_path: RwLock::new(None),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(0),
            update_tx,
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
            version: AtomicValue::<AtomicI32>::new(0),
        }
    }

    pub fn decrement_total_slots(&self) {
        self.slots_total.decrement();
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn get_state_application_status(&self) -> Result<AgentStateApplicationStatus> {
        self.state_application_status_code.get().try_into()
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

    pub fn increment_download_current(&self, size: u64) {
        self.download_current.increment_by(size);
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn increment_total_slots(&self) {
        self.slots_total.increment();
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn register_issue(&self, issue: AgentIssue) {
        if self.issues.insert(issue) {
            self.update_tx.send_replace(());
        }
    }

    pub fn register_fix(&self, fix: &AgentIssueFix) {
        let size_before = self.issues.len();

        self.issues.retain(|issue| !fix.can_fix(issue));

        if self.issues.len() < size_before {
            self.update_tx.send_replace(());
        }
    }

    pub fn reset(&self) {
        self.issues.clear();
        self.set_model_path(None);
        self.slots_processing.reset();
        self.slots_total.reset();
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn reset_download(&self) {
        self.download_current.set(0);
        self.download_total.set(0);
        self.download_indeterminate.set(true);
        self.set_download_filename(None);
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn set_download_status(&self, current: u64, total: Option<u64>, filename: Option<String>) {
        self.download_current.set(current);
        if let Some(value) = total {
            self.download_total.set(value);
            self.download_indeterminate.set(false);
        } else {
            self.download_total.set(0);
            self.download_indeterminate.set(true);
        }
        self.set_download_filename(filename);
    }

    pub fn set_download_filename(&self, filename: Option<String>) {
        {
            let mut filename_lock = self.download_filename.write();

            *filename_lock = filename;
        }

        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn set_model_path(&self, model_path: Option<String>) {
        {
            let mut path_lock = self.model_path.write();

            *path_lock = model_path;
        }

        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn set_state_application_status(&self, status: AgentStateApplicationStatus) {
        self.state_application_status_code.set(status as i32);
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn set_uses_chat_template_override(&self, uses: bool) {
        self.uses_chat_template_override.set(uses);
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn slots_processing_count(&self) -> i32 {
        self.slots_processing.get()
    }
}

impl DispensesSlots for SlotAggregatedStatus {
    fn release_slot(&self) {
        self.slots_processing.decrement();
        self.version.increment();
        self.update_tx.send_replace(());
    }

    fn take_slot(&self) {
        self.slots_processing.increment();
        self.version.increment();
        self.update_tx.send_replace(());
    }
}

impl SubscribesToUpdates for SlotAggregatedStatus {
    fn subscribe_to_updates(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

impl ProducesSnapshot for SlotAggregatedStatus {
    type Snapshot = SlotAggregatedStatusSnapshot;

    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(SlotAggregatedStatusSnapshot {
            issues: self.issues.iter().map(|item| item.clone()).collect(),
            desired_slots_total: self.desired_slots_total,
            download_current: self.download_current.get(),
            download_filename: self.download_filename.read().clone(),
            download_indeterminate: self.download_indeterminate.get(),
            download_total: self.download_total.get(),
            model_path: self.model_path.read().clone(),
            slots_processing: self.slots_processing.get(),
            slots_total: self.slots_total.get(),
            state_application_status: self.state_application_status_code.get().try_into()?,
            uses_chat_template_override: self.uses_chat_template_override.get(),
            version: self.version.get(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use paddler_messaging::agent_issue_params::model_path::ModelPath;
    use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn take_slot_wakes_subscribed_waiter() {
        let status = SlotAggregatedStatus::new(2);
        let mut update_rx = status.subscribe_to_updates();

        status.take_slot();

        timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .unwrap()
            .unwrap();
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
    fn increment_and_decrement_total_slots() {
        let status = SlotAggregatedStatus::new(2);

        status.increment_total_slots();
        status.increment_total_slots();

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.slots_total, 2);

        status.decrement_total_slots();

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.slots_total, 1);
    }

    #[test]
    fn version_increments_on_slot_changes() {
        let status = SlotAggregatedStatus::new(2);

        let initial_version = status.make_snapshot().unwrap().version;

        status.increment_total_slots();

        let updated_version = status.make_snapshot().unwrap().version;
        assert!(updated_version > initial_version);
    }

    #[test]
    fn make_snapshot_returns_correct_values() {
        let status = SlotAggregatedStatus::new(4);

        status.set_model_path(Some("test_model".to_owned()));
        status.increment_total_slots();
        status.increment_total_slots();

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.desired_slots_total, 4);
        assert_eq!(snapshot.model_path, Some("test_model".to_owned()));
        assert_eq!(snapshot.slots_total, 2);
        assert_eq!(snapshot.slots_processing, 0);
        assert_eq!(
            snapshot.state_application_status,
            AgentStateApplicationStatus::Fresh
        );
    }

    #[test]
    fn get_state_application_status_reflects_set_value() {
        let status = SlotAggregatedStatus::new(2);

        assert_eq!(
            status.get_state_application_status().unwrap(),
            AgentStateApplicationStatus::Fresh
        );

        status.set_state_application_status(AgentStateApplicationStatus::Applied);

        assert_eq!(
            status.get_state_application_status().unwrap(),
            AgentStateApplicationStatus::Applied
        );

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(
            snapshot.state_application_status,
            AgentStateApplicationStatus::Applied
        );
    }

    #[test]
    fn make_snapshot_propagates_invalid_state_application_status() {
        let status = SlotAggregatedStatus::new(2);

        status
            .state_application_status_code
            .set(AgentStateApplicationStatus::Stuck as i32 + 1);

        let snapshot_result = status.make_snapshot();

        assert!(snapshot_result.is_err());
    }

    #[test]
    fn register_issue_twice_keeps_single_entry() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::ModelFileDoesNotExist(model_path("model_test"));

        status.register_issue(issue.clone());
        status.register_issue(issue);

        let snapshot = status.make_snapshot().unwrap();
        assert_eq!(snapshot.issues.len(), 1);
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
    fn reset_clears_state() {
        let status = SlotAggregatedStatus::new(2);

        status.set_model_path(Some("test_model".to_owned()));
        status.increment_total_slots();
        status.register_issue(AgentIssue::ModelFileDoesNotExist(model_path("model_test")));

        status.reset();

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.slots_total, 0);
        assert_eq!(snapshot.slots_processing, 0);
        assert_eq!(snapshot.model_path, None);
        assert!(snapshot.issues.is_empty());
    }

    #[test]
    fn take_slot_and_release_slot() {
        let status = SlotAggregatedStatus::new(2);

        status.take_slot();

        assert_eq!(status.make_snapshot().unwrap().slots_processing, 1);

        status.take_slot();

        assert_eq!(status.make_snapshot().unwrap().slots_processing, 2);

        status.release_slot();

        assert_eq!(status.make_snapshot().unwrap().slots_processing, 1);
    }

    #[test]
    fn set_download_status_updates_all_fields() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(100, Some(500), Some("model.gguf".to_owned()));

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 100);
        assert_eq!(snapshot.download_total, 500);
        assert_eq!(snapshot.download_filename, Some("model.gguf".to_owned()));
    }

    #[test]
    fn set_download_status_with_indeterminate_total_keeps_flag_true() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(123, None, Some("model.gguf".to_owned()));

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 123);
        assert_eq!(snapshot.download_total, 0);
        assert!(snapshot.download_indeterminate);
    }

    #[test]
    fn set_download_status_indeterminate_after_known_total_resets_download_total() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(0, Some(5000), Some("model.gguf".to_owned()));
        status.set_download_status(10, None, Some("model.gguf".to_owned()));

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_total, 0);
        assert!(snapshot.download_indeterminate);
    }

    #[test]
    fn set_download_status_with_known_total_flips_indeterminate_false() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(0, Some(5000), Some("model.gguf".to_owned()));

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_total, 5000);
        assert!(!snapshot.download_indeterminate);
    }

    #[test]
    fn increment_download_current_accumulates() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(0, Some(1000), Some("model.gguf".to_owned()));
        status.increment_download_current(100);
        status.increment_download_current(200);

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 300);
        assert_eq!(snapshot.download_total, 1000);
    }

    #[test]
    fn reset_download_clears_download_fields() {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(500, Some(1000), Some("model.gguf".to_owned()));
        status.reset_download();

        let snapshot = status.make_snapshot().unwrap();

        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_total, 0);
        assert!(snapshot.download_indeterminate);
        assert_eq!(snapshot.download_filename, None);
    }

    #[test]
    fn set_uses_chat_template_override() {
        let status = SlotAggregatedStatus::new(2);

        assert!(!status.make_snapshot().unwrap().uses_chat_template_override);

        status.set_uses_chat_template_override(true);

        assert!(status.make_snapshot().unwrap().uses_chat_template_override);

        status.set_uses_chat_template_override(false);

        assert!(!status.make_snapshot().unwrap().uses_chat_template_override);
    }
}
