use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_decision::tokenized_decision_question::TokenizedDecisionQuestion;

#[must_use]
pub fn two_option_question(id: &str) -> TokenizedDecisionQuestion {
    TokenizedDecisionQuestion {
        id: id.to_owned(),
        instructions: vec![LlamaToken::new(0)],
        options: vec![vec![LlamaToken::new(1)], vec![LlamaToken::new(2)]],
    }
}
