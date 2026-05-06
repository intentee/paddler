use llama_cpp_bindings::SampledToken;

pub struct ClassifiedToken {
    pub sampled_token: SampledToken,
    pub was_in_tool_call: bool,
    pub is_in_tool_call: bool,
    /// User-visible decoded piece. Empty when this token is part of a marker
    /// (e.g. `</think>` or `[/THINK]`) — emit phases must skip emission for
    /// empty pieces so marker text never reaches client streams.
    pub visible_piece: String,
}
