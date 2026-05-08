use crate::panel_kind::PanelKind;

const PANEL_COUNT: usize = 4;

pub struct PanelNavigation {
    focused: PanelKind,
    views: [PanelView; PANEL_COUNT],
}

#[derive(Clone, Copy)]
struct PanelView {
    position: usize,
    follow_bottom: bool,
}

impl Default for PanelView {
    fn default() -> Self {
        Self {
            position: 0,
            follow_bottom: true,
        }
    }
}

impl Default for PanelNavigation {
    fn default() -> Self {
        Self {
            focused: PanelKind::Response,
            views: [PanelView::default(); PANEL_COUNT],
        }
    }
}

impl PanelNavigation {
    pub const fn focused(&self) -> PanelKind {
        self.focused
    }

    pub const fn focus(&mut self, panel: PanelKind) {
        self.focused = panel;
    }

    pub const fn cycle_focus_forward(&mut self) {
        self.focused = match self.focused {
            PanelKind::Thinking => PanelKind::Response,
            PanelKind::Response => PanelKind::ToolCalls,
            PanelKind::ToolCalls => PanelKind::Undetermined,
            PanelKind::Undetermined => PanelKind::Thinking,
        };
    }

    pub const fn cycle_focus_backward(&mut self) {
        self.focused = match self.focused {
            PanelKind::Thinking => PanelKind::Undetermined,
            PanelKind::Response => PanelKind::Thinking,
            PanelKind::ToolCalls => PanelKind::Response,
            PanelKind::Undetermined => PanelKind::ToolCalls,
        };
    }

    pub fn scroll_up(&mut self, panel: PanelKind, lines: u16) {
        let view = &mut self.views[panel as usize];
        view.follow_bottom = false;
        view.position = view.position.saturating_sub(lines.into());
    }

    pub fn scroll_down(&mut self, panel: PanelKind, lines: u16) {
        let view = &mut self.views[panel as usize];
        view.position = view.position.saturating_add(lines.into());
    }

    pub const fn jump_to_top(&mut self, panel: PanelKind) {
        let view = &mut self.views[panel as usize];
        view.follow_bottom = false;
        view.position = 0;
    }

    pub const fn jump_to_bottom(&mut self, panel: PanelKind) {
        self.views[panel as usize].follow_bottom = true;
    }

    pub const fn settle(&mut self, panel: PanelKind, content_rows: usize, viewport_rows: usize) {
        let view = &mut self.views[panel as usize];
        let max_position = content_rows.saturating_sub(viewport_rows);
        if view.follow_bottom || view.position >= max_position {
            view.position = max_position;
            view.follow_bottom = true;
        }
    }

    pub const fn position(&self, panel: PanelKind) -> usize {
        self.views[panel as usize].position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_focus_response_and_follow_bottom() {
        let mut nav = PanelNavigation::default();

        assert_eq!(nav.focused(), PanelKind::Response);
        nav.settle(PanelKind::Response, 100, 10);
        assert_eq!(nav.position(PanelKind::Response), 90);
    }

    #[test]
    fn scroll_up_disengages_follow_and_decrements_position() {
        let mut nav = PanelNavigation::default();

        nav.scroll_up(PanelKind::Response, 5);

        nav.settle(PanelKind::Response, 100, 10);
        assert_eq!(nav.position(PanelKind::Response), 0);
    }

    #[test]
    fn scroll_down_after_scroll_up_advances_within_content() {
        let mut nav = PanelNavigation::default();
        nav.scroll_up(PanelKind::Response, 50);

        nav.scroll_down(PanelKind::Response, 10);

        nav.settle(PanelKind::Response, 100, 10);
        assert_eq!(nav.position(PanelKind::Response), 10);
    }

    #[test]
    fn jump_to_bottom_re_engages_auto_follow() {
        let mut nav = PanelNavigation::default();
        nav.scroll_up(PanelKind::Response, 50);

        nav.jump_to_bottom(PanelKind::Response);

        nav.settle(PanelKind::Response, 200, 10);
        assert_eq!(nav.position(PanelKind::Response), 190);
    }

    #[test]
    fn cycle_focus_forward_walks_panels_in_reading_order() {
        let mut nav = PanelNavigation::default();

        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), PanelKind::ToolCalls);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), PanelKind::Undetermined);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), PanelKind::Thinking);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), PanelKind::Response);
    }

    #[test]
    fn scrolling_back_to_bottom_re_engages_auto_follow_for_subsequent_growth() {
        let mut nav = PanelNavigation::default();
        nav.settle(PanelKind::Response, 100, 10);

        nav.scroll_up(PanelKind::Response, 5);
        nav.settle(PanelKind::Response, 100, 10);

        nav.scroll_down(PanelKind::Response, 10);
        nav.settle(PanelKind::Response, 100, 10);

        nav.settle(PanelKind::Response, 110, 10);

        assert_eq!(nav.position(PanelKind::Response), 100);
    }

    #[test]
    fn position_is_clamped_when_content_shorter_than_stored_offset() {
        let mut nav = PanelNavigation::default();
        nav.scroll_up(PanelKind::Response, 0);
        nav.scroll_down(PanelKind::Response, 80);

        nav.settle(PanelKind::Response, 30, 10);
        assert_eq!(nav.position(PanelKind::Response), 20);
    }
}
