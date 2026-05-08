use minijinja::Error;
use minijinja::ErrorKind;
use minijinja::Value;
use minijinja::filters::tojson;
use minijinja::value::Kwargs;

// Python-style `tojson` filter compatible with HuggingFace transformers chat
// templates that pass Jinja2 kwargs (`ensure_ascii`, `sort_keys`, `separators`,
// `indent`). minijinja's built-in `tojson` only accepts `indent`, so any of the
// others crashes rendering with "too many arguments". This wrapper:
//
// 1. Recognises every Python `tojson` kwarg explicitly (whitelist).
// 2. Accepts only values whose semantics match minijinja's defaults
//    (`ensure_ascii=False`, `sort_keys=False`); rejects anything else with a
//    clear error so the template author knows to remove it.
// 3. Forwards `indent` plus an empty kwargs map to minijinja's built-in
//    `tojson` for the actual JSON serialisation, so behaviour and output
//    formatting stay identical.
// 4. Calls `Kwargs::assert_all_used` so unknown kwargs (anything not in our
//    whitelist) hard-error rather than getting silently dropped.
#[expect(
    clippy::needless_pass_by_value,
    reason = "minijinja's Filter trait requires Kwargs by value; taking &Kwargs makes the \
              function unregisterable as a filter"
)]
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

    let separators: Option<Value> = kwargs.get("separators")?;
    if separators.is_some() {
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
    use anyhow::Result;
    use anyhow::anyhow;
    use minijinja::Environment;
    use minijinja::context;

    use super::pyjinja_tojson;

    fn render(template_source: &str, scope: minijinja::Value) -> Result<String> {
        let mut env = Environment::new();
        env.add_filter("tojson", pyjinja_tojson);
        env.add_template_owned("t", template_source.to_owned())?;
        Ok(env.get_template("t")?.render(scope)?)
    }

    fn render_expecting_error(
        template_source: &str,
        scope: minijinja::Value,
    ) -> Result<minijinja::Error> {
        let mut env = Environment::new();
        env.add_filter("tojson", pyjinja_tojson);
        env.add_template_owned("t", template_source.to_owned())?;
        let outcome = env.get_template("t")?.render(scope);

        outcome.err().ok_or_else(|| anyhow!("expected Err, got Ok"))
    }

    #[test]
    fn no_kwargs_emits_quoted_json_string() -> Result<()> {
        let result = render("{{ value | tojson }}", context! { value => "hello" })?;

        assert_eq!(result, "\"hello\"");

        Ok(())
    }

    #[test]
    fn ensure_ascii_false_matches_default_output() -> Result<()> {
        let with_kwarg = render(
            "{{ value | tojson(ensure_ascii=False) }}",
            context! { value => "café" },
        )?;
        let without_kwarg = render("{{ value | tojson }}", context! { value => "café" })?;

        assert_eq!(with_kwarg, without_kwarg);
        assert_eq!(with_kwarg, "\"café\"");

        Ok(())
    }

    #[test]
    fn ensure_ascii_true_returns_error_naming_the_kwarg() -> Result<()> {
        let err = render_expecting_error(
            "{{ value | tojson(ensure_ascii=True) }}",
            context! { value => "x" },
        )?;
        let rendered = err.to_string();

        if !rendered.contains("ensure_ascii=True") {
            return Err(anyhow!(
                "error must name the rejected kwarg; got: {rendered}"
            ));
        }

        Ok(())
    }

    #[test]
    fn sort_keys_false_matches_default_output() -> Result<()> {
        let with_kwarg = render(
            "{{ value | tojson(sort_keys=False) }}",
            context! { value => "x" },
        )?;

        assert_eq!(with_kwarg, "\"x\"");

        Ok(())
    }

    #[test]
    fn sort_keys_true_returns_error_naming_the_kwarg() -> Result<()> {
        let err = render_expecting_error(
            "{{ value | tojson(sort_keys=True) }}",
            context! { value => "x" },
        )?;
        let rendered = err.to_string();

        if !rendered.contains("sort_keys=True") {
            return Err(anyhow!(
                "error must name the rejected kwarg; got: {rendered}"
            ));
        }

        Ok(())
    }

    #[test]
    fn separators_returns_error_naming_the_kwarg() -> Result<()> {
        let err = render_expecting_error(
            "{{ value | tojson(separators=[',', ':']) }}",
            context! { value => "x" },
        )?;
        let rendered = err.to_string();

        if !rendered.contains("separators") {
            return Err(anyhow!(
                "error must name the rejected kwarg; got: {rendered}"
            ));
        }

        Ok(())
    }

    #[test]
    fn indent_kwarg_emits_pretty_printed_json() -> Result<()> {
        let result = render(
            "{{ value | tojson(indent=2) }}",
            context! { value => context! { k => "v" } },
        )?;

        assert_eq!(result, "{\n  \"k\": \"v\"\n}");

        Ok(())
    }

    #[test]
    fn indent_kwarg_combines_with_ensure_ascii_false() -> Result<()> {
        let result = render(
            "{{ value | tojson(ensure_ascii=False, indent=2) }}",
            context! { value => context! { k => "café" } },
        )?;

        assert_eq!(result, "{\n  \"k\": \"café\"\n}");

        Ok(())
    }

    #[test]
    fn unknown_kwarg_returns_error() -> Result<()> {
        let err = render_expecting_error(
            "{{ value | tojson(bogus=42) }}",
            context! { value => "x" },
        )?;
        let rendered = err.to_string();

        if !rendered.contains("bogus") {
            return Err(anyhow!(
                "error must name the unknown kwarg; got: {rendered}"
            ));
        }

        Ok(())
    }

    #[test]
    fn non_ascii_codepoints_emitted_unescaped() -> Result<()> {
        let result = render("{{ value | tojson }}", context! { value => "日本語" })?;

        assert_eq!(result, "\"日本語\"");

        Ok(())
    }
}
