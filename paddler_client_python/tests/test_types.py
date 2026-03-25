from paddler_client.types.agent_controller_pool_snapshot import (
    AgentControllerPoolSnapshot,
)
from paddler_client.types.agent_controller_snapshot import AgentControllerSnapshot
from paddler_client.types.agent_desired_model import AgentDesiredModel
from paddler_client.types.agent_issue import AgentIssue
from paddler_client.types.agent_state_application_status import (
    AgentStateApplicationStatus,
)
from paddler_client.types.balancer_desired_state import BalancerDesiredState
from paddler_client.types.buffered_request_manager_snapshot import (
    BufferedRequestManagerSnapshot,
)
from paddler_client.types.chat_template import ChatTemplate
from paddler_client.types.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.types.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.types.conversation_message import ConversationMessage
from paddler_client.types.conversation_message_content_part import (
    ImageUrlContentPart,
    TextContentPart,
)
from paddler_client.types.embedding import Embedding
from paddler_client.types.embedding_input_document import EmbeddingInputDocument
from paddler_client.types.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.types.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)
from paddler_client.types.huggingface_model_reference import (
    HuggingFaceModelReference,
)
from paddler_client.types.image_url import ImageUrl
from paddler_client.types.inference_parameters import InferenceParameters
from paddler_client.types.model_metadata import ModelMetadata
from paddler_client.types.pooling_type import PoolingType
from paddler_client.types.tool import Function, Tool
from paddler_client.types.validated_parameters_schema import (
    ValidatedParametersSchema,
)

# --- EmbeddingNormalizationMethod ---


def test_embedding_normalization_method_l2_serialization() -> None:
    method = EmbeddingNormalizationMethod.l2()

    assert method.model_dump(mode="json") == "L2"


def test_embedding_normalization_method_none_serialization() -> None:
    method = EmbeddingNormalizationMethod.none()

    assert method.model_dump(mode="json") == "None"


def test_embedding_normalization_method_rms_norm_serialization() -> None:
    method = EmbeddingNormalizationMethod.rms_norm(epsilon=0.001)

    assert method.model_dump(mode="json") == {
        "RmsNorm": {"epsilon": 0.001}
    }


def test_embedding_normalization_method_l2_deserialization() -> None:
    method = EmbeddingNormalizationMethod.model_validate("L2")

    assert method.variant == "L2"


def test_embedding_normalization_method_rms_norm_deserialization() -> None:
    method = EmbeddingNormalizationMethod.model_validate(
        {"RmsNorm": {"epsilon": 0.001}}
    )

    assert method.variant == "RmsNorm"
    assert method.epsilon == 0.001


# --- AgentDesiredModel ---


def test_agent_desired_model_none_serialization() -> None:
    model = AgentDesiredModel.none()

    assert model.model_dump(mode="json") == "None"


def test_agent_desired_model_none_deserialization() -> None:
    model = AgentDesiredModel.model_validate("None")

    assert model.variant == "None"


def test_agent_desired_model_huggingface_serialization() -> None:
    reference = HuggingFaceModelReference(
        filename="model.gguf",
        repo_id="org/model",
        revision="main",
    )
    model = AgentDesiredModel.from_huggingface(reference)
    dumped = model.model_dump(mode="json")

    assert "HuggingFace" in dumped
    assert dumped["HuggingFace"]["repo_id"] == "org/model"


def test_agent_desired_model_huggingface_deserialization() -> None:
    model = AgentDesiredModel.model_validate({
        "HuggingFace": {
            "filename": "model.gguf",
            "repo_id": "org/model",
            "revision": "main",
        }
    })

    assert model.variant == "HuggingFace"
    assert model.huggingface is not None
    assert model.huggingface.repo_id == "org/model"


def test_agent_desired_model_local_to_agent_serialization() -> None:
    model = AgentDesiredModel.local_to_agent("/path/to/model.gguf")
    dumped = model.model_dump(mode="json")

    assert dumped == {"LocalToAgent": "/path/to/model.gguf"}


def test_agent_desired_model_local_to_agent_deserialization() -> None:
    model = AgentDesiredModel.model_validate(
        {"LocalToAgent": "/path/to/model.gguf"}
    )

    assert model.variant == "LocalToAgent"
    assert model.local_path == "/path/to/model.gguf"


# --- ConversationMessage ---


def test_conversation_message_text_content() -> None:
    message = ConversationMessage(content="hello", role="user")
    dumped = message.model_dump(mode="json")

    assert dumped == {"content": "hello", "role": "user"}


def test_conversation_message_parts_content() -> None:
    message = ConversationMessage(
        content=[
            TextContentPart(text="hello"),
            ImageUrlContentPart(
                image_url=ImageUrl(url="http://example.com/img.png"),
            ),
        ],
        role="user",
    )
    dumped = message.model_dump(mode="json")

    assert dumped["content"][0] == {"type": "text", "text": "hello"}
    assert dumped["content"][1] == {
        "type": "image_url",
        "image_url": {"url": "http://example.com/img.png"},
    }


def test_conversation_message_text_deserialization() -> None:
    message = ConversationMessage.model_validate(
        {"content": "hi", "role": "assistant"}
    )

    assert message.content == "hi"
    assert message.role == "assistant"


# --- Tool ---


def test_tool_serialization() -> None:
    tool = Tool(
        function=Function(
            name="get_weather",
            description="Get weather",
        )
    )
    dumped = tool.model_dump(mode="json", exclude_none=True)

    assert dumped == {
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather",
        },
    }


def test_tool_with_parameters_serialization() -> None:
    tool = Tool(
        function=Function(
            name="get_weather",
            description="Get weather",
            parameters=ValidatedParametersSchema(
                schema_type="object",
                properties={"location": {"type": "string"}},
                required=["location"],
            ),
        )
    )
    dumped = tool.model_dump(
        mode="json",
        exclude_none=True,
        by_alias=True,
    )

    assert dumped["function"]["parameters"]["type"] == "object"
    assert "location" in dumped["function"]["parameters"]["properties"]
    assert dumped["function"]["parameters"]["required"] == ["location"]


def test_tool_deserialization() -> None:
    tool = Tool.model_validate({
        "type": "function",
        "function": {"name": "test", "description": "desc"},
    })

    assert tool.function.name == "test"


# --- Request Params ---


def test_continue_from_conversation_history_params_serialization() -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Hello!", role="user"),
        ],
        enable_thinking=False,
        max_tokens=100,
    )
    dumped = params.model_dump(mode="json")

    assert dumped["add_generation_prompt"] is True
    assert dumped["conversation_history"] == [
        {"content": "Hello!", "role": "user"}
    ]
    assert dumped["enable_thinking"] is False
    assert dumped["max_tokens"] == 100
    assert dumped["tools"] == []


def test_continue_from_raw_prompt_params_serialization() -> None:
    params = ContinueFromRawPromptParams(
        max_tokens=50,
        raw_prompt="Once upon a time",
    )
    dumped = params.model_dump(mode="json")

    assert dumped == {"max_tokens": 50, "raw_prompt": "Once upon a time"}


def test_generate_embedding_batch_params_serialization() -> None:
    params = GenerateEmbeddingBatchParams(
        input_batch=[
            EmbeddingInputDocument(content="hello world", id="doc-1"),
        ],
        normalization_method=EmbeddingNormalizationMethod.l2(),
    )
    dumped = params.model_dump(mode="json")

    assert len(dumped["input_batch"]) == 1
    assert dumped["input_batch"][0]["id"] == "doc-1"
    assert dumped["normalization_method"] == "L2"


# --- Management Types ---


def test_inference_parameters_defaults() -> None:
    params = InferenceParameters()

    assert params.temperature == 0.8
    assert params.context_size == 8192
    assert params.pooling_type == PoolingType.LAST
    assert params.enable_embeddings is False


def test_inference_parameters_serialization() -> None:
    params = InferenceParameters(temperature=0.5, top_k=40)
    dumped = params.model_dump(mode="json")

    assert dumped["temperature"] == 0.5
    assert dumped["top_k"] == 40


def test_pooling_type_values() -> None:
    assert PoolingType.MEAN == "Mean"
    assert PoolingType.CLS == "Cls"
    assert PoolingType.LAST == "Last"
    assert PoolingType.NONE == "None"
    assert PoolingType.RANK == "Rank"
    assert PoolingType.UNSPECIFIED == "Unspecified"


def test_agent_state_application_status_values() -> None:
    assert AgentStateApplicationStatus.APPLIED == "Applied"
    assert (
        AgentStateApplicationStatus.ATTEMPTED_AND_NOT_APPLIABLE
        == "AttemptedAndNotAppliable"
    )
    assert AgentStateApplicationStatus.FRESH == "Fresh"
    assert AgentStateApplicationStatus.STUCK == "Stuck"


def test_agent_issue_deserialization() -> None:
    issue = AgentIssue.model_validate(
        {"ModelFileDoesNotExist": {"model_path": "/path/to/model"}}
    )

    assert issue.variant == "ModelFileDoesNotExist"
    assert issue.params["model_path"] == "/path/to/model"


def test_agent_issue_serialization() -> None:
    issue = AgentIssue(
        variant="ModelFileDoesNotExist",
        params={"model_path": "/path/to/model"},
    )
    dumped = issue.model_dump(mode="json")

    assert dumped == {
        "ModelFileDoesNotExist": {"model_path": "/path/to/model"}
    }


def test_agent_controller_snapshot_deserialization() -> None:
    snapshot = AgentControllerSnapshot.model_validate({
        "desired_slots_total": 4,
        "download_current": 100,
        "download_filename": "model.gguf",
        "download_total": 1000,
        "id": "agent-1",
        "issues": [
            {"SlotCannotStart": {"error": "OOM", "slot_index": 0}}
        ],
        "model_path": "/models/test.gguf",
        "name": "my-agent",
        "slots_processing": 1,
        "slots_total": 4,
        "state_application_status": "Fresh",
        "uses_chat_template_override": True,
    })

    assert snapshot.id == "agent-1"
    assert snapshot.download_filename == "model.gguf"
    assert len(snapshot.issues) == 1
    assert snapshot.issues[0].variant == "SlotCannotStart"
    assert snapshot.state_application_status == AgentStateApplicationStatus.FRESH


def test_agent_controller_pool_snapshot_deserialization() -> None:
    pool = AgentControllerPoolSnapshot.model_validate({
        "agents": [
            {
                "desired_slots_total": 2,
                "download_current": 0,
                "download_total": 0,
                "id": "a1",
                "issues": [],
                "slots_processing": 0,
                "slots_total": 2,
                "state_application_status": "Applied",
                "uses_chat_template_override": False,
            }
        ]
    })

    assert len(pool.agents) == 1
    assert pool.agents[0].id == "a1"


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

    assert dumped["chat_template_override"] == {
        "content": "{{ messages }}"
    }
    assert dumped["use_chat_template_override"] is True


def test_buffered_request_manager_snapshot_deserialization() -> None:
    snapshot = BufferedRequestManagerSnapshot.model_validate(
        {"buffered_requests_current": 12}
    )

    assert snapshot.buffered_requests_current == 12


def test_model_metadata_deserialization() -> None:
    metadata = ModelMetadata.model_validate(
        {"metadata": {"architecture": "llama", "params": "7B"}}
    )

    assert metadata.metadata["architecture"] == "llama"
    assert metadata.metadata["params"] == "7B"


def test_model_metadata_empty() -> None:
    metadata = ModelMetadata()

    assert metadata.metadata == {}


def test_embedding_deserialization() -> None:
    embedding = Embedding.model_validate({
        "embedding": [0.1, 0.2, 0.3],
        "normalization_method": "L2",
        "pooling_type": "Mean",
        "source_document_id": "doc-1",
    })

    assert embedding.embedding == [0.1, 0.2, 0.3]
    assert embedding.normalization_method.variant == "L2"
    assert embedding.pooling_type == PoolingType.MEAN
    assert embedding.source_document_id == "doc-1"


def test_embedding_with_rms_norm_deserialization() -> None:
    embedding = Embedding.model_validate({
        "embedding": [1.0],
        "normalization_method": {"RmsNorm": {"epsilon": 1e-6}},
        "pooling_type": "Cls",
        "source_document_id": "doc-2",
    })

    assert embedding.normalization_method.variant == "RmsNorm"
    assert embedding.normalization_method.epsilon == 1e-6


def test_chat_template_serialization() -> None:
    template = ChatTemplate(content="{% for msg in messages %}...")
    dumped = template.model_dump(mode="json")

    assert dumped == {"content": "{% for msg in messages %}..."}


def test_embedding_input_document_serialization() -> None:
    doc = EmbeddingInputDocument(content="hello world", id="d1")
    dumped = doc.model_dump(mode="json")

    assert dumped == {"content": "hello world", "id": "d1"}


def test_huggingface_model_reference_roundtrip() -> None:
    ref = HuggingFaceModelReference(
        filename="model.gguf",
        repo_id="org/model",
        revision="main",
    )
    dumped = ref.model_dump(mode="json")
    restored = HuggingFaceModelReference.model_validate(dumped)

    assert restored.filename == "model.gguf"
    assert restored.repo_id == "org/model"
    assert restored.revision == "main"
