use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;

fn question(id: &str, instructions: &str, options: &[&str]) -> DecisionQuestion {
    DecisionQuestion {
        id: id.to_owned(),
        instructions: instructions.to_owned(),
        options: options.iter().map(|option| (*option).to_owned()).collect(),
    }
}

#[must_use]
pub fn sample_decision() -> RawDecideParams {
    RawDecideParams {
        questions: vec![
            question("paid", "Was the invoice paid?", &["no", "yes"]),
            question(
                "sentiment",
                "How does the customer feel?",
                &["positive", "neutral", "negative: the customer complains"],
            ),
            question(
                "urgency",
                "How urgent is the follow-up?",
                &["none", "low", "medium", "high"],
            ),
        ],
        state: "Ada paid the invoice for order 1042 two days late and wrote that the delivery was \
                slow, but the support team was friendly."
            .to_owned(),
    }
}
