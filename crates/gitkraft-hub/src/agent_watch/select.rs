//! Which repositories a job watches: any forge, public and private, picked by host, visibility
//! and `host/owner/name` globs.

use gitkraft_platforms::model::RepoInfo;
use gitkraft_settings::notify::glob_match;
use serde::{Deserialize, Serialize};

/// Which repositories count by who can see them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    /// Public and private.
    #[default]
    All,
    /// Only public ones.
    Public,
    /// Only private ones (GitLab's internal ones count as private).
    Private,
}

impl Visibility {
    /// `all`, `public` or `private`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" => Some(Self::All),
            "public" => Some(Self::Public),
            "private" => Some(Self::Private),
            _ => None,
        }
    }

    /// The word used in the file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

/// A repository selection. Everything empty means every repository the accounts can see,
/// except forks and archived ones.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Selector {
    /// Public, private or both.
    #[serde(default)]
    pub visibility: Visibility,
    /// Only these hosts (`github.com`, `codeberg.org`, …); empty means all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<String>,
    /// Globs on `host/owner/name` (`*` stays inside one segment, `**` crosses): empty means all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    /// Globs that remove repositories again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// Watch forks too.
    #[serde(default)]
    pub forks: bool,
    /// Watch archived repositories too.
    #[serde(default)]
    pub archived: bool,
}

impl Selector {
    /// Does `info` belong to the selection?
    #[must_use]
    pub fn matches(&self, info: &RepoInfo) -> bool {
        let key = info.repo.key();
        (self.forks || !info.fork)
            && (self.archived || !info.archived)
            && match self.visibility {
                Visibility::All => true,
                Visibility::Public => !info.private,
                Visibility::Private => info.private,
            }
            && (self.hosts.is_empty()
                || self
                    .hosts
                    .iter()
                    .any(|h| h.eq_ignore_ascii_case(&info.repo.host)))
            && (self.include.is_empty() || self.include.iter().any(|g| glob_match(g, &key)))
            && !self.exclude.iter().any(|g| glob_match(g, &key))
    }

    /// The selected repositories of `all`, in the same order.
    #[must_use]
    pub fn pick<'a>(&self, all: &'a [RepoInfo]) -> Vec<&'a RepoInfo> {
        all.iter().filter(|i| self.matches(i)).collect()
    }

    /// One line for a list: `all hosts · public · 2 include, 1 exclude`.
    #[must_use]
    pub fn describe(&self) -> String {
        let hosts = if self.hosts.is_empty() {
            "all forges".to_string()
        } else {
            self.hosts.join(", ")
        };
        let mut s = format!("{hosts} · {}", self.visibility.name());
        if !self.include.is_empty() {
            s.push_str(&format!(" · {} include", self.include.len()));
        }
        if !self.exclude.is_empty() {
            s.push_str(&format!(" · {} exclude", self.exclude.len()));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::RepoId;

    fn info(host: &str, owner: &str, name: &str, private: bool) -> RepoInfo {
        RepoInfo {
            repo: RepoId::new(host, owner, name),
            private,
            ..RepoInfo::default()
        }
    }

    fn all() -> Vec<RepoInfo> {
        vec![
            info("github.com", "me", "pub", false),
            info("github.com", "me", "secret", true),
            info("codeberg.org", "me", "cb", false),
            info("git.lan", "team/sub", "svc", true),
            RepoInfo {
                fork: true,
                ..info("github.com", "me", "forked", false)
            },
            RepoInfo {
                archived: true,
                ..info("github.com", "me", "old", false)
            },
        ]
    }

    fn names(s: &Selector) -> Vec<String> {
        s.pick(&all()).iter().map(|i| i.repo.name.clone()).collect()
    }

    #[test]
    fn by_default_everything_visible_but_forks_and_archived() {
        assert_eq!(names(&Selector::default()), ["pub", "secret", "cb", "svc"]);
    }

    #[test]
    fn visibility_and_hosts_narrow_the_selection_across_forges() {
        let public = Selector {
            visibility: Visibility::Public,
            ..Selector::default()
        };
        assert_eq!(names(&public), ["pub", "cb"]);
        let private = Selector {
            visibility: Visibility::Private,
            ..Selector::default()
        };
        assert_eq!(names(&private), ["secret", "svc"]);
        let cb = Selector {
            hosts: vec!["Codeberg.org".into()],
            ..Selector::default()
        };
        assert_eq!(names(&cb), ["cb"]);
    }

    #[test]
    fn globs_include_and_exclude_by_key_including_nested_groups() {
        let s = Selector {
            include: vec!["github.com/me/*".into(), "git.lan/**".into()],
            exclude: vec!["**/secret".into()],
            ..Selector::default()
        };
        assert_eq!(names(&s), ["pub", "svc"]);
        let s = Selector {
            forks: true,
            archived: true,
            ..Selector::default()
        };
        assert_eq!(names(&s).len(), 6);
    }

    #[test]
    fn a_selector_survives_toml_and_defaults_fill_the_gaps() {
        let s: Selector =
            toml::from_str("visibility = \"private\"\nhosts = [\"git.lan\"]").expect("parse");
        assert_eq!(s.visibility, Visibility::Private);
        assert!(!s.forks && s.include.is_empty());
        assert_eq!(s.describe(), "git.lan · private");
    }
}
