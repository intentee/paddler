use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Position;
use ratatui::layout::Rect;

use crate::view_panel_kind::ViewPanelKind;

const STATUS_BAR_HEIGHT: u16 = 1;

pub struct ViewPanelLayout {
    pub thinking: Rect,
    pub response: Rect,
    pub tool_calls: Rect,
    pub undetermined: Rect,
    pub status_bar: Rect,
}

impl ViewPanelLayout {
    pub fn compute(area: Rect) -> Self {
        let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(STATUS_BAR_HEIGHT)])
            .split(area);
        let rows = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer[0]);
        let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[0]);
        let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        Self {
            thinking: top[0],
            response: top[1],
            tool_calls: bottom[0],
            undetermined: bottom[1],
            status_bar: outer[1],
        }
    }

    pub const fn rect_for(&self, panel: ViewPanelKind) -> Rect {
        match panel {
            ViewPanelKind::Thinking => self.thinking,
            ViewPanelKind::Response => self.response,
            ViewPanelKind::ToolCalls => self.tool_calls,
            ViewPanelKind::Undetermined => self.undetermined,
        }
    }

    pub const fn viewport_rows(&self, panel: ViewPanelKind) -> u16 {
        self.rect_for(panel).height.saturating_sub(2)
    }

    pub fn panel_at(&self, column: u16, row: u16) -> Option<ViewPanelKind> {
        let position = Position { x: column, y: row };
        [
            ViewPanelKind::Thinking,
            ViewPanelKind::Response,
            ViewPanelKind::ToolCalls,
            ViewPanelKind::Undetermined,
        ]
        .into_iter()
        .find(|panel| self.rect_for(*panel).contains(position))
    }
}
