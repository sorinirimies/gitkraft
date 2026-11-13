//! Canvas drawings of the hub: the contribution calendar and the bar chart.

use chrono::{Datelike, Duration, NaiveDate};
use gitkraft_platforms::model::ActivityDay;
use iced::widget::canvas::{self, Path, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, mouse};

use super::state::HubMessage;

const CELL: f32 = 11.0;
const GAP: f32 = 3.0;
const LEFT: f32 = 30.0;
const TOP: f32 = 18.0;

/// Contribution level 0–4 of `count` given the busiest day (quarters of the maximum).
pub fn level(count: u32, max: u32) -> usize {
    if count == 0 || max == 0 {
        return 0;
    }
    ((u64::from(count) * 4).div_ceil(u64::from(max)) as usize).clamp(1, 4)
}

/// The 53 weeks ending `today`, as `(week, weekday, date)` cells, Sunday first.
pub fn calendar_cells(today: NaiveDate) -> Vec<(usize, usize, NaiveDate)> {
    let this_week = today - Duration::days(i64::from(today.weekday().num_days_from_sunday()));
    let first = this_week - Duration::weeks(52);
    let mut cells = Vec::with_capacity(53 * 7);
    for week in 0..53usize {
        for dow in 0..7usize {
            let date = first + Duration::days((week * 7 + dow) as i64);
            if date <= today {
                cells.push((week, dow, date));
            }
        }
    }
    cells
}

/// The day under a point of the calendar (`x`, `y` relative to the canvas), if it is a cell of the calendar.
pub fn cell_at(x: f32, y: f32, today: NaiveDate) -> Option<NaiveDate> {
    let (cx, cy) = (x - LEFT, y - TOP);
    if cx < 0.0 || cy < 0.0 {
        return None;
    }
    let (week, dow) = ((cx / (CELL + GAP)) as usize, (cy / (CELL + GAP)) as usize);
    // the gap between cells belongs to no day
    if cx - week as f32 * (CELL + GAP) > CELL || cy - dow as f32 * (CELL + GAP) > CELL {
        return None;
    }
    calendar_cells(today)
        .into_iter()
        .find(|(w, d, _)| *w == week && *d == dow)
        .map(|(_, _, date)| date)
}

/// GitHub-style calendar of contributions.
#[derive(Debug)]
pub struct Heatmap {
    /// Days with their counts.
    pub days: Vec<ActivityDay>,
    /// The last day shown.
    pub today: NaiveDate,
    /// Colours for level 0–4.
    pub palette: [Color; 5],
    /// Label colour.
    pub label: Color,
    /// The day whose details are shown, outlined.
    pub selected: Option<NaiveDate>,
    /// Outline colour of the selected day.
    pub outline: Color,
}

impl Heatmap {
    /// Pixel size the canvas needs.
    pub fn size() -> Size {
        Size::new(LEFT + 53.0 * (CELL + GAP), TOP + 7.0 * (CELL + GAP) + 4.0)
    }
}

impl canvas::Program<HubMessage> for Heatmap {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<HubMessage>> {
        if let iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let p = cursor.position_in(bounds)?;
            let date = cell_at(p.x, p.y, self.today)?;
            return Some(canvas::Action::publish(HubMessage::PickDay(date)).and_capture());
        }
        None
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match cursor.position_in(bounds) {
            Some(p) if cell_at(p.x, p.y, self.today).is_some() => mouse::Interaction::Pointer,
            _ => mouse::Interaction::default(),
        }
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let counts: std::collections::HashMap<NaiveDate, u32> =
            self.days.iter().map(|d| (d.date, d.count)).collect();
        let max = counts.values().copied().max().unwrap_or(0);
        let mut last_month = 0;
        for (week, dow, date) in calendar_cells(self.today) {
            let x = LEFT + week as f32 * (CELL + GAP);
            let y = TOP + dow as f32 * (CELL + GAP);
            let count = counts.get(&date).copied().unwrap_or(0);
            frame.fill(
                &Path::rectangle(Point::new(x, y), Size::new(CELL, CELL)),
                self.palette[level(count, max)],
            );
            if self.selected == Some(date) {
                frame.stroke(
                    &Path::rectangle(
                        Point::new(x - 1.0, y - 1.0),
                        Size::new(CELL + 2.0, CELL + 2.0),
                    ),
                    canvas::Stroke::default()
                        .with_color(self.outline)
                        .with_width(2.0),
                );
            }
            if dow == 0 && date.month() != last_month && date.day() <= 7 {
                last_month = date.month();
                frame.fill_text(Text {
                    content: [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ][(last_month - 1) as usize]
                        .into(),
                    position: Point::new(x, 2.0),
                    color: self.label,
                    size: 10.0.into(),
                    ..Text::default()
                });
            }
        }
        for (row, label) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
            frame.fill_text(Text {
                content: label.into(),
                position: Point::new(0.0, TOP + row as f32 * (CELL + GAP)),
                color: self.label,
                size: 10.0.into(),
                ..Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

/// Bars, one per value, scaled to the largest.
#[derive(Debug)]
pub struct Bars {
    /// Values, oldest first.
    pub values: Vec<f64>,
    /// Bar colour.
    pub color: Color,
}

impl canvas::Program<HubMessage> for Bars {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let max = self.values.iter().copied().fold(0.0, f64::max);
        let n = self.values.len().max(1) as f32;
        let slot = bounds.width / n;
        for (i, v) in self.values.iter().enumerate() {
            let h = if max > 0.0 {
                (*v / max) as f32 * (bounds.height - 4.0)
            } else {
                0.0
            };
            let h = if *v > 0.0 { h.max(1.0) } else { 0.0 };
            frame.fill(
                &Path::rectangle(
                    Point::new(i as f32 * slot + slot * 0.1, bounds.height - h),
                    Size::new(slot * 0.8, h),
                ),
                self.color,
            );
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_quarters_of_the_busiest_day() {
        assert_eq!([0, 1, 3, 6, 10].map(|c| level(c, 10)), [0, 1, 2, 3, 4]);
        assert_eq!(level(5, 0), 0);
        assert_eq!(level(u32::MAX, 1), 4);
    }

    #[test]
    fn the_calendar_ends_today_and_starts_on_a_sunday_52_weeks_back() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 11).expect("date"); // Wednesday
        let cells = calendar_cells(today);
        assert_eq!(cells.len(), 52 * 7 + 4);
        assert_eq!(cells.last().map(|c| c.2), Some(today));
        let first = cells[0].2;
        assert_eq!(first.weekday().num_days_from_sunday(), 0);
        assert_eq!((today - first).num_days(), 52 * 7 + 3);
        assert!(cells.iter().all(|(w, d, _)| *w < 53 && *d < 7));
    }

    #[test]
    fn a_sunday_today_gives_a_one_cell_last_week() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 8).expect("date");
        assert_eq!(calendar_cells(today).len(), 52 * 7 + 1);
    }

    #[test]
    fn the_heatmap_has_a_fixed_pixel_size() {
        let s = Heatmap::size();
        assert!(s.width > 700.0 && s.height > 100.0);
    }

    #[test]
    fn a_click_finds_the_day_under_it_and_the_gaps_and_margins_find_none() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 10).expect("date"); // a Tuesday
        let cells = calendar_cells(today);
        // every cell's own middle gives back its own date
        for (week, dow, date) in &cells {
            let x = LEFT + *week as f32 * (CELL + GAP) + CELL / 2.0;
            let y = TOP + *dow as f32 * (CELL + GAP) + CELL / 2.0;
            assert_eq!(cell_at(x, y, today), Some(*date));
        }
        // the gap between two cells, the margins, and the days after today
        assert_eq!(cell_at(LEFT + CELL + 1.0, TOP + 2.0, today), None);
        assert_eq!(cell_at(LEFT + 2.0, TOP + CELL + 1.0, today), None);
        assert_eq!(cell_at(2.0, 2.0, today), None);
        assert_eq!(cell_at(LEFT - 1.0, TOP + 2.0, today), None);
        let last = cells.last().expect("cells");
        let x = LEFT + last.0 as f32 * (CELL + GAP) + 2.0;
        let y = TOP + (last.1 + 1) as f32 * (CELL + GAP) + 2.0;
        assert_eq!(cell_at(x, y, today), None, "Wednesday has not happened");
    }
}
