use llama_cpp_bindings_types::ParsedToolCall;
use llama_cpp_bindings_types::ToolCallArguments;
use paddler_types::generation_summary::GenerationSummary;
use ratatui::Frame;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Wrap;

use crate::chat_panel_layout::ChatPanelLayout;
use crate::panel_kind::PanelKind;
use crate::panel_navigation::PanelNavigation;
use crate::streaming_response::StreamingResponse;

pub fn render_chat_panels(
    state: &StreamingResponse,
    navigation: &mut PanelNavigation,
    layout: &ChatPanelLayout,
    frame: &mut Frame<'_>,
) {
    render_text_panel(
        frame,
        layout.thinking,
        PanelKind::Thinking,
        &state.thinking,
        navigation,
    );
    render_text_panel(
        frame,
        layout.response,
        PanelKind::Response,
        &state.response,
        navigation,
    );
    let formatted_tool_calls =
        format_tool_calls(&state.tool_calls, &state.pending_tool_call_buffer);
    render_text_panel(
        frame,
        layout.tool_calls,
        PanelKind::ToolCalls,
        &formatted_tool_calls,
        navigation,
    );
    render_text_panel(
        frame,
        layout.undetermined,
        PanelKind::Undetermined,
        &state.undetermined,
        navigation,
    );
    render_status_bar(frame, layout.status_bar, state);
}

fn render_text_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    panel: PanelKind,
    buffer: &str,
    navigation: &mut PanelNavigation,
) {
    let title = if navigation.focused() == panel {
        format!("[ {} ]", panel.label())
    } else {
        format!(" {} ", panel.label())
    };
    let block = Block::bordered().title(title);
    let inner = block.inner(area);
    let visible_rows = visible_row_count(buffer, inner.width);
    navigation.settle(panel, visible_rows.into(), inner.height.into());
    let position = navigation.position(panel);
    let scroll_offset = u16::try_from(position).unwrap_or(u16::MAX);

    let paragraph = Paragraph::new(buffer)
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

fn format_tool_calls(parsed_calls: &[ParsedToolCall], pending_raw: &str) -> String {
    let mut output = String::new();
    for call in parsed_calls {
        output.push_str(&call.name);
        output.push('\n');
        match &call.arguments {
            ToolCallArguments::ValidJson(value) => match serde_json::to_string_pretty(value) {
                Ok(formatted) => {
                    for line in formatted.lines() {
                        output.push_str("  ");
                        output.push_str(line);
                        output.push('\n');
                    }
                }
                Err(format_error) => {
                    log::error!(
                        "failed to pretty-print tool-call arguments for {name}: {format_error}",
                        name = call.name
                    );
                    output.push_str("  ");
                    output.push_str(&value.to_string());
                    output.push('\n');
                }
            },
            ToolCallArguments::InvalidJson(raw) => {
                output.push_str("  invalid JSON: ");
                output.push_str(raw);
                output.push('\n');
            }
        }
    }
    if !pending_raw.is_empty() {
        output.push_str(pending_raw);
    }
    output
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

fn visible_row_count(buffer: &str, width: u16) -> u16 {
    if width == 0 || buffer.is_empty() {
        return 0;
    }
    let mut total: u16 = 0;
    for line in buffer.split('\n') {
        let char_count = u16::try_from(line.chars().count()).unwrap_or(u16::MAX);
        let rows = char_count.div_ceil(width).max(1);
        total = total.saturating_add(rows);
    }
    total
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::generation_summary::GenerationSummary;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    use super::*;
    use crate::stop_reason::StopReason;

    fn render_to_string(state: &StreamingResponse, width: u16, height: u16) -> Result<String> {
        let mut navigation = PanelNavigation::default();
        let mut terminal = Terminal::new(TestBackend::new(width, height))?;
        terminal.draw(|frame| {
            let layout = ChatPanelLayout::compute(frame.area());
            render_chat_panels(state, &mut navigation, &layout, frame);
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
        state.response.push_str("hello world");

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
}
