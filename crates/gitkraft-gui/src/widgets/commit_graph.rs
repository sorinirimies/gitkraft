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
}

struct CommitGraphProgram {
    visible_rows: Vec<gitkraft_core::GraphRow>,
    offset: usize,
    colors: [Color; 8],
    row_height: f32,
    bg_color: Color,
    content_width: f32,
    column_width: f32,
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
                    }

                    // Not on a node — start drag-to-pan (middle or left button).
                    if matches!(btn, iced::mouse::Button::Left | iced::mouse::Button::Middle) {
                        state.drag = Some((pos.x, state.pan_x));
                        return Some(canvas::Action::request_redraw());
                    }
                }
            }
            canvas::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left | iced::mouse::Button::Middle,
            )) if state.drag.is_some() => {
                state.drag = None;
                return Some(canvas::Action::request_redraw());
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
            if local_row < self.visible_rows.len() {
                let gr = &self.visible_rows[local_row];
                let node_x = lane_x(gr.node_column, state.pan_x);
                let local_mid_y = local_row as f32 * self.row_height + self.row_height / 2.0;
                let dx = pos.x - node_x;
                let dy = pos.y - local_mid_y;
                if (dx * dx + dy * dy).sqrt() <= NODE_RADIUS + 4.0 {
                    return iced::mouse::Interaction::Pointer;
                }
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
