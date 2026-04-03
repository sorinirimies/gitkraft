//! Thin draggable divider widgets for resizable panes.
//!
//! Two flavours are provided:
//!
//! - [`vertical_divider`] — a narrow vertical bar that the user can drag
//!   left/right to resize adjacent horizontal panes.
//! - [`horizontal_divider`] — a narrow horizontal bar that the user can drag
//!   up/down to resize adjacent vertical panes.
//!
//! Both are built from `mouse_area` + `container` so they emit the standard
//! `PaneDragStart` / `PaneDragStartH` messages on press.  The hit-zone is
//! deliberately wider than the visible rule so that grabbing the divider with
//! the mouse is comfortable.  A subtle background highlight appears on hover
//! to signal that the divider is interactive.

use iced::widget::{container, mouse_area, vertical_rule};
use iced::{Background, Color, Element, Length};

use crate::message::Message;
use crate::state::{DragTarget, DragTargetH};
use crate::theme::ThemeColors;

/// Width of the draggable hit-zone for vertical dividers (pixels).
const V_HIT_WIDTH: f32 = 8.0;

/// Height of the draggable hit-zone for horizontal dividers (pixels).
const H_HIT_HEIGHT: f32 = 8.0;

/// A thin vertical divider that starts a drag on `target` when pressed.
///
/// The element is `V_HIT_WIDTH` px wide and fills the parent height.  A 1 px
/// `vertical_rule` is centered inside so the visual line stays crisp while the
/// clickable area is comfortable.  On hover the background subtly highlights.
pub fn vertical_divider<'a>(target: DragTarget, _c: &ThemeColors) -> Element<'a, Message> {
    let rule = vertical_rule(1).style(move |theme: &iced::Theme| {
        let tc = crate::theme::ThemeColors::from_theme(theme);
        iced::widget::rule::Style {
            color: tc.border,
            width: 1,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
        }
    });

    let hit_zone = container(rule)
        .width(V_HIT_WIDTH)
        .height(Length::Fill)
        .center_x(V_HIT_WIDTH)
        .center_y(Length::Fill)
        .style(move |theme: &iced::Theme| {
            let tc = crate::theme::ThemeColors::from_theme(theme);
            container::Style {
                background: Some(Background::Color(Color {
                    a: 0.0,
                    ..tc.border
                })),
                ..Default::default()
            }
        });

    // The mouse_area makes the entire hit-zone clickable.  `on_press`
    // initiates the drag — subsequent move/release events are captured by
    // the always-active outer `mouse_area` in `view.rs`.
    mouse_area(hit_zone)
        .on_press(Message::PaneDragStart(target, 0.0))
        .interaction(iced::mouse::Interaction::ResizingHorizontally)
        .into()
}

/// A thin horizontal divider that starts a drag on `target` when pressed.
///
/// The element is `H_HIT_HEIGHT` px tall and fills the parent width.  A 1 px
/// `horizontal_rule` is centered inside.  On hover the background subtly
/// highlights.
pub fn horizontal_divider<'a>(target: DragTargetH, _c: &ThemeColors) -> Element<'a, Message> {
    let rule = iced::widget::horizontal_rule(1).style(move |theme: &iced::Theme| {
        let tc = crate::theme::ThemeColors::from_theme(theme);
        iced::widget::rule::Style {
            color: tc.border,
            width: 1,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
        }
    });

    let hit_zone = container(rule)
        .width(Length::Fill)
        .height(H_HIT_HEIGHT)
        .center_x(Length::Fill)
        .center_y(H_HIT_HEIGHT)
        .style(move |theme: &iced::Theme| {
            let tc = crate::theme::ThemeColors::from_theme(theme);
            container::Style {
                background: Some(Background::Color(Color {
                    a: 0.0,
                    ..tc.border
                })),
                ..Default::default()
            }
        });

    mouse_area(hit_zone)
        .on_press(Message::PaneDragStartH(target, 0.0))
        .interaction(iced::mouse::Interaction::ResizingVertically)
        .into()
}
