//! The dashboard: an ordered list of sections, each a saved query.
//!
//! ```toml
//! [[section]]
//! name = "Review requested"
//! kind = "prs"
//! involvement = "review_requested"
//!
//! [[section]]
//! name = "gitkraft CI"
//! kind = "ci"
//! repos = ["github.com/sorinirimies/gitkraft"]
//! branch = "master"
//! ```

use serde::{Deserialize, Serialize};

/// What a section lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    /// Pull / merge requests.
    Prs,
    /// Issues.
    Issues,
    /// CI runs.
    Ci,
    /// The notification inbox.
    Inbox,
}

/// How you relate to the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Involvement {
    /// You opened them.
    Authored,
    /// They are assigned to you.
    Assigned,
    /// Your review is requested.
    ReviewRequested,
}

/// Open / closed / all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionState {
    /// Open items.
    #[default]
    Open,
    /// Closed (and merged) items.
    Closed,
    /// Everything.
    All,
}

/// One saved query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Title shown on the section header.
    pub name: String,
    /// What it lists.
    pub kind: SectionKind,
    /// Repos as `host/owner/name`. Empty: search across every account by `involvement`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<String>,
    /// Relation to you (needed when `repos` is empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub involvement: Option<Involvement>,
    /// State filter.
    #[serde(default)]
    pub state: SectionState,
    /// Only items with this label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Branch (CI sections).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Hidden sections stay in the file but are not shown.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The whole dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Dashboard {
    /// Sections in display order.
    #[serde(default, rename = "section")]
    pub sections: Vec<Section>,
}

impl Dashboard {
    /// What a fresh install shows.
    pub fn starter() -> Self {
        let pr = |name: &str, involvement| Section {
            name: name.into(),
            kind: SectionKind::Prs,
            repos: vec![],
            involvement: Some(involvement),
            state: SectionState::Open,
            label: None,
            branch: None,
            hidden: false,
        };
        Self {
            sections: vec![
                pr("Review requested", Involvement::ReviewRequested),
                pr("My pull requests", Involvement::Authored),
                Section {
                    name: "Assigned issues".into(),
                    kind: SectionKind::Issues,
                    involvement: Some(Involvement::Assigned),
                    ..pr("", Involvement::Assigned)
                },
                Section { name: "Inbox".into(), kind: SectionKind::Inbox, involvement: None, ..pr("", Involvement::Assigned) },
            ],
        }
    }

    /// Parses `dashboard.toml`. An empty file gives the starter dashboard.
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Ok(Self::starter());
        }
        let d: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        d.validate()
    }

    fn validate(self) -> Result<Self, String> {
        for s in &self.sections {
            if s.name.trim().is_empty() {
                return Err("a section has no name".into());
            }
            let needs_scope = matches!(s.kind, SectionKind::Prs | SectionKind::Issues);
            if needs_scope && s.repos.is_empty() && s.involvement.is_none() {
                return Err(format!("section \"{}\" needs `repos` or `involvement`", s.name));
            }
            if s.kind == SectionKind::Ci && s.repos.is_empty() {
                return Err(format!("CI section \"{}\" needs `repos`", s.name));
            }
            if let Some(bad) = s.repos.iter().find(|r| r.matches('/').count() < 2) {
                return Err(format!("section \"{}\": \"{bad}\" is not host/owner/name", s.name));
            }
        }
        Ok(self)
    }

    /// Serializes to TOML.
    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Sections that are shown, with their index in `sections`.
    pub fn visible(&self) -> impl Iterator<Item = (usize, &Section)> {
        self.sections.iter().enumerate().filter(|(_, s)| !s.hidden)
    }

    /// Moves the section at `from` so it ends up at `to`. Out of range or equal indexes are a no-op.
    pub fn reorder(&mut self, from: usize, to: usize) -> bool {
        reorder(&mut self.sections, from, to)
    }
}

/// Moves `items[from]` so it ends up at index `to` (as in a drag and drop).
/// Returns whether anything changed.
pub fn reorder<T>(items: &mut Vec<T>, from: usize, to: usize) -> bool {
    if from == to || from >= items.len() || to >= items.len() {
        return false;
    }
    let item = items.remove(from);
    items.insert(to, item);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_moves_forward_back_and_ignores_nonsense() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        assert!(reorder(&mut v, 0, 2));
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        assert!(reorder(&mut v, 3, 0));
        assert_eq!(v, ['d', 'b', 'c', 'a']);
        assert!(!reorder(&mut v, 1, 1), "same index");
        assert!(!reorder(&mut v, 4, 0), "from out of range");
        assert!(!reorder(&mut v, 0, 4), "to out of range");
        assert_eq!(v, ['d', 'b', 'c', 'a']);
        let mut empty: Vec<u8> = vec![];
        assert!(!reorder(&mut empty, 0, 0));
        let mut one = vec![1];
        assert!(!reorder(&mut one, 0, 0));
    }

    #[test]
    fn empty_file_gives_the_starter() {
        let d = Dashboard::parse("  \n").expect("starter");
        assert_eq!(d, Dashboard::starter());
        assert_eq!(d.sections[0].involvement, Some(Involvement::ReviewRequested));
        assert!(d.sections.iter().any(|s| s.kind == SectionKind::Inbox));
    }

    #[test]
    fn parses_sections_with_defaults() {
        let d = Dashboard::parse(
            "[[section]]\nname = \"A\"\nkind = \"prs\"\ninvolvement = \"authored\"\n\n[[section]]\nname = \"CI\"\nkind = \"ci\"\nrepos = [\"github.com/o/r\"]\nbranch = \"master\"\n",
        )
        .expect("valid");
        assert_eq!(d.sections.len(), 2);
        assert_eq!(d.sections[0].state, SectionState::Open);
        assert!(!d.sections[0].hidden);
        assert_eq!(d.sections[1].branch.as_deref(), Some("master"));
    }

    #[test]
    fn validation_explains_what_is_wrong() {
        let err = |t: &str| Dashboard::parse(t).expect_err("invalid");
        assert!(err("[[section]]\nname = \"x\"\nkind = \"prs\"").contains("needs `repos` or `involvement`"));
        assert!(err("[[section]]\nname = \"x\"\nkind = \"ci\"").contains("needs `repos`"));
        assert!(err("[[section]]\nname = \" \"\nkind = \"inbox\"").contains("no name"));
        assert!(err("[[section]]\nname = \"x\"\nkind = \"prs\"\nrepos = [\"o/r\"]").contains("host/owner/name"));
        assert!(err("[[section]]\nname = \"x\"\nkind = \"nope\"").len() > 5);
    }

    #[test]
    fn round_trips_through_toml_and_keeps_order_after_a_drag() {
        let mut d = Dashboard::starter();
        let names: Vec<_> = d.sections.iter().map(|s| s.name.clone()).collect();
        assert!(d.reorder(0, 3));
        let text = d.to_toml().expect("toml");
        let back = Dashboard::parse(&text).expect("valid");
        assert_eq!(back, d);
        assert_eq!(back.sections[3].name, names[0]);
        assert_eq!(back.sections[0].name, names[1]);
    }

    #[test]
    fn hidden_sections_stay_but_are_not_visible() {
        let mut d = Dashboard::starter();
        d.sections[1].hidden = true;
        let shown: Vec<_> = d.visible().map(|(i, _)| i).collect();
        assert_eq!(shown, vec![0, 2, 3]);
        let back = Dashboard::parse(&d.to_toml().expect("toml")).expect("valid");
        assert!(back.sections[1].hidden);
    }
}
