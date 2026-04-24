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
                    iced::widget::rule::horizontal(1),
                    stash,
                    iced::widget::rule::horizontal(1),
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
            Space::new().into()
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

        // ── Search overlay ────────────────────────────────────────────────
        let ma: Element<'_, Message> = if self.search_visible {
            let search_panel = search_overlay(self, &c);
            iced::widget::stack![ma, search_panel].into()
        } else {
            ma
        };

        // ── Context menu overlay ──────────────────────────────────────────
        if self.active_tab().context_menu.is_some() {
            // Transparent full-screen backdrop — clicking it dismisses the menu.
            let backdrop = mouse_area(
                container(Space::new().width(Length::Fill).height(Length::Fill))
                    .style(theme::backdrop_style),
            )
            .on_press(Message::CloseContextMenu)
            .on_right_press(Message::CloseContextMenu);

            let (menu_x, menu_y) = context_menu_position(self);
            let menu_panel = context_menu_panel(self, &c);

            let positioned = column![
                Space::new().height(menu_y),
                row![Space::new().width(menu_x), menu_panel,],
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
        row![icon, Space::new().width(4), label]
            .align_y(Alignment::Center)
            .into()
    } else {
        Space::new().into()
    };

    let repo_state_info: Element<'_, Message> = if let Some(ref info) = tab.repo_info {
        let state_str = format!("{}", info.state);
        if state_str != "Clean" {
            text(state_str).size(12).color(c.yellow).into()
        } else {
            Space::new().into()
        }
    } else {
        Space::new().into()
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
        Space::new().into()
    };

    let bar = row![
        status_label,
        Space::new().width(Length::Fill),
        changes_summary,
        Space::new().width(16),
        zoom_label,
        Space::new().width(16),
        repo_state_info,
        Space::new().width(16),
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
        Space::new().width(8),
        msg,
        Space::new().width(Length::Fill),
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

/// Render the search overlay — a centered panel with an input and results list.
/// When a commit is selected, the panel expands to show changed files and diffs.
fn search_overlay<'a>(state: &'a GitKraft, c: &ThemeColors) -> Element<'a, Message> {
    use iced::widget::{
        button, checkbox, column, container, mouse_area, row, scrollable, text, text_input, Space,
    };
    use iced::{Alignment, Length};

    let has_diff_files = !state.search_diff_files.is_empty();
    let has_diff_content = !state.search_diff_content.is_empty();

    // ── Close button ──────────────────────────────────────────────────────
    let close_btn = button(text("\u{2715}").size(14).color(c.text_secondary))
        .padding([4, 8])
        .style(theme::ghost_button)
        .on_press(Message::ToggleSearch);

    // ── Left panel: search input + commit results ─────────────────────────
    let input = text_input("Search commits…", &state.search_query)
        .on_input(Message::SearchQueryChanged)
        .on_submit(Message::ConfirmSearchResult)
        .padding(10)
        .size(16);

    let mut results_col = column![].spacing(2).width(Length::Fill);

    if state.search_results.is_empty() && state.search_query.len() >= 2 {
        results_col = results_col.push(
            container(text("No results found").size(13).color(c.muted))
                .padding([12, 8])
                .width(Length::Fill)
                .center_x(Length::Fill),
        );
    }

    for (i, commit) in state.search_results.iter().take(50).enumerate() {
        let is_selected = state.search_selected == Some(i);
        let is_diffed = state
            .search_diff_oid
            .as_ref()
            .is_some_and(|oid| *oid == commit.oid);
        let bg_style = if is_diffed {
            theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
        } else if is_selected {
            theme::selected_row_style as fn(&iced::Theme) -> iced::widget::container::Style
        } else {
            theme::surface_style as fn(&iced::Theme) -> iced::widget::container::Style
        };

        let oid_label = text(&commit.short_oid)
            .size(12)
            .color(c.accent)
            .font(iced::Font::MONOSPACE);

        let summary_label = text(&commit.summary).size(13).color(c.text_primary);

        let author_label = text(&commit.author_name).size(11).color(c.text_secondary);

        let time_label = text(commit.relative_time()).size(11).color(c.muted);

        let row_content = row![
            oid_label,
            Space::new().width(8),
            summary_label,
            Space::new().width(Length::Fill),
            author_label,
            Space::new().width(8),
            time_label,
        ]
        .align_y(Alignment::Center)
        .padding([6, 10]);

        let result_btn = button(row_content)
            .padding(0)
            .width(Length::Fill)
            .style(theme::ghost_button)
            .on_press(Message::ConfirmSearchResult);

        let result_row: Element<'a, Message> =
            mouse_area(container(result_btn).width(Length::Fill).style(bg_style))
                .on_press(Message::SelectSearchResult(i))
                .on_right_press(Message::OpenSearchResultContextMenu(i))
                .into();

        results_col = results_col.push(result_row);
    }

    let result_count = if !state.search_results.is_empty() {
        text(format!("{} result(s)", state.search_results.len()))
            .size(11)
            .color(c.muted)
    } else {
        text("").size(1)
    };

    let left_header = row![
        icon!(icons::CLOCK_HISTORY, 16, c.accent),
        Space::new().width(8),
        text("Search Commits").size(16).color(c.text_primary),
        Space::new().width(Length::Fill),
        result_count,
        Space::new().width(8),
        close_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 12]);

    let scrollable_results = scrollable(results_col)
        .height(Length::Fill)
        .direction(crate::view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

    let left_panel = column![left_header, input, scrollable_results]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(4);

    // ── Right panel: file list + diff (only when a commit is selected) ────
    let panel: Element<'a, Message> = if has_diff_content {
        // Show combined diff content for the selected file(s)
        let file_count = state.search_diff_content.len();
        let title_label = if file_count == 1 {
            state.search_diff_content[0].display_path().to_string()
        } else {
            format!("{file_count} file(s)")
        };

        let back_btn = button(
            row![
                text("← ").size(14).color(c.accent),
                text("Back to file list").size(13).color(c.text_primary),
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .style(theme::ghost_button)
        .on_press(Message::SearchDiffBack);

        let close_btn2 = button(text("\u{2715}").size(14).color(c.text_secondary))
            .padding([4, 8])
            .style(theme::ghost_button)
            .on_press(Message::ToggleSearch);

        let diff_header = row![
            back_btn,
            Space::new().width(Length::Fill),
            text(title_label).size(13).color(c.accent),
            Space::new().width(8),
            close_btn2,
        ]
        .align_y(Alignment::Center)
        .padding([4, 8]);

        let mut diff_lines_col = column![].spacing(0).width(Length::Fill);
        for diff in &state.search_diff_content {
            // File separator header
            let status_color = match diff.status.color_category() {
                gitkraft_core::StatusColorCategory::Added => c.green,
                gitkraft_core::StatusColorCategory::Modified => c.yellow,
                gitkraft_core::StatusColorCategory::Deleted => c.red,
                gitkraft_core::StatusColorCategory::Renamed => c.accent,
            };
            if file_count > 1 {
                diff_lines_col = diff_lines_col.push(
                    container(
                        row![
                            text(format!("{}", diff.status))
                                .size(12)
                                .color(status_color)
                                .font(iced::Font::MONOSPACE),
                            Space::new().width(8),
                            text(diff.display_path()).size(13).color(c.text_primary),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(theme::surface_style),
                );
            }
            for hunk in &diff.hunks {
                for line in &hunk.lines {
                    let (prefix, content, color) = match line {
                        gitkraft_core::DiffLine::Context(s) => (" ", s.as_str(), c.text_secondary),
                        gitkraft_core::DiffLine::Addition(s) => ("+", s.as_str(), c.green),
                        gitkraft_core::DiffLine::Deletion(s) => ("-", s.as_str(), c.red),
                        gitkraft_core::DiffLine::HunkHeader(s) => ("@@", s.as_str(), c.accent),
                    };
                    diff_lines_col = diff_lines_col.push(
                        text(format!("{prefix} {content}"))
                            .size(12)
                            .color(color)
                            .font(iced::Font::MONOSPACE),
                    );
                }
            }
        }

        let scrollable_diff = scrollable(
            container(diff_lines_col)
                .padding([4, 8])
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .direction(crate::view_utils::thin_scrollbar())
        .style(crate::theme::overlay_scrollbar);

        let right_panel = column![diff_header, scrollable_diff]
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(4);

        let content = row![
            container(left_panel).width(Length::FillPortion(2)),
            container(right_panel).width(Length::FillPortion(3)),
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(1100)
            .height(600)
            .style(theme::context_menu_style)
            .padding(8)
            .into()
    } else if has_diff_files {
        // Show file list for the selected commit
        let oid_short = state
            .search_diff_oid
            .as_ref()
            .map(|o| &o[..7.min(o.len())])
            .unwrap_or("???");

        let file_count = state.search_diff_files.len();
        let selected_count = state.search_diff_selected.len();

        let select_all_label = if selected_count == file_count {
            "Deselect All"
        } else {
            "Select All"
        };

        let select_all_btn = button(text(select_all_label).size(12).color(c.accent))
            .padding([4, 8])
            .style(theme::ghost_button)
            .on_press(Message::ToggleSearchDiffSelectAll);

        let diff_selected_btn: Element<'a, Message> = if selected_count > 0 {
            button(
                text(format!("Diff Selected ({selected_count})"))
                    .size(12)
                    .color(c.green),
            )
            .padding([4, 8])
            .style(theme::ghost_button)
            .on_press(Message::DiffSelectedFiles)
            .into()
        } else {
            Space::new().width(0).into()
        };

        let close_btn3 = button(text("\u{2715}").size(14).color(c.text_secondary))
            .padding([4, 8])
            .style(theme::ghost_button)
            .on_press(Message::ToggleSearch);

        let right_header = row![
            text(format!("Files changed vs working tree ({oid_short})"))
                .size(14)
                .color(c.text_primary),
            Space::new().width(Length::Fill),
            text(format!("{file_count} file(s)"))
                .size(11)
                .color(c.muted),
            Space::new().width(8),
            diff_selected_btn,
            Space::new().width(4),
            select_all_btn,
            Space::new().width(4),
            close_btn3,
        ]
        .align_y(Alignment::Center)
        .padding([8, 12]);

        let mut files_col = column![].spacing(2).width(Length::Fill);

        for (i, file) in state.search_diff_files.iter().enumerate() {
            let is_checked = state.search_diff_selected.contains(&i);
            let status_str = format!("{}", file.status);
            let status_color = match file.status.color_category() {
                gitkraft_core::StatusColorCategory::Added => c.green,
                gitkraft_core::StatusColorCategory::Modified => c.yellow,
                gitkraft_core::StatusColorCategory::Deleted => c.red,
                gitkraft_core::StatusColorCategory::Renamed => c.accent,
            };

            let file_row = button(
                row![
                    checkbox(is_checked).on_toggle(move |_| Message::ToggleSearchDiffFile(i)),
                    Space::new().width(4),
                    text(status_str)
                        .size(12)
                        .color(status_color)
                        .font(iced::Font::MONOSPACE),
                    Space::new().width(8),
                    text(file.display_path()).size(13).color(c.text_primary),
                    Space::new().width(Length::Fill),
                ]
                .align_y(Alignment::Center)
                .padding([4, 8]),
            )
            .padding(0)
            .width(Length::Fill)
            .style(theme::ghost_button)
            .on_press(Message::ViewSearchDiffFile(i));

            files_col = files_col.push(file_row);
        }

        let scrollable_files = scrollable(files_col)
            .height(Length::Fill)
            .direction(crate::view_utils::thin_scrollbar())
            .style(crate::theme::overlay_scrollbar);

        let right_panel = column![right_header, scrollable_files]
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(4);

        let content = row![
            container(left_panel).width(Length::FillPortion(2)),
            container(right_panel).width(Length::FillPortion(3)),
        ]
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill);

        container(content)
            .width(1100)
            .height(600)
            .style(theme::context_menu_style)
            .padding(8)
            .into()
    } else {
        // No commit selected yet — just show the search panel
        container(left_panel)
            .width(700)
            .height(500)
            .style(theme::context_menu_style)
            .padding(8)
            .into()
    };

    // Center the panel on screen with a backdrop
    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .style(theme::backdrop_style),
    )
    .on_press(Message::ToggleSearch);

    // Wrap the panel in a mouse_area that swallows clicks so they don't
    // bubble up to the backdrop and dismiss the dialog.
    let panel_intercepted = mouse_area(panel).on_press(Message::Noop);

    let centered = container(panel_intercepted)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    iced::widget::stack![backdrop, centered].into()
}

/// Build the context menu panel widget for the currently active menu.
fn context_menu_panel<'a>(state: &'a GitKraft, c: &ThemeColors) -> Element<'a, Message> {
    use iced::widget::{button, column, container, row, text, Space};
    use iced::{Alignment, Length};

    let text_primary = c.text_primary;
    let menu_item = move |label: &str, msg: Message| {
        button(
            row![
                Space::new().width(4),
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
            let multi_count = tab.selected_commits.len();

            if multi_count > 1 {
                // ── Multi-commit ─────────────────────────────────────────────────
                let header = view_utils::context_menu_header::<Message>(
                    format!("{} commits selected", multi_count),
                    c.accent,
                );

                // Collect OIDs for the selected commits in selection order
                let oids: Vec<String> = tab
                    .selected_commits
                    .iter()
                    .filter_map(|&i| tab.commits.get(i).map(|c| c.oid.clone()))
                    .collect();

                let shas_joined = oids
                    .iter()
                    .filter_map(|o| tab.commits.iter().find(|c| c.oid == *o))
                    .map(|c| c.short_oid.clone())
                    .collect::<Vec<_>>()
                    .join("\n");

                let messages_joined = oids
                    .iter()
                    .filter_map(|o| tab.commits.iter().find(|c| c.oid == *o))
                    .map(|c| c.message.trim().to_string())
                    .collect::<Vec<_>>()
                    .join("\n\n");

                let mut col = column![header];
                col = col.push(menu_item(
                    &format!("Cherry-pick {} commits", multi_count),
                    Message::CherryPickCommits(oids.clone()),
                ));
                col = col.push(menu_item(
                    &format!("Revert {} commits", multi_count),
                    Message::RevertCommits(oids),
                ));
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Copy commit SHAs",
                    Message::CopyText(shas_joined),
                ));
                col = col.push(menu_item(
                    "Copy commit messages",
                    Message::CopyText(messages_joined),
                ));
                col.into()
            } else {
                // ── Single commit ────────────────────────────────────────────
                let short = gitkraft_core::utils::short_oid_str(oid);
                let msg_text = tab
                    .commits
                    .get(*index)
                    .map(|c| c.message.clone())
                    .unwrap_or_default();

                let header =
                    view_utils::context_menu_header::<Message>(format!("Commit: {short}"), c.muted);

                let mut col = column![header];

                for (group_idx, group) in gitkraft_core::COMMIT_MENU_GROUPS.iter().enumerate() {
                    if group_idx > 0 {
                        col = col.push(view_utils::context_menu_separator::<Message>());
                    }
                    for &kind in *group {
                        let msg = match kind.as_simple_action() {
                            // No input needed — dispatch directly
                            Some(action) => Message::ExecuteCommitAction(oid.clone(), action),
                            // Needs input — use the existing Begin* messages
                            None => match kind {
                                gitkraft_core::CommitActionKind::CreateBranchHere => {
                                    Message::BeginCreateBranchAtCommit(oid.clone())
                                }
                                gitkraft_core::CommitActionKind::CreateTag => {
                                    Message::BeginCreateTag(oid.clone(), false)
                                }
                                gitkraft_core::CommitActionKind::CreateAnnotatedTag => {
                                    Message::BeginCreateTag(oid.clone(), true)
                                }
                                _ => Message::Noop,
                            },
                        };
                        col = col.push(menu_item(kind.label(), msg));
                    }
                }

                // Copy group — metadata, not a git operation
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col
                    .push(menu_item("Copy commit SHA", Message::CopyText(oid.clone())))
                    .push(menu_item(
                        "Copy commit message",
                        Message::CopyText(msg_text),
                    ));

                col.into()
            }
        }

        Some(crate::state::ContextMenu::Stash { index }) => {
            let index = *index;
            let header =
                view_utils::context_menu_header::<Message>(format!("stash@{{{index}}}"), c.muted);

            column![
                header,
                menu_item("View diff", Message::ViewStashDiff(index)),
                menu_item("Apply (keep stash)", Message::StashApply(index)),
                menu_item("Pop (apply + remove)", Message::StashPop(index)),
                view_utils::context_menu_separator::<Message>(),
                menu_item("Drop (delete)", Message::StashDrop(index)),
            ]
            .into()
        }

        Some(crate::state::ContextMenu::UnstagedFile { path }) => {
            let selected_count = state.active_tab().selected_unstaged.len();
            let is_multi = selected_count > 1;

            let header_text = if is_multi {
                format!("{} files selected", selected_count)
            } else {
                format!("Unstaged: {}", path.rsplit('/').next().unwrap_or(path))
            };
            let header = view_utils::context_menu_header::<Message>(header_text, c.muted);

            let mut col = column![header];

            if is_multi {
                // Batch operations for multi-select
                col = col.push(menu_item(
                    &format!("Stage {} file(s)", selected_count),
                    Message::StageSelected,
                ));
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    &format!("Discard {} file(s)", selected_count),
                    Message::DiscardSelected,
                ));
            } else {
                // Single file operations
                let diff = state
                    .active_tab()
                    .unstaged_changes
                    .iter()
                    .find(|d| d.display_path() == path.as_str())
                    .cloned()
                    .unwrap_or_else(|| gitkraft_core::DiffInfo {
                        old_file: String::new(),
                        new_file: path.clone(),
                        status: gitkraft_core::FileStatus::Modified,
                        hunks: Vec::new(),
                    });

                col = col.push(menu_item("View diff", Message::SelectDiff(diff)));
                col = col.push(menu_item("Stage file", Message::StageFile(path.clone())));
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Discard changes",
                    Message::DiscardFile(path.clone()),
                ));
            }

            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col.push(menu_item(
                "Copy filename",
                Message::CopyText(path.rsplit('/').next().unwrap_or(path).to_string()),
            ));
            col = col.push(menu_item("Copy file path", Message::CopyText(path.clone())));
            col = col.push(menu_item(
                "Open in editor",
                Message::OpenInEditor(path.clone()),
            ));
            col = col.push(menu_item(
                "Open in default program",
                Message::OpenInDefaultProgram(path.clone()),
            ));
            col = col.push(menu_item(
                "Show in folder",
                Message::ShowInFolder(path.clone()),
            ));

            col.into()
        }

        Some(crate::state::ContextMenu::StagedFile { path }) => {
            let selected_count = state.active_tab().selected_staged.len();
            let is_multi = selected_count > 1;

            let header_text = if is_multi {
                format!("{} files selected", selected_count)
            } else {
                format!("Staged: {}", path.rsplit('/').next().unwrap_or(path))
            };
            let header = view_utils::context_menu_header::<Message>(header_text, c.muted);

            let mut col = column![header];

            if is_multi {
                col = col.push(menu_item(
                    &format!("Unstage {} file(s)", selected_count),
                    Message::UnstageSelected,
                ));
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    &format!("Discard {} file(s)", selected_count),
                    Message::DiscardSelected,
                ));
            } else {
                let diff = state
                    .active_tab()
                    .staged_changes
                    .iter()
                    .find(|d| d.display_path() == path.as_str())
                    .cloned()
                    .unwrap_or_else(|| gitkraft_core::DiffInfo {
                        old_file: String::new(),
                        new_file: path.clone(),
                        status: gitkraft_core::FileStatus::Modified,
                        hunks: Vec::new(),
                    });

                col = col.push(menu_item("View diff", Message::SelectDiff(diff)));
                col = col.push(menu_item(
                    "Unstage file",
                    Message::UnstageFile(path.clone()),
                ));
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Discard changes",
                    Message::DiscardStagedFile(path.clone()),
                ));
            }

            col = col.push(view_utils::context_menu_separator::<Message>());
            col = col.push(menu_item(
                "Copy filename",
                Message::CopyText(path.rsplit('/').next().unwrap_or(path).to_string()),
            ));
            col = col.push(menu_item("Copy file path", Message::CopyText(path.clone())));
            col = col.push(menu_item(
                "Open in editor",
                Message::OpenInEditor(path.clone()),
            ));
            col = col.push(menu_item(
                "Open in default program",
                Message::OpenInDefaultProgram(path.clone()),
            ));
            col = col.push(menu_item(
                "Show in folder",
                Message::ShowInFolder(path.clone()),
            ));

            col.into()
        }

        Some(crate::state::ContextMenu::CommitFile { oid, file_path }) => {
            let tab = state.active_tab();
            let multi_count = tab.selected_commit_file_indices.len();

            if multi_count > 1 {
                // ── Multi-file ────────────────────────────────────────────────────
                let header = view_utils::context_menu_header::<Message>(
                    format!("{} files selected", multi_count),
                    c.accent,
                );

                // Collect file paths in selection order
                let file_paths: Vec<String> = tab
                    .selected_commit_file_indices
                    .iter()
                    .filter_map(|&i| {
                        tab.commit_files
                            .get(i)
                            .map(|f| f.display_path().to_string())
                    })
                    .collect();

                let paths_joined = file_paths.join("\n");

                let mut col = column![header];

                // Group 1: actions
                col = col.push(menu_item(
                    &format!("Diff {} files with working tree", multi_count),
                    Message::DiffMultiWithWorkingTree(oid.clone(), file_paths.clone()),
                ));
                col = col.push(menu_item(
                    &format!("Checkout {} files from this commit", multi_count),
                    Message::CheckoutMultiFilesAtCommit(oid.clone(), file_paths),
                ));

                // Group 2: copy
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Copy file paths",
                    Message::CopyText(paths_joined),
                ));
                col = col.push(menu_item("Copy commit SHA", Message::CopyText(oid.clone())));

                col.into()
            } else {
                // ── Single file ───────────────────────────────────────────────────
                let file_name = file_path.rsplit('/').next().unwrap_or(file_path);
                let header = view_utils::context_menu_header::<Message>(
                    format!("File: {}", file_name),
                    c.muted,
                );

                // Group 1: file actions
                let mut col = column![
                    header,
                    menu_item(
                        "Diff with working tree",
                        Message::DiffFileWithWorkingTree(oid.clone(), file_path.clone()),
                    ),
                    menu_item(
                        "Checkout file from this commit",
                        Message::CheckoutFileAtCommit(oid.clone(), file_path.clone()),
                    ),
                ];

                // Group 2: copy info
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Copy filename",
                    Message::CopyText(file_name.to_string()),
                ));
                col = col.push(menu_item(
                    "Copy file path",
                    Message::CopyText(file_path.clone()),
                ));
                col = col.push(menu_item("Copy commit SHA", Message::CopyText(oid.clone())));

                // Group 3: open
                col = col.push(view_utils::context_menu_separator::<Message>());
                col = col.push(menu_item(
                    "Open in editor",
                    Message::OpenInEditor(file_path.clone()),
                ));
                col = col.push(menu_item(
                    "Open in default program",
                    Message::OpenInDefaultProgram(file_path.clone()),
                ));
                col = col.push(menu_item(
                    "Show in folder",
                    Message::ShowInFolder(file_path.clone()),
                ));

                col.into()
            }
        }

        None => Space::new().into(),
    };

    container(content)
        .width(280)
        .style(theme::context_menu_style)
        .into()
}
