use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

/// What can raise a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifyEvent {
    /// Your review was requested.
    ReviewRequested,
    /// You were mentioned.
    Mention,
    /// A pull request was opened.
    PrOpened,
    /// A pull request was merged.
    PrMerged,
    /// A pull request was closed unmerged.
    PrClosed,
    /// An issue was opened.
    IssueOpened,
    /// A new comment.
    Comment,
    /// A CI run failed.
    CiFailed,
    /// A failing CI run went green.
    CiRecovered,
    /// A release was published.
    ReleasePublished,
}

/// A set of events.
pub type EventSet = BTreeSet<NotifyEvent>;

/// How much activity you want to hear about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Everything in the project.
    All,
    /// Things you take part in.
    #[default]
    Involved,
    /// Only your own items.
    Mine,
}

/// Include / exclude rule on names (labels, authors).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LabelRule {
    /// At least one must be present (empty: no requirement).
    #[serde(default)]
    pub include: Vec<String>,
    /// None may be present.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// A daily window in which desktop notifications stay silent; it may wrap midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuietHours {
    /// Start (inclusive).
    pub from: NaiveTime,
    /// End (exclusive).
    pub to: NaiveTime,
}

impl QuietHours {
    /// Parses `"22:00-08:00"`.
    pub fn parse(s: &str) -> Option<Self> {
        let (a, b) = s.split_once('-')?;
        let t = |x: &str| NaiveTime::parse_from_str(x.trim(), "%H:%M").ok();
        Some(Self {
            from: t(a)?,
            to: t(b)?,
        })
    }

    /// Whether `now` falls inside the window.
    pub fn contains(&self, now: NaiveTime) -> bool {
        match self.from.cmp(&self.to) {
            std::cmp::Ordering::Less => now >= self.from && now < self.to,
            std::cmp::Ordering::Greater => now >= self.from || now < self.to,
            std::cmp::Ordering::Equal => false,
        }
    }
}

/// Parses `"30s"`, `"5m"`, `"2h"` or `"1d"`. A bare number is seconds.
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    let (num, mult) = match s.chars().last()? {
        's' => (&s[..s.len() - 1], 1),
        'm' => (&s[..s.len() - 1], 60),
        'h' => (&s[..s.len() - 1], 3600),
        'd' => (&s[..s.len() - 1], 86_400),
        c if c.is_ascii_digit() => (s, 1),
        _ => return None,
    };
    num.trim()
        .parse::<u64>()
        .ok()
        .map(|n| Duration::from_secs(n * mult))
}

/// One layer. Every key is optional; missing keys inherit from the layer below.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotifySettings {
    /// Master switch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Poll interval, e.g. `"60s"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_interval: Option<String>,
    /// Events to notify about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<NotifyEvent>>,
    /// Scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    /// Quiet hours, `"22:00-08:00"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiet_hours: Option<String>,
    /// Show desktop notifications.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop: Option<bool>,
    /// Keep entries in the in-app inbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inbox: Option<bool>,
    /// Play a sound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sound: Option<bool>,
    /// Branch globs CI events apply to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,
    /// Changed-path globs a PR must touch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    /// Label include / exclude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<LabelRule>,
    /// Author include / exclude (`include` empty means anyone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authors: Option<LabelRule>,
}

impl NotifySettings {
    /// `self` over `base`: keys set here win.
    fn over(&self, base: &Self) -> Self {
        Self {
            enabled: self.enabled.or(base.enabled),
            poll_interval: self
                .poll_interval
                .clone()
                .or_else(|| base.poll_interval.clone()),
            events: self.events.clone().or_else(|| base.events.clone()),
            scope: self.scope.or(base.scope),
            quiet_hours: self
                .quiet_hours
                .clone()
                .or_else(|| base.quiet_hours.clone()),
            desktop: self.desktop.or(base.desktop),
            inbox: self.inbox.or(base.inbox),
            sound: self.sound.or(base.sound),
            branches: self.branches.clone().or_else(|| base.branches.clone()),
            paths: self.paths.clone().or_else(|| base.paths.clone()),
            labels: self.labels.clone().or_else(|| base.labels.clone()),
            authors: self.authors.clone().or_else(|| base.authors.clone()),
        }
    }
}

/// The global file: `[defaults]` and `[projects."host/owner/name"]`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotifyConfig {
    /// Applies to every project.
    #[serde(default)]
    pub defaults: NotifySettings,
    /// Per-project overrides, keyed by `host/owner/name`.
    #[serde(default)]
    pub projects: BTreeMap<String, NotifySettings>,
}

/// Minimum poll interval: faster polling only burns rate limit.
const MIN_INTERVAL: Duration = Duration::from_secs(10);

/// Fully resolved settings for one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective {
    /// Notifications on for this project.
    pub enabled: bool,
    /// Time between polls (at least 10 s).
    pub poll_interval: Duration,
    /// Events that raise a notification.
    pub events: EventSet,
    /// Scope.
    pub scope: Scope,
    /// Quiet hours, if any.
    pub quiet_hours: Option<QuietHours>,
    /// Desktop notifications.
    pub desktop: bool,
    /// In-app inbox.
    pub inbox: bool,
    /// Sound.
    pub sound: bool,
    /// Branch globs (empty: every branch).
    pub branches: Vec<String>,
    /// Path globs (empty: any change).
    pub paths: Vec<String>,
    /// Label rule.
    pub labels: LabelRule,
    /// Author rule.
    pub authors: LabelRule,
}

impl NotifyConfig {
    /// Parses a TOML file. A syntax error is returned as text for the UI.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// Resolves one project. `repo_local` is the parsed `.gitkraft/notifications.toml`.
    pub fn resolve(&self, repo_key: &str, repo_local: Option<&NotifySettings>) -> Effective {
        let mut layer = self.defaults.clone();
        if let Some(project) = self.projects.get(repo_key) {
            layer = project.over(&layer);
        }
        if let Some(local) = repo_local {
            layer = local.over(&layer);
        }
        Effective::from_layer(&layer)
    }
}

impl Effective {
    fn from_layer(l: &NotifySettings) -> Self {
        let events = l.events.clone().map_or_else(
            || {
                [
                    NotifyEvent::ReviewRequested,
                    NotifyEvent::Mention,
                    NotifyEvent::PrOpened,
                    NotifyEvent::IssueOpened,
                    NotifyEvent::CiFailed,
                ]
                .into_iter()
                .collect()
            },
            |v| v.into_iter().collect(),
        );
        Self {
            enabled: l.enabled.unwrap_or(false),
            poll_interval: l
                .poll_interval
                .as_deref()
                .and_then(parse_duration)
                .unwrap_or(Duration::from_secs(60))
                .max(MIN_INTERVAL),
            events,
            scope: l.scope.unwrap_or_default(),
            quiet_hours: l.quiet_hours.as_deref().and_then(QuietHours::parse),
            desktop: l.desktop.unwrap_or(true),
            inbox: l.inbox.unwrap_or(true),
            sound: l.sound.unwrap_or(false),
            branches: l.branches.clone().unwrap_or_default(),
            paths: l.paths.clone().unwrap_or_default(),
            labels: l.labels.clone().unwrap_or_default(),
            authors: l.authors.clone().unwrap_or_default(),
        }
    }
}

/// Sets `enabled` for `repo_key` in the TOML `text`, keeping comments and every
/// other key. `repo_key` of `None` sets the global `[defaults]` switch.
pub fn set_enabled(text: &str, repo_key: Option<&str>, enabled: bool) -> Result<String, String> {
    use toml_edit::{DocumentMut, Item, Table, value};
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e: toml_edit::TomlError| e.to_string())?;
    fn table<'a>(parent: &'a mut Item, key: &str) -> Result<&'a mut Table, String> {
        if parent.is_none() {
            *parent = Item::Table(Table::new());
        }
        let t = parent
            .as_table_mut()
            .ok_or_else(|| format!("`{key}` is not a table"))?;
        t.set_implicit(true);
        let entry = t.entry(key).or_insert(Item::Table(Table::new()));
        entry
            .as_table_mut()
            .ok_or_else(|| format!("`{key}` is not a table"))
    }
    let target = match repo_key {
        None => table(doc.as_item_mut(), "defaults")?,
        Some(key) => {
            let projects = table(doc.as_item_mut(), "projects")?;
            let entry = projects.entry(key).or_insert(Item::Table(Table::new()));
            entry
                .as_table_mut()
                .ok_or_else(|| format!("`{key}` is not a table"))?
        }
    };
    target["enabled"] = value(enabled);
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOBAL: &str = r#"
[defaults]
enabled = true
poll_interval = "2m"
events = ["review_requested", "mention"]
quiet_hours = "22:00-08:00"

[projects."github.com/o/noisy"]
enabled = false

[projects."github.com/o/app"]
poll_interval = "30s"
events = ["ci_failed", "pr_merged"]
branches = ["master", "release/*"]
labels = { include = ["bug"], exclude = ["wip"] }
"#;

    #[test]
    fn built_in_defaults_are_off_and_sane() {
        let e = NotifyConfig::default().resolve("github.com/o/r", None);
        assert!(!e.enabled);
        assert_eq!(e.poll_interval, Duration::from_secs(60));
        assert!(e.events.contains(&NotifyEvent::CiFailed));
        assert!(e.desktop && e.inbox && !e.sound);
        assert_eq!(e.scope, Scope::Involved);
    }

    #[test]
    fn layers_override_key_by_key() {
        let cfg = NotifyConfig::parse(GLOBAL).expect("valid");
        let app = cfg.resolve("github.com/o/app", None);
        assert!(app.enabled, "inherited from defaults");
        assert_eq!(app.poll_interval, Duration::from_secs(30));
        assert_eq!(app.events.len(), 2);
        assert!(app.events.contains(&NotifyEvent::PrMerged));
        assert!(app.quiet_hours.is_some(), "inherited");
        assert_eq!(app.labels.include, ["bug"]);

        assert!(!cfg.resolve("github.com/o/noisy", None).enabled);
        assert!(cfg.resolve("github.com/o/other", None).enabled);
    }

    #[test]
    fn repo_local_file_wins_over_the_global_project_table() {
        let cfg = NotifyConfig::parse(GLOBAL).expect("valid");
        let local = NotifySettings {
            enabled: Some(true),
            poll_interval: Some("5m".into()),
            ..Default::default()
        };
        let e = cfg.resolve("github.com/o/noisy", Some(&local));
        assert!(e.enabled);
        assert_eq!(e.poll_interval, Duration::from_secs(300));
    }

    #[test]
    fn interval_is_clamped_and_garbage_falls_back() {
        let mut c = NotifyConfig::default();
        c.defaults.poll_interval = Some("1s".into());
        assert_eq!(
            c.resolve("x/y/z", None).poll_interval,
            Duration::from_secs(10)
        );
        c.defaults.poll_interval = Some("soon".into());
        assert_eq!(
            c.resolve("x/y/z", None).poll_interval,
            Duration::from_secs(60)
        );
    }

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("30s"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
        assert_eq!(parse_duration("2h"), Some(Duration::from_secs(7200)));
        assert_eq!(parse_duration("1d"), Some(Duration::from_secs(86_400)));
        assert_eq!(parse_duration("45"), Some(Duration::from_secs(45)));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("m"), None);
        assert_eq!(parse_duration("-5s"), None);
    }

    #[test]
    fn quiet_hours_wrap_midnight() {
        let q = QuietHours::parse("22:00-08:00").expect("valid");
        let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).expect("time");
        assert!(q.contains(t(23, 0)));
        assert!(q.contains(t(7, 59)));
        assert!(!q.contains(t(8, 0)));
        assert!(!q.contains(t(12, 0)));
        let day = QuietHours::parse("09:00-17:00").expect("valid");
        assert!(day.contains(t(9, 0)) && !day.contains(t(17, 0)));
        assert!(QuietHours::parse("nonsense").is_none());
        assert!(
            !QuietHours::parse("10:00-10:00")
                .expect("valid")
                .contains(t(10, 0))
        );
    }

    #[test]
    fn bad_toml_is_an_error_string() {
        assert!(NotifyConfig::parse("[defaults\nenabled = ").is_err());
        assert!(NotifyConfig::parse("[defaults]\nevents = [\"nope\"]").is_err());
    }

    #[test]
    fn set_enabled_keeps_comments_and_other_keys() {
        let text = "# my notes\n[defaults]\nenabled = true # keep\n\n[projects.\"github.com/o/app\"]\npoll_interval = \"30s\"\n";
        let out = set_enabled(text, Some("github.com/o/app"), true).expect("edit");
        assert!(out.contains("# my notes") && out.contains("# keep"));
        let cfg = NotifyConfig::parse(&out).expect("valid");
        let p = &cfg.projects["github.com/o/app"];
        assert_eq!(p.enabled, Some(true));
        assert_eq!(p.poll_interval.as_deref(), Some("30s"));
    }

    #[test]
    fn set_enabled_creates_missing_tables_and_global_switch() {
        let out = set_enabled("", Some("codeberg.org/me/x"), false).expect("edit");
        let cfg = NotifyConfig::parse(&out).expect("valid");
        assert_eq!(cfg.projects["codeberg.org/me/x"].enabled, Some(false));
        let out = set_enabled(&out, None, true).expect("edit");
        let cfg = NotifyConfig::parse(&out).expect("valid");
        assert_eq!(cfg.defaults.enabled, Some(true));
        assert_eq!(cfg.projects.len(), 1);
    }

    #[test]
    fn set_enabled_rejects_wrong_shapes() {
        assert!(set_enabled("projects = 3", Some("a/b/c"), true).is_err());
        assert!(set_enabled("[defaults", None, true).is_err());
    }
}
