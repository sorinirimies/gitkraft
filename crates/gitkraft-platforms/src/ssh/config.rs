//! The block gitkraft keeps in `~/.ssh/config`: between markers, idempotent and removable.
//! Nothing outside the markers is ever touched.

fn begin(host: &str) -> String {
    format!("# >>> gitkraft ssh {host} >>>")
}

fn end(host: &str) -> String {
    format!("# <<< gitkraft ssh {host} <<<")
}

/// The block for `host` using `identity_file`; `macos` adds the keychain options.
pub fn block(host: &str, identity_file: &str, macos: bool) -> String {
    let mut b = format!(
        "{}\nHost {host}\n  IdentityFile {identity_file}\n  IdentitiesOnly yes\n  AddKeysToAgent yes\n",
        begin(host)
    );
    if macos {
        b.push_str("  IgnoreUnknown UseKeychain\n  UseKeychain yes\n");
    }
    b.push_str(&end(host));
    b.push('\n');
    b
}

fn span(text: &str, host: &str) -> Option<(usize, usize)> {
    let (b, e) = (begin(host), end(host));
    let start = text.find(&b)?;
    let after = text[start..].find(&e)? + start + e.len();
    let after = if text[after..].starts_with('\n') {
        after + 1
    } else {
        after
    };
    Some((start, after))
}

/// Whether gitkraft has a block for `host`.
pub fn has_block(text: &str, host: &str) -> bool {
    span(text, host).is_some()
}

/// The `IdentityFile` ssh would use for `host`: the first one of the first `Host` block (gitkraft's
/// or the user's own) whose patterns match. `*` and `?` wildcards and `!negations` are honoured.
pub fn identity_for(text: &str, host: &str) -> Option<String> {
    let mut matching = false;
    for line in text.lines() {
        let line = line.trim();
        let Some((word, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let rest = rest.trim().trim_start_matches('=').trim();
        if word.eq_ignore_ascii_case("host") {
            let mut hit = false;
            let mut denied = false;
            for p in rest.split_whitespace() {
                match p.strip_prefix('!') {
                    Some(n) if glob(n, host) => denied = true,
                    Some(_) => {}
                    None if glob(p, host) => hit = true,
                    None => {}
                }
            }
            matching = hit && !denied;
        } else if word.eq_ignore_ascii_case("match") {
            matching = false;
        } else if matching && word.eq_ignore_ascii_case("identityfile") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

/// `*` and `?` matching, as in `ssh_config(5)` patterns (case-insensitive).
fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some((b'*', rest)) => (0..=t.len()).any(|i| go(rest, &t[i..])),
            Some((b'?', rest)) => !t.is_empty() && go(rest, &t[1..]),
            Some((c, rest)) => t.first().is_some_and(|x| x == c) && go(rest, &t[1..]),
        }
    }
    go(
        pattern.to_ascii_lowercase().as_bytes(),
        text.to_ascii_lowercase().as_bytes(),
    )
}

/// Adds or replaces the block for `host`.
pub fn upsert(text: &str, host: &str, identity_file: &str, macos: bool) -> String {
    let new = block(host, identity_file, macos);
    match span(text, host) {
        Some((s, e)) => format!("{}{}{}", &text[..s], new, &text[e..]),
        None => {
            let mut out = text.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&new);
            out
        }
    }
}

/// Removes the block for `host`, and the blank line gitkraft put before it.
pub fn remove(text: &str, host: &str) -> String {
    match span(text, host) {
        Some((s, e)) => {
            let head = text[..s]
                .strip_suffix("\n\n")
                .map_or(&text[..s], |h| &text[..h.len() + 1]);
            format!("{head}{}", &text[e..])
        }
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = "Host work\n  HostName 10.0.0.1\n  User me\n";

    #[test]
    fn adds_a_block_after_the_users_own_config() {
        let out = upsert(USER, "github.com", "~/.ssh/gitkraft_github.com", false);
        assert!(out.starts_with(USER));
        assert!(out.contains(
            "Host github.com\n  IdentityFile ~/.ssh/gitkraft_github.com\n  IdentitiesOnly yes"
        ));
        assert!(has_block(&out, "github.com") && !has_block(&out, "gitlab.com"));
        assert!(!out.contains("UseKeychain"));
    }

    #[test]
    fn macos_gets_the_keychain_options_guarded_for_other_openssh() {
        let out = upsert("", "github.com", "k", true);
        assert!(out.contains("IgnoreUnknown UseKeychain\n  UseKeychain yes"));
        assert!(out.starts_with("# >>> gitkraft ssh github.com >>>"));
    }

    #[test]
    fn upserting_twice_replaces_instead_of_duplicating() {
        let once = upsert(USER, "github.com", "old", false);
        let twice = upsert(&once, "github.com", "new", false);
        assert_eq!(twice.matches("Host github.com").count(), 1);
        assert!(twice.contains("IdentityFile new") && !twice.contains("IdentityFile old"));
        assert_eq!(upsert(&twice, "github.com", "new", false), twice);
    }

    #[test]
    fn blocks_of_other_hosts_are_independent() {
        let a = upsert(USER, "github.com", "a", false);
        let b = upsert(&a, "gitlab.com", "b", false);
        let c = remove(&b, "github.com");
        assert!(!has_block(&c, "github.com") && has_block(&c, "gitlab.com"));
        assert!(c.starts_with(USER));
    }

    #[test]
    fn removing_restores_the_original_text() {
        let with = upsert(USER, "github.com", "k", true);
        assert_eq!(remove(&with, "github.com"), USER);
        assert_eq!(remove("", "x"), "");
        assert_eq!(remove(USER, "github.com"), USER);
        assert_eq!(remove(&upsert("", "h", "k", false), "h"), "");
    }

    #[test]
    fn a_file_without_a_final_newline_is_handled() {
        let out = upsert("Host a", "h", "k", false);
        assert!(out.starts_with("Host a\n\n# >>>"));
    }

    #[test]
    fn the_identity_ssh_would_use_comes_from_the_first_matching_host_block() {
        let cfg = "Host *.internal\n  IdentityFile ~/.ssh/int\n\nHost github.com gist.github.com\n  User git\n  IdentityFile ~/.ssh/work\n  IdentityFile ~/.ssh/second\n\nHost *\n  IdentityFile ~/.ssh/any\n";
        assert_eq!(
            identity_for(cfg, "github.com").as_deref(),
            Some("~/.ssh/work")
        );
        assert_eq!(
            identity_for(cfg, "git.internal").as_deref(),
            Some("~/.ssh/int")
        );
        assert_eq!(
            identity_for(cfg, "codeberg.org").as_deref(),
            Some("~/.ssh/any")
        );
        assert_eq!(identity_for("", "github.com"), None);
        let neg = "Host * !github.com\n  IdentityFile x\n";
        assert_eq!(identity_for(neg, "github.com"), None);
        let m = "Match host github.com\n  IdentityFile x\n";
        assert_eq!(
            identity_for(m, "github.com"),
            None,
            "Match blocks are not interpreted"
        );
        assert_eq!(
            identity_for("host GitHub.com\nidentityfile = \"k\"\n", "github.com").as_deref(),
            Some("k")
        );
    }
}
