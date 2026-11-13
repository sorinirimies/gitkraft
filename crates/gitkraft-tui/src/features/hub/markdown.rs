//! Markdown to styled terminal lines: headings, lists, task lists, quotes, code,
//! links, images (as a marker) and the inline styles. Enough for PR and issue
//! bodies; it never fails, whatever the text.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Colours the renderer uses.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Body text.
    pub text: Color,
    /// Headings and bullets.
    pub accent: Color,
    /// Quotes, rules, comments.
    pub muted: Color,
    /// Code.
    pub code: Color,
    /// Links and image markers.
    pub link: Color,
}

/// Renders `md` to lines (no wrapping: the caller wraps).
pub fn render(md: &str, p: &Palette) -> Vec<Line<'static>> {
    let md = strip_html_comments(md);
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut in_code = false;
    let base = Style::default().fg(p.text);
    for raw in md.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            let lang = trimmed.trim_start_matches(['`', '~']).trim();
            if in_code && !lang.is_empty() {
                out.push(Line::from(Span::styled(
                    format!("  ┌ {lang}"),
                    Style::default().fg(p.muted),
                )));
            }
            continue;
        }
        if in_code {
            out.push(Line::from(Span::styled(
                format!("  {line}"),
                Style::default().fg(p.code),
            )));
            continue;
        }
        if trimmed.is_empty() {
            out.push(Line::from(""));
            continue;
        }
        if is_rule(trimmed) {
            out.push(Line::from(Span::styled(
                "────────────────────",
                Style::default().fg(p.muted),
            )));
            continue;
        }
        if let Some(level) = heading_level(trimmed) {
            let text = trimmed[level..].trim().trim_end_matches('#').trim();
            let mut st = Style::default().fg(p.accent).add_modifier(Modifier::BOLD);
            if level == 1 {
                st = st.add_modifier(Modifier::UNDERLINED);
            }
            out.push(Line::from(inline(text, st, p)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('>') {
            let mut spans = vec![Span::styled("▎ ", Style::default().fg(p.muted))];
            spans.extend(inline(rest.trim_start(), Style::default().fg(p.muted), p));
            out.push(Line::from(spans));
            continue;
        }
        if trimmed.starts_with('|') {
            out.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(p.muted),
            )));
            continue;
        }
        let indent = line.len() - trimmed.len();
        if let Some((marker, rest)) = list_item(trimmed) {
            let (box_, rest) = task_box(rest);
            let mut spans = vec![
                Span::raw(" ".repeat(indent)),
                Span::styled(format!("{marker} "), Style::default().fg(p.accent)),
            ];
            if let Some(b) = box_ {
                spans.push(Span::styled(format!("{b} "), Style::default().fg(p.accent)));
            }
            spans.extend(inline(rest, base, p));
            out.push(Line::from(spans));
            continue;
        }
        out.push(Line::from(inline(trimmed, base, p)));
    }
    out
}

fn strip_html_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn is_rule(t: &str) -> bool {
    let c: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    c.len() >= 3
        && (c.chars().all(|c| c == '-')
            || c.chars().all(|c| c == '*')
            || c.chars().all(|c| c == '_'))
}

fn heading_level(t: &str) -> Option<usize> {
    let n = t.chars().take_while(|c| *c == '#').count();
    (1..=6)
        .contains(&n)
        .then_some(n)
        .filter(|n| t[*n..].starts_with(' '))
}

fn list_item(t: &str) -> Option<(&'static str, &str)> {
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(m) {
            return Some(("•", r));
        }
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && digits < 4 && (t[digits..].starts_with(". ") || t[digits..].starts_with(") "))
    {
        return Some(("›", &t[digits + 2..]));
    }
    None
}

fn task_box(rest: &str) -> (Option<&'static str>, &str) {
    if let Some(r) = rest.strip_prefix("[ ] ") {
        (Some("☐"), r)
    } else if let Some(r) = rest
        .strip_prefix("[x] ")
        .or_else(|| rest.strip_prefix("[X] "))
    {
        (Some("☑"), r)
    } else {
        (None, rest)
    }
}

/// Inline styles: `code`, **bold**, *italic*, _italic_, ~~strike~~, [text](url), ![alt](url), <tags>.
fn inline(text: &str, base: Style, p: &Palette) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut style = base;
    let mut i = 0;
    let flush = |buf: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(buf), style));
        }
    };
    let find = |from: usize, pat: &[char]| -> Option<usize> {
        (from..chars.len().saturating_sub(pat.len() - 1)).find(|&j| chars[j..].starts_with(pat))
    };
    while i < chars.len() {
        let c = chars[i];
        match c {
            '`' => {
                if let Some(end) = find(i + 1, &['`']) {
                    flush(&mut buf, &mut spans, style);
                    spans.push(Span::styled(
                        chars[i + 1..end].iter().collect::<String>(),
                        Style::default().fg(p.code),
                    ));
                    i = end + 1;
                    continue;
                }
            }
            '*' | '_' | '~'
                if chars.get(i + 1) == Some(&c) && c != '_'
                    || (c == '~' && chars.get(i + 1) == Some(&'~')) =>
            {
                // ** or ~~
                let m = if c == '*' {
                    Modifier::BOLD
                } else {
                    Modifier::CROSSED_OUT
                };
                flush(&mut buf, &mut spans, style);
                style = if style.add_modifier.contains(m) {
                    style.remove_modifier(m)
                } else {
                    style.add_modifier(m)
                };
                i += 2;
                continue;
            }
            '*' | '_' => {
                // Italic only at word edges, so snake_case and 2*3 stay intact.
                let prev_alnum = i > 0 && chars[i - 1].is_alphanumeric();
                let next_alnum = chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
                let opening = !prev_alnum && next_alnum;
                let closing = prev_alnum && !next_alnum;
                if opening || closing {
                    flush(&mut buf, &mut spans, style);
                    style = if style.add_modifier.contains(Modifier::ITALIC) {
                        style.remove_modifier(Modifier::ITALIC)
                    } else {
                        style.add_modifier(Modifier::ITALIC)
                    };
                    i += 1;
                    continue;
                }
            }
            '!' | '[' => {
                let start = if c == '!' { i + 1 } else { i };
                if chars.get(start) == Some(&'[')
                    && let Some(mid) = find(start + 1, &[']', '('])
                    && let Some(end) = find(mid + 2, &[')'])
                {
                    flush(&mut buf, &mut spans, style);
                    let label: String = chars[start + 1..mid].iter().collect();
                    if c == '!' {
                        let alt = if label.is_empty() {
                            "image".to_string()
                        } else {
                            label
                        };
                        spans.push(Span::styled(
                            format!("🖼 {alt}"),
                            Style::default().fg(p.link),
                        ));
                    } else {
                        spans.push(Span::styled(
                            label,
                            Style::default()
                                .fg(p.link)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                    }
                    i = end + 1;
                    continue;
                }
            }
            '<' => {
                if let Some(end) = find(i + 1, &['>'])
                    && chars
                        .get(i + 1)
                        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '/')
                {
                    let tag: String = chars[i + 1..end].iter().collect::<String>().to_lowercase();
                    flush(&mut buf, &mut spans, style);
                    if tag.starts_with("img") {
                        spans.push(Span::styled("🖼 image", Style::default().fg(p.link)));
                    } else if tag.starts_with("br") {
                        spans.push(Span::raw(" "));
                    }
                    i = end + 1;
                    continue;
                }
            }
            _ => {}
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &mut spans, style);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: Palette = Palette {
        text: Color::White,
        accent: Color::Cyan,
        muted: Color::DarkGray,
        code: Color::Yellow,
        link: Color::Blue,
    };

    fn text(lines: &[Line]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn headings_lists_tasks_quotes_and_rules() {
        let md = "# Title\n## Sub\nplain\n- one\n* two\n1. first\n- [ ] todo\n- [x] done\n> quoted\n---\n####### not a heading";
        assert_eq!(
            text(&render(md, &P)),
            [
                "Title",
                "Sub",
                "plain",
                "• one",
                "• two",
                "› first",
                "• ☐ todo",
                "• ☑ done",
                "▎ quoted",
                "────────────────────",
                "####### not a heading"
            ]
        );
    }

    #[test]
    fn code_blocks_are_verbatim_and_inline_markers_inside_do_not_apply() {
        let lines = render("```rust\nlet a = **b**;\n```\nafter", &P);
        assert_eq!(text(&lines), ["  ┌ rust", "  let a = **b**;", "after"]);
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Yellow));
    }

    #[test]
    fn inline_styles_apply_and_toggle_off() {
        let lines = render("a **bold** and `code` and ~~gone~~ and *it*", &P);
        assert_eq!(text(&lines), ["a bold and code and gone and it"]);
        let spans = &lines[0].spans;
        let by = |t: &str| spans.iter().find(|s| s.content == t).expect("span");
        assert!(by("bold").style.add_modifier.contains(Modifier::BOLD));
        assert!(!by(" and ").style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(by("code").style.fg, Some(Color::Yellow));
        assert!(
            by("gone")
                .style
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
        assert!(by("it").style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn snake_case_and_arithmetic_are_left_alone() {
        assert_eq!(
            text(&render("use snake_case_names and 2*3*4 and a_b", &P)),
            ["use snake_case_names and 2*3*4 and a_b"]
        );
    }

    #[test]
    fn links_and_images_become_readable_markers() {
        let t = text(&render(
            "see [the docs](https://x) ![shot](https://x/a.png) <img src=\"https://x/b.png\"> end",
            &P,
        ));
        assert_eq!(t, ["see the docs 🖼 shot 🖼 image end"]);
        assert_eq!(text(&render("![](https://x/a.png)", &P)), ["🖼 image"]);
    }

    #[test]
    fn html_comments_and_tags_vanish() {
        assert_eq!(
            text(&render(
                "a <!-- hidden\nacross lines --> b\n<details><summary>More</summary>text</details>",
                &P
            )),
            ["a  b", "Moretext"]
        );
        assert_eq!(
            text(&render("unterminated <!-- comment", &P)),
            ["unterminated"]
        );
    }

    #[test]
    fn broken_input_never_panics() {
        for md in [
            "[", "![", "[a](", "`", "**", "~~", "<", "<a", "*", "_", "# ", "- ", "1.", "|", "```",
            "é**é", "[é](é",
        ] {
            let _ = render(md, &P);
        }
    }

    #[test]
    fn tables_stay_raw_and_dim() {
        let lines = render("| a | b |\n|---|---|", &P);
        assert_eq!(text(&lines)[0], "| a | b |");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::DarkGray));
    }
}
