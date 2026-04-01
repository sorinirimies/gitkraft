//! Main view entry point — builds the top-level layout shell and delegates to
//! feature views for each region of the UI.
//!
//! Layout (when a repo is open):
//! ```text
//! ┌──────────────────────────────────────────┐
//! │  header toolbar                          │
//! ├────────┬─────────────────┬───────────────┤
//! │        │                 │               │
//! │ side-  │  commit log     │  diff viewer  │
//! │ bar    │                 │               │
//! │        │                 │               │
//! ├────────┴─────────────────┴───────────────┤
//! │  staging area (unstaged | staged | msg)  │
//! ├──────────────────────────────────────────┤
//! │  status bar                              │
//! └──────────────────────────────────────────┘
//! ```

use iced::widget::{column, container, horizontal_rule, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::features;
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme;
use crate::widgets;

impl GitKraft {
    /// Top-level view — called by the Iced runtime on every frame.
    pub fn view(&self) -> Element<'_, Message> {
        if !self.has_repo() {
            return features::repo::view::welcome_view();
        }

        // ── Header toolbar ────────────────────────────────────────────────
        let header = widgets::header::view(self);

        // ── Sidebar (branches + stash + remotes) ──────────────────────────
        let sidebar: Element<'_, Message> = if self.sidebar_expanded {
            let branches = features::branches::view::view(self);
            let stash = features::stash::view::view(self);
            let remotes = features::remotes::view::view(self);

            container(
                column![
                    branches,
                    horizontal_rule(1),
                    stash,
                    horizontal_rule(1),
                    remotes
                ]
                .width(220)
                .height(Length::Fill),
            )
            .width(220)
            .height(Length::Fill)
            .style(theme::sidebar_style)
            .into()
        } else {
            Space::with_width(0).into()
        };

        // ── Commit log ────────────────────────────────────────────────────
        let commit_log = features::commits::view::view(self);

        // ── Diff viewer ───────────────────────────────────────────────────
        let diff_viewer = features::diff::view::view(self);

        // ── Middle row: sidebar | commit log | diff viewer ────────────────
        let middle = row![sidebar, commit_log, diff_viewer]
            .spacing(1)
            .height(Length::Fill)
            .width(Length::Fill);

        // ── Staging area ──────────────────────────────────────────────────
        let staging = features::staging::view::view(self);

        // ── Status bar ────────────────────────────────────────────────────
        let status_bar = status_bar_view(self);

        // ── Error banner (if any) ─────────────────────────────────────────
        let mut main_col = column![].width(Length::Fill).height(Length::Fill);

        if let Some(ref err) = self.error_message {
            main_col = main_col.push(error_banner(err));
        }

        main_col = main_col
            .push(header)
            .push(middle)
            .push(horizontal_rule(1))
            .push(staging)
            .push(status_bar);

        container(main_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::BG)),
                ..Default::default()
            })
            .into()
    }
}

/// Render the status bar at the very bottom of the window.
fn status_bar_view(state: &GitKraft) -> Element<'_, Message> {
    let status_text = if state.is_loading {
        state
            .status_message
            .as_deref()
            .unwrap_or("Loading…")
            .to_string()
    } else {
        state
            .status_message
            .as_deref()
            .unwrap_or("Ready")
            .to_string()
    };

    let status_label = text(status_text).size(12).color(theme::TEXT_SECONDARY);

    let branch_info: Element<'_, Message> = if let Some(ref branch) = state.current_branch {
        let icon = text('\u{F404}')
            .font(iced_fonts::BOOTSTRAP_FONT)
            .size(12)
            .color(theme::ACCENT);
        let label = text(branch.as_str()).size(12).color(theme::TEXT_PRIMARY);
        row![icon, Space::with_width(4), label]
            .align_y(Alignment::Center)
            .into()
    } else {
        Space::with_width(0).into()
    };

    let repo_state_info: Element<'_, Message> = if let Some(ref info) = state.repo_info {
        let state_str = format!("{}", info.state);
        if state_str != "Clean" {
            text(state_str).size(12).color(theme::YELLOW).into()
        } else {
            Space::with_width(0).into()
        }
    } else {
        Space::with_width(0).into()
    };

    let changes_summary = {
        let unstaged_count = state.unstaged_changes.len();
        let staged_count = state.staged_changes.len();
        if unstaged_count > 0 || staged_count > 0 {
            text(format!("{unstaged_count} unstaged, {staged_count} staged"))
                .size(12)
                .color(theme::MUTED)
        } else {
            text("Working tree clean").size(12).color(theme::MUTED)
        }
    };

    let bar = row![
        status_label,
        Space::with_width(Length::Fill),
        changes_summary,
        Space::with_width(16),
        repo_state_info,
        Space::with_width(16),
        branch_info,
    ]
    .align_y(Alignment::Center)
    .padding([4, 10])
    .width(Length::Fill);

    container(bar)
        .width(Length::Fill)
        .style(theme::header_style)
        .into()
}

/// Render an error banner at the top of the window with a dismiss button.
fn error_banner(message: &str) -> Element<'_, Message> {
    let icon = text('\u{F333}') // exclamation-triangle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(theme::RED);

    let msg = text(message.to_string())
        .size(13)
        .color(theme::TEXT_PRIMARY);

    let dismiss = iced::widget::button(
        text('\u{F62A}') // x-circle
            .font(iced_fonts::BOOTSTRAP_FONT)
            .size(14)
            .color(theme::TEXT_SECONDARY),
    )
    .padding([2, 6])
    .on_press(Message::DismissError);

    let banner_row = row![
        icon,
        Space::with_width(8),
        msg,
        Space::with_width(Length::Fill),
        dismiss,
    ]
    .align_y(Alignment::Center)
    .padding([6, 12])
    .width(Length::Fill);

    container(banner_row)
        .width(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.35,
                g: 0.10,
                b: 0.10,
                a: 1.0,
            })),
            ..Default::default()
        })
        .into()
}
