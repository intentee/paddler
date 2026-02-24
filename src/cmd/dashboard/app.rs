use std::io;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use io::Result as ioResult;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Text;
use ratatui::widgets::Cell;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Row;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Table;
use ratatui::widgets::TableState;
use ratatui::Frame;

use super::ui::TableColors;
use crate::balancer::upstream_peer::UpstreamPeer;
use crate::balancer::upstream_peer_pool::UpstreamPeerPoolInfo;

const ITEM_HEIGHT: usize = 6;
const INFO_TEXT: [&str; 1] = ["(Esc|q) quit | (↑) move up | (↓) move down"];

pub struct App {
    pub colors: TableColors,
    pub is_initial_load: bool,
    pub items: Option<Vec<UpstreamPeer>>,
    pub scroll_state: ScrollbarState,
    pub state: TableState,
    pub ticks: u128,
    pub error: Option<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            colors: TableColors::new(),
            is_initial_load: true,
            items: None,
            scroll_state: ScrollbarState::new(0),
            state: TableState::default().with_selected(0),
            ticks: 0,
            error: None,
        })
    }

    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if let Some(items) = &self.items {
                    if i >= items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                } else {
                    0
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if let Some(items) = &self.items {
                    if i == 0 {
                        items.len() - 1
                    } else {
                        i - 1
                    }
                } else {
                    0
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new();
    }

    pub fn draw(&mut self, frame: &mut Frame) -> ioResult<()> {
        let vertical =
            &Layout::vertical([Constraint::Min(5), Constraint::Length(1), Constraint::Length(1)]);
        let rects = vertical.split(frame.area());

        self.set_colors();

        self.render_table(frame, rects[0])?;
        self.render_scrollbar(frame, rects[0]);
        self.render_ticks(frame, rects[1]);
        self.render_footer(frame, rects[2]);

        Ok(())
    }

    fn render_ticks(&mut self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(format!("current tick: {}", self.ticks))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg)
                    .white(),
            )
            .centered();
        frame.render_widget(info_footer, area);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) -> ioResult<()> {
        if let Some(err) = &self.error {
            let t = Paragraph::new(err.clone().white()).centered().style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            );
            frame.render_widget(t, area);
        } else {
            match &self.items {
                Some(items) => match items.is_empty() {
                    true => {
                        let t = Paragraph::new(Text::from("Paddler 🏓\nThere are no agents registered. If agents are running, please give them a few seconds to register."))
                            .centered()
                            .bg(self.colors.buffer_bg);

                        frame.render_widget(t, area);
                    }
                    false => {
                        let header_style = Style::default()
                            .fg(self.colors.header_fg)
                            .bg(self.colors.header_bg);
                        let selected_row_style = Style::default()
                            .add_modifier(Modifier::REVERSED)
                            .fg(self.colors.selected_row_style_fg);

                        let header = [
                            "Name",
                            "Issues",
                            "Llama.cpp address",
                            "Last update",
                            "Idle slots",
                            "Processing slots",
                        ]
                        .into_iter()
                        .map(Cell::from)
                        .collect::<Row>()
                        .style(header_style)
                        .height(1)
                        .white();

                        let rows = items.iter().map(|agent| {
                            let color = self.colors.normal_row_color;
                            let mut items: [String; 6] = Default::default();

                            if let Ok(array) = ref_array(agent) {
                                items = array;
                            }

                            items
                                .into_iter()
                                .map(|content| Cell::from(Text::from(content).white()))
                                .collect::<Row>()
                                .style(Style::new().fg(self.colors.row_fg).bg(color))
                                .height(1)
                        });

                        let bar = " █ ";
                        let t = Table::new(rows, [Constraint::Ratio(1, 6); 6])
                            .header(header)
                            .row_highlight_style(selected_row_style)
                            .highlight_symbol(Text::from(vec![bar.into()]))
                            .bg(self.colors.buffer_bg)
                            .highlight_spacing(HighlightSpacing::Always)
                            .column_spacing(10);
                        frame.render_stateful_widget(t, area, &mut self.state);
                    }
                },
                None => {
                    let message = if self.is_initial_load {
                        "Paddler 🏓\nLoading agents..."
                    } else {
                        "Paddler 🏓\nThere are no agents registered. If agents are running, please give them a few seconds to register."
                    };

                    let t = Paragraph::new(Text::from(message).white())
                        .centered()
                        .bg(self.colors.buffer_bg);

                    frame.render_widget(t, area);
                }
            }
        }

        Ok(())
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg)
                    .white(),
            )
            .centered();
        frame.render_widget(info_footer, area);
    }

    pub fn set_registered_agents(
        &mut self,
        upstream_peer_pool: UpstreamPeerPoolInfo,
    ) -> Result<()> {
        self.items = Some(upstream_peer_pool.agents);
        self.is_initial_load = false;
        self.error = None;
        self.ticks += 1;

        Ok(())
    }
}

fn ref_array(peer: &UpstreamPeer) -> Result<[String; 6]> {
    let has_issue = match peer.status.error.clone() {
        Some(issue) => issue,
        None => String::from("None"),
    };

    let has_name = peer.status.agent_name.clone().unwrap_or_default();

    let date_as_string = systemtime_strftime(peer.last_update)?;

    Ok([
        has_name,
        has_issue,
        peer.status.external_llamacpp_addr.to_string().clone(),
        date_as_string,
        peer.status.slots_idle.to_string(),
        peer.status.slots_processing.to_string(),
    ])
}

fn systemtime_strftime(dt: SystemTime) -> Result<String> {
    let daration_epoch = dt.duration_since(UNIX_EPOCH)?;
    let datetime: DateTime<Utc> = DateTime::<Utc>::from(UNIX_EPOCH + daration_epoch);
    let formated_date = datetime.format("%Y/%m/%d, %H:%M:%S").to_string();

    Ok(formated_date)
}
