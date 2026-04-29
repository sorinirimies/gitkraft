//! Top toolbar / header bar for the GitKraft main layout.
//!
//! Shows: repo name │ branch name │ fetch button │ toggle sidebar.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::features::editor::editor_selector;
use crate::features::theme::view::theme_selector;
use crate::icons;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::view_utils;

/// Render the top toolbar row.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    // ── Repo name ─────────────────────────────────────────────────────────
    let repo_icon = icon!(icons::FOLDER_OPEN, 14, c.accent);

    let repo_name = text(state.repo_display_name())
        .size(14)
        .color(c.text_primary);

    let separator = || text("|").size(14).color(c.border);

    // ── Branch indicator ──────────────────────────────────────────────────
    let branch_icon = icon!(icons::GIT_BRANCH, 14, c.green);

    let branch_name_str = tab.current_branch.as_deref().unwrap_or("(detached)");

    let branch_label = text(branch_name_str).size(14).color(c.text_primary);

    // ── Repo state badge (if not clean) ───────────────────────────────────
    let state_badge: Element<'_, Message> = if let Some(ref info) = tab.repo_info {
        if info.state != gitkraft_core::RepoState::Clean {
            text(format!(" [{}]", info.state))
                .size(12)
                .color(c.yellow)
                .into()
        } else {
            Space::new().into()
        }
    } else {
        Space::new().into()
    };

    // ── Fetch button ──────────────────────────────────────────────────────
    let fetch_icon = icon!(icons::CLOUD_ARROW_DOWN, 14, c.accent);

    let fetch_msg = (!tab.remotes.is_empty()).then_some(Message::Fetch);
    let fetch_btn = crate::view_utils::on_press_maybe(
        button(
            row![fetch_icon, Space::new().width(4), text("Fetch").size(12)]
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(theme::toolbar_button),
        fetch_msg,
    );

    // ── Open another repo button ──────────────────────────────────────────
    let open_icon = icon!(icons::FOLDER_OPEN, 14, c.text_secondary);

    let open_btn = view_utils::toolbar_btn(open_icon, "Open", Message::OpenRepo);

    // ── Close repo button (return to welcome screen) ──────────────────────
    let close_icon = icon!(icons::X_CIRCLE, 14, c.text_secondary);

    let close_btn = view_utils::toolbar_btn(close_icon, "Close", Message::CloseRepo);

    // ── Toggle sidebar ────────────────────────────────────────────────────
    let sidebar_icon_char = if state.sidebar_expanded {
        icons::CHEVRON_LEFT
    } else {
        icons::CHEVRON_RIGHT
    };
    let sidebar_icon = icon!(sidebar_icon_char, 14, c.text_secondary);

    let sidebar_btn = button(sidebar_icon)
        .padding([4, 8])
        .style(theme::icon_button)
        .on_press(Message::ToggleSidebar);

    // ── Loading indicator ─────────────────────────────────────────────
    // Use FluxFrames::CORNERS from tui-spinner so both GUI and TUI share
    // the same symbol set — no duplication.
    let spinner_frame: String = {
        let frames = tui_spinner::FluxFrames::CORNERS; // &'static [char]
        let ch = frames[state.animation_tick as usize % frames.len()];
        ch.to_string()
    };

    let loading_indicator: Element<'_, Message> = if tab.is_loading {
        row![
            text(spinner_frame)
                .size(15)
                .color(c.accent)
                .font(iced::Font::MONOSPACE),
            iced::widget::Space::new().width(4),
            text("Loading…").size(12).color(c.yellow),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        Space::new().into()
    };

    // ── Assemble ──────────────────────────────────────────────────────────
    let left_items = row![
        sidebar_btn,
        Space::new().width(8),
        repo_icon,
        Space::new().width(6),
        repo_name,
        Space::new().width(10),
        separator(),
        Space::new().width(10),
        branch_icon,
        Space::new().width(6),
        branch_label,
        state_badge,
        Space::new().width(10),
        separator(),
        Space::new().width(10),
        loading_indicator,
    ]
    .align_y(Alignment::Center);

    let right_items = row![
        fetch_btn,
        Space::new().width(4),
        open_btn,
        Space::new().width(4),
        close_btn,
        Space::new().width(8),
        theme_selector(state.current_theme_index),
        Space::new().width(4),
        editor_selector(&state.editor),
    ]
    .align_y(Alignment::Center);

    let toolbar = row![
        container(left_items).width(Length::Fill).clip(true),
        right_items,
    ]
    .align_y(Alignment::Center)
    .padding([6, 12])
    .width(Length::Fill);

    container(toolbar)
        .width(Length::Fill)
        .style(theme::header_style)
        .into()
}
