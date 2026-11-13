//! HTML fragments that are not plain markup: the contribution calendar, bar charts
//! and logs. Pure functions, so they are tested without a server. Everything that
//! came from a platform is escaped here, because these strings go into the page as is.

use chrono::{Datelike, Duration, NaiveDate};
use gitkraft_ci::log::{AnsiColor, LineKind, LogLine, Style};
use gitkraft_platforms::model::ActivityDay;
use gitkraft_settings::theme::{AppTheme, Rgb};

/// Escapes text for HTML content and attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// `#rrggbb`.
pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// The theme as CSS custom properties.
pub fn css_vars(t: &AppTheme) -> String {
    format!(
        ":root{{--bg:{};--surface:{};--border:{};--sel:{};--text:{};--text2:{};--muted:{};--accent:{};--ok:{};--warn:{};--err:{};--add:{};--del:{};--hunk:{};color-scheme:{}}}",
        hex(t.background),
        hex(t.surface),
        hex(t.border),
        hex(t.selection),
        hex(t.text_primary),
        hex(t.text_secondary),
        hex(t.text_muted),
        hex(t.accent),
        hex(t.success),
        hex(t.warning),
        hex(t.error),
        hex(t.diff_add),
        hex(t.diff_del),
        hex(t.diff_hunk),
        if t.is_dark { "dark" } else { "light" },
    )
}

const GREENS: [&str; 5] = ["#2d333b", "#0e4429", "#006d32", "#26a641", "#39d353"];

/// Contribution level 0–4 of `count` given the busiest day.
pub fn level(count: u32, max: u32) -> usize {
    if count == 0 || max == 0 {
        return 0;
    }
    // Quarters of the maximum, like GitHub's own calendar.
    (((u64::from(count) * 4).div_ceil(u64::from(max))) as usize).clamp(1, 4)
}

/// A GitHub-style contribution calendar for the 53 weeks ending `today`, as SVG.
pub fn heatmap_svg(days: &[ActivityDay], today: NaiveDate) -> String {
    const CELL: i64 = 11;
    const GAP: i64 = 3;
    const LEFT: i64 = 28;
    const TOP: i64 = 18;
    let counts: std::collections::HashMap<NaiveDate, u32> =
        days.iter().map(|d| (d.date, d.count)).collect();
    // Weeks start on Sunday; the last column holds today.
    let this_week_start = today - Duration::days(i64::from(today.weekday().num_days_from_sunday()));
    let first = this_week_start - Duration::weeks(52);
    let max = counts.values().copied().max().unwrap_or(0);
    let total: u32 = counts
        .iter()
        .filter(|(d, _)| **d >= first && **d <= today)
        .map(|(_, c)| *c)
        .sum();
    let width = LEFT + 53 * (CELL + GAP);
    let height = TOP + 7 * (CELL + GAP) + 22;
    let mut svg = format!(
        "<svg class=\"heat\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" role=\"img\" aria-label=\"{total} contributions in the last year\">"
    );
    for (row, label) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
        svg.push_str(&format!(
            "<text x=\"0\" y=\"{}\" class=\"lbl\">{label}</text>",
            TOP + row * (CELL + GAP) + CELL - 2
        ));
    }
    let mut last_month = 0;
    for week in 0..53i64 {
        let week_start = first + Duration::weeks(week);
        if week_start.month() != last_month && week_start.day() <= 7 {
            last_month = week_start.month();
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" class=\"lbl\">{}</text>",
                LEFT + week * (CELL + GAP),
                TOP - 6,
                [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec"
                ][(last_month - 1) as usize]
            ));
        }
        for dow in 0..7i64 {
            let date = week_start + Duration::days(dow);
            if date > today {
                break;
            }
            let count = counts.get(&date).copied().unwrap_or(0);
            let noun = if count == 1 {
                "contribution"
            } else {
                "contributions"
            };
            svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{CELL}\" height=\"{CELL}\" rx=\"2\" fill=\"{}\"><title>{count} {noun} on {date}</title></rect>",
                LEFT + week * (CELL + GAP),
                TOP + dow * (CELL + GAP),
                GREENS[level(count, max)]
            ));
        }
    }
    // Legend.
    let ly = TOP + 7 * (CELL + GAP) + 6;
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" class=\"lbl\">Less</text>",
        width - 150,
        ly + 9
    ));
    for (i, color) in GREENS.iter().enumerate() {
        svg.push_str(&format!("<rect x=\"{}\" y=\"{ly}\" width=\"{CELL}\" height=\"{CELL}\" rx=\"2\" fill=\"{color}\"/>", width - 120 + i as i64 * (CELL + 3)));
    }
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" class=\"lbl\">More</text></svg>",
        width - 50,
        ly + 9
    ));
    svg
}

/// Contributions in the 53 weeks ending `today`.
pub fn year_total(days: &[ActivityDay], today: NaiveDate) -> u32 {
    let first = today
        - Duration::days(i64::from(today.weekday().num_days_from_sunday()))
        - Duration::weeks(52);
    days.iter()
        .filter(|d| d.date >= first && d.date <= today)
        .map(|d| d.count)
        .sum()
}

/// A bar chart of `values` (one per day, oldest first) as SVG.
pub fn bars_svg(values: &[f64], unit: &str) -> String {
    let width = 640.0;
    let height = 120.0;
    let max = values.iter().copied().fold(0.0, f64::max);
    let n = values.len().max(1) as f64;
    let slot = width / n;
    let mut svg = format!(
        "<svg class=\"bars\" viewBox=\"0 0 {width} {}\" width=\"100%\" role=\"img\" aria-label=\"{}\">",
        height + 16.0,
        esc(unit)
    );
    for (i, v) in values.iter().enumerate() {
        let h = if max > 0.0 { (v / max) * height } else { 0.0 };
        svg.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"1\" fill=\"var(--accent)\"><title>{} {}</title></rect>",
            i as f64 * slot + slot * 0.1,
            height - h,
            slot * 0.8,
            h.max(if *v > 0.0 { 1.0 } else { 0.0 }),
            v,
            esc(unit)
        ));
    }
    svg.push_str("</svg>");
    svg
}

/// A run's timeline: one bar per job, the critical path in the accent colour.
pub fn gantt_svg(bars: &[gitkraft_ci::timing::Bar]) -> String {
    let jobs: Vec<_> = bars.iter().filter(|b| b.depth == 0).collect();
    let end = jobs
        .iter()
        .map(|b| b.start + b.length)
        .fold(0.0, f64::max)
        .max(1.0);
    let (label_w, width, row_h) = (160.0, 640.0, 20.0);
    let height = jobs.len() as f64 * row_h + 4.0;
    let mut svg = format!(
        "<svg class=\"gantt\" viewBox=\"0 0 {} {height}\" width=\"100%\" role=\"img\" aria-label=\"job timeline\">",
        label_w + width
    );
    for (i, b) in jobs.iter().enumerate() {
        let y = i as f64 * row_h + 2.0;
        let x = label_w + (b.start / end) * width;
        let w = ((b.length / end) * width).max(2.0);
        let fill = if b.critical {
            "var(--accent)"
        } else {
            "var(--muted)"
        };
        svg.push_str(&format!(
            "<text x=\"0\" y=\"{:.1}\" class=\"lbl\">{}</text><rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{:.1}\" rx=\"2\" fill=\"{fill}\"><title>{} · {}</title></rect>",
            y + 13.0,
            esc(&gitkraft_core_truncate(&b.label, 24)),
            row_h - 6.0,
            esc(&b.label),
            gitkraft_ci::timing::format_seconds(b.length)
        ));
    }
    svg.push_str("</svg>");
    svg
}

fn gitkraft_core_truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

const ANSI16: [&str; 16] = [
    "#1c1c1c", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#cccccc",
    "#666666", "#f14c4c", "#23d18b", "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#ffffff",
];

fn ansi_css(c: AnsiColor) -> String {
    match c {
        AnsiColor::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        AnsiColor::Indexed(n) if n < 16 => ANSI16[usize::from(n)].to_string(),
        AnsiColor::Indexed(n) if n >= 232 => {
            let v = 8 + (u16::from(n) - 232) * 10;
            format!("#{v:02x}{v:02x}{v:02x}")
        }
        AnsiColor::Indexed(n) => {
            let n = n - 16;
            let step = |x: u8| if x == 0 { 0 } else { 55 + 40 * u16::from(x) };
            format!(
                "#{:02x}{:02x}{:02x}",
                step(n / 36),
                step((n % 36) / 6),
                step(n % 6)
            )
        }
    }
}

fn style_attr(s: &Style) -> String {
    let mut css = String::new();
    if let Some(fg) = s.fg {
        css.push_str(&format!("color:{};", ansi_css(fg)));
    }
    if let Some(bg) = s.bg {
        css.push_str(&format!("background:{};", ansi_css(bg)));
    }
    if s.bold {
        css.push_str("font-weight:bold;");
    }
    if s.dim {
        css.push_str("opacity:.7;");
    }
    if s.underline {
        css.push_str("text-decoration:underline;");
    }
    css
}

/// A parsed log as HTML: groups become `<details>`, colours become inline styles.
/// Line numbers link to `#L<n>` anchors.
pub fn log_html(lines: &[LogLine]) -> String {
    let mut out = String::from("<pre class=\"log\">");
    let mut open = 0usize;
    for l in lines {
        match &l.kind {
            LineKind::GroupStart(name) => {
                out.push_str(&format!(
                    "<details open id=\"L{}\"><summary>{}</summary>",
                    l.number,
                    esc(name)
                ));
                open += 1;
                continue;
            }
            LineKind::GroupEnd => {
                if open > 0 {
                    out.push_str("</details>");
                    open -= 1;
                }
                continue;
            }
            _ => {}
        }
        let class = match l.kind {
            LineKind::Error => "err",
            LineKind::Warning => "warn",
            LineKind::Command => "cmd",
            _ => "",
        };
        out.push_str(&format!(
            "<span class=\"ln {class}\" id=\"L{0}\"><a href=\"#L{0}\">{0}</a>",
            l.number
        ));
        for s in &l.spans {
            let css = style_attr(&s.style);
            if css.is_empty() {
                out.push_str(&esc(&s.text));
            } else {
                out.push_str(&format!("<span style=\"{css}\">{}</span>", esc(&s.text)));
            }
        }
        out.push_str("</span>\n");
    }
    out.push_str(&"</details>".repeat(open));
    out.push_str("</pre>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_ci::log::parse_log;

    fn day(y: i32, m: u32, d: u32, count: u32) -> ActivityDay {
        ActivityDay {
            date: NaiveDate::from_ymd_opt(y, m, d).expect("date"),
            count,
        }
    }

    #[test]
    fn escaping_covers_markup_and_quotes() {
        assert_eq!(
            esc("<a href=\"x\">'&'</a>"),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
        assert_eq!(esc("plain é ✓"), "plain é ✓");
    }

    #[test]
    fn levels_are_quarters_of_the_busiest_day() {
        assert_eq!(level(0, 10), 0);
        assert_eq!(level(1, 10), 1);
        assert_eq!(level(3, 10), 2);
        assert_eq!(level(5, 10), 2);
        assert_eq!(level(6, 10), 3);
        assert_eq!(level(10, 10), 4);
        assert_eq!(level(5, 0), 0);
        assert_eq!(level(u32::MAX, 1), 4);
    }

    #[test]
    fn the_calendar_has_one_cell_per_day_up_to_today_and_a_legend() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 11).expect("date"); // a Wednesday
        let svg = heatmap_svg(
            &[
                day(2026, 3, 10, 4),
                day(2026, 3, 11, 1),
                day(2020, 1, 1, 99),
            ],
            today,
        );
        // 52 full weeks (7 each) + Sun, Mon, Tue, Wed of this week, + 5 legend squares.
        assert_eq!(svg.matches("<rect").count(), 52 * 7 + 4 + 5);
        assert!(
            svg.contains("4 contributions on 2026-03-10")
                && svg.contains("1 contribution on 2026-03-11")
        );
        assert!(
            svg.contains("5 contributions in the last year")
                || svg.contains("aria-label=\"5 contributions")
        );
        assert!(svg.contains(">Mon<") && svg.contains(">Less<") && svg.contains(">Mar<"));
        assert!(!svg.contains("2026-03-12"), "future days are not drawn");
        assert!(
            !svg.contains("99 contributions"),
            "days older than a year are ignored"
        );
    }

    #[test]
    fn an_empty_calendar_is_all_level_zero() {
        let svg = heatmap_svg(&[], NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"));
        assert_eq!(
            svg.matches(GREENS[4]).count(),
            1,
            "only the legend has the darkest green"
        );
        assert!(svg.matches(GREENS[0]).count() > 300, "every day is level 0");
        assert_eq!(
            year_total(&[], NaiveDate::from_ymd_opt(2026, 1, 1).expect("date")),
            0
        );
    }

    #[test]
    fn year_total_counts_the_window_only() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 11).expect("date");
        assert_eq!(
            year_total(
                &[
                    day(2026, 3, 1, 2),
                    day(2025, 12, 1, 3),
                    day(2024, 1, 1, 50),
                    day(2026, 3, 12, 9)
                ],
                today
            ),
            5
        );
    }

    #[test]
    fn bars_scale_to_the_maximum_and_survive_empty_input() {
        let svg = bars_svg(&[0.0, 5.0, 10.0], "PRs <b>");
        assert_eq!(svg.matches("<rect").count(), 3);
        assert!(
            svg.contains("height=\"120.0\"")
                && svg.contains("height=\"60.0\"")
                && svg.contains("height=\"0.0\"")
        );
        assert!(svg.contains("PRs &lt;b&gt;") && !svg.contains("<b>"));
        assert!(bars_svg(&[], "x").starts_with("<svg"));
        assert!(bars_svg(&[0.0, 0.0], "x").contains("height=\"0.0\""));
    }

    #[test]
    fn the_timeline_marks_the_critical_path_and_escapes_names() {
        use gitkraft_ci::timing::Bar;
        let bars = vec![
            Bar {
                label: "build <1>".into(),
                start: 0.0,
                length: 10.0,
                depth: 0,
                critical: true,
            },
            Bar {
                label: "build <1> › step".into(),
                start: 0.0,
                length: 5.0,
                depth: 1,
                critical: false,
            },
            Bar {
                label: "a very long job name that must be cut".into(),
                start: 10.0,
                length: 30.0,
                depth: 0,
                critical: false,
            },
        ];
        let svg = gantt_svg(&bars);
        assert_eq!(svg.matches("<rect").count(), 2, "steps are not drawn");
        assert!(svg.contains("var(--accent)") && svg.contains("var(--muted)"));
        assert!(svg.contains("build &lt;1&gt;") && !svg.contains("<1>"));
        assert!(svg.contains('…'));
        assert!(gantt_svg(&[]).starts_with("<svg"));
    }

    #[test]
    fn ansi_colours_map_to_css() {
        assert_eq!(ansi_css(AnsiColor::Indexed(1)), "#cd3131");
        assert_eq!(ansi_css(AnsiColor::Indexed(15)), "#ffffff");
        assert_eq!(ansi_css(AnsiColor::Indexed(16)), "#000000");
        assert_eq!(ansi_css(AnsiColor::Indexed(196)), "#ff0000");
        assert_eq!(ansi_css(AnsiColor::Indexed(232)), "#080808");
        assert_eq!(ansi_css(AnsiColor::Rgb(1, 2, 3)), "#010203");
    }

    #[test]
    fn logs_become_escaped_html_with_groups_and_colours() {
        let raw = "##[group]Build <step>\n\u{1b}[31mred <script>alert(1)</script>\u{1b}[0m\n##[error]boom & bust\n##[endgroup]\nafter";
        let html = log_html(&parse_log(raw));
        assert!(html.starts_with(
            "<pre class=\"log\"><details open id=\"L1\"><summary>Build &lt;step&gt;</summary>"
        ));
        assert!(html.contains(
            "<span style=\"color:#cd3131;\">red &lt;script&gt;alert(1)&lt;/script&gt;</span>"
        ));
        assert!(html.contains("class=\"ln err\"") && html.contains("boom &amp; bust"));
        assert!(!html.contains("<script>"));
        assert_eq!(
            html.matches("<details").count(),
            html.matches("</details>").count()
        );
        assert!(html.contains("<a href=\"#L5\">5</a>after"));
    }

    #[test]
    fn unbalanced_groups_are_closed_and_stray_ends_ignored() {
        let open = log_html(&parse_log("##[group]a\n##[group]b\nx"));
        assert_eq!(
            open.matches("<details").count(),
            open.matches("</details>").count()
        );
        let stray = log_html(&parse_log("##[endgroup]\nx"));
        assert!(!stray.contains("</details>"));
        assert_eq!(log_html(&[]), "<pre class=\"log\"></pre>");
    }

    #[test]
    fn theme_variables_are_valid_css() {
        let t = gitkraft_settings::theme::theme_by_index(0);
        let css = css_vars(&t);
        assert!(css.starts_with(":root{--bg:#") && css.ends_with('}'));
        assert!(css.contains("color-scheme:dark") || css.contains("color-scheme:light"));
        assert_eq!(hex(Rgb::new(1, 171, 255)), "#01abff");
    }
}
