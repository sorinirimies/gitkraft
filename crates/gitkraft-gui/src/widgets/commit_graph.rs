//! Reusable Canvas-based commit graph widget — GitKraken / subway-map style.
//!
//! Features:
//! - Click-and-drag to pan horizontally (for wide graphs)
//! - Left-click on a node to select the commit
//! - Right-click on a node for context menu
//! - Canvas sized to visible rows only (GPU-friendly)

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::message::Message;

/// Width of one graph lane in pixels.
pub const LANE_W: f32 = 14.0;

const NODE_RADIUS: f32 = 5.0;
const NODE_RADIUS_MAIN: f32 = 6.0;
const NODE_RING_WIDTH: f32 = 2.5;
const LINE_WIDTH: f32 = 2.0;
const LINE_WIDTH_MAIN: f32 = 3.0;

/// A commit-graph widget with built-in horizontal drag-to-pan.
pub struct CommitGraph {
    pub visible_rows: Vec<gitkraft_core::GraphRow>,
    pub offset: usize,
    pub colors: [Color; 8],
    pub row_height: f32,
    pub bg_color: Color,
    /// Total content width in pixels (max_lanes * LANE_W).
    pub content_width: f32,
    /// Per-visible-row background highlight colour (selected/range/hover),
    /// aligned 1:1 with `visible_rows`. `None` means no highlight is painted
    /// and the panel's base surface colour shows through, matching the
    /// default state of the BRANCH/TAG and COMMIT MESSAGE columns.
    pub row_backgrounds: Vec<Option<Color>>,
}

impl CommitGraph {
    pub fn view(self, column_width: f32) -> Element<'static, Message> {
        let visible_height = self.visible_rows.len() as f32 * self.row_height;
        let program = CommitGraphProgram {
            visible_rows: self.visible_rows,
            offset: self.offset,
            colors: self.colors,
            row_height: self.row_height,
            bg_color: self.bg_color,
            content_width: self.content_width,
            column_width,
            row_backgrounds: self.row_backgrounds,
        };
        iced::widget::canvas(program)
            .width(Length::Fixed(column_width))
            .height(Length::Fixed(visible_height))
            .into()
    }
}

// ── canvas internals ──────────────────────────────────────────────────────────

/// Drag state for horizontal panning, stored as the Canvas `State`.
#[derive(Default)]
pub struct GraphDragState {
    /// Current horizontal pan offset (pixels scrolled to the right).
    pan_x: f32,
    /// If dragging, the cursor X at drag start and the pan_x at that moment.
    drag: Option<(f32, f32)>,
    /// Global commit row index currently under the cursor, if any. Tracked so
    /// we only publish `HoverCommit` when it actually changes (and so the
    /// GRAPH column keeps `hovered_commit` in sync with the BRANCH/TAG and
    /// COMMIT MESSAGE columns even though the graph is one big canvas rather
    /// than a per-row widget).
    hovered_row: Option<usize>,
}

/// Maximum pan movement (in pixels) between button-press and button-release
/// still considered a "click" rather than a drag-to-pan gesture. Keeps
/// left-click-to-select reliable anywhere in the GRAPH column (not just
/// exactly on a commit node) without breaking horizontal panning.
const CLICK_DRAG_THRESHOLD: f32 = 4.0;

struct CommitGraphProgram {
    visible_rows: Vec<gitkraft_core::GraphRow>,
    offset: usize,
    colors: [Color; 8],
    row_height: f32,
    bg_color: Color,
    content_width: f32,
    column_width: f32,
    row_backgrounds: Vec<Option<Color>>,
}

#[inline]
fn lane_x(col: usize, pan: f32) -> f32 {
    col as f32 * LANE_W + LANE_W / 2.0 - pan
}

fn round_stroke(width: f32, color: Color) -> Stroke<'static> {
    Stroke {
        width,
        style: canvas::Style::Solid(color),
        line_cap: canvas::LineCap::Round,
        line_join: canvas::LineJoin::Round,
        line_dash: canvas::LineDash::default(),
    }
}

impl canvas::Program<Message> for CommitGraphProgram {
    type State = GraphDragState;

    fn update(
        &self,
        state: &mut GraphDragState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(btn)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    // Check if clicking on a node first.
                    let local_row = (pos.y / self.row_height) as usize;
                    if local_row < self.visible_rows.len() {
                        let global_row = self.offset + local_row;
                        let gr = &self.visible_rows[local_row];
                        let node_x = lane_x(gr.node_column, state.pan_x);
                        let local_mid_y =
                            local_row as f32 * self.row_height + self.row_height / 2.0;
                        let dx = pos.x - node_x;
                        let dy = pos.y - local_mid_y;
                        if (dx * dx + dy * dy).sqrt() <= NODE_RADIUS + 4.0 {
                            let msg = match btn {
                                iced::mouse::Button::Right => {
                                    Message::OpenCommitContextMenu(global_row)
                                }
                                iced::mouse::Button::Left => Message::SelectCommit(global_row),
                                _ => return None,
                            };
                            return Some(canvas::Action::publish(msg));
                        }

                        // Right-click anywhere in the row (not just on the
                        // node) opens the context menu for that commit --
                        // mirrors the BRANCH/TAG and COMMIT MESSAGE columns.
                        if *btn == iced::mouse::Button::Right {
                            return Some(canvas::Action::publish(Message::OpenCommitContextMenu(
                                global_row,
                            )));
                        }
                    }

                    // Not on a node -- start drag-to-pan (middle or left
                    // button). A plain left click (no significant movement
                    // before release) is resolved as a row selection in the
                    // `ButtonReleased` arm below, so clicking anywhere in the
                    // GRAPH column selects the commit, not just the node dot.
                    if matches!(btn, iced::mouse::Button::Left | iced::mouse::Button::Middle) {
                        state.drag = Some((pos.x, state.pan_x));
                        return Some(canvas::Action::request_redraw());
                    }
                }
            }
            canvas::Event::Mouse(iced::mouse::Event::ButtonReleased(btn)) => {
                if let Some((_, start_pan)) = state.drag.take() {
                    let moved = (state.pan_x - start_pan).abs();
                    if *btn == iced::mouse::Button::Left && moved < CLICK_DRAG_THRESHOLD {
                        if let Some(pos) = cursor.position_in(bounds) {
                            let local_row = (pos.y / self.row_height) as usize;
                            if local_row < self.visible_rows.len() {
                                let global_row = self.offset + local_row;
                                return Some(canvas::Action::publish(Message::SelectCommit(
                                    global_row,
                                )));
                            }
                        }
                    }
                    return Some(canvas::Action::request_redraw());
                }
            }
            canvas::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if let Some((start_x, start_pan)) = state.drag {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let dx = start_x - pos.x;
                        let max_pan = (self.content_width - self.column_width).max(0.0);
                        state.pan_x = (start_pan + dx).clamp(0.0, max_pan);
                        return Some(canvas::Action::request_redraw());
                    }
                }

                // Hover tracking -- without this, moving the mouse over the
                // GRAPH canvas never updates `hovered_commit`, so the row
                // highlight only ever engaged via the COMMIT MESSAGE
                // column's own mouse area. Keep all columns in sync here.
                let hovered = cursor.position_in(bounds).and_then(|pos| {
                    let local_row = (pos.y / self.row_height) as usize;
                    (local_row < self.visible_rows.len()).then(|| self.offset + local_row)
                });
                if hovered != state.hovered_row {
                    state.hovered_row = hovered;
                    return Some(canvas::Action::publish(Message::HoverCommit(hovered)));
                }
            }
            canvas::Event::Mouse(iced::mouse::Event::CursorLeft)
                if state.hovered_row.take().is_some() =>
            {
                return Some(canvas::Action::publish(Message::HoverCommit(None)));
            }
            _ => {}
        }
        None
    }

    fn mouse_interaction(
        &self,
        state: &GraphDragState,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if state.drag.is_some() {
            return iced::mouse::Interaction::Grabbing;
        }
        if let Some(pos) = cursor.position_in(bounds) {
            let local_row = (pos.y / self.row_height) as usize;
            // Any row is now clickable (selects the commit), not just the
            // node dot itself, so show a pointer cursor across the whole
            // row -- matching the BRANCH/TAG and COMMIT MESSAGE columns.
            if local_row < self.visible_rows.len() {
                return iced::mouse::Interaction::Pointer;
            }
            // Show grab cursor when hoverable (content wider than column).
            if self.content_width > self.column_width {
                return iced::mouse::Interaction::Grab;
            }
        }
        iced::mouse::Interaction::default()
    }

    fn draw(
        &self,
        state: &GraphDragState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        // NOTE: no cache — pan_x changes on drag, so we redraw.
        // The visible slice is small (~80 rows), so this is fast.
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let rh = self.row_height;
        let len = self.colors.len();
        let pan = state.pan_x;

        // ── Pass 0: row highlight backgrounds (selected/range/hover) ──────
        // Painted full-width so this column's highlight lines up with the
        // BRANCH/TAG and COMMIT MESSAGE columns, making row selection span
        // the entire table width instead of just the message column.
        for (local_idx, bg) in self.row_backgrounds.iter().enumerate() {
            if let Some(color) = bg {
                let top_y = local_idx as f32 * rh;
                frame.fill_rectangle(
                    Point::new(0.0, top_y),
                    iced::Size::new(self.column_width.max(bounds.width), rh),
                    *color,
                );
            }
        }

        // Main branch = color index 0 (first-parent chain from HEAD).
        // It gets thicker lines and larger nodes to stand out.
        let main_color_idx: usize = 0;

        // ── Pass 1: edges (draw main-branch edges LAST so they're on top) ─
        // First: non-main edges, then main edges.
        for pass in 0..2 {
            for (local_idx, gr) in self.visible_rows.iter().enumerate() {
                let top_y = local_idx as f32 * rh;
                let mid_y = top_y + rh / 2.0;
                let bot_y = top_y + rh;

                for edge in &gr.edges {
                    let is_main = edge.color_index % len == main_color_idx;
                    // Pass 0: non-main, Pass 1: main
                    if (pass == 0) == is_main {
                        continue;
                    }

                    let color = self.colors[edge.color_index % len];
                    let w = if is_main { LINE_WIDTH_MAIN } else { LINE_WIDTH };
                    let stroke = round_stroke(w, color);
                    let from_x = lane_x(edge.from_column, pan);
                    let to_x = lane_x(edge.to_column, pan);
                    let gap = if is_main {
                        NODE_RADIUS_MAIN + w / 2.0 + 1.0
                    } else {
                        NODE_RADIUS + w / 2.0 + 1.0
                    };

                    if edge.from_column == edge.to_column {
                        if edge.from_column == gr.node_column {
                            frame.stroke(
                                &Path::line(
                                    Point::new(from_x, top_y),
                                    Point::new(from_x, mid_y - gap),
                                ),
                                stroke,
                            );
                            frame.stroke(
                                &Path::line(
                                    Point::new(from_x, mid_y + gap),
                                    Point::new(from_x, bot_y),
                                ),
                                stroke,
                            );
                        } else {
                            frame.stroke(
                                &Path::line(Point::new(from_x, top_y), Point::new(from_x, bot_y)),
                                stroke,
                            );
                        }
                    } else {
                        // Merge/branch curve — wider jumps get taller curves.
                        let lane_dist = (edge.from_column as f32 - edge.to_column as f32).abs();
                        let extra_rows = (lane_dist / 2.0).ceil().min(4.0);
                        let curve_height = rh * (0.5 + extra_rows * 0.5);

                        let start = Point::new(from_x, mid_y);
                        let end = Point::new(to_x, mid_y + curve_height);
                        let path = Path::new(|b| {
                            b.move_to(start);
                            b.bezier_curve_to(
                                Point::new(from_x, mid_y + curve_height * 0.5),
                                Point::new(to_x, mid_y + curve_height * 0.3),
                                end,
                            );
                        });
                        frame.stroke(&path, stroke);
                    }
                }
            }
        }

        // ── Pass 2: nodes (main branch gets larger ring) ──────────────
        for (local_idx, gr) in self.visible_rows.iter().enumerate() {
            let mid_y = local_idx as f32 * rh + rh / 2.0;
            let node_x = lane_x(gr.node_column, pan);
            let node_color = self.colors[gr.node_color % len];
            let is_main = gr.node_color % len == main_color_idx;
            let nr = if is_main {
                NODE_RADIUS_MAIN
            } else {
                NODE_RADIUS
            };
            let mask_r = nr + LINE_WIDTH_MAIN / 2.0 + 1.5;

            frame.fill(
                &Path::circle(Point::new(node_x, mid_y), mask_r),
                self.bg_color,
            );
            frame.stroke(
                &Path::circle(Point::new(node_x, mid_y), nr - NODE_RING_WIDTH / 2.0),
                round_stroke(NODE_RING_WIDTH, node_color),
            );
            frame.fill(
                &Path::circle(Point::new(node_x, mid_y), (nr - NODE_RING_WIDTH).max(0.0)),
                self.bg_color,
            );
        }

        vec![frame.into_geometry()]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use canvas::Program;
    use iced::mouse::{Button, Cursor, Event as MouseEvent};

    const TEST_ROW_HEIGHT: f32 = 20.0;

    fn test_row() -> gitkraft_core::GraphRow {
        gitkraft_core::GraphRow {
            width: 1,
            node_column: 0,
            node_color: 0,
            edges: Vec::new(),
        }
    }

    /// Builds a program with `row_count` rows, starting at global `offset`.
    /// `content_width` lets tests opt into a horizontally-scrollable graph
    /// (needed to exercise real drag-to-pan behaviour).
    fn test_program(
        row_count: usize,
        offset: usize,
        column_width: f32,
        content_width: f32,
    ) -> CommitGraphProgram {
        CommitGraphProgram {
            visible_rows: (0..row_count).map(|_| test_row()).collect(),
            offset,
            colors: [Color::WHITE; 8],
            row_height: TEST_ROW_HEIGHT,
            bg_color: Color::BLACK,
            content_width,
            column_width,
            row_backgrounds: vec![None; row_count],
        }
    }

    fn bounds(width: f32, rows: usize) -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width,
            height: rows as f32 * TEST_ROW_HEIGHT,
        }
    }

    fn cursor_moved_at(x: f32, y: f32) -> canvas::Event {
        canvas::Event::Mouse(MouseEvent::CursorMoved {
            position: Point::new(x, y),
        })
    }

    fn published_message<M>(action: Option<canvas::Action<M>>) -> Option<M> {
        action.and_then(|a| a.into_inner().0)
    }

    // ── hover tracking ───────────────────────────────────────────────────

    #[test]
    fn hover_publishes_on_first_entry() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        let cursor = Cursor::Available(Point::new(50.0, 10.0)); // row 0

        let action = program.update(&mut state, &cursor_moved_at(50.0, 10.0), bounds, cursor);
        match published_message(action) {
            Some(Message::HoverCommit(Some(5))) => {}
            other => panic!("expected HoverCommit(Some(5)), got {other:?}"),
        }
        assert_eq!(state.hovered_row, Some(5));
    }

    #[test]
    fn hover_does_not_republish_when_unchanged() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        let cursor = Cursor::Available(Point::new(50.0, 10.0));

        let _ = program.update(&mut state, &cursor_moved_at(50.0, 10.0), bounds, cursor);
        let action = program.update(&mut state, &cursor_moved_at(51.0, 11.0), bounds, cursor);
        assert!(
            published_message(action).is_none(),
            "hovering within the same row should not republish HoverCommit"
        );
    }

    #[test]
    fn hover_updates_when_moving_to_a_different_row() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);

        let _ = program.update(
            &mut state,
            &cursor_moved_at(50.0, 10.0),
            bounds,
            Cursor::Available(Point::new(50.0, 10.0)),
        );
        assert_eq!(state.hovered_row, Some(5));

        let cursor = Cursor::Available(Point::new(50.0, 25.0)); // row 1
        let action = program.update(&mut state, &cursor_moved_at(50.0, 25.0), bounds, cursor);
        match published_message(action) {
            Some(Message::HoverCommit(Some(6))) => {}
            other => panic!("expected HoverCommit(Some(6)), got {other:?}"),
        }
        assert_eq!(state.hovered_row, Some(6));
    }

    #[test]
    fn hover_clears_when_cursor_moves_outside_bounds() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);

        let _ = program.update(
            &mut state,
            &cursor_moved_at(50.0, 10.0),
            bounds,
            Cursor::Available(Point::new(50.0, 10.0)),
        );
        assert_eq!(state.hovered_row, Some(5));

        // Cursor position well outside the canvas's own bounds (still a
        // valid `CursorMoved` event, since iced dispatches these globally).
        let cursor = Cursor::Available(Point::new(50.0, 500.0));
        let action = program.update(&mut state, &cursor_moved_at(50.0, 500.0), bounds, cursor);
        match published_message(action) {
            Some(Message::HoverCommit(None)) => {}
            other => panic!("expected HoverCommit(None), got {other:?}"),
        }
        assert_eq!(state.hovered_row, None);
    }

    #[test]
    fn cursor_left_clears_hover_when_previously_hovering() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState {
            hovered_row: Some(5),
            ..Default::default()
        };
        let bounds = bounds(100.0, 3);
        let event = canvas::Event::Mouse(MouseEvent::CursorLeft);

        let action = program.update(&mut state, &event, bounds, Cursor::Unavailable);
        match published_message(action) {
            Some(Message::HoverCommit(None)) => {}
            other => panic!("expected HoverCommit(None), got {other:?}"),
        }
        assert_eq!(state.hovered_row, None);
    }

    #[test]
    fn cursor_left_is_a_no_op_when_not_hovering() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        let event = canvas::Event::Mouse(MouseEvent::CursorLeft);

        let action = program.update(&mut state, &event, bounds, Cursor::Unavailable);
        assert!(action.is_none());
    }

    // ── click-vs-drag selection ──────────────────────────────────────────

    #[test]
    fn plain_click_off_node_selects_the_row() {
        // Column isn't wide enough to need panning, so any left click
        // anywhere in the row should be treated as a selection.
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        let pos = Point::new(90.0, 10.0); // far from the node at x=7, row 0
        let cursor = Cursor::Available(pos);

        let press = canvas::Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
        let press_action = program.update(&mut state, &press, bounds, cursor);
        // Pressing (without a node underneath) only starts a potential drag;
        // it must not select immediately.
        assert!(published_message(press_action).is_none());
        assert!(state.drag.is_some());

        let release = canvas::Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
        let release_action = program.update(&mut state, &release, bounds, cursor);
        match published_message(release_action) {
            Some(Message::SelectCommit(5)) => {}
            other => panic!("expected SelectCommit(5), got {other:?}"),
        }
        assert!(state.drag.is_none());
    }

    #[test]
    fn dragging_beyond_threshold_pans_instead_of_selecting() {
        // Wide content (200px) inside a 100px column makes panning possible.
        let program = test_program(3, 5, 100.0, 200.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);

        let press_pos = Point::new(90.0, 10.0);
        let press = canvas::Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
        let _ = program.update(&mut state, &press, bounds, Cursor::Available(press_pos));
        assert!(state.drag.is_some());

        // Drag far enough to exceed the click/drag threshold.
        let move_pos = Point::new(40.0, 10.0);
        let _ = program.update(
            &mut state,
            &cursor_moved_at(move_pos.x, move_pos.y),
            bounds,
            Cursor::Available(move_pos),
        );
        assert!(state.pan_x >= CLICK_DRAG_THRESHOLD);

        let release = canvas::Event::Mouse(MouseEvent::ButtonReleased(Button::Left));
        let release_action =
            program.update(&mut state, &release, bounds, Cursor::Available(move_pos));
        assert!(
            published_message(release_action).is_none(),
            "a real pan-drag must not also select a row"
        );
    }

    #[test]
    fn right_click_off_node_opens_context_menu_for_the_row() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        let pos = Point::new(90.0, 10.0);
        let cursor = Cursor::Available(pos);

        let press = canvas::Event::Mouse(MouseEvent::ButtonPressed(Button::Right));
        let action = program.update(&mut state, &press, bounds, cursor);
        match published_message(action) {
            Some(Message::OpenCommitContextMenu(5)) => {}
            other => panic!("expected OpenCommitContextMenu(5), got {other:?}"),
        }
    }

    #[test]
    fn click_directly_on_node_selects_immediately_without_drag_state() {
        let program = test_program(3, 5, 100.0, 100.0);
        let mut state = GraphDragState::default();
        let bounds = bounds(100.0, 3);
        // Node for row 0 sits at lane_x(0, 0.0) = 7.0.
        let pos = Point::new(7.0, 10.0);
        let cursor = Cursor::Available(pos);

        let press = canvas::Event::Mouse(MouseEvent::ButtonPressed(Button::Left));
        let action = program.update(&mut state, &press, bounds, cursor);
        match published_message(action) {
            Some(Message::SelectCommit(5)) => {}
            other => panic!("expected SelectCommit(5), got {other:?}"),
        }
        assert!(
            state.drag.is_none(),
            "a direct node click should not also start a pan drag"
        );
    }
}
