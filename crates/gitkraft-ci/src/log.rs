//! Log parsing: ANSI colours, timestamps, collapsible groups and error lines,
//! turned into lines a UI can draw. Pure and fast enough for 100k-line logs.

use chrono::{DateTime, Utc};

/// A terminal colour from an SGR code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    /// 0–7 (black…white) or 8–15 (bright).
    Indexed(u8),
    /// 24-bit.
    Rgb(u8, u8, u8),
}

/// Text style of one span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Foreground.
    pub fg: Option<AnsiColor>,
    /// Background.
    pub bg: Option<AnsiColor>,
    /// Bold.
    pub bold: bool,
    /// Dim.
    pub dim: bool,
    /// Underline.
    pub underline: bool,
}

/// A run of text with one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// The text, without escape codes.
    pub text: String,
    /// Its style.
    pub style: Style,
}

/// What a line means to the log viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// Ordinary output.
    Normal,
    /// Opens a collapsible group.
    GroupStart(String),
    /// Closes the group.
    GroupEnd,
    /// Marked as an error by the CI system.
    Error,
    /// Marked as a warning.
    Warning,
    /// A command being run (`$ cargo test`, `##[command]…`).
    Command,
}

/// One log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// 1-based line number in the source log.
    pub number: usize,
    /// Timestamp, if the log has them.
    pub time: Option<DateTime<Utc>>,
    /// Plain text, no escapes, no markers.
    pub text: String,
    /// Styled spans of `text`.
    pub spans: Vec<Span>,
    /// Kind.
    pub kind: LineKind,
    /// How many groups are open around this line.
    pub depth: u16,
}

/// Parses a whole log.
pub fn parse_log(raw: &str) -> Vec<LogLine> {
    let mut out = Vec::new();
    let mut depth: u16 = 0;
    // SGR state carries over lines, as in a real terminal.
    let mut style = Style::default();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        let (time, rest) = split_timestamp(line);
        // Progress lines redraw with `\r`; keep the last frame. GitLab section markers
        // carry their own `\r`, so they are recognised before that.
        let rest = if rest.starts_with("section_") {
            rest
        } else {
            rest.rsplit('\r').next().unwrap_or(rest)
        };
        let (kind, content) = classify(rest);
        let (spans, text) = parse_ansi(content, &mut style);
        match &kind {
            LineKind::GroupStart(_) => {
                out.push(LogLine {
                    number: i + 1,
                    time,
                    text,
                    spans,
                    kind,
                    depth,
                });
                depth = depth.saturating_add(1);
            }
            LineKind::GroupEnd => {
                depth = depth.saturating_sub(1);
                out.push(LogLine {
                    number: i + 1,
                    time,
                    text,
                    spans,
                    kind,
                    depth,
                });
            }
            _ => out.push(LogLine {
                number: i + 1,
                time,
                text,
                spans,
                kind,
                depth,
            }),
        }
    }
    out
}

/// `2026-01-02T03:04:05.1234567Z rest` → (time, rest).
fn split_timestamp(line: &str) -> (Option<DateTime<Utc>>, &str) {
    let b = line.as_bytes();
    // `YYYY-MM-DDTHH:MM:SS` is 19 bytes; the `Z` follows the optional fraction.
    if b.len() > 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && let Some(end) = line.find(' ')
        && line[..end].ends_with('Z')
        && let Ok(t) = DateTime::parse_from_rfc3339(&line[..end])
    {
        return (Some(t.with_timezone(&Utc)), &line[end + 1..]);
    }
    (None, line)
}

fn classify(rest: &str) -> (LineKind, &str) {
    if let Some(name) = rest.strip_prefix("##[group]") {
        return (LineKind::GroupStart(strip_ansi(name)), name);
    }
    if rest.starts_with("##[endgroup]") {
        return (LineKind::GroupEnd, "");
    }
    if let Some(m) = rest.strip_prefix("##[error]") {
        return (LineKind::Error, m);
    }
    if let Some(m) = rest.strip_prefix("##[warning]") {
        return (LineKind::Warning, m);
    }
    if let Some(m) = rest.strip_prefix("##[command]") {
        return (LineKind::Command, m);
    }
    // GitLab: `section_start:1700000000:name[collapsed=true]\r\e[0K Title`
    if let Some(r) = rest.strip_prefix("section_start:") {
        let name = r
            .split(':')
            .nth(1)
            .unwrap_or_default()
            .split('[')
            .next()
            .unwrap_or_default();
        let title = rest.split("\u{1b}[0K").nth(1).unwrap_or(name).trim();
        return (LineKind::GroupStart(strip_ansi(title)), title);
    }
    if rest.starts_with("section_end:") {
        return (LineKind::GroupEnd, "");
    }
    if let Some(cmd) = rest.strip_prefix("$ ") {
        return (LineKind::Command, cmd);
    }
    (LineKind::Normal, rest)
}

/// Removes escape sequences.
pub fn strip_ansi(s: &str) -> String {
    let mut style = Style::default();
    parse_ansi(s, &mut style).1
}

/// Splits `s` into styled spans, updating `style` with the SGR codes seen.
fn parse_ansi(s: &str, style: &mut Style) -> (Vec<Span>, String) {
    let mut spans: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    let flush = |cur: &mut String, spans: &mut Vec<Span>, style: Style| {
        if !cur.is_empty() {
            spans.push(Span {
                text: std::mem::take(cur),
                style,
            });
        }
    };
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            cur.push(c);
            plain.push(c);
            continue;
        }
        if chars.peek() != Some(&'[') {
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut fin = None;
        for n in chars.by_ref() {
            if n.is_ascii_digit() || n == ';' || n == ':' || n == '?' {
                params.push(n);
            } else {
                fin = Some(n);
                break;
            }
        }
        if fin == Some('m') {
            flush(&mut cur, &mut spans, *style);
            apply_sgr(&params, style);
        }
    }
    flush(&mut cur, &mut spans, *style);
    (spans, plain)
}

fn apply_sgr(params: &str, style: &mut Style) {
    let nums: Vec<u32> = if params.is_empty() {
        vec![0]
    } else {
        params
            .split([';', ':'])
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let mut i = 0;
    while i < nums.len() {
        match nums[i] {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            4 => style.underline = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            24 => style.underline = false,
            n @ 30..=37 => style.fg = Some(AnsiColor::Indexed((n - 30) as u8)),
            39 => style.fg = None,
            n @ 40..=47 => style.bg = Some(AnsiColor::Indexed((n - 40) as u8)),
            49 => style.bg = None,
            n @ 90..=97 => style.fg = Some(AnsiColor::Indexed((n - 90) as u8 + 8)),
            n @ 100..=107 => style.bg = Some(AnsiColor::Indexed((n - 100) as u8 + 8)),
            n @ (38 | 48) => {
                let color = match (nums.get(i + 1), nums.get(i + 2)) {
                    (Some(5), Some(&v)) => {
                        i += 2;
                        Some(AnsiColor::Indexed(v.min(255) as u8))
                    }
                    (Some(2), Some(&r)) if i + 4 < nums.len() => {
                        let c = AnsiColor::Rgb(
                            r.min(255) as u8,
                            nums[i + 3].min(255) as u8,
                            nums[i + 4].min(255) as u8,
                        );
                        i += 4;
                        Some(c)
                    }
                    _ => None,
                };
                if n == 38 {
                    style.fg = color;
                } else {
                    style.bg = color;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Which lines are visible when some groups are collapsed. `collapsed` holds
/// the indexes of `GroupStart` lines. The group header itself stays visible.
pub fn visible_lines(
    lines: &[LogLine],
    collapsed: &std::collections::HashSet<usize>,
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut hide_until_depth: Option<u16> = None;
    for (i, l) in lines.iter().enumerate() {
        if let Some(d) = hide_until_depth {
            // Inside a collapsed group until its `GroupEnd` brings depth back to `d`.
            if l.kind == LineKind::GroupEnd && l.depth == d {
                hide_until_depth = None;
            }
            continue;
        }
        out.push(i);
        if matches!(l.kind, LineKind::GroupStart(_)) && collapsed.contains(&i) {
            hide_until_depth = Some(l.depth);
        }
    }
    out
}

/// Indexes of lines whose text contains `needle` (case-insensitive).
pub fn search(lines: &[LogLine], needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let needle = needle.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.text.to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect()
}

/// Index of the first line that is an error: a marked one, or an `error` / `FAILED` / `panicked` line.
pub fn first_failure(lines: &[LogLine]) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.kind == LineKind::Error)
        .or_else(|| {
            lines.iter().position(|l| {
                let t = l.text.as_str();
                t.starts_with("error")
                    || t.contains("FAILED")
                    || t.contains("panicked at")
                    || t.contains("Error:")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn colours_and_resets() {
        let lines = parse_log("\u{1b}[31mred\u{1b}[0m plain \u{1b}[1;32mgreen\u{1b}[0m");
        let spans = &lines[0].spans;
        assert_eq!(lines[0].text, "red plain green");
        assert_eq!(spans[0].style.fg, Some(AnsiColor::Indexed(1)));
        assert_eq!(spans[1].style, Style::default());
        assert!(spans[2].style.bold);
        assert_eq!(spans[2].style.fg, Some(AnsiColor::Indexed(2)));
    }

    #[test]
    fn extended_colours() {
        let l = &parse_log("\u{1b}[38;5;208mo\u{1b}[48;2;1;2;3mx")[0];
        assert_eq!(l.spans[0].style.fg, Some(AnsiColor::Indexed(208)));
        assert_eq!(l.spans[1].style.bg, Some(AnsiColor::Rgb(1, 2, 3)));
        assert_eq!(
            l.spans[1].style.fg,
            Some(AnsiColor::Indexed(208)),
            "fg carries over"
        );
    }

    #[test]
    fn style_carries_across_lines_until_reset() {
        let lines = parse_log("\u{1b}[33mone\ntwo\u{1b}[0m\nthree");
        assert_eq!(lines[1].spans[0].style.fg, Some(AnsiColor::Indexed(3)));
        assert_eq!(lines[2].spans[0].style, Style::default());
    }

    #[test]
    fn non_sgr_sequences_are_dropped_and_lone_escape_is_harmless() {
        assert_eq!(parse_log("a\u{1b}[2Kb\u{1b}[?25lc\u{1b}")[0].text, "abc");
        assert_eq!(strip_ansi("\u{1b}[1mbold\u{1b}[0m"), "bold");
        assert_eq!(parse_log("")[..], []);
    }

    #[test]
    fn github_timestamps_are_split_off() {
        let lines = parse_log("2026-01-02T03:04:05.1234567Z hello\n2026-01-02 not a timestamp");
        assert_eq!(lines[0].text, "hello");
        assert!(lines[0].time.is_some());
        assert_eq!(lines[1].text, "2026-01-02 not a timestamp");
        assert!(lines[1].time.is_none());
    }

    #[test]
    fn groups_nest_and_markers_classify() {
        let raw = "##[group]Build\n$ cargo build\n##[group]inner\nx\n##[endgroup]\n##[error]boom\n##[endgroup]\nafter\n##[warning]careful";
        let lines = parse_log(raw);
        assert_eq!(lines[0].kind, LineKind::GroupStart("Build".into()));
        assert_eq!(
            (lines[1].kind.clone(), lines[1].depth),
            (LineKind::Command, 1)
        );
        assert_eq!(lines[3].depth, 2);
        assert_eq!(lines[4].kind, LineKind::GroupEnd);
        assert_eq!(
            (lines[5].kind.clone(), lines[5].depth),
            (LineKind::Error, 1)
        );
        assert_eq!(lines[7].depth, 0);
        assert_eq!(lines[8].kind, LineKind::Warning);
    }

    #[test]
    fn unbalanced_endgroup_does_not_underflow() {
        let lines = parse_log("##[endgroup]\nx");
        assert_eq!(lines[1].depth, 0);
    }

    #[test]
    fn collapsing_hides_the_group_body_only() {
        let raw = "##[group]A\na1\n##[group]B\nb1\n##[endgroup]\na2\n##[endgroup]\ntail";
        let lines = parse_log(raw);
        let none = visible_lines(&lines, &HashSet::new());
        assert_eq!(none.len(), lines.len());
        let outer: HashSet<usize> = [0].into();
        assert_eq!(visible_lines(&lines, &outer), vec![0, 7]);
        let inner: HashSet<usize> = [2].into();
        assert_eq!(visible_lines(&lines, &inner), vec![0, 1, 2, 5, 6, 7]);
    }

    #[test]
    fn gitlab_sections_become_groups() {
        let raw = "section_start:1700000000:build_script\r\u{1b}[0K\u{1b}[36;1mRunning build\u{1b}[0;m\nwork\nsection_end:1700000009:build_script\r\u{1b}[0K";
        let lines = parse_log(raw);
        assert_eq!(lines[0].kind, LineKind::GroupStart("Running build".into()));
        assert_eq!(lines[1].depth, 1);
        assert_eq!(lines[2].kind, LineKind::GroupEnd);
    }

    #[test]
    fn search_and_first_failure() {
        let lines = parse_log(
            "ok\nWarning: x\ntest a ... FAILED\n##[error]Process completed with exit code 1",
        );
        assert_eq!(search(&lines, "warning"), vec![1]);
        assert!(search(&lines, "").is_empty());
        assert_eq!(first_failure(&lines), Some(3), "marked errors win");
        assert_eq!(first_failure(&parse_log("a\nb FAILED")), Some(1));
        assert_eq!(first_failure(&parse_log("all good")), None);
    }

    #[test]
    fn handles_a_very_long_log() {
        let raw = "line\n".repeat(200_000);
        assert_eq!(parse_log(&raw).len(), 200_000);
    }
}
