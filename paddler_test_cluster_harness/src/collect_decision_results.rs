use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response as InferenceResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::collected_decision_results::CollectedDecisionResults;

pub async fn collect_decision_results(
    mut stream: InferenceMessageStream,
) -> Result<CollectedDecisionResults> {
    let mut answers = Vec::new();

    while let Some(item) = stream.next().await {
        match item.context("decision stream yielded an error")? {
            InferenceMessage::Response(ResponseEnvelope {
                response: InferenceResponse::Decision(DecisionResult::QuestionAnswered(answer)),
                ..
            }) => answers.push(answer),
            InferenceMessage::Response(ResponseEnvelope {
                response: InferenceResponse::Decision(terminal_result),
                ..
            }) => {
                return Ok(CollectedDecisionResults {
                    answers,
                    terminal_result,
                });
            }
            unexpected_message => {
                return Err(ClusterHarnessError::DecisionStreamMessageUnexpected {
                    message: Box::new(unexpected_message),
                }
                .into());
            }
        }
    }

    Err(ClusterHarnessError::DecisionStreamEndedWithoutTerminalResult.into())
}
