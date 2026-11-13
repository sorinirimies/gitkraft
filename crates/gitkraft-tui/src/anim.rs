//! A tiny time-based tween for UI animation.
//!
//! The event loop redraws about 30 times a second, so a view asks the tween for its progress at the
//! current time instead of counting frames; the animation takes the same time on a slow terminal.
//! Time is passed in, so tests need no sleeping.

use std::time::{Duration, Instant};

/// Cubic ease-out: fast start, gentle landing. `t` is clamped to `0..=1`.
#[must_use]
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Linear interpolation from `a` to `b`.
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// An eased 0→1 (opening) or 1→0 (closing) progress over a duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tween {
    started: Instant,
    duration: Duration,
    closing: bool,
}

impl Tween {
    /// Starts a tween at `now`. A zero duration is finished at once (reduced motion).
    #[must_use]
    pub fn new(now: Instant, duration: Duration, closing: bool) -> Self {
        Self {
            started: now,
            duration,
            closing,
        }
    }

    fn linear(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let spent = now.saturating_duration_since(self.started);
        (spent.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Eased progress at `now`: `0.0` fully closed, `1.0` fully open.
    #[must_use]
    pub fn progress(&self, now: Instant) -> f32 {
        let eased = ease_out_cubic(self.linear(now));
        if self.closing { 1.0 - eased } else { eased }
    }

    /// Whether the animation has reached its end at `now`.
    #[must_use]
    pub fn finished(&self, now: Instant) -> bool {
        self.linear(now) >= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_starts_at_zero_ends_at_one_and_never_goes_backwards() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_out_cubic(7.0), 1.0, "clamped");
        let mut last = 0.0;
        for i in 0..=20 {
            let v = ease_out_cubic(i as f32 / 20.0);
            assert!(v >= last);
            last = v;
        }
        assert!(ease_out_cubic(0.5) > 0.5, "ease-out is ahead of linear");
        assert_eq!(lerp(2.0, 4.0, 0.5), 3.0);
    }

    #[test]
    fn opening_rises_and_closing_falls_with_time() {
        let t0 = Instant::now();
        let open = Tween::new(t0, Duration::from_millis(200), false);
        assert_eq!(open.progress(t0), 0.0);
        let mid = open.progress(t0 + Duration::from_millis(100));
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(open.progress(t0 + Duration::from_millis(400)), 1.0);
        assert!(!open.finished(t0 + Duration::from_millis(199)));
        assert!(open.finished(t0 + Duration::from_millis(200)));

        let close = Tween::new(t0, Duration::from_millis(200), true);
        assert_eq!(close.progress(t0), 1.0);
        assert_eq!(close.progress(t0 + Duration::from_millis(400)), 0.0);
    }

    #[test]
    fn a_zero_duration_is_instant() {
        let t0 = Instant::now();
        assert_eq!(Tween::new(t0, Duration::ZERO, false).progress(t0), 1.0);
        assert!(Tween::new(t0, Duration::ZERO, true).finished(t0));
        assert_eq!(Tween::new(t0, Duration::ZERO, true).progress(t0), 0.0);
    }
}
