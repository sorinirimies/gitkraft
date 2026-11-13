//! Fuzzy matching for command palettes.

/// How well `query` matches `text`: `None` unless every query letter appears in order.
/// Consecutive letters and letters at a word start score higher; gaps cost a little.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut last: Option<usize> = None;
    for (ti, c) in t.iter().enumerate() {
        if qi < q.len() && *c == q[qi] {
            score += 10;
            if last.is_some_and(|l| l + 1 == ti) {
                score += 15;
            }
            if ti == 0 || !t[ti - 1].is_alphanumeric() {
                score += 12;
            }
            if let Some(l) = last {
                score -= ((ti - l - 1) as i32).min(6);
            }
            last = Some(ti);
            qi += 1;
        }
    }
    (qi == q.len()).then_some(score - t.len() as i32 / 8)
}

/// Indexes into `labels` that match `query`, best first (ties keep list order).
pub fn filter<'a>(labels: impl IntoIterator<Item = &'a str>, query: &str) -> Vec<usize> {
    let mut hits: Vec<(i32, usize)> = labels
        .into_iter()
        .enumerate()
        .filter_map(|(i, l)| fuzzy_score(query, l).map(|s| (s, i)))
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.into_iter().map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_must_appear_in_order_and_case_does_not_matter() {
        assert!(fuzzy_score("mrg", "Pull request: merge").is_some());
        assert!(fuzzy_score("MERGE", "Pull request: merge").is_some());
        assert!(fuzzy_score("gem", "merge").is_none());
        assert!(fuzzy_score("zzz", "merge").is_none());
        assert_eq!(fuzzy_score("", "anything"), Some(0));
        assert_eq!(fuzzy_score("   ", "anything"), Some(0));
    }

    #[test]
    fn consecutive_letters_and_word_starts_win() {
        let tight = fuzzy_score("rerun", "CI run: re-run failed jobs").expect("m");
        let loose = fuzzy_score("rerun", "Pull request: close or reopen, then run").expect("m");
        assert!(tight > loose);
        assert!(
            fuzzy_score("ssh", "Go to SSH keys")
                > fuzzy_score("ssh", "Pulse: previous repository, see slashes here")
        );
    }

    #[test]
    fn filtering_ranks_and_keeps_order_on_ties() {
        let l = ["Refresh", "Go to pulse", "Pulse: next repository"];
        assert_eq!(filter(l, ""), [0, 1, 2]);
        assert_eq!(filter(l, "pulse"), [1, 2]);
        assert_eq!(filter(l, "next"), [2]);
        assert!(filter(l, "xyz").is_empty());
    }
}
