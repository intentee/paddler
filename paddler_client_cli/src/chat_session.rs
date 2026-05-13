use std::io;

use anyhow::Result;
use anyhow::anyhow;
use crossterm::event::Event as CrosstermEvent;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use futures_util::StreamExt;
use paddler_client::InferenceMessageStream;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use tokio_util::sync::CancellationToken;

use crate::chat_session_event::ChatSessionEvent;
use crate::streaming_response::StreamingResponse;
use crate::view_chat_panels::view_chat_panels;
use crate::view_panel_layout::ViewPanelLayout;
use crate::view_panel_navigation::ViewPanelNavigation;
use crate::view_terminal_guard::ViewTerminalGuard;

const MOUSE_WHEEL_LINES: u16 = 3;
const ARROW_KEY_LINES: u16 = 1;

pub struct ChatSession {
    inference_stream: InferenceMessageStream,
    state: StreamingResponse,
    navigation: ViewPanelNavigation,
    shutdown: CancellationToken,
}

impl ChatSession {
    pub fn new(inference_stream: InferenceMessageStream, shutdown: CancellationToken) -> Self {
        Self {
            inference_stream,
            state: StreamingResponse::default(),
            navigation: ViewPanelNavigation::default(),
            shutdown,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let _terminal_guard = ViewTerminalGuard::enter()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
        let mut events = EventStream::new();

        let mut layout = compute_layout(&terminal)?;
        terminal.draw(|frame| {
            view_chat_panels(&self.state, &mut self.navigation, &layout, frame);
        })?;

        loop {
            match self.next_event(&mut events).await {
                ChatSessionEvent::InferenceMessage(message) => {
                    self.state.apply_message(message);
                }
                ChatSessionEvent::InferenceStreamEnded => {
                    if !self.state.is_finished() {
                        self.state.record_wire_error(&anyhow!(
                            "inference stream ended before sending Done"
                        ));
                    }
                }
                ChatSessionEvent::InferenceStreamError(error) => {
                    self.state.record_wire_error(&error);
                }
                ChatSessionEvent::Key(key_event) => {
                    if is_quit(key_event) {
                        return Ok(());
                    }
                    self.handle_navigation_key(key_event, &layout);
                }
                ChatSessionEvent::Mouse(mouse_event) => {
                    self.handle_mouse(mouse_event, &layout);
                }
                ChatSessionEvent::Repaint => {}
                ChatSessionEvent::Shutdown => return Ok(()),
            }
            layout = compute_layout(&terminal)?;
            terminal.draw(|frame| {
                view_chat_panels(&self.state, &mut self.navigation, &layout, frame);
            })?;
        }
    }

    async fn next_event(&mut self, events: &mut EventStream) -> ChatSessionEvent {
        let inference_active = !self.state.is_finished();
        loop {
            tokio::select! {
                biased;
                () = self.shutdown.cancelled() => return ChatSessionEvent::Shutdown,
                maybe_event = events.next() => match maybe_event {
                    Some(Ok(CrosstermEvent::Key(key))) => return ChatSessionEvent::Key(key),
                    Some(Ok(CrosstermEvent::Mouse(mouse))) => return ChatSessionEvent::Mouse(mouse),
                    Some(Ok(CrosstermEvent::Resize(_, _))) => return ChatSessionEvent::Repaint,
                    Some(Ok(_)) => {}
                    Some(Err(read_error)) => {
                        log::error!("terminal event read error: {read_error}");
                        return ChatSessionEvent::Shutdown;
                    }
                    None => return ChatSessionEvent::Shutdown,
                },
                maybe_message = self.inference_stream.next(), if inference_active => match maybe_message {
                    Some(Ok(message)) => return ChatSessionEvent::InferenceMessage(message),
                    Some(Err(stream_error)) => return ChatSessionEvent::InferenceStreamError(stream_error.into()),
                    None => return ChatSessionEvent::InferenceStreamEnded,
                },
            }
        }
    }

    fn handle_navigation_key(&mut self, key_event: KeyEvent, layout: &ViewPanelLayout) {
        let focused = self.navigation.focused();
        let viewport_rows = layout.viewport_rows(focused);
        let page_lines = viewport_rows.saturating_sub(1).max(1);
        match key_event.code {
            KeyCode::Up => self.navigation.scroll_up(focused, ARROW_KEY_LINES),
            KeyCode::Down => self.navigation.scroll_down(focused, ARROW_KEY_LINES),
            KeyCode::PageUp => self.navigation.scroll_up(focused, page_lines),
            KeyCode::PageDown => self.navigation.scroll_down(focused, page_lines),
            KeyCode::Home => self.navigation.jump_to_top(focused),
            KeyCode::End => self.navigation.jump_to_bottom(focused),
            KeyCode::Tab => self.navigation.cycle_focus_forward(),
            KeyCode::BackTab => self.navigation.cycle_focus_backward(),
            _ => {}
        }
    }

    fn handle_mouse(&mut self, mouse_event: MouseEvent, layout: &ViewPanelLayout) {
        let Some(panel) = layout.panel_at(mouse_event.column, mouse_event.row) else {
            return;
        };
        match mouse_event.kind {
            MouseEventKind::ScrollUp => {
                self.navigation.focus(panel);
                self.navigation.scroll_up(panel, MOUSE_WHEEL_LINES);
            }
            MouseEventKind::ScrollDown => {
                self.navigation.focus(panel);
                self.navigation.scroll_down(panel, MOUSE_WHEEL_LINES);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.navigation.focus(panel);
            }
            _ => {}
        }
    }
}

fn compute_layout(terminal: &Terminal<CrosstermBackend<io::Stdout>>) -> Result<ViewPanelLayout> {
    let size = terminal.size()?;
    Ok(ViewPanelLayout::compute(Rect::new(
        0,
        0,
        size.width,
        size.height,
    )))
}

const fn is_quit(key_event: KeyEvent) -> bool {
    if key_event.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key_event.code, KeyCode::Char('c'))
    {
        return true;
    }
    matches!(key_event.code, KeyCode::Char('q' | 'Q') | KeyCode::Esc)
}
