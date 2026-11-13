//! Keys and mouse for the hub screen.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use gitkraft_ci::RerunScope;
use gitkraft_platforms::model::{MergeMethod, MergeOptions, MergeTiming};
use ratatui::layout::Rect;

use super::drag::DragTarget;
use super::palette::{self, Palette};
use super::state::{
    Action, CommunityTab, DetailTab, Focus, HubState, Prompt, PromptPurpose, Target, View,
};
use crate::app::{App, AppScreen};

/// What the caller should do after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Stay on the hub.
    Stay,
    /// Leave the hub.
    Close,
    /// Quit the whole app.
    Quit,
}

/// Handles a key. Returns to the previous screen on `Close`.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    let outcome = match app.hub.as_mut() {
        Some(hub) => key_outcome(hub, key),
        None => Outcome::Close,
    };
    match outcome {
        Outcome::Stay => {}
        Outcome::Close => {
            app.screen = app.hub_return_screen.clone();
            app.hub = None;
        }
        Outcome::Quit => app.should_quit = true,
    }
}

/// Handles a mouse event.
pub fn handle_mouse(app: &mut App, ev: MouseEvent) {
    if let Some(hub) = app.hub.as_mut() {
        mouse(hub, ev);
    }
}

/// Key handling on the state alone, so tests need no `App`.
pub fn key_outcome(hub: &mut HubState, key: KeyEvent) -> Outcome {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Outcome::Quit;
    }
    if let Some(confirm) = hub.confirm.take() {
        // A forced merge needs a deliberate capital Y; Enter or y cannot do it by habit.
        let forced = matches!(confirm.action, Action::MergeWith(o) if o.force);
        let yes = if forced {
            key.code == KeyCode::Char('Y')
        } else {
            matches!(key.code, KeyCode::Char('y' | 'Y') | KeyCode::Enter)
        };
        if yes {
            hub.perform(confirm.action);
        }
        return Outcome::Stay;
    }
    if hub.prompt.is_some() {
        prompt_key(hub, key);
        return Outcome::Stay;
    }
    if hub.palette.is_some() {
        return palette_key(hub, key);
    }
    let ctrl_p = key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL);
    if (ctrl_p || key.code == KeyCode::Char(':')) && !hub.images.open && hub.log.is_none() {
        hub.palette = Some(Palette::default());
        return Outcome::Stay;
    }
    if hub.images.open {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q' | 'i') => hub.images.close(),
            KeyCode::Right | KeyCode::Char('l' | 'n') => {
                hub.images.step(1);
                hub.fetch_current_image();
            }
            KeyCode::Left | KeyCode::Char('h' | 'p') => {
                hub.images.step(-1);
                hub.fetch_current_image();
            }
            _ => {}
        }
        return Outcome::Stay;
    }
    if key.code == KeyCode::Esc {
        if hub.drag.is_dragging() {
            hub.drag.cancel();
            return Outcome::Stay;
        }
        if hub.log.is_some() {
            hub.log = None;
            return Outcome::Stay;
        }
        // an open day or discussion closes before the whole hub does
        if hub.view == View::Community && hub.community.detail.is_some() {
            hub.community.detail = None;
            return Outcome::Stay;
        }
        if hub.view == View::Pulse && hub.pulse.day.is_some() {
            hub.pulse.day = None;
            hub.pulse.day_data = None;
            return Outcome::Stay;
        }
        if hub.focus == Focus::Detail {
            hub.focus = Focus::List;
            return Outcome::Stay;
        }
        return Outcome::Close;
    }
    if hub.log.is_some() {
        return log_key(hub, key);
    }
    match hub.view {
        View::Pulse => pulse_key(hub, key),
        View::Keys => keys_key(hub, key),
        View::Community => community_key(hub, key),
        View::Dashboard => dashboard_key(hub, key),
    }
}

fn prompt_key(hub: &mut HubState, key: KeyEvent) {
    let Some(p) = hub.prompt.as_mut() else { return };
    match key.code {
        KeyCode::Esc => hub.prompt = None,
        KeyCode::Enter => hub.submit_prompt(),
        KeyCode::Backspace => {
            p.buffer.pop();
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => p.buffer.push(c),
        _ => {}
    }
}

fn open_prompt(hub: &mut HubState, purpose: PromptPurpose) {
    if hub.selected_pr().is_none() && hub.selected_issue().is_none() {
        hub.set_notice("select a pull request or issue first".into(), false);
        return;
    }
    if !matches!(purpose, PromptPurpose::Comment | PromptPurpose::Labels)
        && hub.selected_pr().is_none()
    {
        hub.set_notice("reviews only apply to pull requests".into(), false);
        return;
    }
    hub.prompt = Some(Prompt {
        purpose,
        buffer: String::new(),
    });
}

fn log_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    let Some(log) = hub.log.as_mut() else {
        return Outcome::Stay;
    };
    match key.code {
        KeyCode::Char('q') => hub.log = None,
        KeyCode::Down | KeyCode::Char('j') => log.move_cursor(1),
        KeyCode::Up | KeyCode::Char('k') => log.move_cursor(-1),
        KeyCode::PageDown => log.move_cursor(20),
        KeyCode::PageUp => log.move_cursor(-20),
        KeyCode::Char('g') | KeyCode::Home => log.top(),
        KeyCode::Char('G') | KeyCode::End => log.bottom(),
        KeyCode::Char(' ') | KeyCode::Enter => log.toggle_group(),
        KeyCode::Char('n') => log.next_match(),
        KeyCode::Char('N') => log.prev_match(),
        KeyCode::Char('e') => {
            if !log.jump_failure() {
                hub.set_notice("no errors found in this log".into(), true);
            }
        }
        KeyCode::Char('a') => log.show_annotations = !log.show_annotations,
        KeyCode::Char('/') => {
            hub.prompt = Some(Prompt {
                purpose: PromptPurpose::LogSearch,
                buffer: log.query.clone(),
            })
        }
        _ => {}
    }
    Outcome::Stay
}

fn pulse_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Char('q') | KeyCode::Tab | KeyCode::BackTab => hub.view = View::Dashboard,
        KeyCode::Char('r') => hub.load_pulse(hub.pulse.days.max(30)),
        KeyCode::Char('[') => hub.cycle_pulse_repo(-1),
        KeyCode::Char(']') => hub.cycle_pulse_repo(1),
        KeyCode::Char(c @ '1'..='4') => {
            hub.load_pulse([7, 30, 90, 365][(c as usize) - ('1' as usize)])
        }
        KeyCode::Char('n') => hub.toggle_notifications(false),
        KeyCode::Char('N') => hub.toggle_notifications(true),
        // the calendar: a column is a week, a row a weekday
        KeyCode::Left => hub.step_day(-7),
        KeyCode::Right => hub.step_day(7),
        KeyCode::Up => hub.step_day(-1),
        KeyCode::Down => hub.step_day(1),
        KeyCode::Char('t') => hub.pick_day(chrono::Utc::now().date_naive()),
        KeyCode::Esc if hub.pulse.day.is_some() => {
            hub.pulse.day = None;
            hub.pulse.day_data = None;
        }
        _ => {}
    }
    Outcome::Stay
}

fn palette_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    let entries = palette::entries_for(hub);
    let Some(p) = hub.palette.as_mut() else {
        return Outcome::Stay;
    };
    let shown = palette::filter_entries(&entries, &p.query);
    match key.code {
        KeyCode::Esc => hub.palette = None,
        KeyCode::Down | KeyCode::Tab => p.row = (p.row + 1).min(shown.len().saturating_sub(1)),
        KeyCode::Up | KeyCode::BackTab => p.row = p.row.saturating_sub(1),
        KeyCode::Backspace => {
            p.query.pop();
            p.row = 0;
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            p.query.push(c);
            p.row = 0;
        }
        KeyCode::Enter => {
            let pick = shown.get(p.row).and_then(|i| entries.get(*i)).cloned();
            hub.palette = None;
            if let Some(entry) = pick {
                hub.view = entry.view;
                return key_outcome(
                    hub,
                    KeyEvent::new(KeyCode::Char(entry.key), KeyModifiers::NONE),
                );
            }
        }
        _ => {}
    }
    Outcome::Stay
}

fn open_keys(hub: &mut HubState) {
    hub.view = View::Keys;
    if hub.keys.data.is_none() && !hub.keys.loading {
        hub.load_keys();
    }
}

fn open_community(hub: &mut HubState) {
    hub.view = View::Community;
    if hub.community.items.is_none() && !hub.community.loading && hub.community.error.is_none() {
        hub.load_community();
    }
}

fn community_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Char('q') | KeyCode::Tab | KeyCode::BackTab => hub.view = View::Dashboard,
        KeyCode::Esc => hub.community.detail = None,
        KeyCode::Down | KeyCode::Char('j') => hub.move_community(1),
        KeyCode::Up | KeyCode::Char('k') => hub.move_community(-1),
        KeyCode::PageDown => scroll_community(hub, 10),
        KeyCode::PageUp => scroll_community(hub, -10),
        KeyCode::Enter => hub.open_community_item(),
        KeyCode::Char('r') => hub.load_community(),
        KeyCode::Char('[') => hub.cycle_community_repo(-1),
        KeyCode::Char(']') => hub.cycle_community_repo(1),
        KeyCode::Char('d') => hub.set_community_tab(CommunityTab::Discussions),
        KeyCode::Char('w') => hub.set_community_tab(CommunityTab::Wiki),
        KeyCode::Char('c') if hub.community.detail.is_some() => {
            if hub.community.tab == CommunityTab::Discussions {
                hub.prompt = Some(Prompt {
                    purpose: PromptPurpose::DiscussionComment,
                    buffer: String::new(),
                });
            } else {
                hub.set_notice("wiki pages cannot be commented on".into(), false);
            }
        }
        _ => {}
    }
    Outcome::Stay
}

fn scroll_community(hub: &mut HubState, delta: i16) {
    if let Some(d) = hub.community.detail.as_mut() {
        d.scroll = d.scroll.saturating_add_signed(delta);
    }
}

fn keys_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    match key.code {
        KeyCode::Char('q') | KeyCode::Tab | KeyCode::BackTab => hub.view = View::Dashboard,
        KeyCode::Down | KeyCode::Char('j') => hub.move_key(1),
        KeyCode::Up | KeyCode::Char('k') => hub.move_key(-1),
        KeyCode::Char('g') | KeyCode::Home => hub.move_key(isize::MIN / 2),
        KeyCode::Char('G') | KeyCode::End => hub.move_key(isize::MAX / 2),
        KeyCode::Char('r') => hub.load_keys(),
        KeyCode::Char('d') | KeyCode::Delete => hub.ask(Action::RevokeKey),
        KeyCode::Char('s') => hub.setup_keys_host(),
        KeyCode::Char('t') => hub.test_keys_host(),
        _ => {}
    }
    Outcome::Stay
}

fn dashboard_key(hub: &mut HubState, key: KeyEvent) -> Outcome {
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        KeyCode::Char('q') => return Outcome::Close,
        KeyCode::Tab => hub.next_section(1),
        KeyCode::BackTab => hub.next_section(-1),
        KeyCode::Right if alt => move_section(hub, 1),
        KeyCode::Left if alt => move_section(hub, -1),
        KeyCode::Char('K') => open_keys(hub),
        KeyCode::Char('T') => open_community(hub),
        KeyCode::Char('w') => hub.checkout_pr_worktree(),
        KeyCode::Char('}') => move_section(hub, 1),
        KeyCode::Char('{') => move_section(hub, -1),
        KeyCode::Char(c @ '1'..='9') => hub.select_section((c as usize) - ('1' as usize)),
        KeyCode::Down | KeyCode::Char('j') => vertical(hub, 1),
        KeyCode::Up | KeyCode::Char('k') => vertical(hub, -1),
        KeyCode::PageDown => vertical(hub, 10),
        KeyCode::PageUp => vertical(hub, -10),
        KeyCode::Char('g') | KeyCode::Home => {
            if hub.focus == Focus::List {
                hub.set_row(0);
            } else {
                hub.detail.scroll = 0;
            }
        }
        KeyCode::Char('G') | KeyCode::End => {
            if hub.focus == Focus::List {
                hub.set_row(usize::MAX);
            } else {
                hub.detail.scroll = u16::MAX;
            }
        }
        KeyCode::Right | KeyCode::Char('l') => hub.focus = Focus::Detail,
        KeyCode::Left | KeyCode::Char('h') => hub.focus = Focus::List,
        KeyCode::Enter => {
            if hub.selected_run().is_some() {
                hub.open_job_log();
            } else {
                hub.focus = Focus::Detail;
            }
        }
        KeyCode::Char('v') => cycle_tab(hub),
        KeyCode::Char('r') => {
            if let Some(i) = hub.active_index() {
                hub.load_section(i);
            }
        }
        KeyCode::Char('R') => hub.load_all(),
        KeyCode::Char('p') => open_pulse(hub),
        KeyCode::Char('n') => hub.toggle_notifications(false),
        KeyCode::Char('N') => hub.toggle_notifications(true),
        KeyCode::Char('z') => hub.toggle_snooze(),
        KeyCode::Char('y') => copy_url(hub),
        KeyCode::Char('i') => hub.open_images(),
        KeyCode::Char('a') => open_prompt(hub, PromptPurpose::Approve),
        KeyCode::Char('x') => open_prompt(hub, PromptPurpose::RequestChanges),
        KeyCode::Char('c')
            if hub.focus == Focus::Detail
                && hub.selected_pr().is_some()
                && hub.detail.tab == Some(DetailTab::Diff) =>
        {
            hub.open_inline_prompt()
        }
        KeyCode::Char('c') => open_prompt(hub, PromptPurpose::Comment),
        KeyCode::Char('L') => open_prompt(hub, PromptPurpose::Labels),
        KeyCode::Char('m') => hub.ask(Action::Merge(MergeMethod::Merge)),
        KeyCode::Char('M') => hub.ask(Action::Merge(MergeMethod::Squash)),
        KeyCode::Char('W') => hub.ask(merge_with(hub, MergeTiming::WhenGreen, false)),
        KeyCode::Char('Q') => hub.ask(merge_with(hub, MergeTiming::Queue, false)),
        KeyCode::Char('!') => hub.ask(merge_with(hub, MergeTiming::Now, true)),
        KeyCode::Char('u') => hub.ask(Action::UpdateBranch),
        KeyCode::Char('Z') => hub.ask(Action::CancelAutoMerge),
        KeyCode::Char('S') => {
            hub.merge_method = hub.merge_method.next();
            hub.set_notice(
                format!(
                    "merge method for queue, auto-merge and force: {}",
                    hub.merge_method.label()
                ),
                true,
            );
        }
        KeyCode::Char('C') => hub.ask(Action::ToggleOpen),
        KeyCode::Char('f') => hub.ask(Action::Rerun(RerunScope::Failed)),
        KeyCode::Char('F') => hub.ask(Action::Rerun(RerunScope::All)),
        KeyCode::Char('X') => hub.ask(Action::CancelRun),
        KeyCode::Char('D') => hub.open_dispatch_prompt(),
        KeyCode::Char('A') => hub.download_artifacts(),
        _ => {}
    }
    Outcome::Stay
}

fn merge_with(hub: &HubState, timing: MergeTiming, force: bool) -> Action {
    Action::MergeWith(MergeOptions {
        method: hub.merge_method,
        timing,
        force,
    })
}

fn open_pulse(hub: &mut HubState) {
    hub.view = View::Pulse;
    if hub.pulse.data.is_none() && !hub.pulse.loading {
        hub.load_pulse(30);
    }
}

fn move_section(hub: &mut HubState, delta: isize) {
    let to = hub.active as isize + delta;
    if to >= 0 {
        hub.reorder_sections(hub.active, to as usize);
    }
}

fn cycle_tab(hub: &mut HubState) {
    if hub.selected_pr().is_some() {
        hub.detail.tab = Some(hub.detail.tab.unwrap_or(DetailTab::Overview).next());
        hub.detail.scroll = 0;
    }
}

fn vertical(hub: &mut HubState, delta: isize) {
    match hub.focus {
        Focus::List => hub.move_row(delta),
        Focus::Detail => {
            if hub.selected_run().is_some() {
                let n = hub.detail.run.as_ref().map_or(0, |d| d.jobs.len());
                hub.detail.job = super::list::step(hub.detail.job, n, delta);
            } else if hub.selected_pr().is_some() && hub.detail.tab == Some(DetailTab::Diff) {
                hub.move_diff_cursor(delta);
            } else {
                hub.detail.scroll = hub.detail.scroll.saturating_add_signed(delta as i16);
            }
        }
    }
}

fn copy_url(hub: &mut HubState) {
    let Some(url) = hub.selected_url() else {
        hub.set_notice("nothing to copy".into(), false);
        return;
    };
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(url.clone())) {
        Ok(()) => hub.set_notice(format!("copied {url}"), true),
        Err(e) => hub.set_notice(format!("clipboard: {e}"), false),
    }
}

// ── Mouse ────────────────────────────────────────────────────────────────────

/// The area list and detail share, from the last frame's hit regions.
fn body_area(hub: &HubState) -> Rect {
    let rect_of = |t: Target| hub.hits.iter().find(|h| h.target == t).map(|h| h.rect);
    match (rect_of(Target::ListArea), rect_of(Target::DetailArea)) {
        (Some(l), Some(d)) => Rect::new(l.x, l.y, (d.x + d.width).saturating_sub(l.x), l.height),
        _ => Rect::default(),
    }
}

/// Mouse handling on the state alone.
pub fn mouse(hub: &mut HubState, ev: MouseEvent) {
    let (x, y) = (ev.column, ev.row);
    if hub.images.open {
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => hub.images.close(),
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                hub.images.step(if ev.kind == MouseEventKind::ScrollDown {
                    1
                } else {
                    -1
                });
                hub.fetch_current_image();
            }
            _ => {}
        }
        return;
    }
    match ev.kind {
        MouseEventKind::Down(MouseButton::Left) => on_press(hub, x, y),
        MouseEventKind::Drag(MouseButton::Left) => {
            let over = match hub.hit_at(x, y) {
                Some(Target::Section(i)) => Some(i),
                _ => None,
            };
            if hub.drag.motion(x, y, over)
                && let Some(dx) = hub.drag.divider_x()
            {
                let area = body_area(hub);
                hub.resize_to(dx, area);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            let outcome = hub.drag.release();
            let area = body_area(hub);
            hub.apply_drag(outcome, area);
        }
        MouseEventKind::ScrollDown => scroll(hub, x, y, 3),
        MouseEventKind::ScrollUp => scroll(hub, x, y, -3),
        _ => {}
    }
}

fn on_press(hub: &mut HubState, x: u16, y: u16) {
    let Some(target) = hub.hit_at(x, y) else {
        return;
    };
    match target {
        Target::ViewTab(v) => {
            hub.view = v;
            match v {
                View::Pulse => open_pulse(hub),
                View::Keys => open_keys(hub),
                View::Community => open_community(hub),
                View::Dashboard => {}
            }
        }
        Target::Section(i) => hub.drag.press(x, y, Some(DragTarget::Section(i))),
        Target::Divider => hub.drag.press(x, y, Some(DragTarget::Divider)),
        Target::Row(i) if hub.view == View::Keys => hub.keys.row = i,
        Target::Row(i) if hub.view == View::Community => {
            hub.community.row = i;
            if hub.register_click(target) {
                hub.open_community_item();
            }
        }
        Target::Row(i) => {
            let double = hub.register_click(target);
            hub.focus = Focus::List;
            hub.set_row(i);
            if double {
                if hub.selected_run().is_some() {
                    hub.focus = Focus::Detail;
                } else {
                    hub.focus = Focus::Detail;
                    hub.detail.tab = Some(DetailTab::Diff);
                }
            }
        }
        Target::Bell => hub.toggle_notifications(false),
        Target::DetailTab(t) => {
            hub.detail.tab = Some(t);
            hub.detail.scroll = 0;
            hub.focus = Focus::Detail;
        }
        Target::Job(i) => {
            let double = hub.register_click(target);
            hub.focus = Focus::Detail;
            hub.detail.job = i;
            if double {
                hub.open_job_log();
            }
        }
        Target::LogRow(v) => {
            let double = hub.register_click(target);
            if let Some(log) = hub.log.as_mut() {
                log.cursor = v.min(log.visible.len().saturating_sub(1));
                if double {
                    log.toggle_group();
                }
            }
        }
        Target::Annotation(n) => {
            if let Some(log) = hub.log.as_mut() {
                log.jump_annotation(n);
            }
        }
        Target::Day(date) => hub.pick_day(date),
        Target::ListArea | Target::DetailArea => {}
    }
}

fn scroll(hub: &mut HubState, x: u16, y: u16, delta: isize) {
    if let Some(log) = hub.log.as_mut() {
        log.move_cursor(delta);
        return;
    }
    if hub.view == View::Keys {
        hub.move_key(delta);
        return;
    }
    if hub.view == View::Community {
        if hub.community.detail.is_some() {
            scroll_community(hub, delta as i16);
        } else {
            hub.move_community(delta);
        }
        return;
    }
    match hub.hit_at(x, y) {
        Some(Target::ListArea | Target::Row(_)) => hub.move_row(delta),
        Some(Target::DetailArea | Target::DetailTab(_) | Target::Job(_)) => {
            if hub.selected_run().is_some() {
                let n = hub.detail.run.as_ref().map_or(0, |d| d.jobs.len());
                hub.detail.job = super::list::step(hub.detail.job, n, delta.signum());
            } else {
                hub.detail.scroll = hub.detail.scroll.saturating_add_signed(delta as i16);
            }
        }
        _ => {}
    }
}

/// Closing helper for the caller that owns the screen state.
pub fn close(app: &mut App) {
    app.screen = app.hub_return_screen.clone();
    app.hub = None;
}

/// Opens the hub from `from`.
pub fn open(app: &mut App) {
    if app.hub.is_none() {
        let repos = app.hub_repo_remotes();
        app.hub = Some(Box::new(HubState::open(repos)));
    }
    if app.screen != AppScreen::Hub {
        app.hub_return_screen = app.screen.clone();
        app.screen = AppScreen::Hub;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::hub::state::Hit;
    use crate::features::hub::state::fixtures::state;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn ev(kind: MouseEventKind, x: u16, y: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn chars(hub: &mut HubState, text: &str) {
        for c in text.chars() {
            key_outcome(hub, key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn tab_digits_and_movement() {
        let mut s = state();
        assert_eq!(key_outcome(&mut s, key(KeyCode::Tab)), Outcome::Stay);
        assert_eq!(s.active, 1);
        chars(&mut s, "1");
        assert_eq!(s.active, 0);
        chars(&mut s, "jjj");
        assert_eq!(s.row, 3);
        chars(&mut s, "k");
        assert_eq!(s.row, 2);
        chars(&mut s, "G");
        assert_eq!(s.row, 4);
        chars(&mut s, "g");
        assert_eq!(s.row, 0);
    }

    #[test]
    fn focus_moves_between_panes_and_detail_scrolls() {
        let mut s = state();
        chars(&mut s, "l");
        assert_eq!(s.focus, Focus::Detail);
        chars(&mut s, "jj");
        assert_eq!(
            (s.row, s.detail.scroll),
            (0, 2),
            "j scrolls the detail, not the list"
        );
        chars(&mut s, "k");
        assert_eq!(s.detail.scroll, 1);
        chars(&mut s, "h");
        assert_eq!(s.focus, Focus::List);
    }

    #[test]
    fn escape_unwinds_one_level_at_a_time() {
        let mut s = state();
        chars(&mut s, "l");
        assert_eq!(key_outcome(&mut s, key(KeyCode::Esc)), Outcome::Stay);
        assert_eq!(s.focus, Focus::List);
        assert_eq!(key_outcome(&mut s, key(KeyCode::Esc)), Outcome::Close);
        assert_eq!(key_outcome(&mut s, key(KeyCode::Char('q'))), Outcome::Close);
        let ctrl_c = KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..key(KeyCode::Char('c'))
        };
        assert_eq!(key_outcome(&mut s, ctrl_c), Outcome::Quit);
    }

    #[test]
    fn a_comment_prompt_collects_text_and_escape_cancels_it() {
        let mut s = state();
        chars(&mut s, "c");
        assert!(s.prompt.is_some());
        chars(&mut s, "hi!");
        key_outcome(&mut s, key(KeyCode::Backspace));
        assert_eq!(s.prompt.as_ref().map(|p| p.buffer.as_str()), Some("hi"));
        assert_eq!(key_outcome(&mut s, key(KeyCode::Esc)), Outcome::Stay);
        assert!(s.prompt.is_none(), "Esc closed the prompt, not the hub");
        // Keys inside a prompt are text, so `q` must not close anything.
        chars(&mut s, "c");
        assert_eq!(key_outcome(&mut s, key(KeyCode::Char('q'))), Outcome::Stay);
        assert_eq!(s.prompt.as_ref().map(|p| p.buffer.as_str()), Some("q"));
    }

    #[test]
    fn reviews_need_a_pull_request_but_comments_work_on_issues() {
        let mut s = state();
        s.select_section(1);
        chars(&mut s, "a");
        assert!(s.prompt.is_none());
        chars(&mut s, "c");
        assert!(
            s.prompt.is_none(),
            "an empty issue list has nothing to comment on"
        );
    }

    #[test]
    fn destructive_actions_ask_first_and_only_y_confirms() {
        let mut s = state();
        chars(&mut s, "m");
        assert!(s.confirm.is_some());
        chars(&mut s, "n");
        assert!(s.confirm.is_none());
        chars(&mut s, "C");
        assert!(s.confirm.is_some());
        assert_eq!(
            key_outcome(&mut s, key(KeyCode::Esc)),
            Outcome::Stay,
            "Esc declines"
        );
        assert!(s.confirm.is_none());
        assert_eq!(s.focus, Focus::List);
    }

    #[test]
    fn moving_a_section_from_the_keyboard() {
        let mut s = state();
        chars(&mut s, "}");
        assert_eq!(
            (s.active, s.dashboard.sections[1].name.as_str()),
            (1, "Mine")
        );
        chars(&mut s, "{");
        assert_eq!(
            (s.active, s.dashboard.sections[0].name.as_str()),
            (0, "Mine")
        );
        chars(&mut s, "{");
        assert_eq!(s.active, 0, "already first");
        let alt_right = KeyEvent {
            modifiers: KeyModifiers::ALT,
            ..key(KeyCode::Right)
        };
        key_outcome(&mut s, alt_right);
        assert_eq!(s.active, 1);
    }

    fn with_keys(s: &mut HubState) {
        use gitkraft_hub::keys::{KeyRow, KeysData};
        use gitkraft_platforms::model::SshKey;
        let row = |host: &str, id: &str, mine| KeyRow {
            host: host.into(),
            key: SshKey {
                id: id.into(),
                title: format!("t{id}"),
                key: String::new(),
                fingerprint: None,
                created_at: None,
            },
            mine,
        };
        s.keys.data = Some(KeysData {
            hosts: vec!["github.com".into(), "gitlab.com".into()],
            rows: vec![row("github.com", "1", true), row("gitlab.com", "2", false)],
            errors: vec![],
        });
    }

    #[test]
    fn keys_view_navigation_and_revoke_confirmation() {
        let mut s = state();
        chars(&mut s, "K");
        assert_eq!(s.view, View::Keys);
        with_keys(&mut s);
        chars(&mut s, "j");
        assert_eq!(s.keys.row, 1);
        chars(&mut s, "j");
        assert_eq!(s.keys.row, 1, "clamped");
        chars(&mut s, "g");
        assert_eq!(s.keys.row, 0);
        chars(&mut s, "G");
        assert_eq!(s.keys.row, 1);
        assert_eq!(s.keys_host().as_deref(), Some("gitlab.com"));
        chars(&mut s, "k");
        chars(&mut s, "d");
        let c = s.confirm.as_ref().expect("asks first");
        assert!(
            c.text.contains("t1")
                && c.text.contains("github.com")
                && c.text.contains("this machine's key"),
            "{}",
            c.text
        );
        assert_eq!(c.action, Action::RevokeKey);
        s.confirm = None;
        assert_eq!(key_outcome(&mut s, key(KeyCode::Tab)), Outcome::Stay);
        assert_eq!(s.view, View::Dashboard);
    }

    #[test]
    fn revoking_with_no_key_selected_says_so_and_setup_without_accounts_too() {
        let mut s = state();
        s.view = View::Keys;
        chars(&mut s, "d");
        assert!(s.confirm.is_none());
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("select a key") && !ok)
        );
        chars(&mut s, "s");
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("no account") && !ok)
        );
        assert_eq!(s.keys_host(), None);
    }

    #[test]
    fn clicking_and_scrolling_in_the_keys_view_select_keys() {
        let mut s = state();
        s.view = View::Keys;
        with_keys(&mut s);
        s.set_hits(vec![
            Hit {
                rect: Rect::new(0, 3, 50, 1),
                target: Target::Row(0),
            },
            Hit {
                rect: Rect::new(0, 4, 50, 1),
                target: Target::Row(1),
            },
        ]);
        mouse(
            &mut s,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 5,
                row: 4,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(s.keys.row, 1);
        mouse(
            &mut s,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 5,
                row: 4,
                modifiers: KeyModifiers::NONE,
            },
        );
        assert_eq!(s.keys.row, 0);
    }

    #[test]
    fn c_on_the_diff_tab_comments_on_the_line_under_the_cursor() {
        let mut s = state();
        let pr = s.selected_pr().cloned().expect("pr");
        s.detail.pr = Some(gitkraft_hub::PrDetail {
            pr,
            comments: vec![],
            reviews: vec![],
            diff: "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +5 @@\n-old\n+new".into(),
            errors: vec![],
        });
        s.detail.tab = Some(DetailTab::Diff);
        s.focus = Focus::Detail;
        chars(&mut s, "c");
        assert!(s.prompt.is_none(), "cursor 0 is a header line");
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("changed or unchanged") && !ok)
        );
        for _ in 0..5 {
            chars(&mut s, "j");
        }
        assert_eq!(
            s.detail.diff_cursor, 5,
            "j moves the cursor, not the scroll"
        );
        assert_eq!(s.detail.scroll, 0);
        chars(&mut s, "c");
        assert_eq!(
            s.prompt.as_ref().map(|p| p.purpose),
            Some(PromptPurpose::Inline)
        );
        assert_eq!(
            s.inline_anchor.as_ref().map(|a| (a.path.as_str(), a.line)),
            Some(("a", 5))
        );
        for _ in 0..30 {
            chars(&mut s, "j");
        }
        assert_eq!(
            s.detail.diff_cursor, 5,
            "stops at the last line while typing"
        );
        key_outcome(&mut s, key(KeyCode::Esc));
        assert!(s.prompt.is_none());
        // A removed line is commented on at its old line number.
        s.detail.diff_cursor = 4;
        chars(&mut s, "c");
        let a = s.inline_anchor.as_ref().expect("anchor");
        assert_eq!(
            (a.path.as_str(), a.line, a.side),
            ("a", 1, gitkraft_platforms::diffpos::Side::Old)
        );
    }

    #[test]
    fn capital_l_opens_the_labels_prompt_and_bad_text_is_explained() {
        let mut s = state();
        chars(&mut s, "L");
        assert_eq!(
            s.prompt.as_ref().map(|p| p.purpose),
            Some(PromptPurpose::Labels)
        );
        key_outcome(&mut s, key(KeyCode::Enter));
        assert!(s.prompt.is_none());
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("type labels") && !ok)
        );
        chars(&mut s, "L");
        chars(&mut s, "+bug");
        key_outcome(&mut s, key(KeyCode::Esc));
        assert!(s.prompt.is_none());
    }

    #[test]
    fn merge_keys_ask_with_the_chosen_method_and_a_forced_merge_needs_a_capital_y() {
        let mut s = state();
        let asked = |s: &HubState| {
            s.confirm
                .as_ref()
                .map(|c| c.text.clone())
                .unwrap_or_default()
        };

        chars(&mut s, "W");
        assert!(asked(&s).contains("when the checks pass") && asked(&s).contains("(merge)"));
        s.confirm = None;
        chars(&mut s, "S");
        chars(&mut s, "Q");
        assert!(
            asked(&s).contains("merge queue") && asked(&s).contains("(squash)"),
            "{}",
            asked(&s)
        );
        s.confirm = None;
        chars(&mut s, "u");
        assert!(asked(&s).contains("Update this branch"));
        s.confirm = None;
        chars(&mut s, "Z");
        assert!(asked(&s).contains("leave the merge queue"));
        s.confirm = None;

        chars(&mut s, "!");
        assert!(asked(&s).contains("FORCE") && asked(&s).contains("capital"));
        // Enter and lower-case y do not confirm a forced merge
        for k in [key(KeyCode::Enter), key(KeyCode::Char('y'))] {
            key_outcome(&mut s, k);
            assert!(s.confirm.is_none());
            chars(&mut s, "!");
        }
        assert!(s.confirm.is_some());
        key_outcome(&mut s, key(KeyCode::Char('Y')));
        assert!(s.confirm.is_none());
    }

    #[test]
    fn merge_keys_do_not_apply_to_issues_and_the_palette_lists_them() {
        let mut s = state();
        assert!(
            palette::entries_for(&s)
                .iter()
                .any(|e| e.label.contains("merge queue"))
        );
        s.select_section(2);
        for k in ["W", "Q", "!", "u", "Z"] {
            chars(&mut s, k);
            assert!(s.confirm.is_none(), "{k}");
        }
    }

    #[test]
    fn the_palette_offers_an_inline_comment_only_on_the_diff() {
        let mut s = state();
        let has = |s: &HubState| {
            palette::entries_for(s)
                .iter()
                .any(|e| e.label.starts_with("Diff: comment"))
        };
        assert!(!has(&s));
        s.detail.tab = Some(DetailTab::Diff);
        s.focus = Focus::Detail;
        assert!(has(&s));
        s.focus = Focus::List;
        assert!(!has(&s));
    }

    #[test]
    fn artifacts_and_worktree_keys_explain_when_they_do_not_apply() {
        let mut s = state();
        chars(&mut s, "A");
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("select a CI run") && !ok)
        );
        chars(&mut s, "w");
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("no local checkout") && !ok),
            "{:?}",
            s.fresh_notice()
        );
    }

    #[test]
    fn dispatch_needs_a_selected_run_and_opens_a_prefilled_prompt() {
        let mut s = state();
        chars(&mut s, "D");
        assert!(s.prompt.is_none());
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("select a CI run") && !ok)
        );
        let run = gitkraft_ci::Run {
            id: 1,
            workflow: "ci.yml".into(),
            branch: "main".into(),
            ..Default::default()
        };
        s.slots[0].data = Some(gitkraft_hub::SectionData::Runs(
            gitkraft_hub::SectionResult {
                items: vec![run],
                errors: vec![],
            },
        ));
        s.active = 0;
        s.row = 0;
        chars(&mut s, "D");
        let p = s.prompt.as_ref().expect("prompt");
        assert_eq!(
            (p.purpose, p.buffer.as_str()),
            (PromptPurpose::Dispatch, "ci.yml main ")
        );
        // typing and escaping work like any prompt
        chars(&mut s, "x=1");
        assert!(s.prompt.as_ref().is_some_and(|p| p.buffer.ends_with("x=1")));
        key_outcome(&mut s, key(KeyCode::Esc));
        assert!(s.prompt.is_none());
    }

    #[test]
    fn the_palette_opens_filters_and_runs_the_entrys_key() {
        let mut s = state();
        chars(&mut s, ":");
        assert!(s.palette.is_some());
        chars(&mut s, "pulse");
        assert_eq!(s.palette.as_ref().map(|p| p.query.as_str()), Some("pulse"));
        key_outcome(&mut s, key(KeyCode::Enter));
        assert!(s.palette.is_none());
        assert_eq!(s.view, View::Pulse, "ran the pulse entry");

        // from the keys view the palette still reaches dashboard-scoped entries
        s.view = View::Keys;
        let ctrl_p = KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..key(KeyCode::Char('p'))
        };
        key_outcome(&mut s, ctrl_p);
        assert!(s.palette.is_some());
        chars(&mut s, "go to ssh");
        key_outcome(&mut s, key(KeyCode::Enter));
        assert_eq!(s.view, View::Keys);
    }

    #[test]
    fn palette_editing_and_leaving() {
        let mut s = state();
        chars(&mut s, ":");
        chars(&mut s, "zzzz");
        key_outcome(&mut s, key(KeyCode::Enter));
        assert!(
            s.palette.is_none(),
            "no match: closes without doing anything"
        );
        assert_eq!(s.view, View::Dashboard);
        chars(&mut s, ":");
        chars(&mut s, "ab");
        key_outcome(&mut s, key(KeyCode::Backspace));
        assert_eq!(s.palette.as_ref().map(|p| p.query.as_str()), Some("a"));
        key_outcome(&mut s, key(KeyCode::Down));
        key_outcome(&mut s, key(KeyCode::Down));
        assert!(s.palette.as_ref().is_some_and(|p| p.row > 0));
        key_outcome(&mut s, key(KeyCode::Up));
        key_outcome(&mut s, key(KeyCode::Esc));
        assert!(s.palette.is_none());
        assert_eq!(
            key_outcome(&mut s, key(KeyCode::Esc)),
            Outcome::Close,
            "Esc then leaves the hub as before"
        );
    }

    #[test]
    fn the_palette_offers_what_applies_to_the_selection() {
        let mut s = state();
        let labels = |s: &HubState| {
            palette::entries_for(s)
                .into_iter()
                .map(|e| e.label)
                .collect::<Vec<_>>()
        };
        assert!(
            labels(&s).iter().any(|l| l.starts_with("Pull request:")),
            "the fixture selects a PR"
        );
        assert!(!labels(&s).iter().any(|l| l.starts_with("CI run:")));
        assert!(
            labels(&s)
                .iter()
                .any(|l| l.starts_with("Go to section: Mine"))
        );
        s.slots[0].data = None;
        assert!(!labels(&s).iter().any(|l| l.starts_with("Pull request:")));
    }

    #[test]
    fn pulse_view_keys() {
        let mut s = state();
        chars(&mut s, "p");
        assert_eq!(s.view, View::Pulse);
        assert_eq!(key_outcome(&mut s, key(KeyCode::Tab)), Outcome::Stay);
        assert_eq!(s.view, View::Dashboard);
    }

    fn layout(s: &mut HubState) {
        // Three section tabs on row 1, five rows from row 3, list 0..39, divider 40, detail 41..99.
        let mut hits = vec![
            Hit {
                rect: Rect::new(0, 3, 40, 20),
                target: Target::ListArea,
            },
            Hit {
                rect: Rect::new(41, 3, 59, 20),
                target: Target::DetailArea,
            },
            Hit {
                rect: Rect::new(40, 3, 1, 20),
                target: Target::Divider,
            },
        ];
        for i in 0..3u16 {
            hits.push(Hit {
                rect: Rect::new(i * 10, 1, 9, 1),
                target: Target::Section(i as usize),
            });
        }
        for i in 0..5u16 {
            hits.push(Hit {
                rect: Rect::new(0, 3 + i, 40, 1),
                target: Target::Row(i as usize),
            });
        }
        s.set_hits(hits);
    }

    #[test]
    fn clicking_a_row_selects_it_and_a_section_tab_switches() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 5, 5));
        assert_eq!(s.row, 2);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 12, 1));
        mouse(&mut s, ev(MouseEventKind::Up(MouseButton::Left), 12, 1));
        assert_eq!(s.active, 1, "a press and release in place is a click");
    }

    #[test]
    fn dragging_a_tab_over_another_reorders_the_sections() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 2, 1));
        mouse(&mut s, ev(MouseEventKind::Drag(MouseButton::Left), 12, 1));
        assert!(s.drag.is_dragging());
        assert_eq!(s.drag.section_preview(), Some((0, Some(1))));
        mouse(&mut s, ev(MouseEventKind::Drag(MouseButton::Left), 22, 1));
        mouse(&mut s, ev(MouseEventKind::Up(MouseButton::Left), 22, 1));
        assert_eq!(
            s.dashboard
                .sections
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
            ["Issues", "CI", "Mine"]
        );
        assert_eq!(s.active, 2);
    }

    #[test]
    fn escape_during_a_drag_leaves_the_order_alone() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 2, 1));
        mouse(&mut s, ev(MouseEventKind::Drag(MouseButton::Left), 22, 1));
        assert_eq!(key_outcome(&mut s, key(KeyCode::Esc)), Outcome::Stay);
        mouse(&mut s, ev(MouseEventKind::Up(MouseButton::Left), 22, 1));
        assert_eq!(s.dashboard.sections[0].name, "Mine");
    }

    #[test]
    fn dragging_the_divider_resizes_live() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 40, 10));
        mouse(&mut s, ev(MouseEventKind::Drag(MouseButton::Left), 70, 10));
        assert_eq!(s.list_percent, 70);
        mouse(&mut s, ev(MouseEventKind::Drag(MouseButton::Left), 99, 10));
        assert_eq!(s.list_percent, 80, "clamped");
        mouse(&mut s, ev(MouseEventKind::Up(MouseButton::Left), 99, 10));
        assert!(!s.drag.is_dragging());
    }

    #[test]
    fn the_wheel_scrolls_the_pane_under_the_pointer() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::ScrollDown, 5, 5));
        assert_eq!(s.row, 3);
        mouse(&mut s, ev(MouseEventKind::ScrollUp, 5, 5));
        assert_eq!(s.row, 0);
        mouse(&mut s, ev(MouseEventKind::ScrollDown, 60, 5));
        assert_eq!((s.row, s.detail.scroll), (0, 3));
        mouse(&mut s, ev(MouseEventKind::ScrollDown, 500, 500));
        assert_eq!(s.detail.scroll, 3, "outside everything: nothing happens");
    }

    #[test]
    fn the_wheel_moves_through_an_open_log() {
        let mut s = state();
        s.log = Some(crate::features::hub::logview::LogView::new(
            "job",
            "a\nb\nc\nd\ne\nf",
        ));
        mouse(&mut s, ev(MouseEventKind::ScrollDown, 1, 1));
        assert_eq!(s.log.as_ref().map(|l| l.cursor), Some(3));
    }

    #[test]
    fn right_clicks_and_presses_on_nothing_do_nothing() {
        let mut s = state();
        layout(&mut s);
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Right), 5, 5));
        mouse(
            &mut s,
            ev(MouseEventKind::Down(MouseButton::Left), 500, 500),
        );
        mouse(&mut s, ev(MouseEventKind::Up(MouseButton::Left), 500, 500));
        assert_eq!((s.row, s.active), (0, 0));
    }

    #[test]
    fn log_keys_search_fold_and_close() {
        let mut s = state();
        s.log = Some(crate::features::hub::logview::LogView::new(
            "job",
            "start\n##[group]G\nneedle one\n##[endgroup]\nneedle two",
        ));
        chars(&mut s, "/");
        assert!(matches!(
            s.prompt.as_ref().map(|p| p.purpose),
            Some(PromptPurpose::LogSearch)
        ));
        chars(&mut s, "needle");
        key_outcome(&mut s, key(KeyCode::Enter));
        assert_eq!(s.log.as_ref().map(|l| l.matches.len()), Some(2));
        chars(&mut s, "n");
        assert_eq!(s.log.as_ref().and_then(|l| l.current_line()), Some(4));
        assert_eq!(key_outcome(&mut s, key(KeyCode::Esc)), Outcome::Stay);
        assert!(s.log.is_none(), "Esc closes the log, not the hub");
    }

    #[test]
    fn the_image_key_needs_images_and_the_overlay_has_its_own_keys() {
        let mut s = state();
        chars(&mut s, "i");
        assert!(!s.images.open);
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("no images") && !ok)
        );

        if let Some(gitkraft_hub::SectionData::Prs(r)) = s.slots[0].data.as_mut() {
            r.items[0].body = "![a](https://x/1.png) and ![b](https://x/2.png)".into();
        }
        s.set_row(1);
        s.set_row(0);
        chars(&mut s, "i");
        assert!(s.images.open);
        assert_eq!(s.images.urls.len(), 2);
        // Not connected in tests: the fetch fails cleanly instead of hanging.
        assert!(matches!(
            s.images.entry("https://x/1.png"),
            Some(crate::features::hub::images::Entry::Failed(_))
        ));
        chars(&mut s, "l");
        assert_eq!(s.images.current(), Some("https://x/2.png"));
        chars(&mut s, "q");
        assert!(!s.images.open, "q closes the overlay, not the hub");
        assert_eq!(key_outcome(&mut s, key(KeyCode::Char('q'))), Outcome::Close);
    }

    #[test]
    fn clicking_or_scrolling_in_the_overlay() {
        let mut s = state();
        s.images.open_on(vec!["a".into(), "b".into()]);
        mouse(&mut s, ev(MouseEventKind::ScrollDown, 1, 1));
        assert_eq!(s.images.current(), Some("b"));
        mouse(&mut s, ev(MouseEventKind::Down(MouseButton::Left), 1, 1));
        assert!(!s.images.open);
    }

    #[test]
    fn the_arrow_keys_walk_the_calendar_and_t_and_esc_jump_and_close() {
        let mut s = state();
        s.view = View::Pulse;
        let today = chrono::Utc::now().date_naive();
        let key = |s: &mut HubState, code: KeyCode| {
            let _ = pulse_key(s, KeyEvent::new(code, KeyModifiers::NONE));
        };
        key(&mut s, KeyCode::Up);
        assert_eq!(s.pulse.day, Some(today - chrono::Duration::days(1)));
        key(&mut s, KeyCode::Left);
        assert_eq!(s.pulse.day, Some(today - chrono::Duration::days(8)));
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Down);
        assert_eq!(s.pulse.day, Some(today), "never into the future");
        s.pulse.day = Some(today - chrono::Duration::days(370));
        key(&mut s, KeyCode::Left);
        assert_eq!(
            s.pulse.day,
            Some(today - chrono::Duration::days(370)),
            "not past the calendar's year"
        );
        key(&mut s, KeyCode::Char('t'));
        assert_eq!(s.pulse.day, Some(today));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.pulse.day, None);
        assert!(s.pulse.day_data.is_none());
    }

    #[test]
    fn t_opens_discussions_and_the_view_has_its_own_keys() {
        let mut s = state();
        let press = |s: &mut HubState, c: KeyCode| {
            let _ = key_outcome(s, KeyEvent::new(c, KeyModifiers::NONE));
        };
        press(&mut s, KeyCode::Char('T'));
        assert_eq!(s.view, View::Community);
        press(&mut s, KeyCode::Char('c'));
        assert!(s.prompt.is_none(), "nothing is open to comment on");
        s.community.detail = Some(Default::default());
        press(&mut s, KeyCode::Char('c'));
        assert_eq!(
            s.prompt.as_ref().map(|p| p.purpose),
            Some(PromptPurpose::DiscussionComment)
        );
        s.prompt = None;
        s.community.tab = CommunityTab::Wiki;
        press(&mut s, KeyCode::Char('c'));
        assert!(s.prompt.is_none(), "wiki pages take no comments");
        press(&mut s, KeyCode::Esc);
        assert!(
            s.community.detail.is_none(),
            "Esc closes the discussion first"
        );
        assert_eq!(s.view, View::Community, "and not the whole hub");
        press(&mut s, KeyCode::Char('q'));
        assert_eq!(s.view, View::Dashboard);
    }
}
