use percent_encoding::AsciiSet;
use percent_encoding::CONTROLS;

pub const AGENT_LOCAL_MODEL_PATH_SCHEME: &str = "agent";
pub const HUGGINGFACE_DEFAULT_ENDPOINT: &str = "https://huggingface.co";
pub const URL_PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'\\')
    .add(b'`')
    .add(b'{')
    .add(b'}');
