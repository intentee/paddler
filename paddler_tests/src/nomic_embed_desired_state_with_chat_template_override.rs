use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

use crate::desired_state_with_chat_template_override::desired_state_with_chat_template_override;

#[must_use]
pub fn nomic_embed_desired_state_with_chat_template_override(
    chat_template: ChatTemplate,
) -> BalancerDesiredState {
    desired_state_with_chat_template_override(
        nomic_embed_text_v1_5().into_desired_state(),
        chat_template,
    )
}
