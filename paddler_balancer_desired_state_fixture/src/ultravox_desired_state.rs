use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::ultravox_v0_5_llama_3_2_1b::ultravox_v0_5_llama_3_2_1b;
use paddler_model_card_fixture::ultravox_v0_5_llama_3_2_1b_mmproj::ultravox_v0_5_llama_3_2_1b_mmproj;

#[must_use]
pub fn ultravox_desired_state() -> BalancerDesiredState {
    ultravox_v0_5_llama_3_2_1b()
        .into_desired_state_with_multimodal_projection(ultravox_v0_5_llama_3_2_1b_mmproj())
}
