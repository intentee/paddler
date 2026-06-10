use minijinja::Error;
use minijinja::ErrorKind;
use minijinja::Value;
use minijinja::filters::tojson;
use minijinja::value::Kwargs;

pub fn pyjinja_tojson(value: &Value, kwargs: Kwargs) -> Result<Value, Error> {
    let indent: Option<Value> = kwargs.get("indent")?;

    let ensure_ascii: Option<bool> = kwargs.get("ensure_ascii")?;
    if matches!(ensure_ascii, Some(true)) {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "tojson(ensure_ascii=True) is not supported by minijinja: object output already \
             emits non-ASCII characters unescaped (matching ensure_ascii=False). Drop the \
             kwarg or set it to False.",
        ));
    }

    let sort_keys: Option<bool> = kwargs.get("sort_keys")?;
    if matches!(sort_keys, Some(true)) {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "tojson(sort_keys=True) is not supported by minijinja: object key ordering follows \
             insertion order. Drop the kwarg or set it to False.",
        ));
    }

    if kwargs.has("separators") {
        return Err(Error::new(
            ErrorKind::InvalidOperation,
            "tojson(separators=...) is not supported by minijinja: separator strings are fixed.",
        ));
    }

    kwargs.assert_all_used()?;

    let forwarded_kwargs: Kwargs = Kwargs::from_iter(Vec::<(String, Value)>::new());

    tojson(value, indent, forwarded_kwargs)
}

#[cfg(test)]
mod tests {
    use minijinja::Environment;
    use minijinja::context;

    use super::pyjinja_tojson;

    fn render(template_source: &str, scope: minijinja::Value) -> String {
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

    fn render_error_message(template_source: &str, scope: minijinja::Value) -> String {
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
            .to_string()
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
    fn ensure_ascii_true_returns_error_naming_the_kwarg() {
        let rendered = render_error_message(
            "{{ value | tojson(ensure_ascii=True) }}",
            context! { value => "x" },
        );

        assert!(
            rendered.contains("ensure_ascii=True"),
            "error must name the rejected kwarg; got: {rendered}"
        );
    }

    #[test]
    fn ensure_ascii_non_bool_propagates_kwargs_get_error() {
        let rendered = render_error_message(
            "{{ value | tojson(ensure_ascii='nope') }}",
            context! { value => "x" },
        );

        assert!(
            !rendered.is_empty(),
            "a type-mismatched ensure_ascii kwarg must surface an error"
        );
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
    fn sort_keys_true_returns_error_naming_the_kwarg() {
        let rendered = render_error_message(
            "{{ value | tojson(sort_keys=True) }}",
            context! { value => "x" },
        );

        assert!(
            rendered.contains("sort_keys=True"),
            "error must name the rejected kwarg; got: {rendered}"
        );
    }

    #[test]
    fn sort_keys_non_bool_propagates_kwargs_get_error() {
        let rendered = render_error_message(
            "{{ value | tojson(sort_keys='nope') }}",
            context! { value => "x" },
        );

        assert!(
            !rendered.is_empty(),
            "a type-mismatched sort_keys kwarg must surface an error"
        );
    }

    #[test]
    fn separators_returns_error_naming_the_kwarg() {
        let rendered = render_error_message(
            "{{ value | tojson(separators=[',', ':']) }}",
            context! { value => "x" },
        );

        assert!(
            rendered.contains("separators"),
            "error must name the rejected kwarg; got: {rendered}"
        );
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
    fn unknown_kwarg_returns_error() {
        let rendered =
            render_error_message("{{ value | tojson(bogus=42) }}", context! { value => "x" });

        assert!(
            rendered.contains("bogus"),
            "error must name the unknown kwarg; got: {rendered}"
        );
    }

    #[test]
    fn non_ascii_codepoints_emitted_unescaped() {
        let result = render("{{ value | tojson }}", context! { value => "日本語" });

        assert_eq!(result, "\"日本語\"");
    }
}
