/// Matches `text` against a glob: `*` any run without `/`, `**` any run, `?` one char.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') if p.get(1) == Some(&'*') => {
                let rest = &p[2..];
                // `**/` also matches zero directories.
                let rest = rest.strip_prefix(&['/']).unwrap_or(rest);
                (0..=t.len()).any(|i| go(rest, &t[i..]))
            }
            Some('*') => {
                let rest = &p[1..];
                let mut i = 0;
                loop {
                    if go(rest, &t[i..]) {
                        return true;
                    }
                    if i == t.len() || t[i] == '/' {
                        return false;
                    }
                    i += 1;
                }
            }
            Some('?') => !t.is_empty() && t[0] != '/' && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    go(&p, &t)
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn star_stops_at_slash_double_star_does_not() {
        assert!(glob_match("release/*", "release/1.0"));
        assert!(!glob_match("release/*", "release/1.0/x"));
        assert!(glob_match("crates/core/**", "crates/core/src/lib.rs"));
        assert!(glob_match("**/*.rs", "src/a/b.rs"));
        assert!(glob_match("**/*.rs", "b.rs"));
        assert!(glob_match("master", "master"));
        assert!(!glob_match("master", "master2"));
        assert!(glob_match("v?.0", "v1.0"));
        assert!(!glob_match("v?.0", "v10.0"));
    }

    #[test]
    fn empty_pattern_matches_only_empty() {
        assert!(glob_match("", ""));
        assert!(!glob_match("", "a"));
    }
}
