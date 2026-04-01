//! Top toolbar / header bar for the GitKraft main layout.
//!
//! Shows: repo name │ branch name │ fetch button │ refresh button │ toggle sidebar.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;

/// Render the top toolbar row.
pub fn view(state: &GitKraft) -> Element<'_, Message> {
    // ── Repo name ─────────────────────────────────────────────────────────
    let repo_icon = text('\u{F3D8}') // folder icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let repo_name = text(state.repo_display_name())
        .size(14)
        .color(theme::TEXT_PRIMARY);

    let separator = || {
        text("│").size(14).color(theme::BORDER)
    };

    // ── Branch indicator ──────────────────────────────────────────────────
    let branch_icon = text('\u{F404}') // git-branch icon
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::GREEN);

    let branch_name_str = state
        .current_branch
        .as_deref()
        .unwrap_or("(detached)");

    let branch_label = text(branch_name_str)
        .size(14)
        .color(theme::TEXT_PRIMARY);

    // ── Repo state badge (if not clean) ───────────────────────────────────
    let state_badge: Element<'_, Message> = if let Some(ref info) = state.repo_info {
        if info.state != gitkraft_core::RepoState::Clean {
            text(format!(" [{}]", info.state))
                .size(12)
                .color(theme::YELLOW)
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
        .color(theme::ACCENT);

    let fetch_btn = if state.remotes.is_empty() {
        button(
            row![fetch_icon, Space::with_width(4), text("Fetch").size(12)]
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
    } else {
        button(
            row![fetch_icon, Space::with_width(4), text("Fetch").size(12)]
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .on_press(Message::Fetch)
    };

    // ── Refresh button ────────────────────────────────────────────────────
    let refresh_icon = text('\u{F116}') // arrow-clockwise (using arrow-down as fallback)
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::ACCENT);

    let refresh_btn = button(
        row![refresh_icon, Space::with_width(4), text("Refresh").size(12)]
            .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .on_press(Message::RefreshRepo);

    // ── Open another repo button ──────────────────────────────────────────
    let open_icon = text('\u{F3D8}') // folder-open
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::TEXT_SECONDARY);

    let open_btn = button(
        row![open_icon, Space::with_width(4), text("Open").size(12)]
            .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .on_press(Message::OpenRepo);

    // ── Toggle sidebar ────────────────────────────────────────────────────
    let sidebar_icon_char = if state.sidebar_expanded {
        '\u{F284}' // chevron-left
    } else {
        '\u{F285}' // chevron-right
    };
    let sidebar_icon = text(sidebar_icon_char)
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::TEXT_SECONDARY);

    let sidebar_btn = button(sidebar_icon)
        .padding([4, 8])
        .on_press(Message::ToggleSidebar);

    // ── Loading indicator ─────────────────────────────────────────────────
    let loading_indicator: Element<'_, Message> = if state.is_loading {
        text("⟳ Loading…")
            .size(12)
            .color(theme::YELLOW)
            .into()
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
        fetch_btn,
        Space::with_width(4),
        refresh_btn,
        Space::with_width(4),
        open_btn,
    ]
    .align_y(Alignment::Center)
    .padding([6, 12])
    .width(Length::Fill);

    container(toolbar)
        .width(Length::Fill)
        .style(theme::header_style)
        .into()
}
