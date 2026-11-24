use crate::constants::UNENDING_GENERATION_CYCLE;

#[must_use]
pub fn is_unending_generation_text(text: &str) -> bool {
    text.bytes()
        .eq(UNENDING_GENERATION_CYCLE.bytes().cycle().take(text.len()))
}

#[cfg(test)]
mod tests {
    use super::is_unending_generation_text;

    #[test]
    fn accepts_text_that_follows_the_cycle_without_a_gap() {
        assert!(is_unending_generation_text(
            "apple banana cherry apple banana cherry app"
        ));
    }

    #[test]
    fn rejects_text_that_lost_a_token_of_the_cycle() {
        assert!(!is_unending_generation_text(
            "apple banana cherry apple cherry "
        ));
    }
}
