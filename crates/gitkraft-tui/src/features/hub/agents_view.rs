//! Draws the Agents view: jobs on the left, the selected job's settings, repository picker and
//! latest report on the right. Widgets come from the author's own crates: `tui-checkbox` for every
//! switch and pick, `tui-chart` for the findings history, `tui-skeleton` while repositories load
//! and `tui-spinner` while a job runs.

use std::sync::OnceLock;
use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_chart::{BarChart, BarGroup};
use tui_checkbox::Checkbox;
use tui_skeleton::{AnimationMode, SkeletonList};
use tui_spinner::{FluxFrames, FluxSpinner};

use super::agents::{AgentsState, Pane};
use super::state::{Hit, HubState, Target};
use crate::features::theme::palette::UiTheme;

/// Milliseconds since the first frame, the clock of every animation here.
fn elapsed_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn muted(theme: &UiTheme) -> Style {
    Style::default().fg(theme.text_muted)
}

fn block<'a>(title: String, theme: &UiTheme, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused {
            theme.border_active
        } else {
            theme.border_inactive
        }))
        .title(title)
}

/// A checkbox in the view's look.
fn check<'a>(label: String, on: bool, theme: &UiTheme, selected: bool) -> Checkbox<'a> {
    let mut label_style = Style::default().fg(theme.text_primary);
    if selected {
        label_style = label_style.fg(theme.bg).bg(theme.accent);
    }
    Checkbox::new(label, on)
        .checked_symbol("☑")
        .unchecked_symbol("☐")
        .checkbox_style(Style::default().fg(if on { theme.success } else { theme.text_muted }))
        .label_style(label_style)
}

/// Renders the view.
pub fn render(
    hub: &mut HubState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let cols =
        Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)]).split(area);
    let a = &hub.agents;
    let left = Layout::vertical([Constraint::Min(5), Constraint::Length(8)]).split(cols[0]);
    render_jobs(a, frame, left[0], theme, hits);
    render_history(a, frame, left[1], theme);
    let right = Layout::vertical([
        Constraint::Length(8),
        Constraint::Min(5),
        Constraint::Length(9),
    ])
    .split(cols[1]);
    render_settings(a, frame, right[0], theme);
    render_repos(a, frame, right[1], theme, hits);
    render_report(a, frame, right[2], theme);
}

fn render_jobs(
    a: &AgentsState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let title = format!(" Jobs · scheduler {} ", if a.auto { "on" } else { "off" });
    let b = block(title, theme, a.pane == Pane::Jobs);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    if a.running.is_some() && area.width > 6 {
        frame.render_widget(
            FluxSpinner::new(elapsed_ms() / 33)
                .frames(FluxFrames::CORNERS)
                .color(theme.accent),
            Rect::new(area.x + area.width - 4, area.y, 2, 1),
        );
    }
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if let Some(e) = &a.load_error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("✘ {e}"),
                Style::default().fg(theme.error),
            )),
            inner,
        );
        return;
    }
    if a.cfg.jobs.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled("No jobs yet.", muted(theme))),
                Line::from(Span::styled(
                    "n creates one: it checks the CI of the repositories you pick, on a schedule.",
                    muted(theme),
                )),
            ])
            .wrap(ratatui::widgets::Wrap { trim: true }),
            inner,
        );
        return;
    }
    let now = chrono::Local::now().naive_local();
    let per = 2u16;
    let visible = (inner.height / per).max(1) as usize;
    let first = a.row.saturating_sub(visible.saturating_sub(1));
    for (n, (i, job)) in a
        .cfg
        .jobs
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let rect = Rect::new(
            inner.x,
            inner.y + n as u16 * per,
            inner.width,
            per.min(inner.height),
        );
        let selected = i == a.row;
        let report = a.history.get(&job.name).and_then(|h| h.first());
        let bad = report.is_some_and(|r| r.needs_you());
        let unseen = report.is_some_and(|r| !r.seen && r.needs_you());
        let row_style = if selected {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else {
            Style::default().fg(theme.text_primary)
        };
        let dim = if selected { row_style } else { muted(theme) };
        let mark = if !job.enabled {
            "○"
        } else if a.running.as_deref() == Some(&job.name) {
            "◌"
        } else {
            "●"
        };
        let mark_style = if selected {
            row_style
        } else if !job.enabled {
            muted(theme)
        } else if bad {
            Style::default().fg(theme.error)
        } else {
            Style::default().fg(theme.success)
        };
        let next = job
            .parsed_schedule()
            .ok()
            .and_then(|s| s.next(now, report.map(AgentsState::started)));
        let result = report.map_or("never ran".to_string(), |r| {
            format!("{}{}", if unseen { "● " } else { "" }, r.line())
        });
        let lines = vec![
            Line::from(vec![
                Span::styled(format!(" {mark} "), mark_style),
                Span::styled(
                    format!("{:<14}", gitkraft_core::truncate_str(&job.name, 14)),
                    row_style.add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {}", job.schedule), dim),
            ]),
            Line::from(Span::styled(
                format!(
                    "   next {} · {}",
                    if job.enabled {
                        AgentsState::when(next)
                    } else {
                        "paused".into()
                    },
                    gitkraft_core::truncate_str(&result, inner.width.saturating_sub(18) as usize)
                ),
                dim,
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines).style(if selected {
                row_style
            } else {
                Style::default()
            }),
            rect,
        );
        hits.push(Hit {
            rect,
            target: Target::Row(i),
        });
    }
}

/// Findings per run of the selected job, oldest first.
fn render_history(a: &AgentsState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let b = block(" Findings per run ".into(), theme, false);
    let history = a
        .job()
        .and_then(|j| a.history.get(&j.name))
        .filter(|h| !h.is_empty());
    let Some(history) = history else {
        let inner = b.inner(area);
        frame.render_widget(b, area);
        frame.render_widget(
            Paragraph::new(Span::styled("no runs yet", muted(theme))),
            inner,
        );
        return;
    };
    let groups: Vec<BarGroup> = history
        .iter()
        .rev()
        .map(|r| BarGroup::new(String::new(), [r.findings.len() as f64]))
        .collect();
    frame.render_widget(BarChart::new(groups).block(b), area);
}

fn render_settings(a: &AgentsState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let title = a
        .job()
        .map_or(" Job ".to_string(), |j| format!(" {} ", j.name));
    let b = block(title, theme, false);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let Some(job) = a.job() else {
        return;
    };
    if inner.height < 5 {
        return;
    }
    let now = chrono::Local::now().naive_local();
    let label = |s: &str| Span::styled(format!(" {s:<10}"), muted(theme));
    let val = |s: String| Span::styled(s, Style::default().fg(theme.text_primary));
    let lines = vec![
        Line::from(vec![
            label("schedule"),
            val(format!("{}  ", job.schedule)),
            Span::styled(
                format!("next {} · s changes", AgentsState::when(a.next(now))),
                muted(theme),
            ),
        ]),
        Line::from(vec![
            label("harness"),
            val(format!(
                "{}  model {}  ",
                job.harness.name(),
                job.model.as_deref().unwrap_or("default")
            )),
            Span::styled("h harness · m model", muted(theme)),
        ]),
        Line::from(vec![
            label("projects"),
            val(job.projects.describe()),
            Span::styled("  v visibility · H host", muted(theme)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines),
        Rect::new(inner.x, inner.y, inner.width, 3),
    );
    let cells = Layout::horizontal([Constraint::Ratio(1, 5); 5]).split(Rect::new(
        inner.x,
        inner.y + 4,
        inner.width,
        1,
    ));
    let flags = [
        ("on (e)", job.enabled),
        ("inbox (i)", job.notify.inbox),
        ("system (y)", job.notify.system),
        ("forks (f)", job.projects.forks),
        ("archived (x)", job.projects.archived),
    ];
    for (cell, (text, on)) in cells.iter().zip(flags) {
        frame.render_widget(check(text.to_string(), on, theme, false), *cell);
    }
    if inner.height >= 6 {
        let hint = if job.harness.headless() {
            "findings go to the harness only on hosts that allowed it: gitkraft-tui ai-allow HOST watch"
        } else {
            "this harness opens an editor with the prompt: no answer comes back here"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(format!(" {hint}"), muted(theme))),
            Rect::new(inner.x, inner.y + 5, inner.width, 1),
        );
    }
}

fn render_repos(
    a: &AgentsState,
    frame: &mut Frame,
    area: Rect,
    theme: &UiTheme,
    hits: &mut Vec<Hit>,
) {
    let shown = a.shown_repos();
    let watched = shown.iter().filter(|(_, w)| *w).count();
    let title = match &a.repos {
        Some(r) => format!(
            " Repositories · {watched} of {} watched · space checks · o only checked · a all ",
            r.len()
        ),
        None => " Repositories ".to_string(),
    };
    let b = block(title, theme, a.pane == Pane::Repos);
    let inner = b.inner(area);
    frame.render_widget(b, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    if a.repos.is_none() {
        if a.loading_repos {
            frame.render_widget(
                SkeletonList::new(elapsed_ms())
                    .mode(AnimationMode::Sweep)
                    .items(inner.height)
                    .base(theme.border_inactive)
                    .highlight(theme.sel_bg),
                inner,
            );
        } else {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "Press R to list the repositories of every account.",
                    muted(theme),
                )),
                inner,
            );
        }
        return;
    }
    let rows = inner.height as usize;
    let first = a.repo_row.saturating_sub(rows.saturating_sub(1));
    for (n, (i, (info, on))) in shown.iter().enumerate().skip(first).take(rows).enumerate() {
        let rect = Rect::new(inner.x, inner.y + n as u16, inner.width, 1);
        let selected = a.pane == Pane::Repos && i == a.repo_row;
        let vis = if info.private { "private" } else { "public" };
        let tags = [
            Some(vis),
            info.fork.then_some("fork"),
            info.archived.then_some("archived"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
        let label = format!(
            "{}  {}",
            gitkraft_core::truncate_str(&info.repo.key(), inner.width.saturating_sub(24) as usize),
            tags
        );
        frame.render_widget(check(label, *on, theme, selected), rect);
        hits.push(Hit {
            rect,
            target: Target::Repo(i),
        });
    }
    if let Some(e) = a.repo_errors.first() {
        let y = inner.y + inner.height - 1;
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    " ✘ {}",
                    gitkraft_core::truncate_str(e, inner.width.saturating_sub(4) as usize)
                ),
                Style::default().fg(theme.error),
            ))
            .style(Style::default().bg(theme.bg)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
}

fn render_report(a: &AgentsState, frame: &mut Frame, area: Rect, theme: &UiTheme) {
    let b = block(
        " Last report · l opens it · r runs now ".into(),
        theme,
        false,
    );
    let inner = b.inner(area);
    frame.render_widget(b, area);
    let Some(r) = a.latest() else {
        frame.render_widget(
            Paragraph::new(Span::styled("never ran", muted(theme))),
            inner,
        );
        return;
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(
                " {} ",
                r.started
                    .with_timezone(&chrono::Local)
                    .format("%a %d %b %H:%M")
            ),
            muted(theme),
        ),
        Span::styled(
            r.line(),
            Style::default().fg(if r.needs_you() {
                theme.error
            } else {
                theme.success
            }),
        ),
    ])];
    for f in r
        .findings
        .iter()
        .take(inner.height.saturating_sub(2) as usize)
    {
        lines.push(Line::from(Span::styled(
            format!(
                " ✘ {}",
                gitkraft_core::truncate_str(&f.headline(), inner.width.saturating_sub(4) as usize)
            ),
            Style::default().fg(theme.text_primary),
        )));
    }
    if r.findings.len() + 1 > inner.height as usize {
        lines.push(Line::from(Span::styled(
            format!(" … {} more", r.findings.len() + 2 - inner.height as usize),
            muted(theme),
        )));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
