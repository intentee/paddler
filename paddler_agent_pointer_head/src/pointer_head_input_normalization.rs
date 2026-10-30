use std::borrow::Cow;

use icu_normalizer::ComposingNormalizerBorrowed;
use icu_normalizer::DecomposingNormalizerBorrowed;
use llama_cpp_bindings::gguf_context::GgufContext;

use crate::pointer_head_error::PointerHeadError;

const INPUT_NORMALIZATION_KEY: &str = "pointer_head.input_normalization";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerHeadInputNormalization {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
    Unnormalized,
}

impl PointerHeadInputNormalization {
    pub fn load(gguf_context: &GgufContext) -> Result<Self, PointerHeadError> {
        gguf_context
            .find_key(INPUT_NORMALIZATION_KEY)
            .and_then(|key_id| gguf_context.val_str(key_id))
            .map_err(|source| PointerHeadError::KeyUnreadable {
                key: INPUT_NORMALIZATION_KEY.to_owned(),
                source,
            })
            .and_then(Self::try_from)
    }

    #[must_use]
    pub fn normalize<'text>(&self, text: &'text str) -> Cow<'text, str> {
        match self {
            Self::Nfc => ComposingNormalizerBorrowed::new_nfc().normalize(text),
            Self::Nfd => DecomposingNormalizerBorrowed::new_nfd().normalize(text),
            Self::Nfkc => ComposingNormalizerBorrowed::new_nfkc().normalize(text),
            Self::Nfkd => DecomposingNormalizerBorrowed::new_nfkd().normalize(text),
            Self::Unnormalized => Cow::Borrowed(text),
        }
    }
}

impl TryFrom<&str> for PointerHeadInputNormalization {
    type Error = PointerHeadError;

    fn try_from(declared: &str) -> Result<Self, Self::Error> {
        match declared {
            "NFC" => Ok(Self::Nfc),
            "NFD" => Ok(Self::Nfd),
            "NFKC" => Ok(Self::Nfkc),
            "NFKD" => Ok(Self::Nfkd),
            "None" => Ok(Self::Unnormalized),
            unknown => Err(PointerHeadError::InputNormalizationUnknown {
                declared: unknown.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PointerHeadInputNormalization;
    use crate::pointer_head_error::PointerHeadError;

    #[test]
    fn every_declarable_form_is_read_by_its_name() {
        assert_eq!(
            ["NFC", "NFD", "NFKC", "NFKD", "None"]
                .map(PointerHeadInputNormalization::try_from)
                .map(Result::ok),
            [
                Some(PointerHeadInputNormalization::Nfc),
                Some(PointerHeadInputNormalization::Nfd),
                Some(PointerHeadInputNormalization::Nfkc),
                Some(PointerHeadInputNormalization::Nfkd),
                Some(PointerHeadInputNormalization::Unnormalized),
            ]
        );
    }

    #[test]
    fn a_form_a_pointer_head_cannot_declare_is_refused() {
        assert!(matches!(
            PointerHeadInputNormalization::try_from("NFX"),
            Err(PointerHeadError::InputNormalizationUnknown { declared }) if declared == "NFX"
        ));
    }

    #[test]
    fn each_declared_form_normalizes_text_the_way_it_names() {
        let decomposed_e_acute_and_fi_ligature = "e\u{301}\u{fb01}";

        assert_eq!(
            [
                PointerHeadInputNormalization::Nfc,
                PointerHeadInputNormalization::Nfd,
                PointerHeadInputNormalization::Nfkc,
                PointerHeadInputNormalization::Nfkd,
                PointerHeadInputNormalization::Unnormalized,
            ]
            .map(|input_normalization| {
                input_normalization
                    .normalize(decomposed_e_acute_and_fi_ligature)
                    .into_owned()
            }),
            [
                "\u{e9}\u{fb01}".to_owned(),
                "e\u{301}\u{fb01}".to_owned(),
                "\u{e9}fi".to_owned(),
                "e\u{301}fi".to_owned(),
                "e\u{301}\u{fb01}".to_owned(),
            ]
        );
    }
}
