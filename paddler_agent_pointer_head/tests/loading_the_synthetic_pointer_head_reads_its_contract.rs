use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_delimiters::PointerHeadDelimiters;

use crate::fixture_path::fixture_path;

#[test]
fn loading_the_synthetic_pointer_head_reads_its_contract() {
    let pointer_head = PointerHead::load(&fixture_path("qwen3_5_0_8b_synthetic_pointer_head.gguf"))
        .expect("the synthetic pointer head must load");

    assert_eq!(pointer_head.hidden_size(), 1024);
    assert_eq!(pointer_head.key_projection.bias.len(), 16);
    assert_eq!(pointer_head.query_projection.weight.len(), 16 * 1024);
    assert!((pointer_head.logit_scale - 0.25).abs() < f32::EPSILON);
    assert!((pointer_head.temperature - 1.5).abs() < f32::EPSILON);
    assert_eq!(
        pointer_head.delimiters,
        PointerHeadDelimiters {
            decide: "<|fim_suffix|>".to_owned(),
            option_end: "<|box_end|>".to_owned(),
            option_start: "<|box_start|>".to_owned(),
            question: "<|fim_middle|>".to_owned(),
            state: "<|fim_prefix|>".to_owned(),
        }
    );
}
