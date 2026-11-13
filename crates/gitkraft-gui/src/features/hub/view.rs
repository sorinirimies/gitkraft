//! Drawing the hub screen.

use chrono::Utc;
use gitkraft_ci::log::{AnsiColor, LineKind};
use gitkraft_ci::model::Status;
use gitkraft_ci::timing::{format_seconds, job_seconds};
use gitkraft_hub::SectionData;
use gitkraft_platforms::model::{
    ChecksState, ItemState, MergeMethod, MergeTiming, NotificationReason, ReviewVerdict,
};
use gitkraft_settings::notify::NotifyStatus;
use iced::widget::{
    Space, button, canvas, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Color, Element, Length};

use super::charts::{Bars, Heatmap};
use super::state::{HubMessage, HubState, HubView, PrTab};
use crate::message::Message;
use crate::state::GitKraft;
use crate::theme::{self, ThemeColors};

type El<'a> = Element<'a, HubMessage>;

/// The whole hub screen.
pub fn view(app: &GitKraft) -> Element<'_, Message> {
    let Some(hub) = app.hub.as_deref() else {
        return Space::new().into();
    };
    let c = app.colors();
    let body: El<'_> = match hub.view {
        HubView::Dashboard => dashboard(hub, &c),
        HubView::Pulse => pulse(hub, &c),
        HubView::Keys => keys(hub, &c),
    };
    let screen = column![
        top_bar(hub, &c),
        tabs_bar(hub, &c),
        body,
        status_line(hub, &c)
    ]
    .width(Length::Fill)
    .height(Length::Fill);
    let area = mouse_area(
        container(screen)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::bg_style),
    )
    .on_release(HubMessage::DragCancel);
    let base: El<'_> = area.into();
    let shown: El<'_> = if hub.palette.is_some() {
        iced::widget::stack![base, palette(hub, &c)].into()
    } else {
        base
    };
    shown.map(Message::Hub)
}

fn palette<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let query = hub.palette.as_deref().unwrap_or_default();
    let shown = hub.palette_shown();
    let mut list = column![].spacing(2);
    for (n, (label, _)) in shown.iter().take(14).enumerate() {
        list = list.push(
            button(text(label.clone()).size(13))
                .on_press(HubMessage::PaletteRun(n))
                .style(theme::ghost_button)
                .width(Length::Fill)
                .padding([4, 10]),
        );
    }
    if shown.is_empty() {
        list = list.push(text("nothing matches").size(13).color(c.text_secondary));
    }
    let input = text_input("Type a command…  (Enter runs the first)", query)
        .id(iced::widget::Id::new(super::state::PALETTE_INPUT))
        .on_input(HubMessage::PaletteQuery)
        .on_submit(HubMessage::PaletteSubmit)
        .padding(8);
    let card = container(column![input, list].spacing(8).width(Length::Fill))
        .width(Length::Fixed(520.0))
        .padding(12)
        .style(theme::surface_style);
    let backdrop = button(Space::new().width(Length::Fill).height(Length::Fill))
        .on_press(HubMessage::PaletteToggle)
        .style(button::text)
        .width(Length::Fill)
        .height(Length::Fill);
    iced::widget::stack![
        backdrop,
        container(card)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 70.0,
                ..iced::Padding::ZERO
            })
            .align_x(Alignment::Center)
    ]
    .into()
}

fn tint(c: &ThemeColors, ok: bool) -> Color {
    if ok { c.green } else { c.red }
}

fn bell_label(hub: &HubState, c: &ThemeColors) -> (String, Color) {
    match &hub.bell {
        Some(b) => {
            let color = match b.status {
                NotifyStatus::Off => c.text_secondary,
                NotifyStatus::On { .. } => c.green,
                NotifyStatus::Snoozed { .. } => c.yellow,
                NotifyStatus::Error { .. } | NotifyStatus::NoPoller => c.red,
            };
            let badge = if b.unread > 0 {
                format!(" {}", b.unread)
            } else {
                String::new()
            };
            (
                format!(
                    "{}{badge} {}",
                    b.status.glyph(true),
                    b.status.describe(Utc::now())
                ),
                color,
            )
        }
        None => ("[ ] notifications".into(), c.text_secondary),
    }
}

fn top_bar<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let tab = |label: &'static str, view: HubView| {
        let on = hub.view == view;
        button(
            text(label)
                .size(13)
                .color(if on { c.accent } else { c.text_secondary }),
        )
        .on_press(HubMessage::Show(view))
        .style(theme::ghost_button)
        .padding([4, 10])
    };
    let (bell, bell_color) = bell_label(hub, c);
    container(
        row![
            button(text("← Back").size(13))
                .on_press(HubMessage::Close)
                .style(theme::toolbar_button)
                .padding([4, 10]),
            text("GitKraft Hub").size(15).color(c.accent),
            tab("Dashboard", HubView::Dashboard),
            tab("Pulse", HubView::Pulse),
            tab("SSH keys", HubView::Keys),
            Space::new().width(Length::Fill),
            button(text("Commands ⌘P").size(12).color(c.text_secondary))
                .on_press(HubMessage::PaletteToggle)
                .style(theme::ghost_button)
                .padding([4, 10]),
            button(text(bell).size(12).color(bell_color))
                .on_press(HubMessage::ToggleBell)
                .style(theme::ghost_button)
                .padding([4, 10]),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([6, 12])
    .width(Length::Fill)
    .style(theme::header_style)
    .into()
}

fn tabs_bar<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let mut tabs = row![].spacing(4).align_y(Alignment::Center);
    for (vis, idx) in hub.visible_sections().into_iter().enumerate() {
        let section = &hub.dashboard.sections[idx];
        let slot = &hub.slots[idx];
        let suffix = match (&slot.data, slot.loading) {
            (_, true) => " …".to_string(),
            (Some(d), _) => format!(" {}", d.len()),
            (None, _) => String::new(),
        };
        let active = hub.active == vis;
        let (dragging_this, hovered) = match hub.drag {
            Some((from, over)) => (from == vis, over == vis && from != vis),
            None => (false, false),
        };
        let color = if dragging_this {
            c.yellow
        } else if active {
            c.text_primary
        } else {
            c.text_secondary
        };
        let (sel, hover, border) = (c.selection, c.surface_highlight, c.accent);
        let label = container(
            text(format!("{}{suffix}", section.name))
                .size(13)
                .color(color),
        )
        .padding([5, 12])
        .style(move |_| container::Style {
            background: if hovered {
                Some(hover.into())
            } else if active {
                Some(sel.into())
            } else {
                None
            },
            border: iced::Border {
                color: if hovered { border } else { Color::TRANSPARENT },
                width: if hovered { 1.0 } else { 0.0 },
                radius: 4.0.into(),
            },
            ..container::Style::default()
        });
        tabs = tabs.push(
            mouse_area(label)
                .on_press(HubMessage::DragStart(vis))
                .on_enter(HubMessage::DragEnter(vis))
                .on_release(HubMessage::DragDrop(vis)),
        );
    }
    let hint = match hub.drag {
        Some((from, over)) if from != over => {
            text("drop to move the section").size(12).color(c.yellow)
        }
        _ => text("drag a tab to reorder")
            .size(11)
            .color(c.text_secondary),
    };
    container(row![tabs, Space::new().width(Length::Fill), hint].align_y(Alignment::Center))
        .padding([4, 12])
        .width(Length::Fill)
        .into()
}

fn status_line<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let line: El<'a> = match &hub.notice {
        Some((t, ok)) => text(t.clone()).size(12).color(tint(c, *ok)).into(),
        None => text("Pull requests, issues, CI and notifications of every account")
            .size(12)
            .color(c.text_secondary)
            .into(),
    };
    container(line)
        .padding([4, 12])
        .width(Length::Fill)
        .style(theme::header_style)
        .into()
}

// ── Dashboard ────────────────────────────────────────────────────────────────

fn age(dt: Option<chrono::DateTime<Utc>>) -> String {
    dt.map(gitkraft_core::utils::time::relative_time)
        .unwrap_or_default()
}

fn state_color(c: &ThemeColors, s: ItemState) -> Color {
    match s {
        ItemState::Open => c.green,
        ItemState::Merged => c.accent,
        ItemState::Closed => c.red,
    }
}

fn run_color(c: &ThemeColors, s: Status) -> Color {
    match s {
        Status::Success => c.green,
        Status::Failed => c.red,
        Status::Running => c.yellow,
        Status::Cancelled | Status::Skipped => c.text_secondary,
        Status::Queued | Status::Waiting | Status::Manual => c.accent,
    }
}

/// `(mark, colour, title, meta)` of every row of a section.
fn rows_of(data: &SectionData, c: &ThemeColors) -> Vec<(&'static str, Color, String, String)> {
    match data {
        SectionData::Prs(r) => r
            .items
            .iter()
            .map(|p| {
                let (mark, color) = match (p.state, p.checks) {
                    (ItemState::Open, Some(ChecksState::Failure)) => ("✘", c.red),
                    (ItemState::Open, Some(ChecksState::Pending)) => ("●", c.yellow),
                    (ItemState::Open, _) if p.draft => ("◌", c.text_secondary),
                    (s, _) => ("●", state_color(c, s)),
                };
                (
                    mark,
                    color,
                    format!("#{} {}", p.number, p.title),
                    format!(
                        "{} · {} · {}",
                        p.repo.full_name(),
                        p.author.login,
                        age(p.updated_at)
                    ),
                )
            })
            .collect(),
        SectionData::Issues(r) => r
            .items
            .iter()
            .map(|i| {
                (
                    "●",
                    state_color(c, i.state),
                    format!("#{} {}", i.number, i.title),
                    format!(
                        "{} · {} · {}",
                        i.repo.full_name(),
                        i.author.login,
                        age(i.updated_at)
                    ),
                )
            })
            .collect(),
        SectionData::Runs(r) => r
            .items
            .iter()
            .map(|x| {
                (
                    x.status.glyph(false),
                    run_color(c, x.status),
                    format!("{} #{} · {} · {}", x.name, x.number, x.branch, x.title),
                    format!("{} · {}", x.actor, age(x.created_at)),
                )
            })
            .collect(),
        SectionData::Inbox(r) => r
            .items
            .iter()
            .map(|n| {
                let why = match n.reason {
                    NotificationReason::ReviewRequested => "review requested",
                    NotificationReason::Mention => "mention",
                    NotificationReason::Assigned => "assigned",
                    NotificationReason::Comment => "comment",
                    NotificationReason::CiActivity => "CI",
                    NotificationReason::Subscribed | NotificationReason::Other => "watching",
                };
                (
                    if n.unread { "●" } else { "○" },
                    if n.unread { c.accent } else { c.text_secondary },
                    n.title.clone(),
                    format!(
                        "{why} · {}",
                        n.repo.as_ref().map(|r| r.full_name()).unwrap_or_default()
                    ),
                )
            })
            .collect(),
    }
}

fn dashboard<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    if hub.log.is_some() {
        return log_view(hub, c);
    }
    let list: El<'a> = match (hub.active_index().map(|i| &hub.slots[i]), hub.active_data()) {
        (_, Some(data)) if !data.is_empty() => {
            let mut col = column![].spacing(2);
            for (i, (mark, color, title, meta)) in rows_of(data, c).into_iter().enumerate() {
                let selected = hub.row == Some(i);
                let sel = c.selection;
                let line = container(
                    row![
                        text(mark).size(13).color(color),
                        column![text(title).size(13).color(c.text_primary), text(meta).size(11).color(c.text_secondary)].spacing(1),
                    ]
                    .spacing(8),
                )
                .padding([5, 8])
                .width(Length::Fill)
                .style(move |_| container::Style { background: selected.then(|| sel.into()), ..container::Style::default() });
                col = col.push(mouse_area(line).on_press(HubMessage::SelectRow(i)));
            }
            scrollable(col).height(Length::Fill).into()
        }
        (_, Some(data)) => {
            let note = if data.errors().is_empty() { "Nothing here.".to_string() } else { data.errors().join("\n") };
            text(note).size(13).color(c.text_secondary).into()
        }
        (Some(slot), None) if slot.loading => text("Loading…").size(13).color(c.text_secondary).into(),
        _ if !hub.ready => text("Connecting accounts…").size(13).color(c.text_secondary).into(),
        _ if hub.accounts == 0 => text("No accounts found.\n\nSet GITHUB_TOKEN, GITLAB_TOKEN or CODEBERG_TOKEN (or GITKRAFT_TOKEN_<HOST>),\nor log in with `gh auth login`.\nSelf-hosted: add [[account]] to accounts.toml.").size(13).color(c.text_secondary).into(),
        _ => text("Nothing loaded.").size(13).color(c.text_secondary).into(),
    };
    let refresh = button(text("Refresh").size(12))
        .on_press(HubMessage::Refresh)
        .style(theme::toolbar_button)
        .padding([3, 10]);
    let left = column![container(refresh).padding([6, 8]), list]
        .width(Length::FillPortion(4))
        .height(Length::Fill);
    let right = container(detail(hub, c))
        .padding(12)
        .width(Length::FillPortion(6))
        .height(Length::Fill)
        .style(theme::surface_style);
    row![left, right].height(Length::Fill).into()
}

fn detail<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let Some(row_i) = hub.row else {
        return text("Select an item.")
            .size(13)
            .color(c.text_secondary)
            .into();
    };
    if hub.detail.loading {
        return text("Loading…").size(13).color(c.text_secondary).into();
    }
    if let Some(e) = &hub.detail.error {
        return text(e.clone()).size(13).color(c.red).into();
    }
    match hub.active_data() {
        Some(SectionData::Prs(r)) => r
            .items
            .get(row_i)
            .map_or_else(|| Space::new().into(), |p| pr_detail(hub, p, c)),
        Some(SectionData::Issues(r)) => r
            .items
            .get(row_i)
            .map_or_else(|| Space::new().into(), |i| issue_detail(hub, i, c)),
        Some(SectionData::Runs(r)) => r
            .items
            .get(row_i)
            .map_or_else(|| Space::new().into(), |x| run_detail(hub, x, c)),
        Some(SectionData::Inbox(r)) => r.items.get(row_i).map_or_else(
            || Space::new().into(),
            |n| {
                column![
                    text(n.title.clone()).size(15).color(c.text_primary),
                    text(n.repo.as_ref().map(|r| r.key()).unwrap_or_default())
                        .size(12)
                        .color(c.text_secondary),
                    text(n.url.clone()).size(12).color(c.accent)
                ]
                .spacing(6)
                .into()
            },
        ),
        None => Space::new().into(),
    }
}

fn labels_text(labels: &[gitkraft_platforms::model::Label]) -> String {
    labels
        .iter()
        .map(|l| l.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

fn action_row<'a>(c: &ThemeColors, buttons: Vec<(&'static str, HubMessage)>) -> El<'a> {
    let mut r = row![].spacing(6);
    for (label, msg) in buttons {
        r = r.push(
            button(text(label).size(12).color(c.text_primary))
                .on_press(msg)
                .style(theme::toolbar_button)
                .padding([4, 10]),
        );
    }
    r.wrap().into()
}

fn note_input<'a>(hub: &'a HubState, placeholder: &'static str) -> El<'a> {
    text_input(placeholder, &hub.note)
        .on_input(HubMessage::NoteInput)
        .padding(6)
        .size(13)
        .into()
}

fn pr_detail<'a>(
    hub: &'a HubState,
    listed: &'a gitkraft_platforms::model::PullRequest,
    c: &ThemeColors,
) -> El<'a> {
    let pr = hub.detail.pr.as_ref().map_or(listed, |d| &d.pr);
    let word = match (pr.state, pr.draft) {
        (ItemState::Open, true) => "draft",
        (ItemState::Open, false) => "open",
        (ItemState::Merged, _) => "merged",
        (ItemState::Closed, _) => "closed",
    };
    let checks: El<'a> = match pr.checks {
        Some(ChecksState::Success) => text("✔ checks passed").size(12).color(c.green).into(),
        Some(ChecksState::Failure) => text("✘ checks failed").size(12).color(c.red).into(),
        Some(ChecksState::Pending) => text("● checks running").size(12).color(c.yellow).into(),
        None => Space::new().into(),
    };
    let conflict: El<'a> = if pr.mergeable == Some(false) {
        text("⚠ conflicts").size(12).color(c.red).into()
    } else {
        Space::new().into()
    };
    let header = column![
        text(pr.title.clone()).size(17).color(c.text_primary),
        row![
            text(word).size(12).color(state_color(c, pr.state)),
            text(format!(
                "{}#{} by {} · {} → {}",
                pr.repo.full_name(),
                pr.number,
                pr.author.login,
                pr.source_branch,
                pr.target_branch
            ))
            .size(12)
            .color(c.text_secondary),
            checks,
            conflict,
        ]
        .spacing(10),
        text(labels_text(&pr.labels)).size(11).color(c.accent),
    ]
    .spacing(4);
    let tab_btn = |label: String, tab: PrTab| {
        let on = hub.tab == tab;
        button(
            text(label)
                .size(12)
                .color(if on { c.accent } else { c.text_secondary }),
        )
        .on_press(HubMessage::Tab(tab))
        .style(theme::ghost_button)
        .padding([3, 10])
    };
    let comments = hub
        .detail
        .pr
        .as_ref()
        .map_or(pr.comments as usize, |d| d.comments.len());
    let tabs = row![
        tab_btn("Overview".into(), PrTab::Overview),
        tab_btn("Diff".into(), PrTab::Diff),
        tab_btn(format!("Comments {comments}"), PrTab::Comments)
    ]
    .spacing(4);
    let open = pr.state == ItemState::Open;
    let mut buttons = vec![
        ("Approve", HubMessage::Approve),
        ("Request changes", HubMessage::RequestChanges),
        ("Comment", HubMessage::PostComment),
        ("Check out", HubMessage::Worktree),
        ("Labels", HubMessage::EditLabels),
    ];
    if hub.tab == PrTab::Diff {
        buttons.push(("Comment on line", HubMessage::InlineComment));
    }
    if open {
        buttons.push((
            "Merge",
            HubMessage::Merge(gitkraft_platforms::model::MergeMethod::Merge),
        ));
        buttons.push((
            "Squash",
            HubMessage::Merge(gitkraft_platforms::model::MergeMethod::Squash),
        ));
        buttons.push((
            "When green",
            HubMessage::MergeWith(MergeTiming::WhenGreen, false),
        ));
        buttons.push(("Queue", HubMessage::MergeWith(MergeTiming::Queue, false)));
        buttons.push((
            if hub.force_armed {
                "Really force merge?"
            } else {
                "Force merge"
            },
            HubMessage::MergeWith(MergeTiming::Now, true),
        ));
        buttons.push(("Update branch", HubMessage::UpdateBranch));
        buttons.push(("Cancel auto-merge", HubMessage::CancelAutoMerge));
        buttons.push((
            match hub.merge_method {
                MergeMethod::Merge => "Method: merge",
                MergeMethod::Squash => "Method: squash",
                MergeMethod::Rebase => "Method: rebase",
            },
            HubMessage::CycleMethod,
        ));
    }
    buttons.push((
        if open { "Close" } else { "Reopen" },
        HubMessage::ToggleOpen,
    ));
    let content: El<'a> = match (&hub.detail.pr, hub.tab) {
        (None, _) => text("Loading details…")
            .size(13)
            .color(c.text_secondary)
            .into(),
        (Some(d), PrTab::Overview) => {
            let mut col = column![].spacing(6);
            for r in &d.reviews {
                let (g, color, w) = match r.verdict {
                    Some(ReviewVerdict::Approve) => ("✔", c.green, "approved"),
                    Some(ReviewVerdict::RequestChanges) => ("✘", c.red, "requested changes"),
                    _ => ("●", c.text_secondary, "commented"),
                };
                col = col.push(
                    text(format!("{g} {} {w}", r.author.login))
                        .size(13)
                        .color(color),
                );
            }
            let body = if pr.body.trim().is_empty() {
                "No description.".to_string()
            } else {
                pr.body.clone()
            };
            col = col.push(text(body).size(13).color(c.text_primary));
            let images = gitkraft_hub::markdown::image_urls(&pr.body);
            if !images.is_empty() {
                col = col.push(
                    text(format!(
                        "🖼 {} image(s): {}",
                        images.len(),
                        images.join("  ")
                    ))
                    .size(11)
                    .color(c.accent),
                );
            }
            scrollable(col).height(Length::Fill).into()
        }
        (Some(d), PrTab::Diff) => {
            let mut col = column![];
            for (i, l) in d.diff.lines().enumerate().take(4000) {
                let color = if l.starts_with('+') && !l.starts_with("+++") {
                    c.green
                } else if l.starts_with('-') && !l.starts_with("---") {
                    c.red
                } else if l.starts_with("@@") {
                    c.accent
                } else {
                    c.diff_context
                };
                let chosen = hub.detail.diff_line == Some(i);
                let accent = c.accent;
                let line = container(
                    text(l.to_string())
                        .font(iced::Font::MONOSPACE)
                        .size(12)
                        .color(color),
                )
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: chosen
                        .then_some(iced::Background::Color(Color { a: 0.22, ..accent })),
                    ..container::Style::default()
                });
                col = col.push(mouse_area(line).on_press(HubMessage::DiffLine(i)));
            }
            scrollable(col).height(Length::Fill).into()
        }
        (Some(d), PrTab::Comments) => {
            let mut col = column![].spacing(8);
            for cm in &d.comments {
                col = col.push(
                    column![
                        text(format!("{} · {}", cm.author.login, age(cm.created_at)))
                            .size(12)
                            .color(c.accent),
                        text(cm.body.clone()).size(13).color(c.text_primary)
                    ]
                    .spacing(2),
                );
            }
            if d.comments.is_empty() {
                col = col.push(text("No comments yet.").size(13).color(c.text_secondary));
            }
            scrollable(col).height(Length::Fill).into()
        }
    };
    column![
        header,
        tabs,
        content,
        note_input(hub, "Comment, or the note for a review"),
        action_row(c, buttons)
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn issue_detail<'a>(
    hub: &'a HubState,
    it: &'a gitkraft_platforms::model::Issue,
    c: &ThemeColors,
) -> El<'a> {
    let mut col = column![
        text(it.title.clone()).size(17).color(c.text_primary),
        text(format!(
            "{}#{} by {} · {}",
            it.repo.full_name(),
            it.number,
            it.author.login,
            if it.state == ItemState::Open {
                "open"
            } else {
                "closed"
            }
        ))
        .size(12)
        .color(c.text_secondary),
        text(labels_text(&it.labels)).size(11).color(c.accent),
        text(if it.body.trim().is_empty() {
            "No description.".to_string()
        } else {
            it.body.clone()
        })
        .size(13)
        .color(c.text_primary),
    ]
    .spacing(6);
    for cm in &hub.detail.comments {
        col = col.push(
            column![
                text(format!("{} · {}", cm.author.login, age(cm.created_at)))
                    .size(12)
                    .color(c.accent),
                text(cm.body.clone()).size(13).color(c.text_primary)
            ]
            .spacing(2),
        );
    }
    let open = it.state == ItemState::Open;
    column![
        scrollable(col).height(Length::Fill),
        note_input(hub, "Write a comment"),
        action_row(
            c,
            vec![
                ("Comment", HubMessage::PostComment),
                (
                    if open { "Close" } else { "Reopen" },
                    HubMessage::ToggleOpen
                )
            ]
        )
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn run_detail<'a>(hub: &'a HubState, run: &'a gitkraft_ci::Run, c: &ThemeColors) -> El<'a> {
    let dur = run
        .duration()
        .map(|d| format_seconds(d.num_seconds() as f64))
        .unwrap_or_default();
    let header = column![
        row![
            text(run.status.glyph(false))
                .size(16)
                .color(run_color(c, run.status)),
            text(format!("{} #{}", run.name, run.number))
                .size(17)
                .color(c.text_primary),
            text(dur).size(12).color(c.text_secondary)
        ]
        .spacing(8),
        text(format!(
            "{} · {} · {} · {}",
            run.repo.full_name(),
            run.branch,
            run.event,
            run.actor
        ))
        .size(12)
        .color(c.text_secondary),
        text(run.title.clone()).size(12).color(c.text_secondary),
    ]
    .spacing(3);
    let mut jobs = column![].spacing(2);
    if let Some(d) = &hub.detail.run {
        for (i, j) in d.jobs.iter().enumerate() {
            let selected = hub.detail.job == i;
            let sel = c.selection;
            let stage = j
                .stage
                .as_deref()
                .map(|s| format!("{s} · "))
                .unwrap_or_default();
            let line = container(
                row![
                    text(j.status.glyph(false))
                        .size(13)
                        .color(run_color(c, j.status)),
                    text(format!("{stage}{}", j.name))
                        .size(13)
                        .color(c.text_primary),
                    text(job_seconds(j).map(format_seconds).unwrap_or_default())
                        .size(11)
                        .color(c.text_secondary)
                ]
                .spacing(8),
            )
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: selected.then(|| sel.into()),
                ..container::Style::default()
            });
            jobs = jobs.push(mouse_area(line).on_press(HubMessage::SelectJob(i)));
        }
    } else {
        jobs = jobs.push(text("Loading jobs…").size(13).color(c.text_secondary));
    }
    let steps: El<'a> = match hub
        .detail
        .run
        .as_ref()
        .and_then(|d| d.jobs.get(hub.detail.job))
    {
        Some(j) if !j.steps.is_empty() => {
            let mut col = column![].spacing(1);
            for s in &j.steps {
                col = col.push(
                    text(format!("  {} {}", s.status.glyph(false), s.name))
                        .size(12)
                        .color(run_color(c, s.status)),
                );
            }
            col.into()
        }
        _ => Space::new().into(),
    };
    let timeline: El<'a> = match &hub.detail.run {
        Some(d) => {
            let bars: Vec<_> = gitkraft_ci::timing::gantt(d)
                .into_iter()
                .filter(|b| b.depth == 0)
                .collect();
            let end = bars
                .iter()
                .map(|b| b.start + b.length)
                .fold(0.0, f64::max)
                .max(1.0);
            let mut col = column![].spacing(2);
            for b in bars {
                let left = ((b.start / end) * 100.0).round().max(0.0) as u16;
                let len = ((b.length / end) * 100.0).round().max(1.0) as u16;
                let rest = 100u16.saturating_sub(left + len);
                let color = if b.critical {
                    c.accent
                } else {
                    c.text_secondary
                };
                col = col.push(
                    row![
                        container(text(b.label.clone()).size(11).color(c.text_secondary))
                            .width(Length::Fixed(140.0)),
                        Space::new().width(Length::FillPortion(left.max(1))),
                        container(Space::new())
                            .width(Length::FillPortion(len))
                            .height(Length::Fixed(8.0))
                            .style(move |_| container::Style {
                                background: Some(color.into()),
                                border: iced::Border {
                                    radius: 2.0.into(),
                                    ..iced::Border::default()
                                },
                                ..container::Style::default()
                            }),
                        Space::new().width(Length::FillPortion(rest.max(1))),
                    ]
                    .align_y(Alignment::Center),
                );
            }
            col.into()
        }
        None => Space::new().into(),
    };
    let active = run.status.is_active();
    let mut buttons = vec![
        ("Open log", HubMessage::OpenLog),
        (
            "Re-run failed",
            HubMessage::Rerun(gitkraft_ci::RerunScope::Failed),
        ),
        (
            "Re-run all",
            HubMessage::Rerun(gitkraft_ci::RerunScope::All),
        ),
    ];
    if active {
        buttons.push(("Cancel", HubMessage::CancelRun));
    }
    column![
        header,
        scrollable(
            column![
                jobs,
                steps,
                text("Timeline").size(12).color(c.text_secondary),
                timeline
            ]
            .spacing(8)
        )
        .height(Length::Fill),
        note_input(
            hub,
            "Run workflow: [WORKFLOW] REF key=value… (blank: the same again)"
        ),
        action_row(c, buttons)
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

// ── Log ──────────────────────────────────────────────────────────────────────

fn ansi(c: AnsiColor) -> Color {
    const BASE: [(u8, u8, u8); 16] = [
        (28, 28, 28),
        (205, 49, 49),
        (13, 188, 121),
        (229, 229, 16),
        (36, 114, 200),
        (188, 63, 188),
        (17, 168, 205),
        (204, 204, 204),
        (102, 102, 102),
        (241, 76, 76),
        (35, 209, 139),
        (245, 245, 67),
        (59, 142, 234),
        (214, 112, 214),
        (41, 184, 219),
        (255, 255, 255),
    ];
    match c {
        AnsiColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
        AnsiColor::Indexed(n) if n < 16 => {
            let (r, g, b) = BASE[usize::from(n)];
            Color::from_rgb8(r, g, b)
        }
        AnsiColor::Indexed(n) if n >= 232 => {
            let v = (8 + (u16::from(n) - 232) * 10) as u8;
            Color::from_rgb8(v, v, v)
        }
        AnsiColor::Indexed(n) => {
            let n = n - 16;
            let step = |x: u8| {
                if x == 0 {
                    0
                } else {
                    (55 + 40 * u16::from(x)) as u8
                }
            };
            Color::from_rgb8(step(n / 36), step((n % 36) / 6), step(n % 6))
        }
    }
}

fn log_view<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let Some(log) = &hub.log else {
        return Space::new().into();
    };
    let mut col = column![];
    for (i, l) in log.lines.iter().enumerate() {
        let base = match l.kind {
            LineKind::Error => c.red,
            LineKind::Warning => c.yellow,
            LineKind::Command => c.accent,
            LineKind::GroupStart(_) => c.text_primary,
            _ => c.text_secondary,
        };
        let mut spans: Vec<iced::widget::text::Span<'a, HubMessage>> =
            vec![iced::widget::span(format!("{:>5}  ", l.number)).color(c.text_secondary)];
        let indent = "  ".repeat(usize::from(l.depth));
        spans.push(iced::widget::span(indent));
        for s in &l.spans {
            spans.push(iced::widget::span(s.text.clone()).color(s.style.fg.map_or(base, ansi)));
        }
        let line = iced::widget::rich_text(spans)
            .font(iced::Font::MONOSPACE)
            .size(12);
        let focus = log.focus == Some(i);
        let sel = c.selection;
        col = col.push(
            container(line)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: focus.then(|| sel.into()),
                    ..container::Style::default()
                }),
        );
    }
    let mut problems = column![].spacing(2);
    for a in &log.annotations {
        let at = match (&a.file, a.line) {
            (Some(f), Some(l)) => format!("{f}:{l}  "),
            _ => String::new(),
        };
        problems = problems.push(
            button(text(format!("✘ {at}{}", a.message)).size(12).color(c.red))
                .on_press(HubMessage::JumpLog(a.log_index))
                .style(theme::ghost_button)
                .padding([2, 6]),
        );
    }
    let head = row![
        button(text("← Run").size(12))
            .on_press(HubMessage::CloseLog)
            .style(theme::toolbar_button)
            .padding([3, 10]),
        text(log.title.clone()).size(14).color(c.accent),
        text(format!(
            "{} lines · {} problems",
            log.lines.len(),
            log.annotations.len()
        ))
        .size(12)
        .color(c.text_secondary),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let mut screen = column![container(head).padding([6, 12])];
    if !log.annotations.is_empty() {
        screen = screen
            .push(container(scrollable(problems).height(Length::Fixed(110.0))).padding([0, 12]));
    }
    screen
        .push(scrollable(col).height(Length::Fill).width(Length::Fill))
        .height(Length::Fill)
        .into()
}

// ── SSH keys ─────────────────────────────────────────────────────────────────

fn keys<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let mut col = column![].spacing(10).padding(14);
    col = col.push(
        text("SSH keys · git over ssh without passwords")
            .size(14)
            .color(c.text_primary),
    );
    let Some(data) = &hub.keys.data else {
        let note = if hub.keys.loading {
            "Loading keys…"
        } else {
            "No keys loaded yet."
        };
        return col.push(text(note).size(13).color(c.text_secondary)).into();
    };
    let mut list = column![].spacing(2);
    for (i, r) in data.rows.iter().enumerate() {
        let selected = hub.keys.row == i;
        let sel = c.selection;
        let line = container(
            row![
                text(if r.mine { "●" } else { " " }).size(13).color(c.green),
                text(r.host.clone())
                    .size(13)
                    .color(c.accent)
                    .width(Length::Fixed(140.0)),
                text(r.key.title.clone())
                    .size(13)
                    .color(c.text_primary)
                    .width(Length::Fixed(220.0)),
                text(r.key.fingerprint.clone().unwrap_or_else(|| "-".into()))
                    .size(12)
                    .color(c.text_secondary),
                text(
                    r.key
                        .created_at
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                )
                .size(12)
                .color(c.text_secondary),
            ]
            .spacing(10),
        )
        .padding([5, 8])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: selected.then(|| sel.into()),
            ..container::Style::default()
        });
        list = list.push(mouse_area(line).on_press(HubMessage::SelectKey(i)));
    }
    if data.rows.is_empty() {
        list = list.push(
            text("No SSH keys on any account. Set one up for this machine below.")
                .size(13)
                .color(c.text_secondary),
        );
    }
    col = col.push(scrollable(list).height(Length::Fill));
    for e in &data.errors {
        col = col.push(text(format!("✘ {e}")).size(12).color(c.red));
    }
    col = col.push(
        text(format!(
            "● = a key this machine holds · accounts: {}",
            data.hosts.join(", ")
        ))
        .size(11)
        .color(c.text_secondary),
    );
    let host = hub.keys_host();
    let mut actions = row![].spacing(6).align_y(Alignment::Center);
    if hub.keys.confirm_revoke {
        let what = hub
            .selected_key()
            .map(|r| {
                format!(
                    "Remove “{}” from {}?{}",
                    r.key.title,
                    r.host,
                    if r.mine {
                        " It is this machine's key."
                    } else {
                        ""
                    }
                )
            })
            .unwrap_or_default();
        actions = actions
            .push(text(what).size(12).color(c.yellow))
            .push(
                button(text("Yes, remove").size(12).color(c.red))
                    .on_press(HubMessage::ConfirmRevoke)
                    .style(theme::toolbar_button)
                    .padding([4, 10]),
            )
            .push(
                button(text("No").size(12))
                    .on_press(HubMessage::CancelRevoke)
                    .style(theme::toolbar_button)
                    .padding([4, 10]),
            );
    } else {
        let mut set = button(
            text(match &host {
                Some(h) => format!("Set up {h} on this machine"),
                None => "Set up".into(),
            })
            .size(12)
            .color(c.text_primary),
        )
        .style(theme::toolbar_button)
        .padding([4, 10]);
        let mut test = button(text("Test connection").size(12).color(c.text_primary))
            .style(theme::toolbar_button)
            .padding([4, 10]);
        if let Some(h) = &host {
            set = set.on_press(HubMessage::SetupKeys(h.clone()));
            test = test.on_press(HubMessage::TestKeys(h.clone()));
        }
        let mut rm = button(text("Remove key").size(12).color(c.red))
            .style(theme::toolbar_button)
            .padding([4, 10]);
        if hub.selected_key().is_some() {
            rm = rm.on_press(HubMessage::AskRevoke);
        }
        actions = actions.push(set).push(test).push(rm);
    }
    col.push(actions).height(Length::Fill).into()
}

// ── Pulse ────────────────────────────────────────────────────────────────────

fn pulse<'a>(hub: &'a HubState, c: &ThemeColors) -> El<'a> {
    let mut col = column![].spacing(12).padding(14);
    let today = Utc::now().date_naive();
    match &hub.pulse.contributions {
        Some(contrib) => {
            let total = contrib
                .items
                .iter()
                .filter(|d| (today - d.date).num_days() < 371)
                .map(|d| d.count)
                .sum::<u32>();
            col = col.push(
                text(format!(
                    "Contributions · {total} in the last year, all accounts"
                ))
                .size(14)
                .color(c.text_primary),
            );
            let palette = [
                c.surface_highlight,
                Color {
                    a: 1.0,
                    ..mix(c.surface_highlight, c.green, 0.35)
                },
                mix(c.surface_highlight, c.green, 0.6),
                mix(c.surface_highlight, c.green, 0.8),
                c.green,
            ];
            let size = Heatmap::size();
            col = col.push(
                scrollable(
                    canvas(Heatmap {
                        days: contrib.items.clone(),
                        today,
                        palette,
                        label: c.text_secondary,
                    })
                    .width(Length::Fixed(size.width))
                    .height(Length::Fixed(size.height)),
                )
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new(),
                )),
            );
        }
        None => {
            col = col.push(
                text("Loading contributions…")
                    .size(13)
                    .color(c.text_secondary),
            )
        }
    }
    let repo_name = hub
        .pulse
        .repo
        .as_ref()
        .map(|r| r.key())
        .unwrap_or_else(|| "no repository".into());
    let mut picker = row![].spacing(6);
    for (i, w) in hub.watched.iter().enumerate() {
        let on = hub.pulse.repo.as_ref() == Some(w);
        picker = picker.push(
            button(text(w.full_name()).size(12).color(if on {
                c.accent
            } else {
                c.text_secondary
            }))
            .on_press(HubMessage::PulseRepo(i))
            .style(theme::ghost_button)
            .padding([3, 8]),
        );
    }
    let mut ranges = row![].spacing(6);
    for d in [7u32, 30, 90, 365] {
        let on = hub.pulse.days == d;
        ranges = ranges.push(
            button(text(format!("{d} days")).size(12).color(if on {
                c.accent
            } else {
                c.text_secondary
            }))
            .on_press(HubMessage::PulseDays(d))
            .style(theme::ghost_button)
            .padding([3, 8]),
        );
    }
    col = col
        .push(
            text(format!("Pulse · {repo_name}"))
                .size(14)
                .color(c.text_primary),
        )
        .push(
            scrollable(picker).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new(),
            )),
        )
        .push(ranges);
    match &hub.pulse.data {
        Some(p) => {
            let stat = |label: &'static str, n: usize, color: Color| {
                container(
                    column![
                        text(n.to_string()).size(22).color(color),
                        text(label).size(12).color(c.text_secondary)
                    ]
                    .spacing(2),
                )
                .padding(10)
                .width(Length::FillPortion(1))
                .style(theme::surface_style)
            };
            col = col
                .push(
                    row![
                        stat("PRs opened", p.prs_opened, c.green),
                        stat("PRs merged", p.prs_merged, c.accent),
                        stat("PRs closed", p.prs_closed, c.red),
                        stat("PRs open now", p.prs_open_now, c.yellow)
                    ]
                    .spacing(8),
                )
                .push(
                    row![
                        stat("Issues opened", p.issues_opened, c.green),
                        stat("Issues closed", p.issues_closed, c.accent),
                        stat("CI green", p.ci_green, c.green),
                        stat("CI red", p.ci_red, c.red)
                    ]
                    .spacing(8),
                )
                .push(text("PRs opened per day").size(12).color(c.text_secondary))
                .push(
                    canvas(Bars {
                        values: p.prs_per_day.clone(),
                        color: c.accent,
                    })
                    .width(Length::Fill)
                    .height(Length::Fixed(90.0)),
                );
            if !p.top_authors.is_empty() {
                col = col.push(
                    text(format!(
                        "Most active: {}",
                        p.top_authors
                            .iter()
                            .map(|(a, n)| format!("{a} ({n})"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .size(12)
                    .color(c.text_secondary),
                );
            }
        }
        None => col = col.push(text("Loading…").size(13).color(c.text_secondary)),
    }
    scrollable(col).height(Length::Fill).into()
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_hub::{PrDetail, SectionResult};
    use gitkraft_platforms::model::{PullRequest, User};
    use gitkraft_settings::dashboard::{
        Dashboard, Involvement, Section, SectionKind, SectionState,
    };
    use std::sync::Arc;

    fn section(name: &str, kind: SectionKind) -> Section {
        Section {
            name: name.into(),
            kind,
            repos: if kind == SectionKind::Ci {
                vec!["github.com/o/r".into()]
            } else {
                vec![]
            },
            involvement: Some(Involvement::Authored),
            state: SectionState::Open,
            label: None,
            branch: None,
            hidden: false,
        }
    }

    fn app() -> GitKraft {
        let mut app = GitKraft::new();
        let mut hub = HubState::detached(Dashboard {
            sections: vec![
                section("Mine", SectionKind::Prs),
                section("CI", SectionKind::Ci),
            ],
        });
        let pr = PullRequest {
            repo: gitkraft_platforms::RepoId::new("github.com", "o", "r"),
            number: 7,
            title: "Add pulse".into(),
            author: User {
                login: "a".into(),
                avatar_url: None,
            },
            body: "text ![x](https://i/1.png)".into(),
            ..PullRequest::default()
        };
        hub.slots[0].data = Some(Arc::new(SectionData::Prs(SectionResult {
            items: vec![pr.clone()],
            errors: vec![],
        })));
        hub.row = Some(0);
        hub.ready = true;
        hub.detail.pr = Some(Arc::new(PrDetail {
            pr,
            comments: vec![],
            reviews: vec![],
            diff: "+a\n-b\n@@ x".into(),
            errors: vec![],
        }));
        app.hub = Some(Box::new(hub));
        app
    }

    #[test]
    fn every_view_builds_without_panicking() {
        let mut app = app();
        let _ = view(&app);
        for tab in [PrTab::Overview, PrTab::Diff, PrTab::Comments] {
            app.hub.as_mut().expect("hub").tab = tab;
            let _ = view(&app);
        }
        let hub = app.hub.as_mut().expect("hub");
        hub.drag = Some((0, 1));
        let _ = view(&app);
        let hub = app.hub.as_mut().expect("hub");
        hub.drag = None;
        hub.view = HubView::Pulse;
        let _ = view(&app);
        let hub = app.hub.as_mut().expect("hub");
        hub.pulse.data = Some(gitkraft_hub::pulse::Pulse {
            days: 30,
            prs_per_day: vec![1.0; 30],
            ..Default::default()
        });
        hub.pulse.contributions = Some(Arc::new(SectionResult {
            items: vec![],
            errors: vec!["codeberg.org: down".into()],
        }));
        let _ = view(&app);
        let hub = app.hub.as_mut().expect("hub");
        hub.view = HubView::Dashboard;
        let _ = hub.update(HubMessage::LogLoaded(Ok((
            "job".into(),
            "a\n\u{1b}[31merror: x\u{1b}[0m\n --> f.rs:1:1".into(),
        ))));
        let _ = view(&app);
        for q in ["", "ref", "zzzz"] {
            app.hub.as_mut().expect("hub").palette = Some(q.into());
            let _ = view(&app);
        }
        // Without a hub the hub view is an empty placeholder, not a panic.
        let _ = view(&GitKraft::new());
    }

    #[test]
    fn the_keys_view_builds_loading_loaded_and_asking() {
        use gitkraft_hub::keys::{KeyRow, KeysData};
        use gitkraft_platforms::model::SshKey;
        let mut app = app();
        app.hub.as_mut().expect("hub").view = HubView::Keys;
        let _ = view(&app);
        app.hub.as_mut().expect("hub").keys.loading = true;
        let _ = view(&app);
        let key = SshKey {
            id: "1".into(),
            title: "laptop".into(),
            key: String::new(),
            fingerprint: Some("SHA256:x".into()),
            created_at: None,
        };
        let hub = app.hub.as_mut().expect("hub");
        hub.keys.data = Some(Arc::new(KeysData {
            hosts: vec!["github.com".into()],
            rows: vec![KeyRow {
                host: "github.com".into(),
                key,
                mine: true,
            }],
            errors: vec!["x: 403".into()],
        }));
        let _ = view(&app);
        app.hub.as_mut().expect("hub").keys.confirm_revoke = true;
        let _ = view(&app);
        app.hub.as_mut().expect("hub").keys.data = Some(Arc::new(KeysData::default()));
        let _ = view(&app);
    }

    #[test]
    fn rows_show_state_marks_and_run_glyphs() {
        let c = crate::theme::ThemeColors::from_theme(&iced::Theme::Dark);
        let pr = |state, draft| PullRequest {
            number: 1,
            title: "t".into(),
            state,
            draft,
            author: User {
                login: "a".into(),
                avatar_url: None,
            },
            ..PullRequest::default()
        };
        let data = SectionData::Prs(SectionResult {
            items: vec![pr(ItemState::Open, true), pr(ItemState::Merged, false)],
            errors: vec![],
        });
        let rows = rows_of(&data, &c);
        assert_eq!((rows[0].0, rows[1].0), ("◌", "●"));
        assert_eq!(rows[0].2, "#1 t");
        let runs = SectionData::Runs(SectionResult {
            items: vec![gitkraft_ci::Run {
                status: Status::Failed,
                name: "CI".into(),
                ..Default::default()
            }],
            errors: vec![],
        });
        assert_eq!(rows_of(&runs, &c)[0].0, "✘");
    }

    #[test]
    fn ansi_colours_map_to_distinct_iced_colours() {
        assert_ne!(ansi(AnsiColor::Indexed(1)), ansi(AnsiColor::Indexed(2)));
        assert_eq!(ansi(AnsiColor::Rgb(255, 0, 0)), Color::from_rgb8(255, 0, 0));
        assert_eq!(ansi(AnsiColor::Indexed(16)), Color::from_rgb8(0, 0, 0));
        assert_eq!(ansi(AnsiColor::Indexed(232)), Color::from_rgb8(8, 8, 8));
        assert_eq!(ansi(AnsiColor::Indexed(196)), Color::from_rgb8(255, 0, 0));
    }

    #[test]
    fn mixing_colours_interpolates() {
        let m = mix(Color::BLACK, Color::WHITE, 0.5);
        assert!((m.r - 0.5).abs() < 1e-6 && m.a == 1.0);
    }
}
