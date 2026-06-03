use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::ToolCallArguments;
use paddler_messaging::generation_summary::GenerationSummary;
use ratatui::Frame;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Wrap;

use crate::streaming_response::StreamingResponse;
use crate::view_panel_kind::ViewPanelKind;
use crate::view_panel_layout::ViewPanelLayout;
use crate::view_panel_navigation::ViewPanelNavigation;

const TOKEN_PALETTE: [Color; 6] = [
    Color::LightCyan,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightRed,
];

pub fn view_chat_panels(
    state: &StreamingResponse,
    navigation: &mut ViewPanelNavigation,
    layout: &ViewPanelLayout,
    frame: &mut Frame<'_>,
) {
    render_panel_text(
        frame,
        layout.thinking,
        ViewPanelKind::Thinking,
        Text::from(build_colored_token_lines(&state.thinking)),
        navigation,
    );
    render_panel_text(
        frame,
        layout.response,
        ViewPanelKind::Response,
        Text::from(build_colored_token_lines(&state.response)),
        navigation,
    );
    render_panel_text(
        frame,
        layout.tool_calls,
        ViewPanelKind::ToolCalls,
        Text::from(build_tool_calls_lines(
            &state.tool_call_tokens,
            &state.tool_calls,
            Block::bordered().inner(layout.tool_calls).width,
        )),
        navigation,
    );
    render_panel_text(
        frame,
        layout.undetermined,
        ViewPanelKind::Undetermined,
        Text::from(build_colored_token_lines(&state.undetermined)),
        navigation,
    );
    render_status_bar(frame, layout.status_bar, state);
}

fn render_panel_text(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: ViewPanelKind,
    text: Text<'_>,
    navigation: &mut ViewPanelNavigation,
) {
    let title = if navigation.focused() == panel {
        format!("[ {} ]", panel.label())
    } else {
        format!(" {} ", panel.label())
    };
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    let visible_rows = count_text_rows(&text, inner.width);
    navigation.settle(panel, visible_rows.into(), inner.height.into());
    let position = navigation.position(panel);
    let scroll_offset = u16::try_from(position).unwrap_or(u16::MAX);

    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0))
        .block(block);
    frame.render_widget(paragraph, area);

    if visible_rows > inner.height {
        let mut scrollbar_state = ScrollbarState::new(visible_rows.into())
            .position(position)
            .viewport_content_length(inner.height.into());
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("┃")
            .track_symbol(Some("│"))
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn build_colored_token_lines(tokens: &[String]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut had_token = false;

    for (token_index, token) in tokens.iter().enumerate() {
        if token.is_empty() {
            continue;
        }
        had_token = true;
        let is_whitespace_only = token.chars().all(char::is_whitespace);
        let style = if is_whitespace_only {
            Style::default()
                .bg(palette_color(token_index))
                .fg(Color::Black)
        } else {
            Style::default().fg(palette_color(token_index))
        };
        for (piece_index, piece) in token.split('\n').enumerate() {
            if piece_index > 0 {
                if is_whitespace_only {
                    current_line.push(Span::styled("↵", style));
                }
                lines.push(Line::from(std::mem::take(&mut current_line)));
            }
            if !piece.is_empty() {
                let rendered = if is_whitespace_only {
                    piece.replace('\t', "→")
                } else {
                    piece.to_owned()
                };
                if !rendered.is_empty() {
                    current_line.push(Span::styled(rendered, style));
                }
            }
        }
    }

    if had_token {
        lines.push(Line::from(current_line));
    }

    lines
}

fn build_tool_calls_lines(
    token_stream: &[String],
    parsed_calls: &[ParsedToolCall],
    inner_width: u16,
) -> Vec<Line<'static>> {
    let mut lines = build_colored_token_lines(token_stream);
    let has_tokens = !token_stream.is_empty();
    let has_parsed = !parsed_calls.is_empty();
    if has_tokens && has_parsed {
        lines.push(divider_line(inner_width));
    }
    lines.extend(parsed_call_lines(parsed_calls));
    lines
}

fn divider_line(width: u16) -> Line<'static> {
    let label = " parsed ";
    let label_width = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
    let total = width.max(label_width.saturating_add(2));
    let dash_count = total.saturating_sub(label_width);
    let left = dash_count / 2;
    let right = dash_count - left;
    let mut text = String::new();
    for _ in 0..left {
        text.push('─');
    }
    text.push_str(label);
    for _ in 0..right {
        text.push('─');
    }
    Line::from(Span::styled(text, Style::default().fg(Color::Gray)))
}

fn parsed_call_lines(calls: &[ParsedToolCall]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for call in calls {
        lines.push(Line::raw(call.name.clone()));
        match &call.arguments {
            ToolCallArguments::ValidJson(value) => match serde_json::to_string_pretty(value) {
                Ok(formatted) => {
                    for inner_line in formatted.lines() {
                        lines.push(Line::raw(format!("  {inner_line}")));
                    }
                }
                Err(format_error) => {
                    log::error!(
                        "failed to pretty-print tool-call arguments for {name}: {format_error}",
                        name = call.name
                    );
                    lines.push(Line::raw(format!("  {value}")));
                }
            },
            ToolCallArguments::InvalidJson(raw) => {
                lines.push(Line::raw(format!("  invalid JSON: {raw}")));
            }
        }
    }
    lines
}

const fn palette_color(token_index: usize) -> Color {
    TOKEN_PALETTE[token_index % TOKEN_PALETTE.len()]
}

fn count_text_rows(text: &Text<'_>, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    let mut total: u16 = 0;
    for line in &text.lines {
        let chars_count: usize = line
            .spans
            .iter()
            .map(|span| span.content.chars().count())
            .sum();
        let chars = u16::try_from(chars_count).unwrap_or(u16::MAX);
        let rows = chars.div_ceil(width).max(1);
        total = total.saturating_add(rows);
    }
    total
}

fn render_status_bar(frame: &mut Frame<'_>, area: Rect, state: &StreamingResponse) {
    let text = match (&state.stop_reason, &state.summary) {
        (None, _) => {
            "generating… · tab/shift-tab focus · ↑↓ pgup/pgdn home/end scroll · q quit".to_owned()
        }
        (Some(_), Some(summary)) => format_completion_status(summary),
        (Some(reason), None) => format!("stopped — {reason} · press q to quit"),
    };
    frame.render_widget(Paragraph::new(text), area);
}

fn format_completion_status(summary: &GenerationSummary) -> String {
    let usage = summary.usage;
    format!(
        "done · response {response} · thinking {thinking} · tools {tools} · undet {undet} · prompt {prompt} · total {total} · press q to quit",
        response = usage.content_tokens,
        thinking = usage.reasoning_tokens,
        tools = usage.tool_call_tokens,
        undet = usage.undeterminable_tokens,
        prompt = usage.prompt_tokens,
        total = usage.total_tokens(),
    )
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_messaging::generation_summary::GenerationSummary;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::stop_reason::StopReason;

    fn render_to_string(state: &StreamingResponse, width: u16, height: u16) -> Result<String> {
        let mut navigation = ViewPanelNavigation::default();
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|frame| {
            let layout = ViewPanelLayout::compute(frame.area());
            view_chat_panels(state, &mut navigation, &layout, frame);
        })?;
        Ok(buffer_text(terminal.backend().buffer()))
    }

    fn buffer_text(buffer: &Buffer) -> String {
        let area = buffer.area;
        let mut output = String::with_capacity((area.width as usize + 1) * area.height as usize);
        for y in 0..area.height {
            for x in 0..area.width {
                output.push_str(buffer[(x, y)].symbol());
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn empty_state_shows_all_four_panels_and_generating_status() -> Result<()> {
        let state = StreamingResponse::default();

        let rendered = render_to_string(&state, 100, 30)?;

        assert!(rendered.contains("Thinking"));
        assert!(rendered.contains("Response"));
        assert!(rendered.contains("Tool Calls"));
        assert!(rendered.contains("Undetermined"));
        assert!(rendered.contains("generating"));
        assert!(!rendered.contains("done"));
        Ok(())
    }

    #[test]
    fn focused_panel_title_uses_brackets() -> Result<()> {
        let state = StreamingResponse::default();

        let rendered = render_to_string(&state, 100, 30)?;

        assert!(rendered.contains("[ Response ]"));
        assert!(rendered.contains(" Thinking "));
        Ok(())
    }

    #[test]
    fn response_buffer_text_is_visible() -> Result<()> {
        let mut state = StreamingResponse::default();
        state.response.push("hello world".to_owned());

        let rendered = render_to_string(&state, 80, 30)?;

        assert!(rendered.contains("hello world"));
        Ok(())
    }

    #[test]
    fn completed_state_shows_summary_and_quit_hint() -> Result<()> {
        let state = StreamingResponse {
            summary: Some(GenerationSummary::default()),
            stop_reason: Some(StopReason::Completed),
            ..StreamingResponse::default()
        };

        let rendered = render_to_string(&state, 140, 30)?;

        assert!(rendered.contains("done"));
        assert!(rendered.contains("press q to quit"));
        assert!(!rendered.contains("generating"));
        Ok(())
    }

    #[test]
    fn whitespace_only_newline_token_renders_return_marker() -> Result<()> {
        let mut state = StreamingResponse::default();
        state.response.push("hello".to_owned());
        state.response.push("\n".to_owned());
        state.response.push("world".to_owned());

        let rendered = render_to_string(&state, 80, 30)?;

        assert!(rendered.contains("↵"));
        Ok(())
    }

    #[test]
    fn tool_calls_panel_shows_divider_when_tokens_and_parsed_both_present() -> Result<()> {
        let mut state = StreamingResponse::default();
        state
            .tool_call_tokens
            .push("{\"name\":\"calc\"}".to_owned());
        state.tool_calls.push(ParsedToolCall::default());

        let rendered = render_to_string(&state, 120, 30)?;

        assert!(rendered.contains("parsed"));
        Ok(())
    }
}
