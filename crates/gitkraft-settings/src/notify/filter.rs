use super::config::{Effective, NotifyEvent, Scope};
use super::glob::glob_match;

/// A fact the poller found, to be checked against a project's settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Candidate {
    /// What happened.
    pub event: Option<NotifyEvent>,
    /// Branch it concerns (CI events, PR target).
    pub branch: Option<String>,
    /// Files it touches (PR events), when known.
    pub paths: Vec<String>,
    /// Labels on the item.
    pub labels: Vec<String>,
    /// Who caused it.
    pub author: Option<String>,
    /// The item is yours (you authored or are assigned).
    pub mine: bool,
    /// You take part in it (mention, review request, earlier comment…).
    pub involved: bool,
}

impl Effective {
    /// Whether `c` should notify under these settings.
    pub fn accepts(&self, c: &Candidate) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(event) = c.event else { return false };
        if !self.events.contains(&event) {
            return false;
        }
        // Review requests and mentions are addressed to you, whatever the scope.
        let addressed = matches!(event, NotifyEvent::ReviewRequested | NotifyEvent::Mention);
        let in_scope = match self.scope {
            Scope::All => true,
            Scope::Involved => c.involved || c.mine || addressed,
            Scope::Mine => c.mine,
        };
        if !in_scope {
            return false;
        }
        let ci = matches!(event, NotifyEvent::CiFailed | NotifyEvent::CiRecovered);
        if ci
            && !self.branches.is_empty()
            && !c
                .branch
                .as_deref()
                .is_some_and(|b| self.branches.iter().any(|g| glob_match(g, b)))
        {
            return false;
        }
        if !self.paths.is_empty()
            && !c.paths.is_empty()
            && !c
                .paths
                .iter()
                .any(|p| self.paths.iter().any(|g| glob_match(g, p)))
        {
            return false;
        }
        if !self.labels.include.is_empty()
            && !c.labels.iter().any(|l| self.labels.include.contains(l))
        {
            return false;
        }
        if c.labels.iter().any(|l| self.labels.exclude.contains(l)) {
            return false;
        }
        if let Some(author) = &c.author {
            if !self.authors.include.is_empty() && !self.authors.include.contains(author) {
                return false;
            }
            if self.authors.exclude.contains(author) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::NotifyConfig;

    fn eff(toml: &str) -> Effective {
        NotifyConfig::parse(toml)
            .expect("valid")
            .resolve("h/o/r", None)
    }

    fn cand(event: NotifyEvent) -> Candidate {
        Candidate {
            event: Some(event),
            ..Candidate::default()
        }
    }

    #[test]
    fn disabled_or_unlisted_events_never_pass() {
        let off = eff("[defaults]\nevents = [\"ci_failed\"]");
        assert!(!off.accepts(&cand(NotifyEvent::CiFailed)));
        let on = eff("[defaults]\nenabled = true\nscope = \"all\"\nevents = [\"ci_failed\"]");
        assert!(on.accepts(&cand(NotifyEvent::CiFailed)));
        assert!(!on.accepts(&cand(NotifyEvent::PrOpened)));
        assert!(!on.accepts(&Candidate::default()));
    }

    #[test]
    fn scope_involved_lets_addressed_events_through() {
        let e = eff("[defaults]\nenabled = true\nevents = [\"mention\", \"pr_opened\"]");
        assert!(e.accepts(&cand(NotifyEvent::Mention)));
        assert!(!e.accepts(&cand(NotifyEvent::PrOpened)));
        assert!(e.accepts(&Candidate {
            involved: true,
            ..cand(NotifyEvent::PrOpened)
        }));
        let mine = eff("[defaults]\nenabled = true\nscope = \"mine\"\nevents = [\"pr_opened\"]");
        assert!(!mine.accepts(&Candidate {
            involved: true,
            ..cand(NotifyEvent::PrOpened)
        }));
        assert!(mine.accepts(&Candidate {
            mine: true,
            ..cand(NotifyEvent::PrOpened)
        }));
    }

    #[test]
    fn ci_events_respect_branch_globs() {
        let e = eff(
            "[defaults]\nenabled = true\nscope = \"all\"\nbranches = [\"master\", \"release/*\"]\nevents = [\"ci_failed\", \"pr_opened\"]",
        );
        let ci = |b: &str| Candidate {
            branch: Some(b.into()),
            ..cand(NotifyEvent::CiFailed)
        };
        assert!(e.accepts(&ci("master")));
        assert!(e.accepts(&ci("release/2.0")));
        assert!(!e.accepts(&ci("feature/x")));
        assert!(
            !e.accepts(&cand(NotifyEvent::CiFailed)),
            "unknown branch cannot match"
        );
        // Branch globs only constrain CI events.
        assert!(e.accepts(&Candidate {
            branch: Some("feature/x".into()),
            ..cand(NotifyEvent::PrOpened)
        }));
    }

    #[test]
    fn path_label_and_author_rules() {
        let e = eff(
            "[defaults]\nenabled = true\nscope = \"all\"\nevents = [\"pr_opened\"]\npaths = [\"crates/core/**\"]\nlabels = { include = [\"bug\"], exclude = [\"wip\"] }\nauthors = { exclude = [\"dependabot[bot]\"] }",
        );
        let base = Candidate {
            paths: vec!["crates/core/src/lib.rs".into()],
            labels: vec!["bug".into()],
            author: Some("alice".into()),
            ..cand(NotifyEvent::PrOpened)
        };
        assert!(e.accepts(&base));
        assert!(!e.accepts(&Candidate {
            paths: vec!["docs/a.md".into()],
            ..base.clone()
        }));
        assert!(
            e.accepts(&Candidate {
                paths: vec![],
                ..base.clone()
            }),
            "unknown paths do not block"
        );
        assert!(!e.accepts(&Candidate {
            labels: vec![],
            ..base.clone()
        }));
        assert!(!e.accepts(&Candidate {
            labels: vec!["bug".into(), "wip".into()],
            ..base.clone()
        }));
        assert!(!e.accepts(&Candidate {
            author: Some("dependabot[bot]".into()),
            ..base
        }));
    }
}
