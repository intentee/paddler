use std::fs::read_to_string;

use serde::Deserialize;
use serde_json::Value;
use serde_json::from_str;
use serde_json::from_value;
use serde_json::to_value;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_typesafe_translation::system_one_request::SystemOneRequest;

#[derive(Deserialize)]
struct RenderedQuestion {
    instructions: String,
    options: Vec<String>,
}

#[derive(Deserialize)]
struct Rendered {
    questions: Vec<RenderedQuestion>,
    state: String,
}

#[derive(Deserialize)]
struct TypeSafeReference {
    answers: Value,
    probabilities: Vec<Vec<f32>>,
    rendered: Rendered,
    request: Value,
}

#[test]
fn translation_matches_the_kev_typesafe_reference() {
    let references: Vec<TypeSafeReference> = from_str(
        &read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/typesafe_reference.json"
        ))
        .expect("the TypeSafe reference must be readable"),
    )
    .expect("the TypeSafe reference must parse");

    for TypeSafeReference {
        answers,
        probabilities,
        rendered,
        request,
    } in references
    {
        let translated = from_value::<SystemOneRequest>(request)
            .expect("the reference request must parse")
            .translate()
            .expect("the reference request must translate");

        assert_eq!(translated.decide_params.state, rendered.state);
        let translated_questions: Vec<_> = translated
            .decide_params
            .leading_questions
            .iter()
            .chain([&translated.decide_params.last_question])
            .collect();

        assert_eq!(
            translated_questions
                .iter()
                .map(|question| (question.instructions.clone(), question.options.clone()))
                .collect::<Vec<_>>(),
            rendered
                .questions
                .into_iter()
                .map(|question| (question.instructions, question.options))
                .collect::<Vec<_>>()
        );

        let decision_answers: Vec<DecisionAnswer> = translated_questions
            .iter()
            .zip(probabilities)
            .map(|(question, probabilities)| DecisionAnswer {
                id: question.id.clone(),
                probabilities,
            })
            .collect();
        let response = translated
            .respond(
                &decision_answers,
                &DecisionSummary {
                    input_tokens: 1,
                    processing_milliseconds: 1,
                },
            )
            .expect("the reference answers must translate");

        assert_eq!(
            to_value(response.answers).expect("the answers must serialize"),
            answers
        );
    }
}
