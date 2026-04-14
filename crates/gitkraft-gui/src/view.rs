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
        let drag_area = mouse_area(body).on_release(Message::PaneDragEnd);
        let ma: Element<'_, Message> = if self.dragging.is_some() || self.dragging_h.is_some() {
            drag_area
                .on_move(|p| Message::PaneDragMove(p.x, p.y))
                .into()
        } else {
            drag_area.into()
        };

        // ── Context menu overlay ──────────────────────────────────────────
        if self.active_tab().context_menu.is_some() {
            // Transparent full-screen backdrop — clicking it dismisses the menu.
            let backdrop = mouse_area(
                container(Space::new(Length::Fill, Length::Fill)).style(theme::backdrop_style),
            )
            .on_press(Message::CloseContextMenu)
            .on_right_press(Message::CloseContextMenu);

            let (menu_x, menu_y) = context_menu_position(self);
            let menu_panel = context_menu_panel(self, &c);

            let positioned = column![
                Space::with_height(menu_y),
                row![Space::with_width(menu_x), menu_panel,],
            ]
            .width(Length::Fill)
            .height(Length::Fill);

            iced::widget::stack![ma, backdrop, positioned].into()
        } else {
            ma
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

/// Approximate pixel position of the context menu based on what was right-clicked.
fn context_menu_position(state: &GitKraft) -> (f32, f32) {
    // Layout constants — keep in sync with the actual widget sizes.
    const TAB_BAR_H: f32 = 34.0;
    const HEADER_H: f32 = 46.0;
    const SECTION_HEADER_H: f32 = 38.0;
    const BRANCH_ROW_H: f32 = 32.0;
    const COMMIT_LOG_HEADER_H: f32 = 38.0;
    const COMMIT_ROW_H: f32 = 26.0; // matches commits/view.rs ROW_HEIGHT

    // Branch menus open overlapping the sidebar (x ≈ 5) so they appear
    // right next to the branch row rather than all the way over in the
    // commit-log panel.
    // Commit menus open inside the commit-log panel, just after the sidebar.
    let commit_log_x = if state.sidebar_expanded {
        state.sidebar_width + 10.0
    } else {
        10.0
    };

    match &state.active_tab().context_menu {
        Some(crate::state::ContextMenu::Branch { local_index, .. }) => {
            let y = (TAB_BAR_H + HEADER_H + SECTION_HEADER_H + *local_index as f32 * BRANCH_ROW_H)
                .min(500.0);
            // Start at the left edge of the sidebar so the menu overlays it.
            (5.0, y)
        }
        Some(crate::state::ContextMenu::Commit { index, .. }) => {
            let scroll_y = state.active_tab().commit_scroll_offset;
            let first_visible = (scroll_y / COMMIT_ROW_H) as usize;
            let visible_row = index.saturating_sub(first_visible);
            let y =
                (TAB_BAR_H + HEADER_H + COMMIT_LOG_HEADER_H + visible_row as f32 * COMMIT_ROW_H)
                    .min(500.0);
            (commit_log_x, y)
        }
        None => (0.0, 0.0),
    }
}

/// Build the context menu panel widget for the currently active menu.
fn context_menu_panel<'a>(state: &'a GitKraft, c: &ThemeColors) -> Element<'a, Message> {
    use iced::widget::{button, column, container, horizontal_rule, row, text, Space};
    use iced::{Alignment, Length};

    let text_primary = c.text_primary;
    let menu_item = move |label: &str, msg: Message| {
        button(
            row![
                Space::with_width(4),
                text(label.to_string()).size(13).color(text_primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding([7, 12])
        .width(Length::Fill)
        .style(theme::ghost_button)
        .on_press(msg)
    };

    let content: Element<'a, Message> = match &state.active_tab().context_menu {
        Some(crate::state::ContextMenu::Branch {
            name, is_current, ..
        }) => {
            let remote = state
                .active_tab()
                .remotes
                .first()
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "origin".to_string());

            let header = container(text(format!("Branch: {name}")).size(12).color(c.muted))
                .padding(iced::Padding {
                    top: 8.0,
                    right: 14.0,
                    bottom: 6.0,
                    left: 14.0,
                })
                .width(Length::Fill);

            let mut col = column![header, horizontal_rule(1)];

            if !is_current {
                col = col.push(menu_item("Checkout", Message::CheckoutBranch(name.clone())));
            }

            let push_label = format!("Push to {remote}");
            let pull_label = format!("Pull from {remote} (rebase)");
            let rebase_label = format!("Rebase current onto '{name}'");

            col = col
                .push(menu_item(&push_label, Message::PushBranch(name.clone())))
                .push(menu_item(&pull_label, Message::PullBranch(name.clone())))
                .push(menu_item(&rebase_label, Message::RebaseOnto(name.clone())))
                .push(horizontal_rule(1))
                .push(menu_item(
                    "Rename\u{2026}",
                    Message::BeginRenameBranch(name.clone()),
                ))
                .push(menu_item("Delete", Message::DeleteBranch(name.clone())))
                .push(horizontal_rule(1))
                .push(menu_item(
                    "Copy branch name",
                    Message::CopyText(name.clone()),
                ));

            col.into()
        }

        Some(crate::state::ContextMenu::Commit { index, oid }) => {
            let tab = state.active_tab();
            let short = &oid[..7.min(oid.len())];
            let msg_text = tab
                .commits
                .get(*index)
                .map(|c| c.message.clone())
                .unwrap_or_default();

            let header = container(text(format!("Commit: {short}")).size(12).color(c.muted))
                .padding(iced::Padding {
                    top: 8.0,
                    right: 14.0,
                    bottom: 6.0,
                    left: 14.0,
                })
                .width(Length::Fill);

            column![
                header,
                horizontal_rule(1),
                menu_item(
                    "Checkout (detached HEAD)",
                    Message::CheckoutCommitDetached(oid.clone()),
                ),
                menu_item(
                    "Rebase current branch onto this",
                    Message::RebaseOntoCommit(oid.clone()),
                ),
                menu_item("Revert commit", Message::RevertCommit(oid.clone())),
                horizontal_rule(1),
                menu_item("Copy commit SHA", Message::CopyText(oid.clone())),
                menu_item("Copy commit message", Message::CopyText(msg_text)),
            ]
            .into()
        }

        None => Space::with_width(0).into(),
    };

    container(content)
        .width(280)
        .style(theme::context_menu_style)
        .into()
}
