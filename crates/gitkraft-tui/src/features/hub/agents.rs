//! The Agents view's state: scheduled CI-watching jobs, the repositories they pick from and the
//! latest reports. Everything here is plain data and file I/O on small files; the network work
//! (listing repositories, running a job) lives in [`super::state`].

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;
use gitkraft_hub::agent_watch::{AgentWatch, Job, Report, Store, due, local, next_run};
use gitkraft_platforms::model::RepoInfo;

/// How often the in-app scheduler looks at the clock.
pub const TICK: Duration = Duration::from_secs(30);
/// Reports kept in memory per job (the bar chart shows these).
pub const HISTORY: usize = 14;

/// Which list the keys act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    /// The jobs.
    #[default]
    Jobs,
    /// The repositories of the selected job.
    Repos,
}

/// State of the Agents view.
#[derive(Debug)]
pub struct AgentsState {
    /// Where `agent-watch.toml` lives (None: nowhere, nothing is saved).
    pub dir: Option<PathBuf>,
    /// Where reports live.
    pub store: Option<Store>,
    /// The jobs.
    pub cfg: AgentWatch,
    /// Why the file could not be read.
    pub load_error: Option<String>,
    /// Selected job.
    pub row: usize,
    /// Focused list.
    pub pane: Pane,
    /// Every repository the accounts can see, once loaded.
    pub repos: Option<Vec<RepoInfo>>,
    /// Accounts that could not be listed.
    pub repo_errors: Vec<String>,
    /// Selected repository (index into the shown ones).
    pub repo_row: usize,
    /// A listing is running.
    pub loading_repos: bool,
    /// The job that is running now.
    pub running: Option<String>,
    /// Run due jobs while this view's hub is open.
    pub auto: bool,
    /// Last time the scheduler looked at the clock.
    pub last_tick: Option<Instant>,
    /// Newest reports per job, newest first.
    pub history: BTreeMap<String, Vec<Report>>,
}

impl Default for AgentsState {
    fn default() -> Self {
        Self {
            dir: None,
            store: None,
            cfg: AgentWatch::default(),
            load_error: None,
            row: 0,
            pane: Pane::Jobs,
            repos: None,
            repo_errors: Vec::new(),
            repo_row: 0,
            loading_repos: false,
            running: None,
            auto: true,
            last_tick: None,
            history: BTreeMap::new(),
        }
    }
}

impl AgentsState {
    /// State for the user's own config and data directories.
    #[must_use]
    pub fn system() -> Self {
        Self {
            dir: gitkraft_hub::commit_ai::global_dir(),
            store: Store::default_dir().map(Store::new),
            ..Self::default()
        }
    }

    /// State for explicit directories (tests).
    #[must_use]
    pub fn in_dirs(config: PathBuf, reports: PathBuf) -> Self {
        Self {
            dir: Some(config),
            store: Some(Store::new(reports)),
            ..Self::default()
        }
    }

    /// Reads the jobs and the newest reports again.
    pub fn reload(&mut self) {
        if let Some(dir) = &self.dir {
            match AgentWatch::load(dir) {
                Ok(cfg) => {
                    self.cfg = cfg;
                    self.load_error = None;
                }
                Err(e) => self.load_error = Some(e),
            }
        }
        self.history = match &self.store {
            Some(s) => self
                .cfg
                .jobs
                .iter()
                .map(|j| {
                    let mut r = s.reports(&j.name);
                    r.truncate(HISTORY);
                    (j.name.clone(), r)
                })
                .collect(),
            None => BTreeMap::new(),
        };
        self.row = self.row.min(self.cfg.jobs.len().saturating_sub(1));
    }

    /// Reports nobody has opened yet that need attention.
    #[must_use]
    pub fn unseen(&self) -> usize {
        self.history
            .values()
            .filter_map(|h| h.first())
            .filter(|r| !r.seen && r.needs_you())
            .count()
    }

    /// The selected job.
    #[must_use]
    pub fn job(&self) -> Option<&Job> {
        self.cfg.jobs.get(self.row)
    }

    /// The newest report of the selected job.
    #[must_use]
    pub fn latest(&self) -> Option<&Report> {
        self.history.get(&self.job()?.name)?.first()
    }

    /// The repositories the selected job's filters and picks allow, and every other one after
    /// them, as `(repo, watched)`.
    #[must_use]
    pub fn shown_repos(&self) -> Vec<(&RepoInfo, bool)> {
        let (Some(job), Some(all)) = (self.job(), &self.repos) else {
            return Vec::new();
        };
        all.iter().map(|i| (i, job.projects.matches(i))).collect()
    }

    /// When the selected job runs next.
    #[must_use]
    pub fn next(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        next_run(self.job()?, self.store.as_ref()?, now)
    }

    /// The first enabled job that is owed a run at `now`, when the scheduler may start one.
    #[must_use]
    pub fn owed(&self, now: NaiveDateTime) -> Option<String> {
        if !self.auto || self.running.is_some() {
            return None;
        }
        let store = self.store.as_ref()?;
        due(&self.cfg, store, now).first().map(|j| j.name.clone())
    }

    /// Is it time for the scheduler to look? Remembers that it did.
    pub fn tick_due(&mut self, at: Instant) -> bool {
        if self.last_tick.is_some_and(|t| at.duration_since(t) < TICK) {
            return false;
        }
        self.last_tick = Some(at);
        true
    }

    /// Moves the job selection.
    pub fn move_job(&mut self, delta: isize) {
        let n = self.cfg.jobs.len();
        if n > 0 {
            self.row = (self.row as isize + delta).clamp(0, n as isize - 1) as usize;
            self.repo_row = 0;
        }
    }

    /// Moves the repository selection.
    pub fn move_repo(&mut self, delta: isize) {
        let n = self.repos.as_ref().map_or(0, Vec::len);
        if n > 0 {
            self.repo_row = (self.repo_row as isize + delta).clamp(0, n as isize - 1) as usize;
        }
    }

    fn save(&self) -> Result<(), String> {
        match &self.dir {
            Some(dir) => self.cfg.save(dir),
            None => Ok(()),
        }
    }

    /// Changes the selected job with `f`, saves, and returns `say(job)` as the notice.
    fn change(
        &mut self,
        f: impl FnOnce(&mut Job),
        say: impl FnOnce(&Job) -> String,
    ) -> Result<String, String> {
        let job = self
            .cfg
            .jobs
            .get_mut(self.row)
            .ok_or("no job selected: press n to create one")?;
        f(job);
        let text = say(job);
        self.save()?;
        Ok(text)
    }

    /// Switches the selected job on or off.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn toggle_enabled(&mut self) -> Result<String, String> {
        self.change(
            |j| j.enabled = !j.enabled,
            |j| format!("{} {}", j.name, if j.enabled { "on" } else { "paused" }),
        )
    }

    /// The next common schedule.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn cycle_schedule(&mut self) -> Result<String, String> {
        self.change(Job::next_schedule, |j| format!("runs {}", j.schedule))
    }

    /// The next harness.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn cycle_harness(&mut self) -> Result<String, String> {
        self.change(
            |j| j.harness = j.harness.next(),
            |j| {
                if j.harness.headless() {
                    format!("harness: {}", j.harness.name())
                } else {
                    format!(
                        "harness: {} (opens the editor, no answer comes back)",
                        j.harness.name()
                    )
                }
            },
        )
    }

    /// The next visibility filter.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn cycle_visibility(&mut self) -> Result<String, String> {
        self.change(
            |j| j.projects.visibility = j.projects.visibility.next(),
            |j| format!("watching {} repositories", j.projects.visibility.name()),
        )
    }

    /// Cycles the host filter through "all forges" and each host seen in the repository list.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn cycle_host(&mut self) -> Result<String, String> {
        let mut hosts: Vec<String> = Vec::new();
        for i in self.repos.iter().flatten() {
            if !hosts.contains(&i.repo.host) {
                hosts.push(i.repo.host.clone());
            }
        }
        self.change(
            |j| {
                let now = j
                    .projects
                    .hosts
                    .first()
                    .and_then(|h| hosts.iter().position(|x| x == h));
                j.projects.hosts = match (j.projects.hosts.is_empty(), now) {
                    (true, _) => hosts.first().cloned().into_iter().collect(),
                    (false, Some(i)) => hosts.get(i + 1).cloned().into_iter().collect(),
                    (false, None) => Vec::new(),
                };
            },
            |j| {
                if j.projects.hosts.is_empty() {
                    "watching every forge".to_string()
                } else {
                    format!("watching {}", j.projects.hosts.join(", "))
                }
            },
        )
    }

    /// Flips one of the job's flags.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn flip(&mut self, flag: Flag) -> Result<String, String> {
        self.change(
            |j| match flag {
                Flag::Forks => j.projects.forks = !j.projects.forks,
                Flag::Archived => j.projects.archived = !j.projects.archived,
                Flag::Inbox => j.notify.inbox = !j.notify.inbox,
                Flag::System => j.notify.system = !j.notify.system,
            },
            |j| {
                let (name, on) = match flag {
                    Flag::Forks => ("forks", j.projects.forks),
                    Flag::Archived => ("archived repositories", j.projects.archived),
                    Flag::Inbox => ("inbox entries", j.notify.inbox),
                    Flag::System => ("system notification", j.notify.system),
                };
                format!("{name} {}", if on { "on" } else { "off" })
            },
        )
    }

    /// Sets the model of the selected job (empty: the harness default).
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn set_model(&mut self, text: &str) -> Result<String, String> {
        let t = text.trim().to_string();
        self.change(
            |j| j.model = (!t.is_empty()).then(|| t.clone()),
            |j| format!("model: {}", j.model.as_deref().unwrap_or("harness default")),
        )
    }

    /// Creates a job with the defaults and selects it.
    ///
    /// # Errors
    /// The name is empty or taken, or the file could not be saved.
    pub fn add_job(&mut self, name: &str) -> Result<String, String> {
        self.cfg.add(Job::new(name.trim(), "daily 08:00"))?;
        self.row = self.cfg.jobs.len() - 1;
        self.save()?;
        self.reload();
        Ok(format!(
            "added {}: pick its repositories, then press r to run it",
            name.trim()
        ))
    }

    /// Deletes the selected job (its reports stay on disk).
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn delete_job(&mut self) -> Result<String, String> {
        let name = self.job().ok_or("no job selected")?.name.clone();
        self.cfg.remove(&name);
        self.save()?;
        self.reload();
        Ok(format!("removed {name}"))
    }

    /// Checks or unchecks the selected repository for the selected job.
    ///
    /// # Errors
    /// Nothing selected, the filters keep that repository out, or the file could not be saved.
    pub fn toggle_repo(&mut self) -> Result<String, String> {
        let info = self
            .repos
            .as_ref()
            .and_then(|r| r.get(self.repo_row))
            .cloned()
            .ok_or("no repository selected")?;
        let job = self.cfg.jobs.get_mut(self.row).ok_or("no job selected")?;
        let on = job.projects.toggle(&info)?;
        self.save()?;
        Ok(format!(
            "{} {}",
            info.repo.key(),
            if on { "watched" } else { "not watched" }
        ))
    }

    /// Narrows the job to exactly the repositories checked now.
    ///
    /// # Errors
    /// No job or no repository list, or the file could not be saved.
    pub fn only_checked(&mut self) -> Result<String, String> {
        let all = self
            .repos
            .clone()
            .ok_or("the repository list is still loading")?;
        let job = self.cfg.jobs.get_mut(self.row).ok_or("no job selected")?;
        let picked = job.projects.pick(&all);
        let n = picked.len();
        let keep: Vec<RepoInfo> = picked.into_iter().cloned().collect();
        job.projects.only(&keep.iter().collect::<Vec<_>>());
        self.save()?;
        Ok(format!(
            "only the {n} checked repositories; new repositories stay out until you check them"
        ))
    }

    /// Drops hand-picked repositories: whatever the filters allow.
    ///
    /// # Errors
    /// No job, or the file could not be saved.
    pub fn all_again(&mut self) -> Result<String, String> {
        self.change(
            |j| j.projects.all_again(),
            |_| "picks cleared: every repository the filters allow".into(),
        )
    }

    /// Marks the selected job's newest report as seen.
    pub fn mark_seen(&mut self) {
        let (Some(job), Some(store)) = (self.job(), &self.store) else {
            return;
        };
        let name = job.name.clone();
        if store.mark_seen(&name).is_ok()
            && let Some(r) = self.history.get_mut(&name).and_then(|h| h.first_mut())
        {
            r.seen = true;
        }
    }

    /// Text for "next run" and "last run" lines: `Mon 07:30` or `-`.
    #[must_use]
    pub fn when(t: Option<NaiveDateTime>) -> String {
        t.map_or("-".into(), |t| t.format("%a %H:%M").to_string())
    }

    /// The local start time of a report.
    #[must_use]
    pub fn started(r: &Report) -> NaiveDateTime {
        local(r.started)
    }
}

/// A switch on a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// Watch forks.
    Forks,
    /// Watch archived repositories.
    Archived,
    /// Entries in GitKraft's inbox.
    Inbox,
    /// The system notification.
    System,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::RepoId;

    fn state(tmp: &tempfile::TempDir) -> AgentsState {
        let mut s = AgentsState::in_dirs(tmp.path().join("cfg"), tmp.path().join("reports"));
        s.reload();
        s
    }

    fn repo(owner: &str, name: &str, private: bool, host: &str) -> RepoInfo {
        RepoInfo {
            repo: RepoId::new(host, owner, name),
            private,
            ..RepoInfo::default()
        }
    }

    #[test]
    fn jobs_are_created_changed_saved_and_deleted() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mut s = state(&tmp);
        assert!(s.cycle_schedule().is_err(), "no job yet");
        s.add_job("oss").expect("add");
        assert!(s.add_job("oss").is_err(), "names are unique");
        s.cycle_schedule().expect("schedule");
        s.cycle_harness().expect("harness");
        s.cycle_visibility().expect("visibility");
        s.flip(Flag::System).expect("flag");
        s.set_model("m/x").expect("model");
        s.toggle_enabled().expect("pause");
        // a fresh state reads what was saved
        let mut t = AgentsState::in_dirs(tmp.path().join("cfg"), tmp.path().join("reports"));
        t.reload();
        let j = t.job().expect("job");
        assert_eq!(
            (
                j.schedule.as_str(),
                j.harness.name(),
                j.enabled,
                j.notify.system
            ),
            ("mon-fri 07:30", "pi", false, false)
        );
        assert_eq!(j.projects.visibility.name(), "public");
        assert_eq!(j.model.as_deref(), Some("m/x"));
        t.delete_job().expect("delete");
        assert!(t.job().is_none());
        assert!(
            AgentWatch::load(&tmp.path().join("cfg"))
                .expect("file")
                .jobs
                .is_empty()
        );
    }

    #[test]
    fn the_repository_picker_checks_narrows_and_resets() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mut s = state(&tmp);
        s.add_job("j").expect("add");
        s.repos = Some(vec![
            repo("me", "a", false, "github.com"),
            repo("me", "b", true, "github.com"),
            repo("me", "c", false, "codeberg.org"),
        ]);
        assert_eq!(s.shown_repos().iter().filter(|(_, w)| *w).count(), 3);
        s.repo_row = 1;
        assert!(s.toggle_repo().expect("off").contains("not watched"));
        assert_eq!(s.shown_repos().iter().filter(|(_, w)| *w).count(), 2);
        s.only_checked().expect("only");
        s.repo_row = 1;
        s.toggle_repo().expect("on again");
        assert_eq!(s.shown_repos().iter().filter(|(_, w)| *w).count(), 3);
        s.all_again().expect("reset");
        assert!(s.job().expect("job").projects.include.is_empty());
        // hosts cycle through the ones seen, then back to all
        assert!(s.cycle_host().expect("h1").contains("github.com"));
        assert!(s.cycle_host().expect("h2").contains("codeberg.org"));
        assert!(s.cycle_host().expect("h3").contains("every forge"));
    }

    #[test]
    fn a_filtered_out_repository_explains_instead_of_changing_the_file() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mut s = state(&tmp);
        s.add_job("j").expect("add");
        s.cycle_visibility().expect("public");
        s.repos = Some(vec![repo("me", "secret", true, "github.com")]);
        assert!(s.toggle_repo().is_err_and(|e| e.contains("public")));
        assert!(s.job().expect("job").projects.include.is_empty());
    }

    #[test]
    fn the_scheduler_looks_at_the_clock_twice_a_minute_and_skips_paused_jobs() {
        let tmp = tempfile::tempdir().expect("tmp");
        let mut s = state(&tmp);
        s.add_job("j").expect("add");
        let now = NaiveDateTime::parse_from_str("2026-03-03 09:00", "%Y-%m-%d %H:%M").expect("t");
        assert_eq!(s.owed(now).as_deref(), Some("j"), "never ran: owed");
        s.running = Some("j".into());
        assert!(s.owed(now).is_none(), "one run at a time");
        s.running = None;
        s.auto = false;
        assert!(s.owed(now).is_none(), "auto off");
        s.auto = true;
        s.toggle_enabled().expect("pause");
        assert!(s.owed(now).is_none(), "paused");
        let t0 = Instant::now();
        assert!(s.tick_due(t0));
        assert!(!s.tick_due(t0 + Duration::from_secs(5)));
        assert!(s.tick_due(t0 + TICK));
    }
}
