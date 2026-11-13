//! Menu state and its open/close animation.

use std::time::{Duration, Instant};

use crate::anim::Tween;

/// How long the slide takes.
pub const DURATION: Duration = Duration::from_millis(220);

/// One line of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// Cycle the colour theme (System plus every preset).
    Theme,
    /// Turn animations off.
    ReducedMotion,
    /// Edit the project's AI commit message style.
    CommitStyle,
    /// Open the settings file in the editor.
    SettingsFile,
    /// The keybinding reference.
    Keys,
}

impl Row {
    /// Every row in display order.
    pub const ALL: [Self; 5] = [
        Self::Theme,
        Self::ReducedMotion,
        Self::CommitStyle,
        Self::SettingsFile,
        Self::Keys,
    ];

    /// The label on the left.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::ReducedMotion => "Reduced motion",
            Self::CommitStyle => "Commit message style",
            Self::SettingsFile => "Settings file",
            Self::Keys => "Keys",
        }
    }

    /// One line shown under the list for the selected row.
    #[must_use]
    pub fn help(self) -> &'static str {
        match self {
            Self::Theme => "←/→ change · System follows the OS light/dark setting",
            Self::ReducedMotion => "Space toggles · menus open instantly when on",
            Self::CommitStyle => {
                "Enter edits .gitkraft/ai.toml: format, scopes, template (g drafts)"
            }
            Self::SettingsFile => "Enter opens tui-settings.json in your editor",
            Self::Keys => "Enter shows every key binding",
        }
    }
}

/// Open/closed state, the cursor and the running animation.
#[derive(Debug, Clone, Default)]
pub struct Menu {
    visible: bool,
    closing: bool,
    tween: Option<Tween>,
    /// Index into [`Row::ALL`].
    pub row: usize,
}

impl Menu {
    /// Opens (animates in) unless it is open already.
    pub fn open(&mut self, now: Instant, reduced_motion: bool) {
        if self.is_open() {
            return;
        }
        self.visible = true;
        self.closing = false;
        self.row = 0;
        self.tween = Some(Tween::new(now, Self::duration(reduced_motion), false));
    }

    /// Closes (animates out). The menu stays drawn until the slide ends.
    pub fn close(&mut self, now: Instant, reduced_motion: bool) {
        if !self.is_open() {
            return;
        }
        self.closing = true;
        self.tween = Some(Tween::new(now, Self::duration(reduced_motion), true));
    }

    fn duration(reduced_motion: bool) -> Duration {
        if reduced_motion {
            Duration::ZERO
        } else {
            DURATION
        }
    }

    /// Open and taking keys (not on its way out).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.visible && !self.closing
    }

    /// Something of the menu is on screen, including a closing slide.
    #[must_use]
    pub fn is_shown(&self) -> bool {
        self.visible
    }

    /// Call every frame: hides the menu once the closing slide is done.
    pub fn tick(&mut self, now: Instant) {
        if self.closing && self.tween.is_none_or(|t| t.finished(now)) {
            self.visible = false;
            self.closing = false;
            self.tween = None;
        }
    }

    /// How far open the menu is: `0.0` closed, `1.0` fully open.
    #[must_use]
    pub fn progress(&self, now: Instant) -> f32 {
        match (self.visible, self.tween) {
            (false, _) => 0.0,
            (true, Some(t)) => t.progress(now),
            (true, None) => 1.0,
        }
    }

    /// The selected row.
    #[must_use]
    pub fn selected(&self) -> Row {
        Row::ALL[self.row.min(Row::ALL.len() - 1)]
    }

    /// Moves the cursor by `delta`, wrapping.
    pub fn step(&mut self, delta: isize) {
        let n = Row::ALL.len() as isize;
        self.row = (self.row as isize + delta).rem_euclid(n) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_slides_open_and_hides_itself_after_the_closing_slide() {
        let t0 = Instant::now();
        let mut m = Menu::default();
        assert!(!m.is_shown() && m.progress(t0) == 0.0);
        m.open(t0, false);
        assert!(m.is_open() && m.is_shown());
        assert!(m.progress(t0) < 0.05);
        assert_eq!(m.progress(t0 + DURATION * 2), 1.0);

        let t1 = t0 + DURATION * 3;
        m.close(t1, false);
        assert!(!m.is_open(), "closing stops taking keys");
        assert!(m.is_shown(), "but is still drawn while it slides");
        m.tick(t1 + DURATION / 2);
        assert!(m.is_shown());
        m.tick(t1 + DURATION * 2);
        assert!(!m.is_shown());
        assert_eq!(m.progress(t1 + DURATION * 2), 0.0);
    }

    #[test]
    fn reduced_motion_opens_and_closes_at_once() {
        let t0 = Instant::now();
        let mut m = Menu::default();
        m.open(t0, true);
        assert_eq!(m.progress(t0), 1.0);
        m.close(t0, true);
        m.tick(t0);
        assert!(!m.is_shown());
    }

    #[test]
    fn the_cursor_wraps_both_ways() {
        let mut m = Menu::default();
        m.step(-1);
        assert_eq!(m.selected(), Row::Keys);
        m.step(1);
        assert_eq!(m.selected(), Row::Theme);
        m.step(2);
        assert_eq!(m.selected(), Row::CommitStyle);
    }

    #[test]
    fn opening_twice_does_not_restart_or_reset_the_cursor() {
        let t0 = Instant::now();
        let mut m = Menu::default();
        m.open(t0, false);
        m.step(2);
        m.open(t0 + DURATION, false);
        assert_eq!(m.row, 2);
    }
}
