#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;
use std::pin::pin;

use futures_util::FutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::startup_outcome::StartupOutcome;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;

#[test]
fn a_shutdown_while_a_model_loads_cancels_the_load() {
    single_blocking_thread_runtime()
        .expect("the test runtime must build")
        .block_on(async {
            let agent_shutdown = CancellationToken::new();
            let occupied_blocking_thread = OccupiedBlockingThread::occupy();
            let mut model_load = pin!(LoadedLlamaModel::load_unless_shutting_down(
                &agent_shutdown,
                agent_inference_runtime_context(1),
                LlamaModelSettings {
                    model_path: KevCheckpoint::Kev0_8b.target_path("model.gguf"),
                    n_gpu_layers: ALL_GPU_LAYERS,
                },
            ));

            assert!(model_load.as_mut().now_or_never().is_none());

            agent_shutdown.cancel();
            occupied_blocking_thread
                .release()
                .await
                .expect("the blocking thread occupant must be released");

            assert_eq!(
                discriminant(
                    &model_load
                        .await
                        .expect("the model must load before the shutdown is noticed")
                ),
                discriminant(&StartupOutcome::Cancelled)
            );
        });
}
