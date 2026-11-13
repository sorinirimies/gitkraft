//! Mouse drag as a small state machine, independent of the terminal.
//!
//! A press on a draggable thing starts as `Pressed`; moving two or more cells
//! turns it into `Dragging`; release then drops (or, without enough movement,
//! counts as a click). Dividers start dragging at once. `Esc` cancels.

/// What can be dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    /// A section tab, by visible index.
    Section(usize),
    /// The divider between list and detail.
    Divider,
}

/// What a release meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOutcome {
    /// Pressed and released without moving: a plain click on the target.
    Click(DragTarget),
    /// A section was dropped on another position.
    Drop {
        /// Visible index it came from.
        from: usize,
        /// Visible index it was dropped on.
        to: usize,
    },
    /// The divider was released at column `x`.
    Resized {
        /// Column of the release.
        x: u16,
    },
    /// Dropped nowhere useful, or cancelled.
    Cancelled,
    /// Nothing was going on.
    None,
}

/// Cells the pointer must move before a press becomes a drag.
const THRESHOLD: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    #[default]
    Idle,
    Pressed {
        target: DragTarget,
        x: u16,
        y: u16,
    },
    Dragging {
        target: DragTarget,
        over: Option<usize>,
        x: u16,
    },
}

/// The drag state machine.
#[derive(Debug, Clone, Copy, Default)]
pub struct Drag {
    state: State,
}

impl Drag {
    /// A mouse button went down at `(x, y)` on `target` (or on nothing).
    pub fn press(&mut self, x: u16, y: u16, target: Option<DragTarget>) {
        self.state = match target {
            Some(DragTarget::Divider) => State::Dragging {
                target: DragTarget::Divider,
                over: None,
                x,
            },
            Some(t) => State::Pressed { target: t, x, y },
            None => State::Idle,
        };
    }

    /// The pointer moved with the button held. `over` is the section tab under it, if any.
    /// Returns whether a drag is in progress.
    pub fn motion(&mut self, x: u16, y: u16, over: Option<usize>) -> bool {
        match self.state {
            State::Pressed {
                target,
                x: x0,
                y: y0,
            } => {
                if x.abs_diff(x0) >= THRESHOLD || y.abs_diff(y0) >= THRESHOLD {
                    self.state = State::Dragging { target, over, x };
                }
            }
            State::Dragging { target, .. } => self.state = State::Dragging { target, over, x },
            State::Idle => {}
        }
        self.is_dragging()
    }

    /// The button was released.
    pub fn release(&mut self) -> DragOutcome {
        let state = std::mem::take(&mut self.state);
        match state {
            State::Idle => DragOutcome::None,
            State::Pressed { target, .. } => DragOutcome::Click(target),
            State::Dragging {
                target: DragTarget::Divider,
                x,
                ..
            } => DragOutcome::Resized { x },
            State::Dragging {
                target: DragTarget::Section(from),
                over,
                ..
            } => match over {
                Some(to) if to != from => DragOutcome::Drop { from, to },
                _ => DragOutcome::Cancelled,
            },
        }
    }

    /// Abandons the drag (`Esc`).
    pub fn cancel(&mut self) -> DragOutcome {
        let was = self.is_dragging() || matches!(self.state, State::Pressed { .. });
        self.state = State::Idle;
        if was {
            DragOutcome::Cancelled
        } else {
            DragOutcome::None
        }
    }

    /// A drag is moving something.
    pub fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging { .. })
    }

    /// For drawing: the section being dragged and the tab it hovers over.
    pub fn section_preview(&self) -> Option<(usize, Option<usize>)> {
        match self.state {
            State::Dragging {
                target: DragTarget::Section(from),
                over,
                ..
            } => Some((from, over)),
            _ => None,
        }
    }

    /// For live resizing: the column of the divider while it is dragged.
    pub fn divider_x(&self) -> Option<u16> {
        match self.state {
            State::Dragging {
                target: DragTarget::Divider,
                x,
                ..
            } => Some(x),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_and_release_in_place_is_a_click() {
        let mut d = Drag::default();
        d.press(10, 1, Some(DragTarget::Section(2)));
        assert!(!d.motion(11, 1, Some(2)), "one cell is below the threshold");
        assert_eq!(d.release(), DragOutcome::Click(DragTarget::Section(2)));
        assert_eq!(d.release(), DragOutcome::None, "state resets");
    }

    #[test]
    fn dragging_a_tab_onto_another_drops_it() {
        let mut d = Drag::default();
        d.press(10, 1, Some(DragTarget::Section(0)));
        assert!(d.motion(14, 1, Some(1)));
        assert_eq!(d.section_preview(), Some((0, Some(1))));
        d.motion(30, 1, Some(3));
        assert_eq!(d.release(), DragOutcome::Drop { from: 0, to: 3 });
        assert_eq!(d.section_preview(), None);
    }

    #[test]
    fn dropping_on_itself_or_nothing_cancels() {
        let mut d = Drag::default();
        d.press(10, 1, Some(DragTarget::Section(1)));
        d.motion(20, 1, Some(1));
        assert_eq!(d.release(), DragOutcome::Cancelled);
        d.press(10, 1, Some(DragTarget::Section(1)));
        d.motion(40, 5, None);
        assert_eq!(d.release(), DragOutcome::Cancelled);
    }

    #[test]
    fn vertical_movement_also_starts_a_drag() {
        let mut d = Drag::default();
        d.press(5, 1, Some(DragTarget::Section(0)));
        assert!(d.motion(5, 3, Some(1)));
    }

    #[test]
    fn divider_drags_at_once_and_reports_its_column() {
        let mut d = Drag::default();
        d.press(30, 5, Some(DragTarget::Divider));
        assert!(d.is_dragging());
        d.motion(37, 5, None);
        assert_eq!(d.divider_x(), Some(37));
        assert_eq!(d.release(), DragOutcome::Resized { x: 37 });
        assert_eq!(d.divider_x(), None);
    }

    #[test]
    fn escape_cancels_and_a_press_on_nothing_does_nothing() {
        let mut d = Drag::default();
        d.press(1, 1, Some(DragTarget::Section(0)));
        d.motion(9, 1, Some(2));
        assert_eq!(d.cancel(), DragOutcome::Cancelled);
        assert_eq!(d.release(), DragOutcome::None);
        d.press(1, 1, None);
        assert!(!d.motion(9, 9, Some(1)));
        assert_eq!(d.release(), DragOutcome::None);
        assert_eq!(d.cancel(), DragOutcome::None);
    }
}
