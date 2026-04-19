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
use crate::icons;
use crate::message::Message;
use crate::state::{DragTarget, DragTargetH, GitKraft};
use crate::theme;
use crate::theme::ThemeColors;
use crate::view_utils;
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
            Space::new(0, 0).into()
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

        // on_move is always active so cursor_pos stays current for context
        // menus.  Virtual scrolling keeps the per-frame rebuild cost low
        // (~66 commit rows instead of 500) so this is acceptable.
        let ma: Element<'_, Message> = mouse_area(body)
            .on_move(|p| Message::PaneDragMove(p.x, p.y))
            .on_release(Message::PaneDragEnd)
            .into();

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
                Space::new(0, menu_y),
                row![Space::new(menu_x, 0), menu_panel,],
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
        let icon = icon!(icons::GIT_BRANCH, 12, c.accent);
        let label = text(branch.as_str()).size(12).color(c.text_primary);
        row![icon, Space::new(4, 0), label]
            .align_y(Alignment::Center)
            .into()
    } else {
        Space::new(0, 0).into()
    };

    let repo_state_info: Element<'_, Message> = if let Some(ref info) = tab.repo_info {
        let state_str = format!("{}", info.state);
        if state_str != "Clean" {
            text(state_str).size(12).color(c.yellow).into()
        } else {
            Space::new(0, 0).into()
        }
    } else {
        Space::new(0, 0).into()
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

    let zoom_label: Element<'_, Message> = if (state.ui_scale - 1.0).abs() > 0.01 {
        text(format!("{}%", (state.ui_scale * 100.0).round() as u32))
            .size(11)
            .color(c.muted)
            .into()
    } else {
        Space::new(0, 0).into()
    };

    let bar = row![
        status_label,
        Space::new(Length::Fill, 0),
        changes_summary,
        Space::new(16, 0),
        zoom_label,
        Space::new(16, 0),
        repo_state_info,
        Space::new(16, 0),
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
    let icon = icon!(icons::EXCLAMATION_TRIANGLE, 14, c.red);

    let msg = text(message.to_string()).size(13).color(c.text_primary);

    let dismiss = iced::widget::button(icon!(icons::X_CIRCLE, 14, c.text_secondary))
        .padding([2, 6])
        .on_press(Message::DismissError);

    let banner_row = row![
        icon,
        Space::new(8, 0),
        msg,
        Space::new(Length::Fill, 0),
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
    // Use the position that was frozen when the menu opened, not the live
    // cursor_pos — otherwise the panel would follow the mouse.
    // Nudge right/down by 2 px so the pointer tip sits just inside the panel.
    let (x, y) = state.active_tab().context_menu_pos;
    ((x + 2.0).max(2.0), (y + 2.0).max(2.0))
}

/// Build the context menu panel widget for the currently active menu.
fn context_menu_panel<'a>(state: &'a GitKraft, c: &ThemeColors) -> Element<'a, Message> {
    use iced::widget::{button, column, container, row, text, Space};
    use iced::{Alignment, Length};

    let text_primary = c.text_primary;
    let menu_item = move |label: &str, msg: Message| {
        button(
            row![
                Space::new(4, 0),
                text(label.to_string()).size(13).color(text_primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding([7, 12])
        .width(Length::Fill)
        .style(theme::context_menu_item)
        .on_press(msg)
    };

    let content: Element<'a, Message> = match &state.active_tab().context_menu {
        Some(crate::state::ContextMenu::Branch {
            name, is_current, ..
        }) => {
            let tab = state.active_tab();
            let remote = tab
                .remotes
                .first()
                .map(|r| r.name.clone())
                .unwrap_or_else(|| "origin".to_string());

            // Look up the branch tip OID for SHA copy and tag creation.
            let tip_oid: Option<String> = tab
                .branches
                .iter()
                .find(|b| &b.name == name)
                .and_then(|b| b.target_oid.clone());

            let header =
                view_utils::context_menu_header::<Message>(format!("Branch: {name}"), c.muted);

            let mut col = column![header];

            // Group 1: Checkout (when not on this branch)
            if !is_current {
                col = col.push(menu_item("Checkout", Message::CheckoutBranch(name.clone())));
            }

            // Group 2: Remote sync
            let push_label = format!("Push to {remote}");
            let pull_label = format!("Pull from {remote} (rebase)");
            col = col
                .push(menu_item(&push_label, Message::PushBranch(name.clone())))
                .push(menu_item(&pull_label, Message::PullBranch(name.clone())));

            // Group 3: Rebase / merge
            col = col.push(view_utils::context_menu_separator::<Message>());
            let rebase_label = format!("Rebase current onto '{name}'");
            col = col.push(menu_item(&rebase_label, Message::RebaseOnto(name.clone())));
            if !is_current {
                col = col.push(menu_item(
                    "Merge into current branch",
                    Message::MergeBranch(name.clone()),
                ));
            }

            // Group 4: Branch management
            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col
                .push(menu_item(
                    "Rename\u{2026}",
                    Message::BeginRenameBranch(name.clone()),
                ))
                .push(menu_item("Delete", Message::DeleteBranch(name.clone())));

            // Group 5: Copy info
            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col.push(menu_item(
                "Copy branch name",
                Message::CopyText(name.clone()),
            ));
            if let Some(ref oid) = tip_oid {
                col = col.push(menu_item(
                    "Copy tip commit SHA",
                    Message::CopyText(oid.clone()),
                ));
            }

            // Group 6: Tag creation
            if tip_oid.is_some() {
                col = col.push(view_utils::context_menu_separator::<Message>());
                let oid = tip_oid.clone().unwrap();
                col = col
                    .push(menu_item(
                        "Create tag here",
                        Message::BeginCreateTag(oid.clone(), false),
                    ))
                    .push(menu_item(
                        "Create annotated tag here\u{2026}",
                        Message::BeginCreateTag(oid, true),
                    ));
            }

            col.into()
        }

        Some(crate::state::ContextMenu::RemoteBranch { name }) => {
            // Extract remote and branch parts for display
            let (remote, short_name) = name.split_once('/').unwrap_or(("", name.as_str()));

            let header =
                view_utils::context_menu_header::<Message>(format!("Remote: {name}"), c.muted);

            // Check if a local branch with the same short name already exists
            let local_exists =
                state.active_tab().branches.iter().any(|b| {
                    b.branch_type == gitkraft_core::BranchType::Local && b.name == short_name
                });

            let mut col = column![header];

            // Checkout (only if no local branch with same name exists)
            if !local_exists {
                col = col.push(menu_item(
                    &format!("Checkout as '{short_name}'"),
                    Message::CheckoutRemoteBranch(name.clone()),
                ));
            }

            // Delete from remote
            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col.push(menu_item(
                &format!("Delete from {remote}"),
                Message::DeleteRemoteBranch(name.clone()),
            ));

            // Copy info
            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col.push(menu_item(
                "Copy branch name",
                Message::CopyText(name.clone()),
            ));
            col = col.push(menu_item(
                &format!("Copy short name '{short_name}'"),
                Message::CopyText(short_name.to_string()),
            ));

            // Look up tip OID
            let tip_oid: Option<String> = state
                .active_tab()
                .branches
                .iter()
                .find(|b| &b.name == name)
                .and_then(|b| b.target_oid.clone());

            if let Some(ref oid) = tip_oid {
                col = col.push(menu_item(
                    "Copy tip commit SHA",
                    Message::CopyText(oid.clone()),
                ));
            }

            col.into()
        }

        Some(crate::state::ContextMenu::Commit { index, oid }) => {
            let tab = state.active_tab();
            let short = gitkraft_core::utils::short_oid_str(oid);
            let msg_text = tab
                .commits
                .get(*index)
                .map(|c| c.message.clone())
                .unwrap_or_default();

            let header =
                view_utils::context_menu_header::<Message>(format!("Commit: {short}"), c.muted);

            column![
                header,
                menu_item(
                    "Checkout (detached HEAD)",
                    Message::CheckoutCommitDetached(oid.clone()),
                ),
                menu_item(
                    "Rebase current branch onto this",
                    Message::RebaseOntoCommit(oid.clone()),
                ),
                menu_item("Revert commit", Message::RevertCommit(oid.clone())),
                menu_item(
                    "Reset here — soft (keep staged)",
                    Message::ResetSoft(oid.clone())
                ),
                menu_item(
                    "Reset here — mixed (keep files)",
                    Message::ResetMixed(oid.clone())
                ),
                menu_item(
                    "Reset here — hard (discard all)",
                    Message::ResetHard(oid.clone())
                ),
                menu_item("Copy commit SHA", Message::CopyText(oid.clone())),
                menu_item("Copy commit message", Message::CopyText(msg_text)),
            ]
            .into()
        }

        None => Space::new(0, 0).into(),
    };

    container(content)
        .width(280)
        .style(theme::context_menu_style)
        .into()
}
