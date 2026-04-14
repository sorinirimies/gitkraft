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
//!
//! All vertical and horizontal dividers between panes are **draggable** — the
//! user can resize the sidebar, commit-log, diff-viewer, and staging area by
//! grabbing the thin divider lines and dragging.
//!
//! The outer-most widget is a `mouse_area` that captures `on_release` events
//! unconditionally, and `on_move` events **only while a drag is in progress**.
//! This avoids firing `PaneDragMove → update() → view()` on every cursor
//! movement when no resize drag is active.

use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Alignment, Element, Length};

use crate::features;
use crate::message::Message;
use crate::state::{DragTarget, DragTargetH, GitKraft};
use crate::theme;
use crate::theme::ThemeColors;
use crate::widgets;

impl GitKraft {
    /// Top-level view — called by the Iced runtime on every frame.
    pub fn view(&self) -> Element<'_, Message> {
        let c = self.colors();

        // ── Tab bar (always visible) ──────────────────────────────────────
        let tab_bar = widgets::tab_bar::view(self);

        if !self.has_repo() {
            // Show the tab bar above the welcome screen so users can
            // switch between tabs even when the active one has no repo.
            let welcome = features::repo::view::welcome_view(self);
            let outer = column![tab_bar, welcome]
                .width(Length::Fill)
                .height(Length::Fill);
            return container(outer)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::bg_style)
                .into();
        }

        let tab = self.active_tab();

        // ── Header toolbar ────────────────────────────────────────────────
        let header = widgets::header::view(self);

        // ── Sidebar (branches + stash + remotes) ──────────────────────────
        let sidebar: Element<'_, Message> = if self.sidebar_expanded {
            let branches = features::branches::view::view(self);
            let stash = features::stash::view::view(self);
            let remotes = features::remotes::view::view(self);

            let sidebar_content = container(
                column![
                    branches,
                    iced::widget::horizontal_rule(1),
                    stash,
                    iced::widget::horizontal_rule(1),
                    remotes
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fixed(self.sidebar_width))
            .height(Length::Fill)
            .style(theme::sidebar_style);

            let divider = widgets::divider::vertical_divider(DragTarget::SidebarRight, &c);

            row![sidebar_content, divider].height(Length::Fill).into()
        } else {
            Space::with_width(0).into()
        };

        // ── Commit log ────────────────────────────────────────────────────
        let commit_log_content = container(features::commits::view::view(self))
            .width(Length::Fixed(self.commit_log_width))
            .height(Length::Fill);

        let commit_divider = widgets::divider::vertical_divider(DragTarget::CommitLogRight, &c);

        let commit_log: Element<'_, Message> = row![commit_log_content, commit_divider]
            .height(Length::Fill)
            .into();

        // ── Diff viewer (fills all remaining horizontal space) ────────────
        let diff_viewer = container(features::diff::view::view(self))
            .width(Length::Fill)
            .height(Length::Fill);

        // ── Middle row: sidebar | divider | commit log | divider | diff ───
        let middle = row![sidebar, commit_log, diff_viewer]
            .height(Length::Fill)
            .width(Length::Fill);

        // ── Horizontal divider between middle and staging ─────────────────
        let h_divider = widgets::divider::horizontal_divider(DragTargetH::StagingTop, &c);

        // ── Staging area ──────────────────────────────────────────────────
        let staging = container(features::staging::view::view(self))
            .width(Length::Fill)
            .height(Length::Fixed(self.staging_height));

        // ── Status bar ────────────────────────────────────────────────────
        let status_bar = status_bar_view(self);

        // ── Error banner (if any) ─────────────────────────────────────────
        let mut main_col = column![].width(Length::Fill).height(Length::Fill);

        main_col = main_col.push(tab_bar);

        if let Some(ref err) = tab.error_message {
            main_col = main_col.push(error_banner(err, &c));
        }

        main_col = main_col
            .push(header)
            .push(middle)
            .push(h_divider)
            .push(staging)
            .push(status_bar);

        let body = container(main_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::bg_style);

        // Only wire on_move while a drag is actually in progress.
        // Without this, every cursor movement fires PaneDragMove which forces a
        // full view rebuild (including the O(n_commits) commit log) on every frame.
        let ma = mouse_area(body).on_release(Message::PaneDragEnd);
        if self.dragging.is_some() || self.dragging_h.is_some() {
            ma.on_move(|p| Message::PaneDragMove(p.x, p.y)).into()
        } else {
            ma.into()
        }
    }
}

/// Render the status bar at the very bottom of the window.
fn status_bar_view(state: &GitKraft) -> Element<'_, Message> {
    let tab = state.active_tab();
    let c = state.colors();

    let status_text = if tab.is_loading {
        tab.status_message
            .as_deref()
            .unwrap_or("Loading…")
            .to_string()
    } else {
        tab.status_message.as_deref().unwrap_or("Ready").to_string()
    };

    let status_label = text(status_text).size(12).color(c.text_secondary);

    let branch_info: Element<'_, Message> = if let Some(ref branch) = tab.current_branch {
        let icon = text('\u{F404}')
            .font(iced_fonts::BOOTSTRAP_FONT)
            .size(12)
            .color(c.accent);
        let label = text(branch.as_str()).size(12).color(c.text_primary);
        row![icon, Space::with_width(4), label]
            .align_y(Alignment::Center)
            .into()
    } else {
        Space::with_width(0).into()
    };

    let repo_state_info: Element<'_, Message> = if let Some(ref info) = tab.repo_info {
        let state_str = format!("{}", info.state);
        if state_str != "Clean" {
            text(state_str).size(12).color(c.yellow).into()
        } else {
            Space::with_width(0).into()
        }
    } else {
        Space::with_width(0).into()
    };

    let changes_summary = {
        let unstaged_count = tab.unstaged_changes.len();
        let staged_count = tab.staged_changes.len();
        if unstaged_count > 0 || staged_count > 0 {
            text(format!("{unstaged_count} unstaged, {staged_count} staged"))
                .size(12)
                .color(c.muted)
        } else {
            text("Working tree clean").size(12).color(c.muted)
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
fn error_banner<'a>(message: &str, c: &ThemeColors) -> Element<'a, Message> {
    let icon = text('\u{F333}') // exclamation-triangle
        .font(iced_fonts::BOOTSTRAP_FONT)
        .size(14)
        .color(c.red);

    let msg = text(message.to_string()).size(13).color(c.text_primary);

    let dismiss = iced::widget::button(
        text('\u{F62A}') // x-circle
            .font(iced_fonts::BOOTSTRAP_FONT)
            .size(14)
            .color(c.text_secondary),
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
        .style(theme::error_banner_style)
        .into()
}
