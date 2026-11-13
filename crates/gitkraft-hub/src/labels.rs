//! The text a user types to change labels: `+bug -wontfix "good first issue"`.

/// Labels to add and to remove.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LabelEdit {
    /// Names to add.
    pub add: Vec<String>,
    /// Names to remove.
    pub remove: Vec<String>,
}

/// `+name` adds, `-name` removes, a bare name adds; double quotes keep spaces in a name.
pub fn parse(text: &str) -> Result<LabelEdit, String> {
    let mut edit = LabelEdit::default();
    let mut chars = text.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let Some(&first) = chars.peek() else { break };
        let remove = first == '-';
        if first == '+' || first == '-' {
            chars.next();
        }
        let mut name = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(c) => name.push(c),
                    None => return Err("a quote is not closed".into()),
                }
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                name.push(c);
                chars.next();
            }
        }
        if name.is_empty() {
            return Err("a label needs a name after + or -".into());
        }
        if remove {
            &mut edit.remove
        } else {
            &mut edit.add
        }
        .push(name);
    }
    if edit.add.is_empty() && edit.remove.is_empty() {
        return Err("type labels to add (+bug) or remove (-wontfix)".into());
    }
    Ok(edit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn plus_minus_bare_and_quoted_names() {
        assert_eq!(
            parse("+bug -wontfix docs \"good first issue\" -\"needs info\""),
            Ok(LabelEdit {
                add: v(&["bug", "docs", "good first issue"]),
                remove: v(&["wontfix", "needs info"]),
            })
        );
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(parse("").expect_err("empty").contains("type labels"));
        assert!(parse("  ").is_err());
        assert!(parse("+").expect_err("no name").contains("needs a name"));
        assert!(parse("\"open").expect_err("quote").contains("quote"));
    }
}
