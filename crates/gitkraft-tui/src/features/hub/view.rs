//! Drawing the hub screen. Rendering also records the clickable regions in
//! [`HubState::hits`], so the mouse handler never needs the layout itself.

use chrono::{Datelike, Duration as ChronoDuration, Utc};
use gitkraft_ci::log::{AnsiColor, LineKind};
use gitkraft_ci::model::Status;
use gitkraft_ci::timing::{self, format_seconds};
use gitkraft_hub::SectionData;
use gitkraft_platforms::model::{ChecksState, ItemState, NotificationReason, ReviewVerdict};
use gitkraft_settings::dashboard::SectionKind;
use gitkraft_settings::notify::NotifyStatus;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tui_chart::{BarChart, BarGroup, CalendarHeatmap, Gantt, Task, days_from_civil};

use super::images::Entry;
use super::list;
use super::markdown;
use super::state::{DetailTab, Focus, Hit, HubState, PromptPurpose, Target, View};
use crate::app::App;
use crate::features::theme::palette::UiTheme;
use ratatui_image::{Resize, StatefulImage};

/// Draws the hub.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let theme = app.theme();
    let Some(hub) = app.hub.as_mut() else { return };
    let mut hits: Vec<Hit> = Vec::new();
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
    render_title(hub, frame, rows[0], &theme, &mut hits);
    match hub.view {
        View::Dashboard => {
            render_section_tabs(hub, frame, rows[1], &theme, &mut hits);
            if hub.images.open {
                render_images(hub, frame, rows[2], &theme);
            } else if hub.log.is_some() {
                render_log(hub, frame, rows[2], &theme, &mut hits);
            } else {
                render_dashboard(hub, frame, rows[2], &theme, &mut hits);
            }
        }
        View::Pulse => {
            render_pulse(hub, frame, rows[2], &theme);
        }
        View::Keys => render_keys(hub, frame, rows[2], &theme, &mut hits),
    }
    render_status(hub, frame, rows[3], &theme);
    render_palette(hub, frame, area, &theme);
    hub.set_hits(hits);
}

fn render_palette(hub: &HubState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let Some(p) = &hub.palette else { return };
    if area.width < 20 || area.height < 5 {
        return;
    }
    let entries = super::palette::entries_for(hub);
    let shown = super::palette::filter_entries(&entries, &p.query);
    let w = area.width.saturating_sub(8).clamp(20, 72);
    let h = (shown.len().min(12) as u16 + 3)
        .min(area.height.saturating_sub(2))
        .max(4);
    let rect = Rect::new(
        area.x + (area.width - w) / 2,
        area.y + 2.min(area.height.saturating_sub(h)),
        w,
        h,
    );
    frame.render_widget(ratatui::widgets::Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Command palette · Enter run · Esc close ");
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    if inner.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("› ", Style::default().fg(theme.accent)),
            Span::styled(
                format!("{}█", p.query),
                Style::default().fg(theme.text_primary),
            ),
        ])),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let visible = inner.height.saturating_sub(1) as usize;
    let first = p.row.saturating_sub(visible.saturating_sub(1));
    let lines: Vec<Line> = shown
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .map(|(n, i)| {
            let e = &entries[*i];
            let style = if n == p.row {
                Style::default().fg(theme.bg).bg(theme.accent)
            } else {
                Style::default().fg(theme.text_primary)
            };
            Line::from(vec![
                Span::styled(
                    format!(
                        " {:<width$}",
                        e.label,
                        width = inner.width.saturating_sub(6) as usize
                    ),
                    style,
                ),
                Span::styled(
                    format!(" {} ", e.key),
                    if n == p.row { style } else { muted(theme) },
                ),
            ])
        })
        .collect();
    let body = if lines.is_empty() {
        vec![Line::from(Span::styled(" nothing matches", muted(theme)))]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(body),
        Rect::new(inner.x, inner.y + 1, inner.width, inner.height - 1),
    );
}

fn muted(theme: &UiTheme) -> Style {
    Style::default().fg(theme.text_muted)
}

fn state_color(theme: &UiTheme, s: ItemState) -> Color {
    match s {
        ItemState::Open => theme.success,
        ItemState::Merged => theme.accent,
        ItemState::Closed => theme.error,
    }
}

fn run_color(theme: &UiTheme, s: Status) -> Color {
    match s {
        Status::Success => theme.success,
        Status::Failed => theme.error,
        Status::Running => theme.warning,
        Status::Cancelled | Status::Skipped => theme.text_muted,
        Status::Queued | Status::Waiting | Status::Manual => theme.accent,
    }
}

fn clip(s: &str, max: usize) -> String {
    gitkraft_core::truncate_str(s, max)
}

fn age(dt: Option<chrono::DateTime<Utc>>) -> String {
    dt.map(gitkraft_core::utils::time::relative_time)
        .unwrap_or_default()
}

// ── Title and tabs ───────────────────────────────────────────────────────────

fn bell_text(hub: &HubState, ascii: bool) -> (String, Color) {
    let _ = ascii;
    match hub.bell() {
        Some((_, st, unread)) => {
            let color = match st {
                NotifyStatus::Off => Color::DarkGray,
                NotifyStatus::On { .. } => Color::Green,
                NotifyStatus::Snoozed { .. } => Color::Yellow,
                NotifyStatus::Error { .. } | NotifyStatus::NoPoller => Color::Red,
            };
            let badge = if unread > 0 {
                format!(" {unread}")
            } else {
                String::new()
            };
            (
                format!("{}{badge} {}", st.glyph(true), short_status(&st)),
                color,
            )
        }
        None => ("[ ] no repo".into(), Color::DarkGray),
    }
}

fn short_status(st: &NotifyStatus) -> &'static str {
    match st {
        NotifyStatus::Off => "off",
        NotifyStatus::On { .. } => "on",
        NotifyStatus::Snoozed { .. } => "snoozed",
        NotifyStatus::Error { .. } => "error",
        NotifyStatus::NoPoller => "no poller",
    }
}

fn render_title(
    hub: &HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.border_inactive)),
        area,
    );
    let mut x = area.x;
    let brand = " GitKraft Hub ";
    frame.render_widget(
        Paragraph::new(Span::styled(
            brand,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Rect::new(x, area.y, brand.len() as u16, 1),
    );
    x += brand.len() as u16;
    for (view, label) in [
        (View::Dashboard, " Dashboard "),
        (View::Pulse, " Pulse "),
        (View::Keys, " SSH keys "),
    ] {
        let w = label.len() as u16;
        if x + w > area.x + area.width {
            break;
        }
        let style = if hub.view == view {
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_secondary)
        };
        let rect = Rect::new(x, area.y, w, 1);
        frame.render_widget(Paragraph::new(Span::styled(label, style)), rect);
        hits.push(Hit {
            rect,
            target: Target::ViewTab(view),
        });
        x += w;
    }
    let (text, color) = bell_text(hub, true);
    let text = format!(" {text} ");
    let w = text.chars().count() as u16;
    if w < area.width {
        let rect = Rect::new(area.x + area.width - w, area.y, w, 1);
        frame.render_widget(
            Paragraph::new(Span::styled(
                text,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            rect,
        );
        hits.push(Hit {
            rect,
            target: Target::Bell,
        });
    }
}

fn render_section_tabs(
    hub: &HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let preview = hub.drag.section_preview();
    let mut x = area.x;
    for (vis, idx) in hub.visible_sections().into_iter().enumerate() {
        let section = &hub.dashboard.sections[idx];
        let count = hub.slots[idx].data.as_ref().map(|d| d.len().to_string());
        let loading = hub.slots[idx].loading;
        let suffix = match (&count, loading) {
            (_, true) => " …".to_string(),
            (Some(n), _) => format!(" {n}"),
            (None, _) => String::new(),
        };
        let label = format!(" {}{suffix} ", clip(&section.name, 24));
        let w = label.chars().count() as u16;
        if x + w > area.x + area.width {
            break;
        }
        let mut style = if hub.active == vis {
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(theme.text_secondary)
        };
        if let Some((from, over)) = preview {
            if from == vis {
                style = style.fg(theme.bg).bg(theme.warning);
            } else if over == Some(vis) {
                style = style.bg(theme.sel_bg).add_modifier(Modifier::BOLD);
            }
        }
        let rect = Rect::new(x, area.y, w, 1);
        frame.render_widget(Paragraph::new(Span::styled(label, style)), rect);
        hits.push(Hit {
            rect,
            target: Target::Section(vis),
        });
        x += w + 1;
    }
    if let Some((from, Some(over))) = preview
        && from != over
    {
        let hint = " ⇄ drop to move ";
        let w = hint.chars().count() as u16;
        if x + w < area.x + area.width {
            frame.render_widget(
                Paragraph::new(Span::styled(hint, Style::default().fg(theme.warning))),
                Rect::new(x, area.y, w, 1),
            );
        }
    }
}

// ── Dashboard ────────────────────────────────────────────────────────────────

fn render_dashboard(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let (lw, dw) = list::split_widths(area.width, hub.list_percent);
    let list_rect = Rect::new(area.x, area.y, lw, area.height);
    let div_rect = Rect::new(area.x + lw, area.y, 1, area.height);
    let det_rect = Rect::new(area.x + lw + 1, area.y, dw, area.height);
    hits.push(Hit {
        rect: list_rect,
        target: Target::ListArea,
    });
    hits.push(Hit {
        rect: det_rect,
        target: Target::DetailArea,
    });
    hits.push(Hit {
        rect: div_rect,
        target: Target::Divider,
    });
    let div_style = if hub.drag.divider_x().is_some() {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.border_inactive)
    };
    let bar: Vec<Line> = (0..div_rect.height)
        .map(|_| Line::from(Span::styled("│", div_style)))
        .collect();
    frame.render_widget(Paragraph::new(bar), div_rect);
    render_list(hub, frame, list_rect, theme, hits);
    if dw > 0 {
        render_detail(hub, frame, det_rect, theme, hits);
    }
}

fn row_line(data: &SectionData, i: usize, width: usize, theme: &UiTheme) -> Line<'static> {
    let m = muted(theme);
    match data {
        SectionData::Prs(r) => {
            let p = &r.items[i];
            let glyph = match (p.state, p.checks) {
                (ItemState::Open, Some(ChecksState::Failure)) => ("✘", theme.error),
                (ItemState::Open, Some(ChecksState::Pending)) => ("●", theme.warning),
                (ItemState::Open, _) if p.draft => ("◌", theme.text_muted),
                (s, _) => ("●", state_color(theme, s)),
            };
            let head = format!(" #{} ", p.number);
            let tail = format!("  {} · {}", p.author.login, age(p.updated_at));
            let repo = format!("{} ", p.repo.name);
            let room =
                width.saturating_sub(head.len() + tail.chars().count() + repo.chars().count() + 3);
            Line::from(vec![
                Span::styled(glyph.0, Style::default().fg(glyph.1)),
                Span::styled(head, m),
                Span::styled(repo, Style::default().fg(theme.accent)),
                Span::styled(
                    clip(&p.title, room),
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(tail, m),
            ])
        }
        SectionData::Issues(r) => {
            let it = &r.items[i];
            let head = format!(" #{} ", it.number);
            let tail = format!("  {} · {}", it.author.login, age(it.updated_at));
            let repo = format!("{} ", it.repo.name);
            let room =
                width.saturating_sub(head.len() + tail.chars().count() + repo.chars().count() + 3);
            Line::from(vec![
                Span::styled("●", Style::default().fg(state_color(theme, it.state))),
                Span::styled(head, m),
                Span::styled(repo, Style::default().fg(theme.accent)),
                Span::styled(
                    clip(&it.title, room),
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(tail, m),
            ])
        }
        SectionData::Runs(r) => {
            let run = &r.items[i];
            let tail = format!("  {} · {}", run.actor, age(run.created_at));
            let head = format!(" {} #{} {} ", run.name, run.number, run.branch);
            let room = width.saturating_sub(head.chars().count() + tail.chars().count() + 2);
            Line::from(vec![
                Span::styled(
                    run.status.glyph(false),
                    Style::default().fg(run_color(theme, run.status)),
                ),
                Span::styled(head, Style::default().fg(theme.accent)),
                Span::styled(
                    clip(&run.title, room),
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(tail, m),
            ])
        }
        SectionData::Inbox(r) => {
            let n = &r.items[i];
            let (why, color) = match n.reason {
                NotificationReason::ReviewRequested => ("review", theme.warning),
                NotificationReason::Mention => ("mention", theme.accent),
                NotificationReason::Assigned => ("assigned", theme.accent),
                NotificationReason::Comment => ("comment", theme.text_secondary),
                NotificationReason::CiActivity => ("ci", theme.error),
                NotificationReason::Subscribed | NotificationReason::Other => {
                    ("watching", theme.text_muted)
                }
            };
            let repo = n.repo.as_ref().map(|r| r.name.clone()).unwrap_or_default();
            let head = format!(" {why:<8} {repo} ");
            let tail = format!("  {}", age(n.updated_at));
            let room = width.saturating_sub(head.chars().count() + tail.chars().count() + 2);
            Line::from(vec![
                Span::styled(if n.unread { "●" } else { "○" }, Style::default().fg(color)),
                Span::styled(head, Style::default().fg(color)),
                Span::styled(
                    clip(&n.title, room),
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(tail, m),
            ])
        }
    }
}

fn render_list(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let focused = hub.focus == Focus::List;
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(if focused {
            theme.border_active
        } else {
            theme.border_inactive
        }));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(idx) = hub.active_index() else {
        frame.render_widget(
            Paragraph::new("No sections. Add some to dashboard.toml.").style(muted(theme)),
            inner,
        );
        return;
    };
    let slot = &hub.slots[idx];
    let Some(data) = slot.data.as_ref() else {
        let text = if !hub.ready {
            "Connecting accounts…".to_string()
        } else if hub.accounts == 0 {
            "No accounts found.\n\nSet GITHUB_TOKEN, GITLAB_TOKEN or CODEBERG_TOKEN\n(or GITKRAFT_TOKEN_<HOST>), or log in with `gh auth login`.\nSelf-hosted: add [[account]] to accounts.toml.".to_string()
        } else if slot.loading {
            "Loading…".to_string()
        } else {
            "Nothing loaded. Press r to retry.".to_string()
        };
        frame.render_widget(
            Paragraph::new(text)
                .style(muted(theme))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    };
    if data.is_empty() {
        let text = if data.errors().is_empty() {
            "Nothing here.".to_string()
        } else {
            data.errors().join("\n")
        };
        frame.render_widget(
            Paragraph::new(text)
                .style(muted(theme))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    let height = inner.height as usize;
    hub.offset = list::ensure_visible(hub.row, hub.offset, height);
    for (n, i) in (hub.offset..data.len()).take(height).enumerate() {
        let rect = Rect::new(inner.x, inner.y + n as u16, inner.width, 1);
        let mut line = row_line(data, i, inner.width as usize, theme);
        if i == hub.row {
            let bg = if focused {
                theme.sel_bg
            } else {
                theme.border_inactive
            };
            line = line.style(Style::default().bg(bg));
            // Fill the rest of the row so the highlight spans the pane.
            frame.render_widget(Block::default().style(Style::default().bg(bg)), rect);
        }
        frame.render_widget(Paragraph::new(line), rect);
        hits.push(Hit {
            rect,
            target: Target::Row(i),
        });
    }
}

// ── Detail ───────────────────────────────────────────────────────────────────

fn render_detail(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let focused = hub.focus == Focus::Detail;
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(if focused {
            theme.border_active
        } else {
            theme.border_inactive
        }));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if hub.detail.loading && hub.selected_note().is_none() {
        frame.render_widget(Paragraph::new("Loading…").style(muted(theme)), inner);
        return;
    }
    if let Some(e) = &hub.detail.error {
        frame.render_widget(
            Paragraph::new(e.clone())
                .style(Style::default().fg(theme.error))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    if hub.selected_pr().is_some() {
        render_pr(hub, frame, inner, theme, hits);
    } else if hub.selected_issue().is_some() {
        render_issue(hub, frame, inner, theme);
    } else if hub.selected_run().is_some() {
        render_run(hub, frame, inner, theme, hits);
    } else if let Some(n) = hub.selected_note() {
        let lines = vec![
            Line::from(Span::styled(
                n.title.clone(),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                n.repo.as_ref().map(|r| r.key()).unwrap_or_default(),
                muted(theme),
            )),
            Line::from(""),
            Line::from(Span::styled(
                n.url.clone(),
                Style::default().fg(theme.accent),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    } else {
        frame.render_widget(Paragraph::new("Select an item.").style(muted(theme)), inner);
    }
}

fn labels_line(labels: &[gitkraft_platforms::model::Label], theme: &UiTheme) -> Line<'static> {
    let mut spans = Vec::new();
    for l in labels {
        let color = u32::from_str_radix(&l.color, 16)
            .ok()
            .map_or(theme.accent, |c| {
                Color::Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
            });
        spans.push(Span::styled(
            format!(" {} ", l.name),
            Style::default().fg(color),
        ));
    }
    Line::from(spans)
}

fn render_pr(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let Some(listed) = hub.selected_pr().cloned() else {
        return;
    };
    // The detail has what the list lacks: checks, conflicts, size.
    let pr = hub.detail.pr.as_ref().map_or(listed, |d| d.pr.clone());
    let tab = hub.detail.tab.unwrap_or(DetailTab::Overview);
    let state_word = match (pr.state, pr.draft) {
        (ItemState::Open, true) => "draft",
        (ItemState::Open, false) => "open",
        (ItemState::Merged, _) => "merged",
        (ItemState::Closed, _) => "closed",
    };
    let checks = match pr.checks {
        Some(ChecksState::Success) => {
            Span::styled(" ✔ checks passed", Style::default().fg(theme.success))
        }
        Some(ChecksState::Failure) => {
            Span::styled(" ✘ checks failed", Style::default().fg(theme.error))
        }
        Some(ChecksState::Pending) => {
            Span::styled(" ● checks running", Style::default().fg(theme.warning))
        }
        None => Span::raw(""),
    };
    let conflict = if pr.mergeable == Some(false) {
        Span::styled("  ⚠ conflicts", Style::default().fg(theme.error))
    } else {
        Span::raw("")
    };
    let size = match (pr.additions, pr.deletions) {
        (Some(a), Some(d)) => Span::styled(format!("  +{a} −{d}"), muted(theme)),
        _ => Span::raw(""),
    };
    let images = hub.item_image_urls().len();
    let image_hint = if images > 0 {
        Span::styled(
            format!("  🖼 {images} (i)"),
            Style::default().fg(theme.diff_hunk),
        )
    } else {
        Span::raw("")
    };
    let header = vec![
        Line::from(Span::styled(
            pr.title.clone(),
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("{} ", state_word),
                Style::default()
                    .fg(state_color(theme, pr.state))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{}#{} by {}  ",
                    pr.repo.full_name(),
                    pr.number,
                    pr.author.login
                ),
                muted(theme),
            ),
            Span::styled(
                format!("{} → {}", pr.source_branch, pr.target_branch),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![checks, conflict, size, image_hint]),
        labels_line(&pr.labels, theme),
    ];
    let rows = Layout::vertical([
        Constraint::Length(header.len() as u16),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(header), rows[0]);
    // Tab strip.
    let mut x = rows[1].x;
    for (t, label) in [
        (DetailTab::Overview, " Overview "),
        (DetailTab::Diff, " Diff "),
        (DetailTab::Comments, " Comments "),
    ] {
        let label = if t == DetailTab::Comments {
            let n = hub
                .detail
                .pr
                .as_ref()
                .map_or(pr.comments as usize, |d| d.comments.len());
            format!(" Comments {n} ")
        } else {
            label.to_string()
        };
        let w = label.chars().count() as u16;
        let style = if tab == t {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else {
            Style::default().fg(theme.text_secondary)
        };
        let rect = Rect::new(
            x,
            rows[1].y,
            w.min(rows[1].width.saturating_sub(x - rows[1].x)),
            1,
        );
        frame.render_widget(Paragraph::new(Span::styled(label, style)), rect);
        hits.push(Hit {
            rect,
            target: Target::DetailTab(t),
        });
        x += w + 1;
    }
    let body = rows[2];
    let Some(detail) = hub.detail.pr.as_ref() else {
        frame.render_widget(Paragraph::new("Loading details…").style(muted(theme)), body);
        return;
    };
    let lines: Vec<Line> = match tab {
        DetailTab::Overview => {
            let mut v: Vec<Line> = Vec::new();
            if !detail.reviews.is_empty() {
                for r in &detail.reviews {
                    let (g, c, w) = match r.verdict {
                        Some(ReviewVerdict::Approve) => ("✔", theme.success, "approved"),
                        Some(ReviewVerdict::RequestChanges) => {
                            ("✘", theme.error, "requested changes")
                        }
                        _ => ("●", theme.text_muted, "commented"),
                    };
                    v.push(Line::from(vec![
                        Span::styled(format!("{g} "), Style::default().fg(c)),
                        Span::raw(format!("{} {w}", r.author.login)),
                    ]));
                }
                v.push(Line::from(""));
            }
            if !pr.reviewers.is_empty() {
                let names: Vec<_> = pr.reviewers.iter().map(|u| u.login.clone()).collect();
                v.push(Line::from(Span::styled(
                    format!("Review requested: {}", names.join(", ")),
                    muted(theme),
                )));
                v.push(Line::from(""));
            }
            let text = if pr.body.trim().is_empty() {
                "No description."
            } else {
                pr.body.as_str()
            };
            v.extend(markdown::render(text, &md_palette(theme)));
            v
        }
        DetailTab::Diff => {
            if detail.diff.is_empty() {
                vec![Line::from(Span::styled("No diff available.", muted(theme)))]
            } else {
                let cursor = (hub.focus == Focus::Detail).then_some(hub.detail.diff_cursor);
                detail
                    .diff
                    .lines()
                    .enumerate()
                    .map(|(i, l)| {
                        let line = diff_line(l, theme);
                        if cursor == Some(i) {
                            line.patch_style(Modifier::REVERSED)
                        } else {
                            line
                        }
                    })
                    .collect()
            }
        }
        DetailTab::Comments => {
            let mut v = Vec::new();
            for c in &detail.comments {
                v.push(Line::from(vec![
                    Span::styled(
                        c.author.login.clone(),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}", age(c.created_at)), muted(theme)),
                ]));
                v.extend(
                    markdown::render(&c.body, &md_palette(theme))
                        .into_iter()
                        .map(indent),
                );
                v.push(Line::from(""));
            }
            if v.is_empty() {
                v.push(Line::from(Span::styled(
                    "No comments. Press c to write one.",
                    muted(theme),
                )));
            }
            v
        }
    };
    let wrap = tab != DetailTab::Diff;
    hub.detail.body_rows = body.height;
    if tab == DetailTab::Diff && hub.focus == Focus::Detail && body.height > 0 {
        let (c, h) = (hub.detail.diff_cursor, body.height as usize);
        if c < hub.detail.scroll as usize {
            hub.detail.scroll = c as u16;
        } else if c >= hub.detail.scroll as usize + h {
            hub.detail.scroll = (c + 1 - h) as u16;
        }
    }
    let max_scroll = lines.len().saturating_sub(body.height as usize) as u16;
    hub.detail.scroll = hub
        .detail
        .scroll
        .min(if wrap { u16::MAX } else { max_scroll });
    let mut p = Paragraph::new(lines).scroll((hub.detail.scroll, 0));
    if wrap {
        p = p.wrap(Wrap { trim: false });
    }
    frame.render_widget(p, body);
}

fn indent(mut line: Line<'static>) -> Line<'static> {
    line.spans.insert(0, Span::raw("  "));
    line
}

fn diff_line(l: &str, theme: &UiTheme) -> Line<'static> {
    let color = if l.starts_with("+++") || l.starts_with("---") || l.starts_with("diff ") {
        theme.text_secondary
    } else if l.starts_with('+') {
        theme.diff_add
    } else if l.starts_with('-') {
        theme.diff_del
    } else if l.starts_with("@@") {
        theme.diff_hunk
    } else {
        theme.diff_context
    };
    Line::from(Span::styled(l.to_string(), Style::default().fg(color)))
}

fn render_issue(hub: &mut HubState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let Some(it) = hub.selected_issue().cloned() else {
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            it.title.clone(),
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                if it.state == ItemState::Open {
                    "open "
                } else {
                    "closed "
                },
                Style::default()
                    .fg(state_color(theme, it.state))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "{}#{} by {}",
                    it.repo.full_name(),
                    it.number,
                    it.author.login
                ),
                muted(theme),
            ),
        ]),
        labels_line(&it.labels, theme),
    ];
    if let Some(m) = &it.milestone {
        lines.push(Line::from(Span::styled(
            format!("milestone {m}"),
            muted(theme),
        )));
    }
    lines.push(Line::from(""));
    let body = if it.body.trim().is_empty() {
        "No description."
    } else {
        it.body.as_str()
    };
    lines.extend(markdown::render(body, &md_palette(theme)));
    for c in &hub.detail.comments {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                c.author.login.clone(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", age(c.created_at)), muted(theme)),
        ]));
        lines.extend(c.body.lines().map(|l| Line::from(format!("  {l}"))));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((hub.detail.scroll, 0)),
        area,
    );
}

fn render_run(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let Some(run) = hub.selected_run().cloned() else {
        return;
    };
    let dur = run
        .duration()
        .map(|d| format_seconds(d.num_seconds() as f64))
        .unwrap_or_default();
    let header = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", run.status.glyph(false)),
                Style::default().fg(run_color(theme, run.status)),
            ),
            Span::styled(
                format!("{} #{}", run.name, run.number),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {dur}"), muted(theme)),
        ]),
        Line::from(Span::styled(
            format!(
                "{} · {} · {} · {}",
                run.repo.full_name(),
                run.branch,
                run.event,
                run.actor
            ),
            muted(theme),
        )),
        Line::from(Span::styled(
            run.title.clone(),
            Style::default().fg(theme.text_secondary),
        )),
    ];
    let Some(detail) = hub.detail.run.as_ref() else {
        frame.render_widget(Paragraph::new(header), area);
        return;
    };
    let jobs_h = (detail.jobs.len() as u16 + 1)
        .min(area.height.saturating_sub(header.len() as u16 + 1) / 2 + 1);
    let rows = Layout::vertical([
        Constraint::Length(header.len() as u16),
        Constraint::Length(jobs_h.max(1)),
        Constraint::Min(0),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(header), rows[0]);
    hub.detail.job = hub.detail.job.min(detail.jobs.len().saturating_sub(1));
    let sel = hub.detail.job;
    let off = list::ensure_visible(sel, 0, rows[1].height as usize);
    for (n, (i, job)) in detail
        .jobs
        .iter()
        .enumerate()
        .skip(off)
        .take(rows[1].height as usize)
        .enumerate()
    {
        let _ = n;
        let rect = Rect::new(rows[1].x, rows[1].y + (i - off) as u16, rows[1].width, 1);
        let secs = timing::job_seconds(job)
            .map(format_seconds)
            .unwrap_or_default();
        let stage = job
            .stage
            .as_deref()
            .map(|s| format!("{s} · "))
            .unwrap_or_default();
        let line = Line::from(vec![
            Span::styled(
                format!("{} ", job.status.glyph(false)),
                Style::default().fg(run_color(theme, job.status)),
            ),
            Span::styled(
                format!("{stage}{}", job.name),
                Style::default().fg(theme.text_primary),
            ),
            Span::styled(format!("  {secs}"), muted(theme)),
        ]);
        let line = if i == sel {
            line.style(Style::default().bg(theme.sel_bg))
        } else {
            line
        };
        if i == sel {
            frame.render_widget(
                Block::default().style(Style::default().bg(theme.sel_bg)),
                rect,
            );
        }
        frame.render_widget(Paragraph::new(line), rect);
        hits.push(Hit {
            rect,
            target: Target::Job(i),
        });
    }
    let rest = rows[2];
    if rest.height == 0 {
        return;
    }
    // Steps of the selected job, then the Gantt of the whole run if there is room.
    let steps: Vec<Line> = detail
        .jobs
        .get(sel)
        .map(|j| {
            j.steps
                .iter()
                .map(|s| {
                    let secs = match (s.started_at, s.finished_at) {
                        (Some(a), Some(b)) => {
                            format_seconds((b - a).num_milliseconds() as f64 / 1000.0)
                        }
                        _ => String::new(),
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("  {} ", s.status.glyph(false)),
                            Style::default().fg(run_color(theme, s.status)),
                        ),
                        Span::styled(s.name.clone(), Style::default().fg(theme.text_secondary)),
                        Span::styled(format!("  {secs}"), muted(theme)),
                    ])
                })
                .collect()
        })
        .unwrap_or_default();
    let steps_h = (steps.len() as u16).min(rest.height / 2);
    let parts = Layout::vertical([Constraint::Length(steps_h), Constraint::Min(0)]).split(rest);
    frame.render_widget(Paragraph::new(steps), parts[0]);
    let bars: Vec<_> = timing::gantt(detail)
        .into_iter()
        .filter(|b| b.depth == 0)
        .collect();
    if parts[1].height >= 4 && !bars.is_empty() {
        let g = Gantt::new(
            bars.iter()
                .map(|b| Task::new(b.label.clone(), b.start, b.start + b.length.max(0.5))),
        )
        .block(Block::default().borders(Borders::TOP).title(" Timeline "));
        frame.render_widget(&g, parts[1]);
    }
}

// ── Images and markdown ──────────────────────────────────────────────────────

fn md_palette(theme: &UiTheme) -> markdown::Palette {
    markdown::Palette {
        text: theme.text_primary,
        accent: theme.accent,
        muted: theme.text_muted,
        code: theme.warning,
        link: theme.diff_hunk,
    }
}

fn render_images(hub: &mut HubState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let (index, total) = (hub.images.index, hub.images.urls.len());
    let url = hub.images.current().unwrap_or_default().to_string();
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" Image {}/{total} ", index + 1),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(clip(&url, rows[0].width as usize), muted(theme)),
        ])),
        rows[0],
    );
    match hub.images.entry_mut(&url) {
        Some(Entry::Ready(p)) => frame.render_stateful_widget(
            StatefulImage::default().resize(Resize::Scale(None)),
            rows[1],
            &mut p.0,
        ),
        Some(Entry::Failed(why)) => frame.render_widget(
            Paragraph::new(format!(" {why}")).style(Style::default().fg(theme.error)),
            rows[1],
        ),
        _ => frame.render_widget(
            Paragraph::new(" loading image…").style(muted(theme)),
            rows[1],
        ),
    }
}

// ── Log viewer ───────────────────────────────────────────────────────────────

fn ansi(c: AnsiColor) -> Color {
    match c {
        AnsiColor::Indexed(n) => Color::Indexed(n),
        AnsiColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn render_log(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let Some(log) = hub.log.as_mut() else { return };
    let ann_h = if log.show_annotations {
        (log.annotations.len() as u16).min(6).min(area.height / 3)
    } else {
        0
    };
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(ann_h),
        Constraint::Min(1),
    ])
    .split(area);
    let search = if log.query.is_empty() {
        String::new()
    } else {
        format!(
            "  /{} {}/{}",
            log.query,
            if log.matches.is_empty() {
                0
            } else {
                log.match_pos + 1
            },
            log.matches.len()
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", log.title),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{} lines{search}", log.lines.len()), muted(theme)),
        ])),
        rows[0],
    );
    for (n, a) in log.annotations.iter().take(ann_h as usize).enumerate() {
        let rect = Rect::new(rows[1].x, rows[1].y + n as u16, rows[1].width, 1);
        let loc = match (&a.file, a.line) {
            (Some(f), Some(l)) => format!("{f}:{l}  "),
            (Some(f), None) => format!("{f}  "),
            _ => String::new(),
        };
        let color = if a.severity == gitkraft_ci::annotations::Severity::Error {
            theme.error
        } else {
            theme.warning
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ✘ ", Style::default().fg(color)),
                Span::styled(loc, Style::default().fg(theme.accent)),
                Span::styled(
                    clip(&a.message, rect.width as usize),
                    Style::default().fg(theme.text_primary),
                ),
            ])),
            rect,
        );
        hits.push(Hit {
            rect,
            target: Target::Annotation(n),
        });
    }
    let body = rows[2];
    hits.push(Hit {
        rect: body,
        target: Target::DetailArea,
    });
    log.offset = list::ensure_visible(log.cursor, log.offset, body.height as usize);
    let num_w = log.lines.len().to_string().len().max(3);
    for (n, vis) in (log.offset..log.visible.len())
        .take(body.height as usize)
        .enumerate()
    {
        let li = log.visible[vis];
        let l = &log.lines[li];
        let rect = Rect::new(body.x, body.y + n as u16, body.width, 1);
        let is_match = log.matches.binary_search(&li).is_ok();
        let marker = match &l.kind {
            LineKind::GroupStart(_) => {
                if log.collapsed.contains(&li) {
                    "▸ "
                } else {
                    "▾ "
                }
            }
            _ => "  ",
        };
        let indent = "  ".repeat(usize::from(l.depth));
        let mut spans = vec![
            Span::styled(format!("{:>num_w$} ", l.number), muted(theme)),
            Span::raw(indent),
            Span::styled(marker, Style::default().fg(theme.accent)),
        ];
        let base = match l.kind {
            LineKind::Error => Style::default().fg(theme.error),
            LineKind::Warning => Style::default().fg(theme.warning),
            LineKind::Command => Style::default().fg(theme.accent),
            LineKind::GroupStart(_) => Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
            _ => Style::default().fg(theme.text_secondary),
        };
        for s in &l.spans {
            let mut st = base;
            if let Some(fg) = s.style.fg {
                st = st.fg(ansi(fg));
            }
            if let Some(bg) = s.style.bg {
                st = st.bg(ansi(bg));
            }
            if s.style.bold {
                st = st.add_modifier(Modifier::BOLD);
            }
            if s.style.dim {
                st = st.add_modifier(Modifier::DIM);
            }
            if s.style.underline {
                st = st.add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(s.text.clone(), st));
        }
        let mut line = Line::from(spans);
        if vis == log.cursor {
            line = line.style(Style::default().bg(theme.sel_bg));
        } else if is_match {
            line = line.style(Style::default().bg(theme.warning).fg(theme.bg));
        }
        frame.render_widget(Paragraph::new(line), rect);
        hits.push(Hit {
            rect,
            target: Target::LogRow(vis),
        });
    }
}

// ── SSH keys ─────────────────────────────────────────────────────────────────

fn render_keys(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive))
        .title(" SSH keys · git over ssh without passwords ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let Some(data) = hub.keys.data.as_ref() else {
        let text = if hub.keys.loading {
            "Loading keys…"
        } else {
            "No keys loaded. Press r."
        };
        frame.render_widget(Paragraph::new(Span::styled(text, muted(theme))), inner);
        return;
    };
    let mut lines: Vec<Line> = Vec::new();
    let mut shown = Vec::new();
    for (i, r) in data.rows.iter().enumerate() {
        let selected = i == hub.keys.row;
        let mark = if r.mine { "●" } else { " " };
        let when = r
            .key
            .created_at
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        let style = if selected {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else {
            Style::default().fg(theme.text_primary)
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {mark} "),
                if selected {
                    style
                } else {
                    Style::default().fg(theme.success)
                },
            ),
            Span::styled(format!("{:<16} ", r.host), style),
            Span::styled(format!("{:<28} ", r.key.title), style),
            Span::styled(
                format!("{} ", r.key.fingerprint.as_deref().unwrap_or("-")),
                if selected { style } else { muted(theme) },
            ),
            Span::styled(when, if selected { style } else { muted(theme) }),
        ]));
        shown.push(i);
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No SSH keys on any account. Press s to create one for this machine and register it.",
            muted(theme),
        )));
    }
    lines.push(Line::default());
    for e in &data.errors {
        lines.push(Line::from(Span::styled(
            format!(" ✘ {e}"),
            Style::default().fg(theme.error),
        )));
    }
    lines.push(Line::from(Span::styled(
        format!(
            " ● = a key this machine holds · accounts: {}",
            data.hosts.join(", ")
        ),
        muted(theme),
    )));
    for (n, i) in shown.iter().enumerate() {
        if (n as u16) < inner.height {
            hits.push(Hit {
                rect: Rect::new(inner.x, inner.y + n as u16, inner.width, 1),
                target: Target::Row(*i),
            });
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Pulse ────────────────────────────────────────────────────────────────────

fn render_pulse(hub: &mut HubState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let rows = Layout::vertical([
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Min(0),
    ])
    .split(area);
    // Contribution calendar over the last 53 weeks.
    let today = Utc::now().date_naive();
    let start =
        today - ChronoDuration::days(7 * 52 + i64::from(today.weekday().num_days_from_monday()));
    let epoch = |d: chrono::NaiveDate| i64::from(d.num_days_from_ce()) - 719_163;
    let heat_block = |title: String| {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_inactive))
            .title(title)
    };
    match &hub.pulse.contributions {
        Some(c) => {
            let days = (epoch(today) - epoch(start) + 1).max(1) as usize;
            let mut values = vec![0.0f64; days];
            for d in &c.items {
                let i = epoch(d.date) - epoch(start);
                if i >= 0 && (i as usize) < days {
                    values[i as usize] = f64::from(d.count);
                }
            }
            let total: u32 = c.items.iter().map(|d| d.count).sum();
            let chart = CalendarHeatmap::new(
                days_from_civil(i64::from(start.year()), start.month(), start.day()),
                values,
            )
            .zero_empty(true)
            .block(heat_block(format!(
                " Contributions · {total} in the last year "
            )));
            frame.render_widget(&chart, rows[0]);
        }
        None => frame.render_widget(
            Paragraph::new(" loading contributions… (press r)")
                .style(muted(theme))
                .block(heat_block(" Contributions ".into())),
            rows[0],
        ),
    }
    // Numbers.
    let repo_name = hub
        .pulse
        .repo
        .as_ref()
        .map(|r| r.key())
        .unwrap_or_else(|| "no repository".into());
    let title = format!(" Pulse · {repo_name} · {} days ", hub.pulse.days.max(1));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive))
        .title(title);
    match &hub.pulse.data {
        Some(p) => {
            let inner = block.inner(rows[1]);
            frame.render_widget(block, rows[1]);
            let cols = Layout::horizontal([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(inner);
            let stat = |label: &str, n: usize, color: Color| {
                Line::from(vec![
                    Span::styled(
                        format!("{n:>4} "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(label.to_string(), Style::default().fg(theme.text_secondary)),
                ])
            };
            frame.render_widget(
                Paragraph::new(vec![
                    stat("PRs opened", p.prs_opened, theme.success),
                    stat("PRs merged", p.prs_merged, theme.accent),
                    stat("PRs closed", p.prs_closed, theme.error),
                    stat("PRs open now", p.prs_open_now, theme.warning),
                ]),
                cols[0],
            );
            frame.render_widget(
                Paragraph::new(vec![
                    stat("issues opened", p.issues_opened, theme.success),
                    stat("issues closed", p.issues_closed, theme.accent),
                    stat("CI green", p.ci_green, theme.success),
                    stat("CI red", p.ci_red, theme.error),
                ]),
                cols[1],
            );
            let authors: Vec<Line> = p
                .top_authors
                .iter()
                .map(|(a, n)| stat(a, *n, theme.accent))
                .collect();
            frame.render_widget(Paragraph::new(authors), cols[2]);
        }
        None => frame.render_widget(
            Paragraph::new(if hub.pulse.loading {
                " loading…"
            } else {
                " press r to load"
            })
            .style(muted(theme))
            .block(block),
            rows[1],
        ),
    }
    // PRs per day.
    if let Some(p) = &hub.pulse.data
        && rows[2].height >= 5
    {
        let n = p.prs_per_day.len();
        let groups: Vec<BarGroup> = p
            .prs_per_day
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let label = if n <= 14 || (n - 1 - i) % 7 == 0 {
                    format!("{}", (n - 1 - i))
                } else {
                    String::new()
                };
                BarGroup::new(label, [*v])
            })
            .collect();
        let chart = BarChart::new(groups).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_inactive))
                .title(" PRs opened per day (days ago) "),
        );
        frame.render_widget(&chart, rows[2]);
    }
}

// ── Status line ──────────────────────────────────────────────────────────────

fn hints(hub: &HubState) -> &'static str {
    if hub.images.open {
        return "←/→ previous / next image · Esc close";
    }
    if hub.log.is_some() {
        return "j/k scroll · / search · n/N next · e first error · a errors · space fold · g/G ends · Esc close";
    }
    if hub.view == View::Keys {
        return "j/k select · : palette · s set up this machine · t test · d remove key · r refresh · Tab dashboard · q back";
    }
    if hub.view == View::Pulse {
        return "r refresh · [ ] repo · 1/2/3/4 = 7/30/90/365 days · Tab dashboard · q back";
    }
    match hub
        .active_index()
        .and_then(|i| hub.dashboard.sections.get(i))
        .map(|s| s.kind)
    {
        Some(SectionKind::Prs) => {
            "a approve · x changes · c comment · m/M merge · W when green · Q queue · ! force · u update · S method · C close · w worktree · q back"
        }
        Some(SectionKind::Issues) => "c comment · C close/reopen · r refresh · n bell · q back",
        Some(SectionKind::Ci) => {
            "Enter log · f rerun failed · F rerun all · D run workflow · A artifacts · X cancel · r refresh · q back"
        }
        _ => {
            "Tab section · : palette · {/} move section · p pulse · K ssh keys · r refresh · n bell · q back"
        }
    }
}

fn render_status(hub: &HubState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.border_inactive)),
        area,
    );
    let line = if let Some(c) = &hub.confirm {
        Line::from(Span::styled(
            format!(" {} ", c.text),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ))
    } else if let Some(p) = &hub.prompt {
        let label = match p.purpose {
            PromptPurpose::Comment => "comment",
            PromptPurpose::Approve => "approve note (Enter to send)",
            PromptPurpose::RequestChanges => "what must change",
            PromptPurpose::LogSearch => "search",
            PromptPurpose::Dispatch => "run workflow: WORKFLOW REF key=value…",
            PromptPurpose::Inline => "inline comment",
            PromptPurpose::Labels => "labels: +add -remove \"two words\"",
        };
        Line::from(vec![
            Span::styled(format!(" {label} › "), Style::default().fg(theme.accent)),
            Span::styled(
                format!("{}█", p.buffer),
                Style::default().fg(theme.text_primary),
            ),
        ])
    } else if let Some((text, ok)) = hub.fresh_notice() {
        Line::from(Span::styled(
            format!(" {text}"),
            Style::default().fg(if ok { theme.success } else { theme.error }),
        ))
    } else {
        Line::from(Span::styled(
            format!(" {}", hints(hub)),
            Style::default().fg(theme.text_secondary),
        ))
    };
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppScreen;
    use crate::features::hub::logview::LogView;
    use crate::features::hub::state::fixtures::state;
    use gitkraft_hub::{PrDetail, SectionResult};
    use gitkraft_platforms::model::{Comment, Review, User};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn app_with(hub: HubState) -> App {
        let mut app = App::new();
        app.screen = AppScreen::Hub;
        app.hub = Some(Box::new(hub));
        app
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).expect("terminal");
        term.draw(|f| render(app, f, f.area())).expect("draw");
        let buf = term.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn dashboard_shows_tabs_rows_and_records_what_is_clickable() {
        let mut app = app_with(state());
        let text = draw(&mut app, 120, 30);
        for needle in [
            "GitKraft Hub",
            "Dashboard",
            "Pulse",
            "Mine 5",
            "Issues 0",
            "pr 1",
            "pr 5",
            "#3",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
        let hub = app.hub.as_ref().expect("hub");
        let count = |f: fn(&Target) -> bool| hub.hits.iter().filter(|h| f(&h.target)).count();
        assert_eq!(count(|t| matches!(t, Target::Section(_))), 3);
        assert_eq!(count(|t| matches!(t, Target::Row(_))), 5);
        assert_eq!(count(|t| matches!(t, Target::Divider)), 1);
        assert_eq!(count(|t| matches!(t, Target::Bell)), 1);
        assert_eq!(count(|t| matches!(t, Target::ViewTab(_))), 3);
    }

    fn keys_data() -> gitkraft_hub::keys::KeysData {
        use gitkraft_hub::keys::{KeyRow, KeysData};
        use gitkraft_platforms::model::SshKey;
        let key = |id: &str, title: &str| SshKey {
            id: id.into(),
            title: title.into(),
            key: String::new(),
            fingerprint: Some(format!("SHA256:{id}")),
            created_at: None,
        };
        KeysData {
            hosts: vec!["github.com".into(), "gitlab.com".into()],
            rows: vec![
                KeyRow {
                    host: "github.com".into(),
                    key: key("1", "gitkraft laptop"),
                    mine: true,
                },
                KeyRow {
                    host: "gitlab.com".into(),
                    key: key("2", "old"),
                    mine: false,
                },
            ],
            errors: vec!["codeberg.org: 403".into()],
        }
    }

    #[test]
    fn the_keys_view_lists_keys_marks_ours_and_shows_failures() {
        let mut hub = state();
        hub.view = View::Keys;
        hub.keys.data = Some(keys_data());
        let mut app = app_with(hub);
        let text = draw(&mut app, 120, 20);
        for want in [
            "SSH keys",
            "gitkraft laptop",
            "gitlab.com",
            "● ",
            "codeberg.org: 403",
            "accounts: github.com, gitlab.com",
        ] {
            assert!(text.contains(want), "{want} in\n{text}");
        }
        let hub = app.hub.as_ref().expect("hub");
        assert_eq!(
            hub.hits
                .iter()
                .filter(|h| matches!(h.target, Target::Row(_)))
                .count(),
            2
        );
    }

    #[test]
    fn the_keys_view_explains_empty_and_loading_states() {
        let mut hub = state();
        hub.view = View::Keys;
        let mut app = app_with(hub);
        assert!(draw(&mut app, 100, 12).contains("No keys loaded"));
        app.hub.as_mut().expect("hub").keys.loading = true;
        assert!(draw(&mut app, 100, 12).contains("Loading keys"));
        app.hub.as_mut().expect("hub").keys.data = Some(Default::default());
        assert!(draw(&mut app, 100, 12).contains("Press s to create one"));
        for (w, h) in [(1, 1), (5, 3), (30, 5)] {
            draw(&mut app, w, h);
        }
    }

    #[test]
    fn the_palette_draws_its_query_matches_and_keys() {
        let mut hub = state();
        hub.palette = Some(crate::features::hub::palette::Palette {
            query: "merge".into(),
            row: 0,
        });
        let mut app = app_with(hub);
        let text = draw(&mut app, 100, 24);
        assert!(
            text.contains("Command palette")
                && text.contains("merge█")
                && text.contains("Pull request: merge"),
            "{text}"
        );
        app.hub.as_mut().expect("hub").palette = Some(crate::features::hub::palette::Palette {
            query: "qqq".into(),
            row: 0,
        });
        assert!(draw(&mut app, 100, 24).contains("nothing matches"));
        app.hub.as_mut().expect("hub").palette = Some(Default::default());
        for (w, h) in [(1, 1), (10, 3), (22, 6), (300, 80)] {
            draw(&mut app, w, h);
        }
    }

    #[test]
    fn the_diff_cursor_stays_in_view_and_is_highlighted() {
        let mut hub = state();
        let pr = hub.selected_pr().cloned().expect("pr");
        let diff: String = (0..80).map(|i| format!("+line {i}\n")).collect();
        hub.detail.pr = Some(gitkraft_hub::PrDetail {
            pr,
            comments: vec![],
            reviews: vec![],
            diff,
            errors: vec![],
        });
        hub.detail.tab = Some(DetailTab::Diff);
        hub.focus = Focus::Detail;
        hub.detail.diff_cursor = 70;
        let mut app = app_with(hub);
        let text = draw(&mut app, 120, 24);
        assert!(text.contains("+line 70"), "{text}");
        assert!(app.hub.as_ref().expect("hub").detail.scroll > 0);
    }

    #[test]
    fn hit_rectangles_stay_inside_the_screen() {
        let mut app = app_with(state());
        draw(&mut app, 80, 24);
        for h in &app.hub.as_ref().expect("hub").hits {
            assert!(
                h.rect.x + h.rect.width <= 80 && h.rect.y + h.rect.height <= 24,
                "{h:?}"
            );
        }
    }

    #[test]
    fn survives_tiny_and_huge_terminals() {
        for (w, h) in [(1, 1), (5, 3), (20, 4), (40, 10), (300, 80)] {
            let mut app = app_with(state());
            draw(&mut app, w, h);
            app.hub.as_mut().expect("hub").view = View::Pulse;
            draw(&mut app, w, h);
            app.hub.as_mut().expect("hub").view = View::Dashboard;
            app.hub.as_mut().expect("hub").log =
                Some(LogView::new("j", "a\nerror: x\n --> f.rs:1:1"));
            draw(&mut app, w, h);
        }
    }

    #[test]
    fn empty_and_unconnected_states_explain_what_to_do() {
        let mut hub = state();
        hub.slots[0].data = None;
        hub.ready = true;
        hub.accounts = 0;
        let mut app = app_with(hub);
        let text = draw(&mut app, 100, 20);
        assert!(
            text.contains("No accounts found") && text.contains("GITHUB_TOKEN"),
            "{text}"
        );
        app.hub.as_mut().expect("hub").ready = false;
        assert!(draw(&mut app, 100, 20).contains("Connecting accounts"));
    }

    #[test]
    fn a_pull_request_detail_shows_header_reviews_and_tabs() {
        let mut hub = state();
        let mut pr = crate::features::hub::state::fixtures::pr(1, "pr 1");
        pr.body = "Adds the thing.".into();
        pr.checks = Some(ChecksState::Failure);
        pr.mergeable = Some(false);
        hub.detail.key = Some("github.com/o/r#1".into());
        hub.detail.tab = Some(DetailTab::Overview);
        hub.detail.pr = Some(PrDetail {
            pr: pr.clone(),
            comments: vec![Comment {
                id: 1,
                author: User {
                    login: "bob".into(),
                    avatar_url: None,
                },
                body: "nice".into(),
                created_at: None,
            }],
            reviews: vec![Review {
                author: User {
                    login: "carol".into(),
                    avatar_url: None,
                },
                verdict: Some(ReviewVerdict::Approve),
                body: String::new(),
                submitted_at: None,
            }],
            diff: "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new".into(),
            errors: vec![],
        });
        hub.detail.loading = false;
        let mut app = app_with(hub);
        let text = draw(&mut app, 140, 30);
        for needle in [
            "feat → master",
            "checks failed",
            "conflicts",
            "carol approved",
            "Adds the thing.",
            "Overview",
            "Diff",
            "Comments 1",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
        let hub = app.hub.as_mut().expect("hub");
        hub.detail.tab = Some(DetailTab::Diff);
        let text = draw(&mut app, 140, 30);
        assert!(text.contains("+new") && text.contains("-old"));
        let hub = app.hub.as_mut().expect("hub");
        hub.detail.tab = Some(DetailTab::Comments);
        assert!(draw(&mut app, 140, 30).contains("nice"));
    }

    #[test]
    fn a_dragged_tab_is_marked_and_a_log_shows_errors_and_folds() {
        let mut hub = state();
        hub.drag.press(
            2,
            1,
            Some(crate::features::hub::drag::DragTarget::Section(0)),
        );
        hub.drag.motion(15, 1, Some(1));
        let mut app = app_with(hub);
        assert!(draw(&mut app, 100, 20).contains("drop to move"));
        let hub = app.hub.as_mut().expect("hub");
        hub.drag.cancel();
        hub.log = Some(LogView::new(
            "CI · build",
            "##[group]Build\nerror[E0425]: no x\n --> src/a.rs:1:2\n##[endgroup]",
        ));
        let text = draw(&mut app, 100, 20);
        for needle in ["CI · build", "src/a.rs:1", "no x", "▾"] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
    }

    #[test]
    fn pulse_renders_numbers_and_a_calendar() {
        let mut hub = state();
        hub.view = View::Pulse;
        hub.pulse.repo = Some(gitkraft_platforms::RepoId::new("github.com", "o", "r"));
        hub.pulse.days = 30;
        hub.pulse.data = Some(gitkraft_hub::pulse::Pulse {
            days: 30,
            prs_opened: 7,
            prs_merged: 4,
            top_authors: vec![("alice".into(), 5)],
            prs_per_day: vec![1.0; 30],
            ..Default::default()
        });
        let today = Utc::now().date_naive();
        hub.pulse.contributions = Some(SectionResult {
            items: (0..30)
                .map(|i| gitkraft_platforms::model::ActivityDay {
                    date: today - ChronoDuration::days(i),
                    count: i as u32 % 5,
                })
                .collect(),
            errors: vec![],
        });
        let mut app = app_with(hub);
        let text = draw(&mut app, 120, 40);
        for needle in [
            "Contributions",
            "Pulse · github.com/o/r",
            "PRs opened",
            "alice",
            "PRs opened per day",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
    }

    #[test]
    fn the_status_line_shows_notices_prompts_and_confirmations() {
        let mut hub = state();
        hub.set_notice("section moved".into(), true);
        let mut app = app_with(hub);
        assert!(draw(&mut app, 100, 12).contains("section moved"));
        let hub = app.hub.as_mut().expect("hub");
        hub.notice = None;
        hub.prompt = Some(crate::features::hub::state::Prompt {
            purpose: PromptPurpose::Comment,
            buffer: "hel".into(),
        });
        assert!(draw(&mut app, 100, 12).contains("comment › hel"));
        let hub = app.hub.as_mut().expect("hub");
        hub.prompt = None;
        hub.confirm = Some(crate::features::hub::state::Confirm {
            action: crate::features::hub::state::Action::CancelRun,
            text: "Cancel this run? y/n".into(),
        });
        assert!(draw(&mut app, 100, 12).contains("Cancel this run?"));
    }

    #[test]
    fn the_image_overlay_draws_loading_failed_and_decoded_images() {
        let mut hub = state();
        hub.images.open_on(vec![
            "https://x/1.png".into(),
            "https://x/2.png".into(),
            "https://x/3.png".into(),
        ]);
        hub.images.mark_loading("https://x/1.png");
        hub.images.set_failed("https://x/2.png", "HTTP 404".into());
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            64,
            64,
            image::Rgba([220, 40, 40, 255]),
        ));
        hub.set_image("https://x/3.png", img);
        let mut app = app_with(hub);
        let text = draw(&mut app, 80, 20);
        assert!(
            text.contains("Image 1/3") && text.contains("loading image"),
            "{text}"
        );
        app.hub.as_mut().expect("hub").images.step(1);
        assert!(draw(&mut app, 80, 20).contains("HTTP 404"));
        app.hub.as_mut().expect("hub").images.step(1);
        let text = draw(&mut app, 80, 20);
        assert!(text.contains("Image 3/3"));
        // The half-block fallback paints the picture as coloured cells.
        let mut term = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        term.draw(|f| render(&mut app, f, f.area())).expect("draw");
        let red = Color::Rgb(220, 40, 40);
        let painted = term
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|c| c.fg == red || c.bg == red)
            .count();
        assert!(painted > 50, "only {painted} image cells");
    }

    #[test]
    fn markdown_bodies_and_the_image_count_show_in_the_detail() {
        let mut hub = state();
        if let Some(gitkraft_hub::SectionData::Prs(r)) = hub.slots[0].data.as_mut() {
            r.items[0].body = "# Plan\n- [x] done\n![shot](https://x/a.png)".into();
        }
        hub.detail.tab = Some(DetailTab::Overview);
        hub.detail.pr = Some(PrDetail {
            pr: hub.selected_pr().cloned().expect("pr"),
            comments: vec![],
            reviews: vec![],
            diff: String::new(),
            errors: vec![],
        });
        hub.detail.key = Some("github.com/o/r#1".into());
        let mut app = app_with(hub);
        let text = draw(&mut app, 140, 30);
        for needle in ["Plan", "• ☑ done", "🖼 shot", "🖼 1 (i)"] {
            assert!(text.contains(needle), "missing {needle:?} in\n{text}");
        }
        assert!(!text.contains("# Plan") && !text.contains("[x]"));
    }
}
