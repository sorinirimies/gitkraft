//! Small helpers on Markdown bodies that every front-end needs.

/// URLs of the images in a Markdown or HTML body, in order, without duplicates.
/// Covers `![alt](url "title")`, `<img src="url">` and reference-free forms; `data:` URLs are skipped.
pub fn image_urls(md: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |u: &str| {
        let u = u.trim();
        if (u.starts_with("http://") || u.starts_with("https://")) && !out.iter().any(|x| x == u) {
            out.push(u.to_string());
        }
    };
    let b = md.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'!' && b.get(i + 1) == Some(&b'[') {
            // ![alt](url ...)
            if let Some(close) = md[i + 2..].find("](") {
                let start = i + 2 + close + 2;
                if let Some(end) = md[start..].find(')') {
                    let target = &md[start..start + end];
                    let url = target.split_whitespace().next().unwrap_or_default();
                    push(url.trim_matches(['<', '>']));
                    i = start + end;
                    continue;
                }
            }
        } else if b[i] == b'<'
            && md[i..].len() > 4
            && md[i..i + 4].eq_ignore_ascii_case("<img")
            && let Some(end) = md[i..].find('>')
        {
            let tag = &md[i..i + end];
            if let Some(p) = tag.to_ascii_lowercase().find("src=") {
                let rest = &tag[p + 4..];
                if let Some(q @ ('"' | '\'')) = rest.chars().next()
                    && let Some(e) = rest[1..].find(q)
                {
                    push(&rest[1..1 + e]);
                }
            }
            i += end;
            continue;
        }
        i += md[i..].chars().next().map_or(1, char::len_utf8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_markdown_and_html_images_in_order_without_duplicates() {
        let md = "Intro ![a](https://x/1.png) text\n<img width=\"10\" src=\"https://x/2.png\" />\n![dup](https://x/1.png \"title\")\n![t](<https://x/3.png>)";
        assert_eq!(
            image_urls(md),
            ["https://x/1.png", "https://x/2.png", "https://x/3.png"]
        );
    }

    #[test]
    fn skips_data_relative_and_broken_forms() {
        assert!(
            image_urls(
                "![a](data:image/png;base64,AAA) ![b](rel/path.png) ![c](https://unterminated"
            )
            .is_empty()
        );
        assert!(image_urls("<img src=https://noquotes/x.png>").is_empty());
        assert!(image_urls("no images, just ! and [x](https://link) and é ✓").is_empty());
        assert!(image_urls("").is_empty());
    }

    #[test]
    fn plain_links_are_not_images_and_single_quotes_work() {
        assert_eq!(
            image_urls("[link](https://x/a.png) <IMG SRC='https://x/b.png'>"),
            ["https://x/b.png"]
        );
    }
}
