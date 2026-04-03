//! Top toolbar / header bar for the GitKraft main layout.
//!
//! Shows: repo name │ branch name │ fetch button │ refresh button │ toggle sidebar.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::features::theme::view::theme_selector;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the top toolbar row.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    let c = state.colors();

    // ── Repo name ─────────────────────────────────────────────────────────
    let repo_icon = text('\u{F3D8}') // folder icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let repo_name = text(state.repo_display_name())
        .size(14)
        .color(c.text_primary);

    let separator = || text("│").size(14).color(c.border);

    // ── Branch indicator ──────────────────────────────────────────────────
    let branch_icon = text('\u{F404}') // git-branch icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.green);

    let branch_name_str = state.current_branch.as_deref().unwrap_or("(detached)");

    let branch_label = text(branch_name_str).size(14).color(c.text_primary);

    // ── Repo state badge (if not clean) ───────────────────────────────────
    let state_badge: Element<'_, Message> = if let Some(ref info) = state.repo_info {
        if info.state != gitkraft_core::RepoState::Clean {
            text(format!(" [{}]", info.state))
                .size(12)
                .color(c.yellow)
                .into()
        } else {
            Space::with_width(0).into()
        }
    } else {
        Space::with_width(0).into()
    };

    // ── Fetch button ──────────────────────────────────────────────────────
    let fetch_icon = text('\u{F116}') // arrow-down-circle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let fetch_btn = if state.remotes.is_empty() {
        button(
            row![fetch_icon, Space::with_width(4), text("Fetch").size(12)]
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(theme::toolbar_button)
    } else {
        button(
            row![fetch_icon, Space::with_width(4), text("Fetch").size(12)]
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(theme::toolbar_button)
        .on_press(Message::Fetch)
    };

    // ── Refresh button ────────────────────────────────────────────────────
    let refresh_icon = text('\u{F130}') // arrow-clockwise
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.accent);

    let refresh_btn = button(
        row![refresh_icon, Space::with_width(4), text("Refresh").size(12)]
            .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(theme::toolbar_button)
    .on_press(Message::RefreshRepo);

    // ── Open another repo button ──────────────────────────────────────────
    let open_icon = text('\u{F3D8}') // folder-open
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.text_secondary);

    let open_btn = button(
        row![open_icon, Space::with_width(4), text("Open").size(12)].align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(theme::toolbar_button)
    .on_press(Message::OpenRepo);

    // ── Close repo button (return to welcome screen) ──────────────────────
    let close_icon = text('\u{F62A}') // x-circle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.text_secondary);

    let close_btn = button(
        row![close_icon, Space::with_width(4), text("Close").size(12)].align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(theme::toolbar_button)
    .on_press(Message::CloseRepo);

    // ── Toggle sidebar ────────────────────────────────────────────────────
    let sidebar_icon_char = if state.sidebar_expanded {
        '\u{F284}' // chevron-left
    } else {
        '\u{F285}' // chevron-right
    };
    let sidebar_icon = text(sidebar_icon_char)
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.text_secondary);

    let sidebar_btn = button(sidebar_icon)
        .padding([4, 8])
        .style(theme::icon_button)
        .on_press(Message::ToggleSidebar);

    // ── Loading indicator ─────────────────────────────────────────────────
    let loading_indicator: Element<'_, Message> = if state.is_loading {
        text("⟳ Loading…").size(12).color(c.yellow).into()
    } else {
        Space::with_width(0).into()
    };

    // ── Assemble ──────────────────────────────────────────────────────────
    let toolbar = row![
        sidebar_btn,
        Space::with_width(8),
        repo_icon,
        Space::with_width(6),
        repo_name,
        Space::with_width(10),
        separator(),
        Space::with_width(10),
        branch_icon,
        Space::with_width(6),
        branch_label,
        state_badge,
        Space::with_width(10),
        separator(),
        Space::with_width(10),
        loading_indicator,
        Space::with_width(Length::Fill),
        theme_selector(state.current_theme_index),
        Space::with_width(8),
        fetch_btn,
        Space::with_width(4),
        refresh_btn,
        Space::with_width(4),
        open_btn,
        Space::with_width(4),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([6, 12])
    .width(Length::Fill);

    container(toolbar)
        .width(Length::Fill)
        .style(theme::header_style)
        .into()
}
