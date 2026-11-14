//! Commit messages drafted by a model from the staged diff, in the style a project asks for.
//!
//! The style lives in `<repo>/.gitkraft/ai.toml` (over `<config dir>/gitkraft/ai.toml`, field by
//! field), so each project can have its own format, scopes, language and template:
//!
//! ```toml
//! [commit]
//! format = "conventional"          # conventional | plain | template
//! types = ["feat", "fix", "docs", "refactor", "test", "chore"]
//! scopes = ["tui", "gui", "core"]  # optional: restricts the scope
//! max_subject = 72
//! language = "English"
//! instructions = "Mention the module in the body. No emoji."
//! ticket_prefix = "MARIO-"         # a ticket like MARIO-123 in the branch name is picked up
//! examples_from_history = 8        # recent subjects shown to the model as style examples
//! template = "{type}({scope}): {subject}\n\n{body}\n\nRefs: {ticket}"
//! ```
//!
//! The model fills `type`, `scope`, `subject` and `body`; the template, not the model, decides the
//! layout. A template line whose placeholders are all empty is dropped. The result is a draft: it
//! goes into the message box for the user to edit, never straight into a commit.

use std::path::Path;

use gitkraft_core::features::diff::ops::get_staged_diff;
use gitkraft_core::features::diff::{DiffLine, FileStatus};
use serde::Deserialize;
use serde_json::Value;

use crate::ai::{AiError, Llm, redact};

/// Characters of diff a prompt may carry.
pub const MAX_DIFF_CHARS: usize = 14_000;

/// What a project asked for; every field is optional so files can be layered.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct RawStyle {
    /// `conventional`, `plain` or `template`.
    pub format: Option<String>,
    /// Allowed types.
    pub types: Option<Vec<String>>,
    /// Allowed scopes.
    pub scopes: Option<Vec<String>>,
    /// Longest subject line.
    pub max_subject: Option<usize>,
    /// Language of the message.
    pub language: Option<String>,
    /// Free text added to the instructions.
    pub instructions: Option<String>,
    /// Ticket key prefix such as `MARIO-`.
    pub ticket_prefix: Option<String>,
    /// How many recent subjects to show as examples (0 for none).
    pub examples_from_history: Option<usize>,
    /// Layout with `{type} {scope} {subject} {body} {ticket} {branch}`.
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct StyleFile {
    #[serde(default)]
    commit: RawStyle,
}

impl RawStyle {
    /// Parses the text of an `ai.toml`.
    ///
    /// # Errors
    /// The TOML error, so a typo is shown instead of silently ignored.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str::<StyleFile>(text)
            .map(|f| f.commit)
            .map_err(|e| e.to_string())
    }

    /// `self` with every field `over` sets replaced.
    #[must_use]
    pub fn over(self, over: Self) -> Self {
        Self {
            format: over.format.or(self.format),
            types: over.types.or(self.types),
            scopes: over.scopes.or(self.scopes),
            max_subject: over.max_subject.or(self.max_subject),
            language: over.language.or(self.language),
            instructions: over.instructions.or(self.instructions),
            ticket_prefix: over.ticket_prefix.or(self.ticket_prefix),
            examples_from_history: over.examples_from_history.or(self.examples_from_history),
            template: over.template.or(self.template),
        }
    }

    /// Reads the global file then the project file; a missing file is fine, a broken one is not.
    ///
    /// # Errors
    /// Which file is broken and why.
    pub fn load(repo_dir: &Path, global_dir: Option<&Path>) -> Result<Self, String> {
        let read = |p: &Path| -> Result<Option<Self>, String> {
            match std::fs::read_to_string(p) {
                Ok(t) => Self::parse(&t)
                    .map(Some)
                    .map_err(|e| format!("{}: {e}", p.display())),
                Err(_) => Ok(None),
            }
        };
        let global = match global_dir {
            Some(d) => read(&d.join("ai.toml"))?,
            None => None,
        };
        let project = read(&repo_dir.join(".gitkraft").join("ai.toml"))?;
        Ok(global.unwrap_or_default().over(project.unwrap_or_default()))
    }
}

/// A starter file for `commit-message --init`.
pub const EXAMPLE: &str = r#"# GitKraft AI settings for this project. Layered over <config dir>/gitkraft/ai.toml.
[commit]
format = "conventional"          # conventional | plain | template
types = ["feat", "fix", "docs", "refactor", "test", "chore", "perf", "ci"]
# scopes = ["tui", "gui", "core"]  # restrict the scope; leave out to let the model choose
max_subject = 72
language = "English"
# instructions = "Mention the module in the body. No emoji."
# ticket_prefix = "MARIO-"         # MARIO-123 in the branch name becomes {ticket}
examples_from_history = 8
# template = "{type}({scope}): {subject}\n\n{body}\n\nRefs: {ticket}"
"#;

/// What the model gets to look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInput {
    /// Current branch.
    pub branch: String,
    /// The staged diff as text, already cut to size.
    pub diff: String,
    /// Subjects of recent commits.
    pub recent: Vec<String>,
}

fn is_noise(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.ends_with(".lock")
        || name.ends_with("-lock.json")
        || name.ends_with("lock.yaml")
        || name == "go.sum"
        || name.ends_with(".min.js")
        || name.ends_with(".snap")
}

/// Cuts `diff` text per file so one huge file cannot crowd out the others.
fn cap(files: Vec<(String, String)>, max: usize) -> String {
    let each = (max / files.len().max(1)).max(600);
    let mut out = String::new();
    for (head, body) in files {
        out.push_str(&head);
        out.push('\n');
        if body.chars().count() > each {
            out.extend(body.chars().take(each));
            out.push_str("\n[... file diff cut ...]\n");
        } else {
            out.push_str(&body);
        }
    }
    if out.chars().count() > max {
        out = out.chars().take(max).collect();
        out.push_str("\n[... diff cut ...]\n");
    }
    out
}

/// Reads the staged diff, branch and recent subjects of the repository at `dir`.
///
/// # Errors
/// Not a repository, or nothing is staged.
pub fn gather(dir: &Path, history: usize) -> Result<CommitInput, String> {
    let repo = git2::Repository::discover(dir).map_err(|e| e.to_string())?;
    let infos = get_staged_diff(&repo).map_err(|e| format!("{e:#}"))?;
    if infos.is_empty() {
        return Err("nothing is staged: stage changes first".into());
    }
    let files = infos
        .iter()
        .map(|i| {
            let path = i.display_path().to_string();
            let head = format!("--- {} ({})", path, i.status);
            if is_noise(&path) || i.status == FileStatus::Deleted && i.hunks.len() > 3 {
                return (head, "[contents left out]\n".to_string());
            }
            let mut body = String::new();
            for h in &i.hunks {
                for l in &h.lines {
                    match l {
                        DiffLine::HunkHeader(t) => body.push_str(t.trim_end()),
                        DiffLine::Context(t) => {
                            body.push(' ');
                            body.push_str(t.trim_end());
                        }
                        DiffLine::Addition(t) => {
                            body.push('+');
                            body.push_str(t.trim_end());
                        }
                        DiffLine::Deletion(t) => {
                            body.push('-');
                            body.push_str(t.trim_end());
                        }
                    }
                    body.push('\n');
                }
            }
            (head, body)
        })
        .collect();
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(str::to_string))
        .unwrap_or_default();
    let recent = if history == 0 {
        Vec::new()
    } else {
        gitkraft_core::features::commits::ops::list_commits(&repo, history)
            .map(|c| c.into_iter().map(|c| c.summary).collect())
            .unwrap_or_default()
    };
    Ok(CommitInput {
        branch,
        diff: redact(&cap(files, MAX_DIFF_CHARS)),
        recent,
    })
}

/// The ticket key in `branch`: the prefix followed by digits, e.g. `MARIO-123` in
/// `feature/MARIO-123-fix-it`.
#[must_use]
pub fn ticket_in(branch: &str, prefix: &str) -> Option<String> {
    if prefix.is_empty() {
        return None;
    }
    let lower = branch.to_ascii_lowercase();
    let start = lower.find(&prefix.to_ascii_lowercase())?;
    let after = &branch[start + prefix.len()..];
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    (!digits.is_empty())
        .then(|| format!("{}{digits}", &branch[start..start + prefix.len()]).to_ascii_uppercase())
}

/// The parts the model fills in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parts {
    /// Conventional type.
    pub kind: String,
    /// Scope.
    pub scope: String,
    /// Subject line.
    pub subject: String,
    /// Body.
    pub body: String,
}

/// Reads the model's answer: JSON (possibly fenced or with chatter around it), else plain text
/// whose first line is the subject.
#[must_use]
pub fn parse_answer(text: &str) -> Parts {
    let json = text.find('{').zip(text.rfind('}')).and_then(|(a, b)| {
        (a < b)
            .then(|| serde_json::from_str::<Value>(&text[a..=b]).ok())
            .flatten()
    });
    if let Some(v) = json {
        let s = |k: &str| v[k].as_str().unwrap_or_default().trim().to_string();
        let parts = Parts {
            kind: s("type"),
            scope: s("scope"),
            subject: s("subject"),
            body: s("body"),
        };
        if !parts.subject.is_empty() {
            return parts;
        }
    }
    let text = text.trim().trim_matches('`').trim();
    let mut lines = text.lines();
    Parts {
        subject: lines.next().unwrap_or_default().trim().to_string(),
        body: lines.collect::<Vec<_>>().join("\n").trim().to_string(),
        ..Parts::default()
    }
}

/// Lays `parts` out. `template` of `None` follows `format`.
#[must_use]
pub fn render(style: &RawStyle, parts: &Parts, branch: &str) -> String {
    let default = match style.format.as_deref() {
        Some("plain") => "{subject}\n\n{body}",
        _ => "{type}({scope}): {subject}\n\n{body}",
    };
    let template = style.template.as_deref().unwrap_or(default);
    let ticket = style
        .ticket_prefix
        .as_deref()
        .and_then(|p| ticket_in(branch, p))
        .unwrap_or_default();
    let values = [
        ("{type}", parts.kind.as_str()),
        ("{scope}", parts.scope.as_str()),
        ("{subject}", parts.subject.as_str()),
        ("{body}", parts.body.as_str()),
        ("{ticket}", ticket.as_str()),
        ("{branch}", branch),
    ];
    let mut out: Vec<String> = Vec::new();
    for line in template.lines() {
        let used: Vec<&(&str, &str)> = values.iter().filter(|(k, _)| line.contains(k)).collect();
        if !used.is_empty() && used.iter().all(|(_, v)| v.is_empty()) {
            continue;
        }
        let mut l = line.to_string();
        for (k, v) in &values {
            l = l.replace(k, v);
        }
        l = l.replace("()", "").replace("[]", "");
        out.push(
            l.trim_end()
                .trim_start_matches(':')
                .trim_start()
                .to_string(),
        );
    }
    let mut text = out.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    format!("{}\n", text.trim())
}

fn system_prompt(style: &RawStyle) -> String {
    let mut s = String::from(
        "You write git commit messages from a staged diff. Answer with one JSON object and nothing else: \
{\"type\": string, \"scope\": string, \"subject\": string, \"body\": string}. \
The subject is imperative mood, no trailing period, and says what the change does, not which files changed. \
The body explains why and any consequence a reviewer must know; leave it empty for a trivial change. \
Describe only what the diff shows.",
    );
    let max = style.max_subject.unwrap_or(72);
    s.push_str(&format!(" The subject is at most {max} characters."));
    match style.format.as_deref() {
        Some("plain") => s.push_str(" Leave type and scope empty."),
        _ => s.push_str(" The type follows Conventional Commits."),
    }
    if let Some(t) = &style.types {
        s.push_str(&format!(" Allowed types: {}.", t.join(", ")));
    }
    if let Some(sc) = &style.scopes {
        s.push_str(&format!(
            " The scope is one of: {}; or empty when none fits.",
            sc.join(", ")
        ));
    }
    if let Some(l) = &style.language {
        s.push_str(&format!(" Write in {l}."));
    }
    if let Some(i) = &style.instructions {
        s.push_str(&format!(" Project rules: {i}"));
    }
    s
}

/// The user part of the prompt, shown by `--dry-run`.
#[must_use]
pub fn user_prompt(input: &CommitInput) -> String {
    let mut s = format!("Branch: {}\n", input.branch);
    if !input.recent.is_empty() {
        s.push_str("\nRecent commit subjects of this project (match their voice):\n");
        for r in &input.recent {
            s.push_str(&format!("- {r}\n"));
        }
    }
    s.push_str("\nStaged diff:\n");
    s.push_str(&input.diff);
    s
}

/// A drafted message and what the project's rules say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// The full message, ready for the message box.
    pub message: String,
    /// Where the draft breaks a rule of the style (it is still returned).
    pub warnings: Vec<String>,
}

/// Asks `llm` for a message for `input` in `style`.
///
/// # Errors
/// Whatever the model call fails with.
pub async fn suggest(
    llm: &impl Llm,
    style: &RawStyle,
    input: &CommitInput,
) -> Result<Suggestion, AiError> {
    let answer = llm
        .complete(&system_prompt(style), &user_prompt(input))
        .await?;
    let parts = parse_answer(&answer);
    let mut warnings = Vec::new();
    let max = style.max_subject.unwrap_or(72);
    if parts.subject.chars().count() > max {
        warnings.push(format!("subject is longer than {max} characters"));
    }
    if let Some(types) = &style.types
        && !parts.kind.is_empty()
        && !types.contains(&parts.kind)
    {
        warnings.push(format!(
            "type '{}' is not one of {}",
            parts.kind,
            types.join(", ")
        ));
    }
    if let Some(scopes) = &style.scopes
        && !parts.scope.is_empty()
        && !scopes.contains(&parts.scope)
    {
        warnings.push(format!(
            "scope '{}' is not one of {}",
            parts.scope,
            scopes.join(", ")
        ));
    }
    Ok(Suggestion {
        message: render(style, &parts, &input.branch),
        warnings,
    })
}

/// Splits a message into its subject line and the body after it, for a front-end whose commit
/// input has one line.
#[must_use]
pub fn split_message(message: &str) -> (String, String) {
    let message = message.trim();
    match message.split_once('\n') {
        Some((subject, body)) => (subject.trim().to_string(), body.trim().to_string()),
        None => (message.to_string(), String::new()),
    }
}

/// The global settings folder (`<config dir>/gitkraft`), where a user-wide `ai.toml` lives.
#[must_use]
pub fn global_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("gitkraft"))
}

/// A drafted message for what is staged in the repository at `dir`, in the style of its
/// `.gitkraft/ai.toml` over the global one. Reads files and git, so call it off the UI thread.
///
/// # Errors
/// Nothing staged, a broken style file, or the model call.
pub async fn draft(
    llm: &impl Llm,
    dir: &Path,
    global: Option<&Path>,
) -> Result<Suggestion, String> {
    let style = RawStyle::load(dir, global)?;
    let input = gather(dir, style.examples_from_history.unwrap_or(8))?;
    suggest(llm, &style, &input)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::FakeLlm;

    fn style(t: &str) -> RawStyle {
        RawStyle::parse(&format!("[commit]\n{t}")).expect("style")
    }

    fn parts() -> Parts {
        Parts {
            kind: "fix".into(),
            scope: "tui".into(),
            subject: "keep the cursor on resize".into(),
            body: "The cursor jumped.".into(),
        }
    }

    #[test]
    fn the_project_file_wins_field_by_field_over_the_global_one() {
        let global = style("language = \"German\"\nmax_subject = 50\n");
        let project = style("language = \"English\"\n");
        let s = global.over(project);
        assert_eq!(s.language.as_deref(), Some("English"));
        assert_eq!(s.max_subject, Some(50));
        assert!(RawStyle::parse("[commit]\nmax_subject = \"x\"").is_err());
        assert!(RawStyle::parse(EXAMPLE).is_ok());
    }

    #[test]
    fn files_are_read_from_the_project_and_a_broken_one_names_itself() {
        let g = tempfile::tempdir().expect("dir");
        let p = tempfile::tempdir().expect("dir");
        std::fs::write(
            g.path().join("ai.toml"),
            "[commit]\nlanguage = \"German\"\n",
        )
        .expect("w");
        std::fs::create_dir(p.path().join(".gitkraft")).expect("d");
        std::fs::write(
            p.path().join(".gitkraft/ai.toml"),
            "[commit]\nformat = \"plain\"\n",
        )
        .expect("w");
        let s = RawStyle::load(p.path(), Some(g.path())).expect("load");
        assert_eq!(
            (s.language.as_deref(), s.format.as_deref()),
            (Some("German"), Some("plain"))
        );
        assert_eq!(
            RawStyle::load(g.path(), None).expect("none"),
            RawStyle::default()
        );
        std::fs::write(p.path().join(".gitkraft/ai.toml"), "[commit\n").expect("w");
        assert!(
            RawStyle::load(p.path(), None)
                .expect_err("broken")
                .contains("ai.toml")
        );
    }

    #[test]
    fn conventional_and_plain_layouts() {
        let b = "main";
        assert_eq!(
            render(&RawStyle::default(), &parts(), b),
            "fix(tui): keep the cursor on resize\n\nThe cursor jumped.\n"
        );
        let mut p = parts();
        p.scope.clear();
        p.body.clear();
        assert_eq!(
            render(&RawStyle::default(), &p, b),
            "fix: keep the cursor on resize\n"
        );
        assert_eq!(
            render(&style("format = \"plain\""), &parts(), b),
            "keep the cursor on resize\n\nThe cursor jumped.\n"
        );
    }

    #[test]
    fn a_template_with_a_ticket_drops_its_line_when_the_branch_has_none() {
        let s = style(
            "ticket_prefix = \"MARIO-\"\ntemplate = \"[{ticket}] {type}: {subject}\\n\\n{body}\\n\\nRefs: {ticket}\"",
        );
        let with = render(&s, &parts(), "feature/mario-42-cursor");
        assert!(with.starts_with("[MARIO-42] fix: keep"), "{with}");
        assert!(with.ends_with("Refs: MARIO-42\n"));
        let without = render(&s, &parts(), "main");
        assert!(!without.contains("Refs"), "{without}");
        assert!(without.starts_with("fix: keep"), "{without}");
    }

    #[test]
    fn tickets_need_digits() {
        assert_eq!(
            ticket_in("x/MARIO-123-y", "MARIO-").as_deref(),
            Some("MARIO-123")
        );
        assert_eq!(ticket_in("MARIO-x", "MARIO-"), None);
        assert_eq!(ticket_in("main", ""), None);
    }

    #[test]
    fn answers_are_read_as_json_fenced_json_or_plain_text() {
        let j = r#"{"type":"feat","scope":"gui","subject":"add a thing","body":""}"#;
        assert_eq!(parse_answer(j).subject, "add a thing");
        assert_eq!(
            parse_answer(&format!("Here you go:\n```json\n{j}\n```")).kind,
            "feat"
        );
        let p = parse_answer("add a thing\n\nbecause");
        assert_eq!(
            (p.subject.as_str(), p.body.as_str()),
            ("add a thing", "because")
        );
    }

    #[tokio::test]
    async fn a_suggestion_follows_the_template_and_flags_rule_breaks() {
        let llm = FakeLlm::saying(
            r#"{"type":"wip","scope":"web","subject":"a very long subject line that goes on and on","body":"b"}"#,
        );
        let s = style(
            "types = [\"feat\", \"fix\"]\nscopes = [\"tui\"]\nmax_subject = 20\nlanguage = \"Romanian\"\ninstructions = \"No emoji.\"",
        );
        let input = CommitInput {
            branch: "main".into(),
            diff: "--- a.rs (M)\n+x\n".into(),
            recent: vec!["fix(tui): old".into()],
        };
        let got = suggest(&llm, &s, &input).await.expect("suggestion");
        assert_eq!(got.warnings.len(), 3, "{:?}", got.warnings);
        assert!(got.message.starts_with("wip(web): a very long"));
        let (system, user) = &llm.prompts()[0];
        assert!(
            system.contains("Romanian")
                && system.contains("feat, fix")
                && system.contains("No emoji")
        );
        assert!(user.contains("fix(tui): old") && user.contains("+x"));
    }

    #[test]
    fn gather_reads_the_staged_diff_and_redacts_it() {
        let dir = tempfile::tempdir().expect("dir");
        let repo = git2::Repository::init(dir.path()).expect("init");
        assert!(
            gather(dir.path(), 5)
                .expect_err("empty")
                .contains("nothing is staged")
        );
        std::fs::write(dir.path().join("a.txt"), "token=abcdef123456\nhello\n").expect("w");
        std::fs::write(dir.path().join("Cargo.lock"), "lots\n").expect("w");
        let mut idx = repo.index().expect("index");
        idx.add_path(Path::new("a.txt")).expect("add");
        idx.add_path(Path::new("Cargo.lock")).expect("add");
        idx.write().expect("write");
        let got = gather(dir.path(), 5).expect("gather");
        assert!(got.diff.contains("+hello") && got.diff.contains("[redacted]"));
        assert!(!got.diff.contains("abcdef123456") && !got.diff.contains("lots"));
        assert!(got.diff.contains("Cargo.lock") && got.recent.is_empty());
    }

    #[test]
    fn a_message_splits_into_subject_and_body() {
        assert_eq!(
            split_message("  feat: a\n\n  why  \n"),
            ("feat: a".to_string(), "why".to_string())
        );
        assert_eq!(split_message("only"), ("only".to_string(), String::new()));
    }

    #[tokio::test]
    async fn a_draft_describes_the_staged_change_in_the_project_style() {
        let dir = tempfile::tempdir().expect("dir");
        let repo = git2::Repository::init(dir.path()).expect("init");
        std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write");
        let mut index = repo.index().expect("index");
        index.add_path(Path::new("a.txt")).expect("add");
        index.write().expect("write index");
        std::fs::create_dir_all(dir.path().join(".gitkraft")).expect("dir");
        std::fs::write(
            dir.path().join(".gitkraft/ai.toml"),
            "[commit]\nformat = \"plain\"\n",
        )
        .expect("style");
        let llm = crate::ai::FakeLlm::saying(r#"{"type":"feat","subject":"Add a","body":"why"}"#);
        let s = draft(&llm, dir.path(), None).await.expect("draft");
        assert!(s.message.starts_with("Add a"), "{}", s.message);
        assert!(
            llm.prompts()[0].1.contains("a.txt"),
            "the diff reaches the model"
        );

        // Nothing staged: an error before any model call.
        let empty = tempfile::tempdir().expect("dir");
        git2::Repository::init(empty.path()).expect("init");
        let llm = crate::ai::FakeLlm::saying("x");
        let err = draft(&llm, empty.path(), None)
            .await
            .expect_err("nothing staged");
        assert!(err.contains("nothing is staged"), "{err}");
        assert!(llm.prompts().is_empty());
    }
}
