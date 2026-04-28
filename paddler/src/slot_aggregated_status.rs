use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use dashmap::DashSet;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::agent_state_application_status::AgentStateApplicationStatus;
use paddler_types::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;
use tokio::sync::watch;

use crate::agent_issue_fix::AgentIssueFix;
use crate::atomic_value::AtomicValue;
use crate::dispenses_slots::DispensesSlots;
use crate::produces_snapshot::ProducesSnapshot;
use crate::subscribes_to_updates::SubscribesToUpdates;

pub struct SlotAggregatedStatus {
    desired_slots_total: i32,
    download_current: AtomicValue<AtomicUsize>,
    download_filename: RwLock<Option<String>>,
    download_total: AtomicValue<AtomicUsize>,
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
            download_current: AtomicValue::<AtomicUsize>::new(0),
            download_filename: RwLock::new(None),
            download_total: AtomicValue::<AtomicUsize>::new(0),
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

    pub fn increment_download_current(&self, size: usize) {
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
        self.set_download_filename(None);
        self.version.increment();
        self.update_tx.send_replace(());
    }

    pub fn set_download_status(&self, current: usize, total: usize, filename: Option<String>) {
        self.download_current.set(current);
        self.download_total.set(total);
        self.set_download_filename(filename);
    }

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn set_download_filename(&self, filename: Option<String>) {
        {
            let mut filename_lock = self
                .download_filename
                .write()
                .expect("Lock poisoned when setting download filename");

            *filename_lock = filename;
        }

        self.version.increment();
        self.update_tx.send_replace(());
    }

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    pub fn set_model_path(&self, model_path: Option<String>) {
        {
            let mut path_lock = self
                .model_path
                .write()
                .expect("Lock poisoned when setting model path");

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

    #[expect(clippy::expect_used, reason = "mutex lock poison is unrecoverable")]
    fn make_snapshot(&self) -> Result<Self::Snapshot> {
        Ok(SlotAggregatedStatusSnapshot {
            issues: self.issues.iter().map(|item| item.clone()).collect(),
            desired_slots_total: self.desired_slots_total,
            download_current: self.download_current.get(),
            download_filename: self
                .download_filename
                .read()
                .expect("Lock poisoned when getting download filename")
                .clone(),
            download_total: self.download_total.get(),
            model_path: self
                .model_path
                .read()
                .expect("Lock poisoned when getting model path")
                .clone(),
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

    use anyhow::Result;
    use paddler_types::agent_issue_params::ModelPath;
    use paddler_types::agent_issue_params::SlotCannotStartParams;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn take_slot_wakes_subscribed_waiter() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);
        let mut update_rx = status.subscribe_to_updates();

        status.take_slot();

        timeout(Duration::from_secs(1), update_rx.changed())
            .await
            .map_err(|err| anyhow::anyhow!("subscriber did not observe within deadline: {err}"))?
            .map_err(|err| anyhow::anyhow!("watch sender dropped: {err}"))?;

        Ok(())
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

    #[test]
    fn has_issue_like_matches_with_predicate() {
        let status = SlotAggregatedStatus::new(2);
        let issue = AgentIssue::SlotCannotStart(SlotCannotStartParams {
            error: "failed".to_owned(),
            slot_index: 3,
        });

        status.register_issue(issue);

        assert!(status.has_issue_like(|agent_issue| {
            matches!(agent_issue, AgentIssue::SlotCannotStart(_))
        }));

        assert!(!status.has_issue_like(|agent_issue| {
            matches!(agent_issue, AgentIssue::ModelCannotBeLoaded(_))
        }));
    }

    #[test]
    fn increment_and_decrement_total_slots() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.increment_total_slots();
        status.increment_total_slots();

        let snapshot = status.make_snapshot()?;
        assert_eq!(snapshot.slots_total, 2);

        status.decrement_total_slots();

        let snapshot = status.make_snapshot()?;
        assert_eq!(snapshot.slots_total, 1);

        Ok(())
    }

    #[test]
    fn version_increments_on_slot_changes() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        let initial_version = status.make_snapshot()?.version;

        status.increment_total_slots();

        let updated_version = status.make_snapshot()?.version;
        assert!(updated_version > initial_version);

        Ok(())
    }

    #[test]
    fn make_snapshot_returns_correct_values() -> Result<()> {
        let status = SlotAggregatedStatus::new(4);

        status.set_model_path(Some("test_model".to_owned()));
        status.increment_total_slots();
        status.increment_total_slots();

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.desired_slots_total, 4);
        assert_eq!(snapshot.model_path, Some("test_model".to_owned()));
        assert_eq!(snapshot.slots_total, 2);
        assert_eq!(snapshot.slots_processing, 0);
        assert_eq!(
            snapshot.state_application_status,
            AgentStateApplicationStatus::Fresh
        );

        Ok(())
    }

    #[test]
    fn reset_clears_state() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.set_model_path(Some("test_model".to_owned()));
        status.increment_total_slots();
        status.register_issue(AgentIssue::ModelFileDoesNotExist(model_path("model_test")));

        status.reset();

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.slots_total, 0);
        assert_eq!(snapshot.slots_processing, 0);
        assert_eq!(snapshot.model_path, None);
        assert!(snapshot.issues.is_empty());

        Ok(())
    }

    #[test]
    fn take_slot_and_release_slot() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.take_slot();

        assert_eq!(status.make_snapshot()?.slots_processing, 1);

        status.take_slot();

        assert_eq!(status.make_snapshot()?.slots_processing, 2);

        status.release_slot();

        assert_eq!(status.make_snapshot()?.slots_processing, 1);

        Ok(())
    }

    #[test]
    fn set_download_status_updates_all_fields() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(100, 500, Some("model.gguf".to_owned()));

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.download_current, 100);
        assert_eq!(snapshot.download_total, 500);
        assert_eq!(snapshot.download_filename, Some("model.gguf".to_owned()));

        Ok(())
    }

    #[test]
    fn increment_download_current_accumulates() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(0, 1000, Some("model.gguf".to_owned()));
        status.increment_download_current(100);
        status.increment_download_current(200);

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.download_current, 300);
        assert_eq!(snapshot.download_total, 1000);

        Ok(())
    }

    #[test]
    fn reset_download_clears_download_fields() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        status.set_download_status(500, 1000, Some("model.gguf".to_owned()));
        status.reset_download();

        let snapshot = status.make_snapshot()?;

        assert_eq!(snapshot.download_current, 0);
        assert_eq!(snapshot.download_total, 0);
        assert_eq!(snapshot.download_filename, None);

        Ok(())
    }

    #[test]
    fn set_uses_chat_template_override() -> Result<()> {
        let status = SlotAggregatedStatus::new(2);

        assert!(!status.make_snapshot()?.uses_chat_template_override);

        status.set_uses_chat_template_override(true);

        assert!(status.make_snapshot()?.uses_chat_template_override);

        status.set_uses_chat_template_override(false);

        assert!(!status.make_snapshot()?.uses_chat_template_override);

        Ok(())
    }
}
