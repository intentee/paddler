use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::sync::watch;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_spawn_resuming_panic::spawn_resuming_panic::spawn_resuming_panic;
use paddler_state_database::state_database::StateDatabase;
use paddler_state_database::state_database_error::StateDatabaseError;
use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;

use crate::balancer_applicable_state::BalancerApplicableState;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;

pub struct BalancerDesiredStateStore {
    balancer_applicable_state_tx: watch::Sender<Arc<BalancerApplicableState>>,
    replacement_lock: Arc<Mutex<()>>,
    state_database: Arc<StateDatabase>,
}

impl BalancerDesiredStateStore {
    pub async fn load(state_database: Arc<StateDatabase>) -> Result<Self, StateDatabaseError> {
        let balancer_desired_state = match state_database.read_balancer_desired_state().await? {
            StoredBalancerDesiredState::NothingStored => BalancerDesiredState::default(),
            StoredBalancerDesiredState::Stored(stored_desired_state) => stored_desired_state,
        };
        let (balancer_applicable_state_tx, _initial_balancer_applicable_state_rx) = watch::channel(
            Arc::new(BalancerApplicableState::initial(balancer_desired_state)),
        );

        Ok(Self {
            balancer_applicable_state_tx,
            replacement_lock: Arc::new(Mutex::new(())),
            state_database,
        })
    }

    #[must_use]
    pub fn applicable_state_holder(&self) -> BalancerApplicableStateHolder {
        BalancerApplicableStateHolder::new(self.balancer_applicable_state_tx.subscribe())
    }

    pub async fn replace(
        &self,
        balancer_desired_state: BalancerDesiredState,
    ) -> Result<(), StateDatabaseError> {
        let replacement_guard = self.replacement_lock.clone().lock_owned().await;
        let balancer_applicable_state_tx = self.balancer_applicable_state_tx.clone();
        let state_database = self.state_database.clone();

        spawn_resuming_panic(async move {
            if balancer_applicable_state_tx.borrow().balancer_desired_state
                == balancer_desired_state
            {
                return Ok(());
            }

            state_database
                .store_balancer_desired_state(&balancer_desired_state)
                .await?;

            let superseding_applicable_state = balancer_applicable_state_tx
                .borrow()
                .superseded_by(balancer_desired_state);

            balancer_applicable_state_tx.send_replace(Arc::new(superseding_applicable_state));
            drop(replacement_guard);

            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::FutureExt as _;
    use tempfile::TempDir;

    use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
    use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_state_database::state_database::StateDatabase;
    use paddler_state_database::state_database_file::StateDatabaseFile;
    use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;

    use super::BalancerDesiredStateStore;

    fn state_with_model(model_uri: &str) -> BalancerDesiredState {
        BalancerDesiredState {
            model: AgentDesiredModel::Uri(model_uri.to_owned()),
            ..BalancerDesiredState::default()
        }
    }

    #[tokio::test]
    async fn starts_from_the_default_state_when_nothing_is_stored() {
        let store = BalancerDesiredStateStore::load(Arc::new(StateDatabase::in_memory(
            StoredBalancerDesiredState::NothingStored,
        )))
        .await
        .unwrap();

        assert_eq!(
            store.applicable_state_holder().get_balancer_desired_state(),
            BalancerDesiredState::default()
        );
    }

    #[tokio::test]
    async fn starts_from_the_stored_state() {
        let store = BalancerDesiredStateStore::load(Arc::new(StateDatabase::in_memory(
            StoredBalancerDesiredState::Stored(state_with_model("stored_model_uri")),
        )))
        .await
        .unwrap();

        assert_eq!(
            store.applicable_state_holder().get_balancer_desired_state(),
            state_with_model("stored_model_uri")
        );
    }

    #[tokio::test]
    async fn a_replaced_state_is_stored_and_published() {
        let state_database = Arc::new(StateDatabase::in_memory(
            StoredBalancerDesiredState::NothingStored,
        ));
        let store = BalancerDesiredStateStore::load(state_database.clone())
            .await
            .unwrap();
        let mut applicable_state_rx = store.applicable_state_holder().subscribe();

        store
            .replace(state_with_model("replacing_model_uri"))
            .await
            .unwrap();

        assert_eq!(
            state_database.read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::Stored(state_with_model("replacing_model_uri"))
        );
        assert_eq!(
            applicable_state_rx
                .borrow_and_update()
                .balancer_desired_state,
            state_with_model("replacing_model_uri")
        );
    }

    #[tokio::test]
    async fn an_identical_state_is_neither_stored_nor_published() {
        let state_database = Arc::new(StateDatabase::in_memory(
            StoredBalancerDesiredState::NothingStored,
        ));
        let store = BalancerDesiredStateStore::load(state_database.clone())
            .await
            .unwrap();
        let applicable_state_rx = store.applicable_state_holder().subscribe();

        store
            .replace(BalancerDesiredState::default())
            .await
            .unwrap();

        assert_eq!(
            state_database.read_balancer_desired_state().await.unwrap(),
            StoredBalancerDesiredState::NothingStored
        );
        assert!(!applicable_state_rx.has_changed().unwrap());
    }

    #[test]
    fn a_replacement_its_caller_abandons_is_applied_before_the_next_one() {
        single_blocking_thread_runtime().unwrap().block_on(async {
            let temp_dir = TempDir::new().unwrap();
            let state_path = temp_dir.path().join("state.json");
            let store = BalancerDesiredStateStore::load(Arc::new(StateDatabase::File(
                StateDatabaseFile::new(state_path.clone()),
            )))
            .await
            .unwrap();
            let occupied_blocking_thread = OccupiedBlockingThread::occupy();

            assert!(
                store
                    .replace(state_with_model("abandoned_model_uri"))
                    .now_or_never()
                    .is_none()
            );

            occupied_blocking_thread.release().await.unwrap();
            store
                .replace(state_with_model("next_model_uri"))
                .await
                .unwrap();

            assert_eq!(
                store
                    .applicable_state_holder()
                    .get_balancer_applicable_state()
                    .version,
                2
            );
            assert_eq!(
                StateDatabaseFile::new(state_path)
                    .read_balancer_desired_state()
                    .await
                    .unwrap(),
                StoredBalancerDesiredState::Stored(state_with_model("next_model_uri"))
            );
        });
    }

    #[tokio::test]
    async fn a_state_that_cannot_be_stored_is_not_published() {
        let directory_in_place_of_the_state_file = TempDir::new().unwrap();
        let store =
            BalancerDesiredStateStore::load(Arc::new(StateDatabase::File(StateDatabaseFile::new(
                directory_in_place_of_the_state_file
                    .path()
                    .join("state.json"),
            ))))
            .await
            .unwrap();
        let applicable_state_rx = store.applicable_state_holder().subscribe();

        directory_in_place_of_the_state_file.close().unwrap();

        assert!(
            store
                .replace(state_with_model("unstorable_model_uri"))
                .await
                .is_err()
        );
        assert!(!applicable_state_rx.has_changed().unwrap());
    }
}
