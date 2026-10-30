use paddler_messaging::decision_question::DecisionQuestion;

use crate::system_one_answer_layout::SystemOneAnswerLayout;

pub struct TranslatedSystemOneQuestion {
    pub answer_layout: SystemOneAnswerLayout,
    pub decision_question: DecisionQuestion,
}
