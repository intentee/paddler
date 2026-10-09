use minijinja::Error;
use minijinja::ErrorKind;
use minijinja::Value;
use minijinja::filters::tojson;
use minijinja::value::Kwargs;

const ENSURE_ASCII_UNSUPPORTED: &str = "tojson(ensure_ascii=True) is not supported by minijinja: object output already \
     emits non-ASCII characters unescaped (matching ensure_ascii=False). Drop the \
     kwarg or set it to False.";

const SORT_KEYS_UNSUPPORTED: &str = "tojson(sort_keys=True) is not supported by minijinja: object key ordering follows \
     insertion order. Drop the kwarg or set it to False.";

const SEPARATORS_UNSUPPORTED: &str =
    "tojson(separators=...) is not supported by minijinja: separator strings are fixed.";

pub fn pyjinja_tojson(value: &Value, kwargs: Kwargs) -> Result<Value, Error> {
    let ensure_ascii: Option<bool> = kwargs.get("ensure_ascii")?;
    if matches!(ensure_ascii, Some(true)) {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            ENSURE_ASCII_UNSUPPORTED,
        ));
    }

    let sort_keys: Option<bool> = kwargs.get("sort_keys")?;
    if matches!(sort_keys, Some(true)) {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            SORT_KEYS_UNSUPPORTED,
        ));
    }

    if kwargs.has("separators") {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            SEPARATORS_UNSUPPORTED,
        ));
    }

    tojson(value, None, kwargs)
}

#[cfg(test)]
mod tests {
    use minijinja::Environment;
    use minijinja::Error;
    use minijinja::ErrorKind;
    use minijinja::Value;
    use minijinja::context;

    use super::ENSURE_ASCII_UNSUPPORTED;
    use super::SEPARATORS_UNSUPPORTED;
    use super::SORT_KEYS_UNSUPPORTED;
    use super::pyjinja_tojson;

    fn render(template_source: &str, scope: Value) -> String {
        let mut environment = Environment::new();
        environment.add_filter("tojson", pyjinja_tojson);
        environment
            .add_template_owned("t", template_source.to_owned())
            .unwrap();

        environment
            .get_template("t")
            .unwrap()
            .render(scope)
            .unwrap()
    }

    fn render_error(template_source: &str, scope: Value) -> Error {
        let mut environment = Environment::new();
        environment.add_filter("tojson", pyjinja_tojson);
        environment
            .add_template_owned("t", template_source.to_owned())
            .unwrap();

        environment
            .get_template("t")
            .unwrap()
            .render(scope)
            .unwrap_err()
    }

    #[test]
    fn no_kwargs_emits_quoted_json_string() {
        let result = render("{{ value | tojson }}", context! { value => "hello" });

        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn ensure_ascii_false_matches_default_output() {
        let with_kwarg = render(
            "{{ value | tojson(ensure_ascii=False) }}",
            context! { value => "café" },
        );
        let without_kwarg = render("{{ value | tojson }}", context! { value => "café" });

        assert_eq!(with_kwarg, without_kwarg);
        assert_eq!(with_kwarg, "\"café\"");
    }

    #[test]
    fn ensure_ascii_true_is_rejected_as_unsupported() {
        let error = render_error(
            "{{ value | tojson(ensure_ascii=True) }}",
            context! { value => "x" },
        );

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert_eq!(error.detail(), Some(ENSURE_ASCII_UNSUPPORTED));
    }

    #[test]
    fn ensure_ascii_non_bool_propagates_kwargs_get_error() {
        let error = render_error(
            "{{ value | tojson(ensure_ascii='nope') }}",
            context! { value => "x" },
        );

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert_eq!(error.detail(), Some("cannot convert string to bool"));
    }

    #[test]
    fn sort_keys_false_matches_default_output() {
        let with_kwarg = render(
            "{{ value | tojson(sort_keys=False) }}",
            context! { value => "x" },
        );

        assert_eq!(with_kwarg, "\"x\"");
    }

    #[test]
    fn sort_keys_true_is_rejected_as_unsupported() {
        let error = render_error(
            "{{ value | tojson(sort_keys=True) }}",
            context! { value => "x" },
        );

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert_eq!(error.detail(), Some(SORT_KEYS_UNSUPPORTED));
    }

    #[test]
    fn sort_keys_non_bool_propagates_kwargs_get_error() {
        let error = render_error(
            "{{ value | tojson(sort_keys='nope') }}",
            context! { value => "x" },
        );

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert_eq!(error.detail(), Some("cannot convert string to bool"));
    }

    #[test]
    fn separators_are_rejected_as_unsupported() {
        let error = render_error(
            "{{ value | tojson(separators=[',', ':']) }}",
            context! { value => "x" },
        );

        assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        assert_eq!(error.detail(), Some(SEPARATORS_UNSUPPORTED));
    }

    #[test]
    fn indent_kwarg_emits_pretty_printed_json() {
        let result = render(
            "{{ value | tojson(indent=2) }}",
            context! { value => context! { k => "v" } },
        );

        assert_eq!(result, "{\n  \"k\": \"v\"\n}");
    }

    #[test]
    fn indent_kwarg_combines_with_ensure_ascii_false() {
        let result = render(
            "{{ value | tojson(ensure_ascii=False, indent=2) }}",
            context! { value => context! { k => "café" } },
        );

        assert_eq!(result, "{\n  \"k\": \"café\"\n}");
    }

    #[test]
    fn unknown_kwarg_is_rejected_as_an_extra_argument() {
        let error = render_error("{{ value | tojson(bogus=42) }}", context! { value => "x" });

        assert_eq!(error.kind(), ErrorKind::TooManyArguments);
        assert_eq!(error.detail(), Some("unknown keyword argument 'bogus'"));
    }

    #[test]
    fn non_ascii_codepoints_emitted_unescaped() {
        let result = render("{{ value | tojson }}", context! { value => "日本語" });

        assert_eq!(result, "\"日本語\"");
    }
}
