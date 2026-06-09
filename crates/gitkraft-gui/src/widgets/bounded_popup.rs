//! Reusable bounded popup — positions content near a cursor point while
//! keeping it fully within the window bounds.
//!
//! Works correctly for any window size (maximized, windowed, small).

use iced::widget::{column, container, row, scrollable, Space};
use iced::{Element, Length};

use crate::message::Message;
use crate::view_utils;

/// Minimum distance from window edges (pixels).
const EDGE_MARGIN: f32 = 2.0;

/// Compute the (x, y) position for a popup, clamped within window bounds.
///
/// This is a pure function so it can be tested independently of rendering.
pub fn compute_popup_position(
    cursor_x: f32,
    cursor_y: f32,
    popup_w: f32,
    popup_max_h: f32,
    window_w: f32,
    window_h: f32,
) -> (f32, f32) {
    // Horizontal: prefer right of cursor; flip left if it would overflow.
    let px = if cursor_x + popup_w + EDGE_MARGIN > window_w {
        (cursor_x - popup_w - EDGE_MARGIN).max(EDGE_MARGIN)
    } else {
        (cursor_x + EDGE_MARGIN).max(EDGE_MARGIN)
    };

    // Vertical: prefer below cursor; push up if it would overflow bottom.
    let py = if cursor_y + popup_max_h + EDGE_MARGIN > window_h {
        let overflow = cursor_y + popup_max_h + EDGE_MARGIN - window_h;
        (cursor_y - overflow).max(EDGE_MARGIN)
    } else {
        (cursor_y + EDGE_MARGIN).max(EDGE_MARGIN)
    };

    (px, py)
}

/// A builder for a bounds-aware popup.
pub struct BoundedPopup<'a> {
    content: Element<'a, Message>,
    cursor_x: f32,
    cursor_y: f32,
    window_w: f32,
    window_h: f32,
    popup_width: f32,
    popup_max_height: f32,
    style_fn: Option<fn(&iced::Theme) -> iced::widget::container::Style>,
}

/// Create a bounded popup that stays within the window.
pub fn bounded_popup<'a>(
    content: impl Into<Element<'a, Message>>,
    cursor_x: f32,
    cursor_y: f32,
    window_w: f32,
    window_h: f32,
) -> BoundedPopup<'a> {
    BoundedPopup {
        content: content.into(),
        cursor_x,
        cursor_y,
        window_w,
        window_h,
        popup_width: 280.0,
        popup_max_height: 500.0,
        style_fn: None,
    }
}

impl<'a> BoundedPopup<'a> {
    /// Set the popup width (default 280).
    pub fn width(mut self, w: f32) -> Self {
        self.popup_width = w;
        self
    }

    /// Set the maximum popup height before scrolling (default 500).
    pub fn max_height(mut self, h: f32) -> Self {
        self.popup_max_height = h;
        self
    }

    /// Set the container style for the popup panel.
    pub fn style(mut self, s: fn(&iced::Theme) -> iced::widget::container::Style) -> Self {
        self.style_fn = Some(s);
        self
    }

    /// Build the positioned popup element.
    pub fn build(self) -> Element<'a, Message> {
        let (px, py) = compute_popup_position(
            self.cursor_x,
            self.cursor_y,
            self.popup_width,
            self.popup_max_height,
            self.window_w,
            self.window_h,
        );

        // Cap max_height to the AVAILABLE space below the computed position.
        // This ensures the popup never extends past the window bottom,
        // regardless of window size (maximized, windowed, small).
        let available_h = (self.window_h - py - EDGE_MARGIN).max(100.0);
        let effective_max_h = self.popup_max_height.min(available_h);

        let scrollable_content = scrollable(self.content)
            .direction(view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar);

        let mut panel = container(scrollable_content)
            .width(self.popup_width)
            .max_height(effective_max_h);

        if let Some(s) = self.style_fn {
            panel = panel.style(s);
        }

        let panel_element: Element<'a, Message> = panel.into();

        column![
            Space::new().height(py),
            row![Space::new().width(px), panel_element],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_within_bounds(px: f32, py: f32, popup_w: f32, win_w: f32) {
        assert!(px >= EDGE_MARGIN, "px={px} below margin");
        assert!(py >= EDGE_MARGIN, "py={py} below margin");
        assert!(
            px + popup_w <= win_w + 0.01,
            "right edge overflow: px={px} + w={popup_w} > win_w={win_w}"
        );
    }

    /// Simulate the effective_max_h calculation from build().
    fn effective_max_h(py: f32, popup_max_h: f32, win_h: f32) -> f32 {
        let available = (win_h - py - EDGE_MARGIN).max(100.0);
        popup_max_h.min(available)
    }

    /// Assert the popup bottom never exceeds the window bottom.
    fn assert_no_bottom_overflow(py: f32, popup_max_h: f32, win_h: f32) {
        let h = effective_max_h(py, popup_max_h, win_h);
        assert!(
            py + h <= win_h + 0.01,
            "bottom overflow: py={py} + h={h} = {} > win_h={win_h}",
            py + h
        );
    }

    #[test]
    fn fits_normally_at_cursor() {
        let (px, py) = compute_popup_position(100.0, 200.0, 280.0, 500.0, 1400.0, 800.0);
        assert!((px - 102.0).abs() < 0.1);
        assert!((py - 202.0).abs() < 0.1);
        assert_within_bounds(px, py, 280.0, 1400.0);
        assert_no_bottom_overflow(py, 500.0, 800.0);
    }

    #[test]
    fn flips_left_when_right_overflow() {
        let (px, _) = compute_popup_position(1300.0, 100.0, 280.0, 500.0, 1400.0, 800.0);
        assert!(px < 1300.0, "should flip left");
        assert!(px + 280.0 <= 1400.0);
    }

    #[test]
    fn pushes_up_when_bottom_overflow() {
        let (_, py) = compute_popup_position(100.0, 700.0, 280.0, 500.0, 1400.0, 800.0);
        assert!(py < 700.0, "should push up");
        assert!(py >= EDGE_MARGIN);
        assert_no_bottom_overflow(py, 500.0, 800.0);
    }

    #[test]
    fn bottom_right_corner() {
        let (px, py) = compute_popup_position(1350.0, 780.0, 280.0, 500.0, 1400.0, 800.0);
        assert!(px < 1350.0);
        assert!(py < 780.0);
        assert!(px >= EDGE_MARGIN);
        assert!(py >= EDGE_MARGIN);
    }

    #[test]
    fn top_left_corner() {
        let (px, py) = compute_popup_position(0.0, 0.0, 280.0, 500.0, 1400.0, 800.0);
        assert!((px - EDGE_MARGIN).abs() < 0.1);
        assert!((py - EDGE_MARGIN).abs() < 0.1);
    }

    #[test]
    fn small_windowed_mode_400x300() {
        let (px, py) = compute_popup_position(200.0, 150.0, 280.0, 500.0, 400.0, 300.0);
        assert!(px >= EDGE_MARGIN);
        assert!(py >= EDGE_MARGIN);
        assert_within_bounds(px, py, 280.0, 400.0);
        assert_no_bottom_overflow(py, 500.0, 300.0);
    }

    #[test]
    fn window_smaller_than_popup() {
        let (px, py) = compute_popup_position(50.0, 50.0, 280.0, 500.0, 200.0, 200.0);
        assert!(px >= EDGE_MARGIN);
        assert!(py >= EDGE_MARGIN);
    }

    #[test]
    fn negative_cursor_clamped() {
        let (px, py) = compute_popup_position(-10.0, -20.0, 280.0, 500.0, 1400.0, 800.0);
        assert!(px >= EDGE_MARGIN);
        assert!(py >= EDGE_MARGIN);
    }

    #[test]
    fn windowed_800x600() {
        let (px, py) = compute_popup_position(500.0, 400.0, 280.0, 350.0, 800.0, 600.0);
        assert!(px + 280.0 <= 800.0, "overflows right in 800px window");
        assert!(py >= EDGE_MARGIN);
    }

    #[test]
    fn windowed_right_click_near_edge() {
        let (px, py) = compute_popup_position(700.0, 500.0, 280.0, 350.0, 800.0, 600.0);
        assert!(px < 700.0, "should flip left");
        assert!(py < 500.0, "should push up");
        assert!(px >= EDGE_MARGIN);
        assert!(py >= EDGE_MARGIN);
        assert_no_bottom_overflow(py, 350.0, 600.0);
    }

    #[test]
    fn free_floating_window_900px() {
        // Simulates the user's screenshot: ~900px window, cursor at y=570
        let (_, py) = compute_popup_position(200.0, 570.0, 280.0, 500.0, 1000.0, 900.0);
        assert!(py >= EDGE_MARGIN);
        assert_no_bottom_overflow(py, 500.0, 900.0);
    }

    #[test]
    fn very_tall_menu_in_short_window() {
        // 600px popup in 400px window — must scroll, never overflow
        let (_, py) = compute_popup_position(100.0, 200.0, 280.0, 600.0, 800.0, 400.0);
        assert!(py >= EDGE_MARGIN);
        assert_no_bottom_overflow(py, 600.0, 400.0);
    }

    #[test]
    fn exact_fit_no_adjustment() {
        let win_w = 100.0 + 280.0 + EDGE_MARGIN;
        let win_h = 100.0 + 500.0 + EDGE_MARGIN;
        let (px, py) = compute_popup_position(100.0, 100.0, 280.0, 500.0, win_w, win_h);
        assert!((px - 102.0).abs() < 0.1);
        assert!((py - 102.0).abs() < 0.1);
    }
}
