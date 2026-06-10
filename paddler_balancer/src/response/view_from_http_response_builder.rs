use actix_web::HttpResponse;
use actix_web::HttpResponseBuilder;
use actix_web::Result;
use actix_web::error::ErrorInternalServerError;
use askama::Template;

pub fn view_from_http_response_builder<TTemplate: Template>(
    mut http_response_builder: HttpResponseBuilder,
    template: TTemplate,
) -> Result<HttpResponse> {
    let rendered = template.render().map_err(ErrorInternalServerError)?;

    Ok(http_response_builder
        .content_type("text/html; charset=utf-8")
        .body(rendered))
}

#[cfg(test)]
mod tests {
    use std::fmt;
    use std::mem::discriminant;

    use actix_web::HttpResponse;
    use actix_web::http::StatusCode;
    use actix_web::http::header::CONTENT_TYPE;
    use askama::Error as AskamaError;
    use askama::FastWritable;
    use askama::Template;
    use askama::Values;

    use super::view_from_http_response_builder;

    struct FailingWriter;

    impl fmt::Write for FailingWriter {
        fn write_str(&mut self, _content: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    struct RenderingTemplate;

    impl fmt::Display for RenderingTemplate {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.render_into(formatter)
                .map_err(|_askama_error| fmt::Error)
        }
    }

    impl FastWritable for RenderingTemplate {
        fn write_into<TWriter: fmt::Write + ?Sized>(
            &self,
            destination: &mut TWriter,
            values: &dyn Values,
        ) -> askama::Result<()> {
            self.render_into_with_values(destination, values)
        }
    }

    impl Template for RenderingTemplate {
        const SIZE_HINT: usize = 0;

        fn render_into_with_values<TWriter: fmt::Write + ?Sized>(
            &self,
            writer: &mut TWriter,
            _values: &dyn Values,
        ) -> askama::Result<()> {
            writer.write_str("<p>rendered</p>")?;

            Ok(())
        }
    }

    struct FailingTemplate;

    impl fmt::Display for FailingTemplate {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.render_into(formatter)
                .map_err(|_askama_error| fmt::Error)
        }
    }

    impl FastWritable for FailingTemplate {
        fn write_into<TWriter: fmt::Write + ?Sized>(
            &self,
            destination: &mut TWriter,
            values: &dyn Values,
        ) -> askama::Result<()> {
            self.render_into_with_values(destination, values)
        }
    }

    impl Template for FailingTemplate {
        const SIZE_HINT: usize = 0;

        fn render_into_with_values<TWriter: fmt::Write + ?Sized>(
            &self,
            _writer: &mut TWriter,
            _values: &dyn Values,
        ) -> askama::Result<()> {
            Err(AskamaError::ValueMissing)
        }
    }

    #[test]
    fn renders_template_into_ok_html_response() {
        let http_response =
            view_from_http_response_builder(HttpResponse::Ok(), RenderingTemplate).unwrap();

        assert_eq!(http_response.status(), StatusCode::OK);
        assert_eq!(
            http_response
                .headers()
                .get(CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "text/html; charset=utf-8",
        );
    }

    #[test]
    fn maps_render_failure_to_internal_server_error() {
        let render_error =
            view_from_http_response_builder(HttpResponse::Ok(), FailingTemplate).unwrap_err();

        assert_eq!(
            render_error.as_response_error().status_code(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }

    #[test]
    fn displays_rendering_template_markup() {
        assert_eq!(RenderingTemplate.to_string(), "<p>rendered</p>");
    }

    #[test]
    fn displays_failing_template_as_fmt_error() {
        use std::fmt::Write as _;

        let mut destination = String::new();
        let display_error = write!(destination, "{FailingTemplate}").err().unwrap();

        assert_eq!(display_error, fmt::Error);
    }

    #[test]
    fn rendering_template_fast_writable_writes_markup() {
        let mut destination = String::new();

        FastWritable::write_into(&RenderingTemplate, &mut destination, askama::NO_VALUES).unwrap();

        assert_eq!(destination, "<p>rendered</p>");
    }

    #[test]
    fn failing_template_fast_writable_propagates_value_missing() {
        let mut destination = String::new();

        let write_error =
            FastWritable::write_into(&FailingTemplate, &mut destination, askama::NO_VALUES)
                .err()
                .unwrap();

        assert_eq!(
            discriminant(&write_error),
            discriminant(&AskamaError::ValueMissing),
        );
    }

    #[test]
    fn rendering_template_maps_writer_failure_to_fmt_error() {
        let mut failing_writer = FailingWriter;

        let write_error = RenderingTemplate
            .render_into_with_values(&mut failing_writer, askama::NO_VALUES)
            .err()
            .unwrap();

        assert_eq!(discriminant(&write_error), discriminant(&AskamaError::Fmt),);
    }
}
