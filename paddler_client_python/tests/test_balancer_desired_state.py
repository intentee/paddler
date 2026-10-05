from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.chat_template import ChatTemplate


def test_balancer_desired_state_defaults() -> None:
    state = BalancerDesiredState()
    dumped = state.model_dump(mode="json")

    assert dumped["model"] == "None"
    assert dumped["multimodal_projection"] == "None"
    assert dumped["use_chat_template_override"] is False
    assert dumped["inference_parameters"]["temperature"] == 0.8


def test_balancer_desired_state_with_chat_template() -> None:
    state = BalancerDesiredState(
        chat_template_override=ChatTemplate(content="{{ messages }}"),
        use_chat_template_override=True,
    )
    dumped = state.model_dump(mode="json")

    assert dumped["chat_template_override"] == {"content": "{{ messages }}"}
    assert dumped["use_chat_template_override"] is True
