use crate::view_panel_kind::ViewPanelKind;

const PANEL_COUNT: usize = 4;

pub struct ViewPanelNavigation {
    focused: ViewPanelKind,
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

impl Default for ViewPanelNavigation {
    fn default() -> Self {
        Self {
            focused: ViewPanelKind::Response,
            views: [PanelView::default(); PANEL_COUNT],
        }
    }
}

impl ViewPanelNavigation {
    pub const fn focused(&self) -> ViewPanelKind {
        self.focused
    }

    pub const fn focus(&mut self, panel: ViewPanelKind) {
        self.focused = panel;
    }

    pub const fn cycle_focus_forward(&mut self) {
        self.focused = match self.focused {
            ViewPanelKind::Thinking => ViewPanelKind::Response,
            ViewPanelKind::Response => ViewPanelKind::ToolCalls,
            ViewPanelKind::ToolCalls => ViewPanelKind::Undetermined,
            ViewPanelKind::Undetermined => ViewPanelKind::Thinking,
        };
    }

    pub const fn cycle_focus_backward(&mut self) {
        self.focused = match self.focused {
            ViewPanelKind::Thinking => ViewPanelKind::Undetermined,
            ViewPanelKind::Response => ViewPanelKind::Thinking,
            ViewPanelKind::ToolCalls => ViewPanelKind::Response,
            ViewPanelKind::Undetermined => ViewPanelKind::ToolCalls,
        };
    }

    pub fn scroll_up(&mut self, panel: ViewPanelKind, lines: u16) {
        let view = &mut self.views[panel as usize];
        view.follow_bottom = false;
        view.position = view.position.saturating_sub(lines.into());
    }

    pub fn scroll_down(&mut self, panel: ViewPanelKind, lines: u16) {
        let view = &mut self.views[panel as usize];
        view.position = view.position.saturating_add(lines.into());
    }

    pub const fn jump_to_top(&mut self, panel: ViewPanelKind) {
        let view = &mut self.views[panel as usize];
        view.follow_bottom = false;
        view.position = 0;
    }

    pub const fn jump_to_bottom(&mut self, panel: ViewPanelKind) {
        self.views[panel as usize].follow_bottom = true;
    }

    pub const fn settle(
        &mut self,
        panel: ViewPanelKind,
        content_rows: usize,
        viewport_rows: usize,
    ) {
        let view = &mut self.views[panel as usize];
        let max_position = content_rows.saturating_sub(viewport_rows);
        if view.follow_bottom || view.position >= max_position {
            view.position = max_position;
            view.follow_bottom = true;
        }
    }

    pub const fn position(&self, panel: ViewPanelKind) -> usize {
        self.views[panel as usize].position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_focus_response_and_follow_bottom() {
        let mut nav = ViewPanelNavigation::default();

        assert_eq!(nav.focused(), ViewPanelKind::Response);
        nav.settle(ViewPanelKind::Response, 100, 10);
        assert_eq!(nav.position(ViewPanelKind::Response), 90);
    }

    #[test]
    fn scroll_up_disengages_follow_and_decrements_position() {
        let mut nav = ViewPanelNavigation::default();

        nav.scroll_up(ViewPanelKind::Response, 5);

        nav.settle(ViewPanelKind::Response, 100, 10);
        assert_eq!(nav.position(ViewPanelKind::Response), 0);
    }

    #[test]
    fn scroll_down_after_scroll_up_advances_within_content() {
        let mut nav = ViewPanelNavigation::default();
        nav.scroll_up(ViewPanelKind::Response, 50);

        nav.scroll_down(ViewPanelKind::Response, 10);

        nav.settle(ViewPanelKind::Response, 100, 10);
        assert_eq!(nav.position(ViewPanelKind::Response), 10);
    }

    #[test]
    fn jump_to_bottom_re_engages_auto_follow() {
        let mut nav = ViewPanelNavigation::default();
        nav.scroll_up(ViewPanelKind::Response, 50);

        nav.jump_to_bottom(ViewPanelKind::Response);

        nav.settle(ViewPanelKind::Response, 200, 10);
        assert_eq!(nav.position(ViewPanelKind::Response), 190);
    }

    #[test]
    fn cycle_focus_forward_walks_panels_in_reading_order() {
        let mut nav = ViewPanelNavigation::default();

        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), ViewPanelKind::ToolCalls);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), ViewPanelKind::Undetermined);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), ViewPanelKind::Thinking);
        nav.cycle_focus_forward();
        assert_eq!(nav.focused(), ViewPanelKind::Response);
    }

    #[test]
    fn scrolling_back_to_bottom_re_engages_auto_follow_for_subsequent_growth() {
        let mut nav = ViewPanelNavigation::default();
        nav.settle(ViewPanelKind::Response, 100, 10);

        nav.scroll_up(ViewPanelKind::Response, 5);
        nav.settle(ViewPanelKind::Response, 100, 10);

        nav.scroll_down(ViewPanelKind::Response, 10);
        nav.settle(ViewPanelKind::Response, 100, 10);

        nav.settle(ViewPanelKind::Response, 110, 10);

        assert_eq!(nav.position(ViewPanelKind::Response), 100);
    }

    #[test]
    fn position_is_clamped_when_content_shorter_than_stored_offset() {
        let mut nav = ViewPanelNavigation::default();
        nav.scroll_up(ViewPanelKind::Response, 0);
        nav.scroll_down(ViewPanelKind::Response, 80);

        nav.settle(ViewPanelKind::Response, 30, 10);
        assert_eq!(nav.position(ViewPanelKind::Response), 20);
    }
}
