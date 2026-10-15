use llama_cpp_bindings::error::MetaValError;
use llama_cpp_bindings::model::LlamaModel;

use paddler_agent_decision::reads_model_architecture::ReadsModelArchitecture;

pub const UNREADABLE_ARCHITECTURE_RETURN_CODE: i32 = -1;

pub struct ModelArchitectureThatIsUnreadable;

impl ReadsModelArchitecture for ModelArchitectureThatIsUnreadable {
    fn model_architecture(&self, _model: &LlamaModel) -> Result<String, MetaValError> {
        Err(MetaValError::NegativeReturn(
            UNREADABLE_ARCHITECTURE_RETURN_CODE,
        ))
    }
}
