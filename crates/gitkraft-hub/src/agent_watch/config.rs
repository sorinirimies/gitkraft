//! `agent-watch.toml`: the jobs. No secrets in it: keys stay in the environment and the keyring.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::schedule::Schedule;
use super::select::Selector;

/// Who looks at the red CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HarnessKind {
    /// GitKraft's own model call (`GITKRAFT_AI_*`): no extra tool needed.
    #[default]
    Builtin,
    /// `pi -p`.
    Pi,
    /// The GitHub Copilot CLI, `copilot -p`.
    Copilot,
    /// Opens VS Code chat with the prompt (no headless mode: you read the answer there).
    Vscode,
    /// Opens Zed with the prompt in a file (no headless mode).
    Zed,
}

impl HarnessKind {
    /// Every harness.
    pub const ALL: [Self; 5] = [
        Self::Builtin,
        Self::Pi,
        Self::Copilot,
        Self::Vscode,
        Self::Zed,
    ];

    /// The word used in the file and on the command line.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Pi => "pi",
            Self::Copilot => "copilot",
            Self::Vscode => "vscode",
            Self::Zed => "zed",
        }
    }

    /// The harness called `s`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|h| h.name() == s || (s == "code" && *h == Self::Vscode))
    }

    /// Does it give an answer back (the editors only open)?
    #[must_use]
    pub fn headless(self) -> bool {
        !matches!(self, Self::Vscode | Self::Zed)
    }
}

/// Where a finding is announced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notify {
    /// GitKraft's inbox.
    #[serde(default = "yes")]
    pub inbox: bool,
    /// The system notification centre.
    #[serde(default = "yes")]
    pub system: bool,
}

impl Default for Notify {
    fn default() -> Self {
        Self {
            inbox: true,
            system: true,
        }
    }
}

fn yes() -> bool {
    true
}

/// One scheduled check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Name, also the report folder.
    pub name: String,
    /// Switched on.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// When it runs: `daily 08:00`, `mon-fri 07:30`, `every 6h`.
    pub schedule: String,
    /// Which repositories.
    #[serde(default)]
    pub projects: Selector,
    /// Who looks at what is red.
    #[serde(default)]
    pub harness: HarnessKind,
    /// The model the harness uses (`provider/id` for pi); the harness default when empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Replaces the built-in instructions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Where findings are announced.
    #[serde(default)]
    pub notify: Notify,
}

impl Job {
    /// A new job with the defaults: every public and private repo, daily, GitKraft's own model.
    #[must_use]
    pub fn new(name: &str, schedule: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            schedule: schedule.to_string(),
            projects: Selector::default(),
            harness: HarnessKind::default(),
            model: None,
            prompt: None,
            notify: Notify::default(),
        }
    }

    /// The parsed schedule.
    ///
    /// # Errors
    /// The schedule text is not valid.
    pub fn parsed_schedule(&self) -> Result<Schedule, String> {
        Schedule::parse(&self.schedule)
    }
}

/// The whole file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentWatch {
    /// The jobs.
    #[serde(default, rename = "job")]
    pub jobs: Vec<Job>,
}

impl AgentWatch {
    /// The file under `dir`.
    #[must_use]
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("agent-watch.toml")
    }

    /// Reads the file; a missing one is no jobs.
    ///
    /// # Errors
    /// The file exists but is not valid, or a job has a bad schedule or a duplicate name.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(Self::path(dir)) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
        };
        let me: Self = toml::from_str(&text).map_err(|e| format!("agent-watch.toml: {e}"))?;
        me.validate()?;
        Ok(me)
    }

    /// Checks names and schedules.
    ///
    /// # Errors
    /// Names the job and what is wrong.
    pub fn validate(&self) -> Result<(), String> {
        for (i, j) in self.jobs.iter().enumerate() {
            if j.name.trim().is_empty() {
                return Err("a job has no name".into());
            }
            if self.jobs[..i].iter().any(|o| o.name == j.name) {
                return Err(format!("two jobs are called `{}`", j.name));
            }
            j.parsed_schedule()
                .map_err(|e| format!("job `{}`: {e}", j.name))?;
        }
        Ok(())
    }

    /// Writes the file under `dir`.
    ///
    /// # Errors
    /// The jobs are invalid or the file cannot be written.
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        self.validate()?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(dir), text).map_err(|e| e.to_string())
    }

    /// The job called `name`.
    #[must_use]
    pub fn job(&self, name: &str) -> Option<&Job> {
        self.jobs.iter().find(|j| j.name == name)
    }

    /// The job called `name`, to change.
    pub fn job_mut(&mut self, name: &str) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.name == name)
    }

    /// Adds `job`.
    ///
    /// # Errors
    /// The name is taken or the schedule is invalid.
    pub fn add(&mut self, job: Job) -> Result<(), String> {
        if self.job(&job.name).is_some() {
            return Err(format!("a job called `{}` exists", job.name));
        }
        job.parsed_schedule()
            .map_err(|e| format!("job `{}`: {e}", job.name))?;
        self.jobs.push(job);
        Ok(())
    }

    /// Removes the job called `name`; false when there was none.
    pub fn remove(&mut self, name: &str) -> bool {
        let n = self.jobs.len();
        self.jobs.retain(|j| j.name != name);
        self.jobs.len() != n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hand_written_file_loads_with_defaults_and_round_trips() {
        let tmp = tempfile::tempdir().expect("tmp");
        std::fs::write(
            AgentWatch::path(tmp.path()),
            r#"
[[job]]
name = "public"
schedule = "mon-fri 07:30"
harness = "pi"
model = "github-copilot/claude-haiku-5.5"
[job.projects]
visibility = "public"

[[job]]
name = "work"
schedule = "every 6h"
[job.projects]
hosts = ["git.lan"]
include = ["git.lan/team/**"]
"#,
        )
        .expect("write");
        let w = AgentWatch::load(tmp.path()).expect("load");
        assert_eq!(w.jobs.len(), 2);
        assert_eq!(w.jobs[0].harness, HarnessKind::Pi);
        assert!(w.jobs[1].enabled && w.jobs[1].notify.inbox && w.jobs[1].notify.system);
        assert_eq!(w.jobs[1].harness, HarnessKind::Builtin);
        w.save(tmp.path()).expect("save");
        assert_eq!(AgentWatch::load(tmp.path()).expect("reload"), w);
    }

    #[test]
    fn bad_schedules_and_duplicate_names_are_refused_with_the_job_named() {
        let mut w = AgentWatch::default();
        w.add(Job::new("a", "daily 08:00")).expect("add");
        assert!(
            w.add(Job::new("a", "daily 09:00"))
                .is_err_and(|e| e.contains("exists"))
        );
        assert!(
            w.add(Job::new("b", "whenever"))
                .is_err_and(|e| e.contains("job `b`"))
        );
        assert!(w.remove("a") && !w.remove("a"));
        let tmp = tempfile::tempdir().expect("tmp");
        assert_eq!(
            AgentWatch::load(tmp.path()),
            Ok(AgentWatch::default()),
            "no file, no jobs"
        );
    }

    #[test]
    fn harness_names_parse_and_the_editors_have_no_headless_mode() {
        assert_eq!(HarnessKind::parse("Code"), Some(HarnessKind::Vscode));
        assert_eq!(HarnessKind::parse("zed"), Some(HarnessKind::Zed));
        assert!(HarnessKind::parse("emacs").is_none());
        assert!(HarnessKind::Pi.headless() && !HarnessKind::Vscode.headless());
    }
}
