#![cfg(feature = "tests_that_use_llms")]

use std::future::Future as _;
use std::pin::pin;
use std::task::Context;
use std::task::Waker;

use tokio_util::sync::CancellationToken;

use paddler_agent::inference_capabilities::InferenceCapabilities;
use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;

use crate::kev_model_serving_without_decisions::KevModelServingWithoutDecisions;

#[test]
fn retaining_a_served_model_restarts_its_changed_capabilities_at_once() {
    single_blocking_thread_runtime()
        .expect("the test runtime must build")
        .block_on(async {
            let KevModelServingWithoutDecisions {
                model_serving_settings,
                served_model,
                ..
            } = KevModelServingWithoutDecisions::start(&CancellationToken::new()).await;
            let embeddings_preparations = served_model
                .capabilities
                .embeddings
                .request_preparer
                .preparation_tasks
                .clone();
            let text_generation_preparations = served_model
                .capabilities
                .text_generation
                .request_preparer
                .preparation_tasks
                .clone();
            let desired_settings = ModelServingSettings {
                capabilities: InferenceCapabilities {
                    embeddings: PoolingType::Mean,
                    text_generation: TextGenerationSettings {
                        sampling_parameters: SamplingParameters {
                            temperature: model_serving_settings
                                .capabilities
                                .text_generation
                                .sampling_parameters
                                .temperature
                                / 2.0,
                            ..model_serving_settings
                                .capabilities
                                .text_generation
                                .sampling_parameters
                                .clone()
                        },
                        ..model_serving_settings.capabilities.text_generation.clone()
                    },
                    ..model_serving_settings.capabilities.clone()
                },
                ..model_serving_settings
            };
            let occupied_blocking_thread = OccupiedBlockingThread::occupy();
            let mut retaining = pin!(served_model.retain_for(desired_settings));

            assert!(
                retaining
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            assert!(embeddings_preparations.is_closed());
            assert!(text_generation_preparations.is_closed());

            occupied_blocking_thread
                .release()
                .await
                .expect("the blocking thread occupant must be released");
            drop(retaining.await);
        });
}
