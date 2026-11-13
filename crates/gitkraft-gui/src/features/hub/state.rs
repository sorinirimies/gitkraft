//! State and update logic of the hub screen in the desktop app.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use gitkraft_ci::annotations::{Annotation, extract};
use gitkraft_ci::log::{LogLine, first_failure, parse_log};
use gitkraft_ci::{RerunScope, RunDetail};
use gitkraft_hub::accounts::{AccountsFile, discover};
use gitkraft_hub::poller::{Engine, run_cycle};
use gitkraft_hub::pulse::Pulse;
use gitkraft_hub::{Hub, PrDetail, SectionData, SectionResult};
use gitkraft_platforms::model::{ActivityDay, Comment, ItemState, MergeMethod, ReviewVerdict};
use gitkraft_platforms::notify::{Notifier, desktop::DesktopNotifier};
use gitkraft_platforms::{CachedClient, ItemKind, RepoId, ReqwestClient};
use gitkraft_settings::dashboard::{Dashboard, reorder};
use gitkraft_settings::files;
use gitkraft_settings::notify::{
    NotifyConfig, NotifySettings, NotifyStatus, StatusInput, set_enabled, status,
};
use iced::Task;

/// The HTTP client every account shares.
pub type Client = Arc<CachedClient<ReqwestClient>>;
/// The hub over that client.
pub type Api = Hub<Client>;

/// A shared handle that can sit in a message.
#[derive(Clone)]
pub struct ApiHandle(pub Arc<Api>);

impl std::fmt::Debug for ApiHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiHandle(..)")
    }
}

/// What setup found.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The hub.
    pub api: ApiHandle,
    /// Projects to watch for notifications.
    pub watched: Vec<RepoId>,
    /// Number of accounts.
    pub accounts: usize,
}

/// Top-level views of the hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubView {
    /// Sections with list and detail.
    Dashboard,
    /// Activity graphs.
    Pulse,
    /// SSH keys of every account.
    Keys,
}

/// Tabs of the PR detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrTab {
    /// Description and reviews.
    Overview,
    /// The diff.
    Diff,
    /// Comments.
    Comments,
}

/// Messages of the hub.
#[derive(Debug, Clone)]
pub enum HubMessage {
    /// Open the hub screen.
    Open,
    /// Leave it.
    Close,
    /// Setup finished.
    Ready(Result<Setup, String>),
    /// Switch view.
    Show(HubView),
    /// Pick a section (visible index).
    SelectSection(usize),
    /// Pick a row.
    SelectRow(usize),
    /// Reload the active section.
    Refresh,
    /// A section was loaded.
    SectionLoaded {
        idx: usize,
        generation: u64,
        data: Arc<SectionData>,
    },
    /// A PR detail was loaded.
    PrLoaded {
        key: String,
        result: Result<Arc<PrDetail>, String>,
    },
    /// Issue comments were loaded.
    CommentsLoaded {
        key: String,
        result: Result<Vec<Comment>, String>,
    },
    /// A run detail was loaded.
    RunLoaded {
        key: String,
        result: Result<Arc<RunDetail>, String>,
    },
    /// Pick a PR tab.
    Tab(PrTab),
    /// Pick a job of the run.
    SelectJob(usize),
    /// Open the log of the selected job.
    OpenLog,
    /// A log arrived.
    LogLoaded(Result<(String, String), String>),
    /// Close the log.
    CloseLog,
    /// Jump to a log line.
    JumpLog(usize),
    /// The note / comment text changed.
    NoteInput(String),
    /// Post the note as a comment.
    PostComment,
    /// Approve with the note.
    Approve,
    /// Request changes with the note.
    RequestChanges,
    /// Merge.
    Merge(MergeMethod),
    /// Close or reopen.
    ToggleOpen,
    /// Re-run CI.
    Rerun(RerunScope),
    /// Cancel the run.
    CancelRun,
    /// Download the selected run's artifacts.
    Artifacts,
    /// Start the selected run's workflow again, with the note as `[WORKFLOW] REF key=value…`.
    Dispatch,
    /// An action finished.
    ActionDone(Result<String, String>),
    /// A drag of section tab `i` began.
    DragStart(usize),
    /// The pointer entered tab `i` while dragging.
    DragEnter(usize),
    /// The button was released over tab `i`.
    DragDrop(usize),
    /// The button was released elsewhere.
    DragCancel,
    /// SSH keys arrived.
    KeysLoaded(Arc<gitkraft_hub::keys::KeysData>),
    /// Pick a key row.
    SelectKey(usize),
    /// Ask before removing the selected key.
    AskRevoke,
    /// Confirmed.
    ConfirmRevoke,
    /// Not now.
    CancelRevoke,
    /// Create or reuse this machine's key for a host, register it, configure ssh.
    SetupKeys(String),
    /// Test `ssh -T git@host`.
    TestKeys(String),
    /// A key action finished.
    KeysActionDone(Result<String, String>),
    /// Pulse numbers arrived.
    PulseLoaded {
        repo: String,
        days: u32,
        data: Box<Pulse>,
    },
    /// Contributions arrived.
    ContributionsLoaded(Arc<SectionResult<ActivityDay>>),
    /// Load the pulse for another repo.
    PulseRepo(usize),
    /// Pulse period in days.
    PulseDays(u32),
    /// Switch notifications on or off for the current repo.
    ToggleBell,
    /// A timer tick (poll, refresh the bell).
    Tick,
    /// The poller finished a cycle.
    Polled,
}

/// One section's data.
#[derive(Debug, Default)]
pub struct Slot {
    /// Loaded data.
    pub data: Option<Arc<SectionData>>,
    /// A load is running.
    pub loading: bool,
    generation: u64,
}

/// The detail pane's data.
#[derive(Debug, Default)]
pub struct Detail {
    /// Key of the item shown.
    pub key: Option<String>,
    /// A PR with comments, reviews and diff.
    pub pr: Option<Arc<PrDetail>>,
    /// Issue comments.
    pub comments: Vec<Comment>,
    /// A run with jobs.
    pub run: Option<Arc<RunDetail>>,
    /// Loading.
    pub loading: bool,
    /// Failure.
    pub error: Option<String>,
    /// Selected job.
    pub job: usize,
}

/// A log being read.
#[derive(Debug)]
pub struct LogState {
    /// Title.
    pub title: String,
    /// Parsed lines.
    pub lines: Vec<LogLine>,
    /// Problems found.
    pub annotations: Vec<Annotation>,
    /// Line to highlight (from a jump).
    pub focus: Option<usize>,
}

/// Pulse view state.
#[derive(Debug, Default)]
pub struct PulseState {
    /// Repo shown.
    pub repo: Option<RepoId>,
    /// Numbers.
    pub data: Option<Pulse>,
    /// Contribution calendar.
    pub contributions: Option<Arc<SectionResult<ActivityDay>>>,
    /// Days covered.
    pub days: u32,
}

/// SSH keys view state.
#[derive(Debug, Default)]
pub struct KeysState {
    /// Loaded keys.
    pub data: Option<Arc<gitkraft_hub::keys::KeysData>>,
    /// Selected row.
    pub row: usize,
    /// A load is running.
    pub loading: bool,
    /// Waiting for a yes on removing the selected key.
    pub confirm_revoke: bool,
}

/// The bell of the current repo.
#[derive(Debug, Clone)]
pub struct Bell {
    /// Project key.
    pub key: String,
    /// Status.
    pub status: NotifyStatus,
    /// Unread inbox entries.
    pub unread: usize,
}

/// Everything the hub screen needs.
#[derive(Debug)]
pub struct HubState {
    /// Active view.
    pub view: HubView,
    /// The dashboard.
    pub dashboard: Dashboard,
    /// Data per section.
    pub slots: Vec<Slot>,
    /// Active section (visible index).
    pub active: usize,
    /// Selected row.
    pub row: Option<usize>,
    /// Detail pane.
    pub detail: Detail,
    /// PR tab.
    pub tab: PrTab,
    /// Log viewer.
    pub log: Option<LogState>,
    /// Comment / note input.
    pub note: String,
    /// Section drag: `(from, over)`.
    pub drag: Option<(usize, usize)>,
    /// Pulse view.
    pub pulse: PulseState,
    /// SSH keys view.
    pub keys: KeysState,
    /// Status line text and success flag.
    pub notice: Option<(String, bool)>,
    /// Setup finished.
    pub ready: bool,
    /// Accounts found.
    pub accounts: usize,
    /// Watched projects.
    pub watched: Vec<RepoId>,
    /// Parsed `notifications.toml`.
    pub notify_config: NotifyConfig,
    /// The bell of the current repo.
    pub bell: Option<Bell>,
    api: Option<ApiHandle>,
    engine: Arc<tokio::sync::Mutex<Engine>>,
    dashboard_file: Option<PathBuf>,
    notify_file: Option<PathBuf>,
}

impl HubState {
    /// A state without network access (tests, and before setup finishes).
    pub fn detached(dashboard: Dashboard) -> Self {
        let slots = dashboard.sections.iter().map(|_| Slot::default()).collect();
        Self {
            view: HubView::Dashboard,
            dashboard,
            slots,
            active: 0,
            row: None,
            detail: Detail::default(),
            tab: PrTab::Overview,
            log: None,
            note: String::new(),
            drag: None,
            pulse: PulseState::default(),
            keys: KeysState::default(),
            notice: None,
            ready: false,
            accounts: 0,
            watched: Vec::new(),
            notify_config: NotifyConfig::default(),
            bell: None,
            api: None,
            engine: Arc::new(tokio::sync::Mutex::new(Engine::default())),
            dashboard_file: None,
            notify_file: None,
        }
    }

    /// Opens the hub: loads the dashboard file and starts discovering accounts.
    pub fn open(repos: Vec<Vec<String>>) -> (Self, Task<HubMessage>) {
        let mut notice = None;
        let dashboard = match files::dashboard_path()
            .ok()
            .and_then(|p| files::read_optional(&p).ok().flatten())
        {
            Some(text) => Dashboard::parse(&text).unwrap_or_else(|e| {
                notice = Some(format!("dashboard.toml: {e}; showing the default"));
                Dashboard::starter()
            }),
            None => Dashboard::starter(),
        };
        let mut state = Self::detached(dashboard);
        state.dashboard_file = files::dashboard_path().ok();
        state.notify_file = files::notify_config_path().ok();
        state.notify_config = read_config(state.notify_file.as_deref());
        if let Some(n) = notice {
            state.notice = Some((n, false));
        }
        let sections: Vec<String> = state
            .dashboard
            .sections
            .iter()
            .flat_map(|s| s.repos.clone())
            .collect();
        let task = Task::perform(setup(repos, sections), HubMessage::Ready);
        (state, task)
    }

    /// Indexes into `dashboard.sections` of the shown sections.
    pub fn visible_sections(&self) -> Vec<usize> {
        self.dashboard.visible().map(|(i, _)| i).collect()
    }

    /// The active section's index in `dashboard.sections`.
    pub fn active_index(&self) -> Option<usize> {
        self.visible_sections().get(self.active).copied()
    }

    /// The active section's data.
    pub fn active_data(&self) -> Option<&SectionData> {
        self.slots.get(self.active_index()?)?.data.as_deref()
    }

    /// Repo of the selected item, else the first watched repo.
    pub fn current_repo(&self) -> Option<RepoId> {
        let row = self.row?;
        let from_row = match self.active_data()? {
            SectionData::Prs(r) => r.items.get(row).map(|p| p.repo.clone()),
            SectionData::Issues(r) => r.items.get(row).map(|i| i.repo.clone()),
            SectionData::Runs(r) => r.items.get(row).map(|x| x.repo.clone()),
            SectionData::Inbox(r) => r.items.get(row).and_then(|n| n.repo.clone()),
        };
        from_row.or_else(|| self.watched.first().cloned())
    }

    fn repo_for_bell(&self) -> Option<RepoId> {
        self.current_repo()
            .or_else(|| self.watched.first().cloned())
    }

    fn handle(&self) -> Option<Arc<Api>> {
        self.api.as_ref().map(|a| a.0.clone())
    }

    /// Updates the state; returns follow-up work.
    pub fn update(&mut self, msg: HubMessage) -> Task<HubMessage> {
        match msg {
            // Open and Close are handled by the app, which owns the screen.
            HubMessage::Open | HubMessage::Close => Task::none(),
            HubMessage::Ready(Ok(setup)) => {
                self.ready = true;
                self.accounts = setup.accounts;
                self.watched = setup.watched;
                self.api = Some(setup.api);
                if self.accounts == 0 {
                    self.notice = Some(("No accounts: set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or log in with `gh`".into(), false));
                }
                self.load_all()
            }
            HubMessage::Ready(Err(e)) => {
                self.ready = true;
                self.notice = Some((e, false));
                Task::none()
            }
            HubMessage::Show(v) => {
                self.view = v;
                if v == HubView::Pulse && self.pulse.data.is_none() {
                    return self.load_pulse(30);
                }
                if v == HubView::Keys && self.keys.data.is_none() && !self.keys.loading {
                    return self.load_keys();
                }
                Task::none()
            }
            HubMessage::SelectSection(vis) => self.select_section(vis),
            HubMessage::SelectRow(i) => {
                self.row = Some(i);
                self.log = None;
                self.on_selection()
            }
            HubMessage::Refresh => match self.active_index() {
                Some(i) => self.load_section(i),
                None => Task::none(),
            },
            HubMessage::SectionLoaded {
                idx,
                generation,
                data,
            } => {
                let Some(slot) = self.slots.get_mut(idx) else {
                    return Task::none();
                };
                if slot.generation != generation {
                    return Task::none();
                }
                slot.loading = false;
                if let Some(first) = data.errors().first() {
                    self.notice = Some((first.clone(), false));
                }
                slot.data = Some(data);
                if self.active_index() == Some(idx) {
                    let len = self.active_data().map_or(0, SectionData::len);
                    self.row =
                        self.row
                            .filter(|r| *r < len)
                            .or(if len > 0 { Some(0) } else { None });
                    self.detail.key = None;
                    return self.on_selection();
                }
                Task::none()
            }
            HubMessage::PrLoaded { key, result } => {
                if self.detail.key.as_deref() != Some(&key) {
                    return Task::none();
                }
                self.detail.loading = false;
                match result {
                    Ok(d) => {
                        if let Some(e) = d.errors.first() {
                            self.notice = Some((e.clone(), false));
                        }
                        self.detail.pr = Some(d);
                    }
                    Err(e) => self.detail.error = Some(e),
                }
                Task::none()
            }
            HubMessage::CommentsLoaded { key, result } => {
                if self.detail.key.as_deref() == Some(&key) {
                    self.detail.loading = false;
                    match result {
                        Ok(c) => self.detail.comments = c,
                        Err(e) => self.detail.error = Some(e),
                    }
                }
                Task::none()
            }
            HubMessage::RunLoaded { key, result } => {
                if self.detail.key.as_deref() == Some(&key) {
                    self.detail.loading = false;
                    match result {
                        Ok(d) => self.detail.run = Some(d),
                        Err(e) => self.detail.error = Some(e),
                    }
                }
                Task::none()
            }
            HubMessage::Tab(t) => {
                self.tab = t;
                Task::none()
            }
            HubMessage::SelectJob(i) => {
                self.detail.job = i;
                Task::none()
            }
            HubMessage::OpenLog => self.open_log(),
            HubMessage::LogLoaded(Ok((title, raw))) => {
                let lines = parse_log(&raw);
                let annotations = extract(&lines);
                let focus = first_failure(&lines);
                self.log = Some(LogState {
                    title,
                    lines,
                    annotations,
                    focus,
                });
                self.notice = None;
                Task::none()
            }
            HubMessage::LogLoaded(Err(e)) => {
                self.notice = Some((format!("log: {e}"), false));
                Task::none()
            }
            HubMessage::CloseLog => {
                self.log = None;
                Task::none()
            }
            HubMessage::JumpLog(i) => {
                if let Some(l) = &mut self.log {
                    l.focus = Some(i);
                }
                Task::none()
            }
            HubMessage::NoteInput(t) => {
                self.note = t;
                Task::none()
            }
            HubMessage::PostComment => self.post_comment(),
            HubMessage::Approve => self.review(ReviewVerdict::Approve),
            HubMessage::RequestChanges => self.review(ReviewVerdict::RequestChanges),
            HubMessage::Merge(m) => self.merge(m),
            HubMessage::ToggleOpen => self.toggle_open(),
            HubMessage::Rerun(scope) => self.rerun(scope),
            HubMessage::CancelRun => self.cancel_run(),
            HubMessage::Dispatch => self.dispatch(),
            HubMessage::Artifacts => self.artifacts(),
            HubMessage::ActionDone(result) => {
                match result {
                    Ok(text) => {
                        self.notice = Some((text, true));
                        self.note.clear();
                        self.detail.key = None;
                        return match self.active_index() {
                            Some(i) => self.load_section(i),
                            None => Task::none(),
                        };
                    }
                    Err(e) => self.notice = Some((e, false)),
                }
                Task::none()
            }
            HubMessage::DragStart(i) => {
                self.drag = Some((i, i));
                Task::none()
            }
            HubMessage::DragEnter(i) => {
                if let Some((from, _)) = self.drag {
                    self.drag = Some((from, i));
                }
                Task::none()
            }
            HubMessage::DragDrop(to) => {
                let Some((from, _)) = self.drag.take() else {
                    return Task::none();
                };
                if from == to {
                    return self.select_section(to);
                }
                if self.reorder_sections(from, to) {
                    self.notice = Some(("section moved".into(), true));
                }
                Task::none()
            }
            HubMessage::DragCancel => {
                self.drag = None;
                Task::none()
            }
            HubMessage::KeysLoaded(data) => {
                self.keys.loading = false;
                if let Some(e) = data.errors.first() {
                    self.notice = Some((e.clone(), false));
                }
                self.keys.row = self.keys.row.min(data.rows.len().saturating_sub(1));
                self.keys.data = Some(data);
                Task::none()
            }
            HubMessage::SelectKey(i) => {
                self.keys.row = i;
                self.keys.confirm_revoke = false;
                Task::none()
            }
            HubMessage::AskRevoke => {
                if self.selected_key().is_some() {
                    self.keys.confirm_revoke = true;
                } else {
                    self.notice = Some(("select a key first".into(), false));
                }
                Task::none()
            }
            HubMessage::CancelRevoke => {
                self.keys.confirm_revoke = false;
                Task::none()
            }
            HubMessage::ConfirmRevoke => {
                self.keys.confirm_revoke = false;
                let (Some(api), Some(row)) = (self.handle(), self.selected_key().cloned()) else {
                    return Task::none();
                };
                Task::perform(
                    async move { gitkraft_hub::keys::revoke(&api, &row.host, &row.key.id).await },
                    HubMessage::KeysActionDone,
                )
            }
            HubMessage::SetupKeys(host) => {
                let Some(api) = self.handle() else {
                    self.notice = Some(("no account to set up".into(), false));
                    return Task::none();
                };
                self.notice = Some((format!("setting up ssh for {host}…"), true));
                Task::perform(
                    async move {
                        let dir = ssh_dir();
                        gitkraft_hub::keys::setup_host(
                            &api,
                            &host,
                            &dir,
                            &gitkraft_platforms::ssh::SystemSsh,
                            &machine_name(),
                        )
                        .await
                        .map(|t| t.lines().collect::<Vec<_>>().join(" · "))
                    },
                    HubMessage::KeysActionDone,
                )
            }
            HubMessage::TestKeys(host) => {
                self.notice = Some((format!("testing ssh to {host}…"), true));
                Task::perform(
                    async move {
                        let (p, line) = tokio::task::spawn_blocking(move || {
                            gitkraft_hub::keys::test_host(
                                &gitkraft_platforms::ssh::SystemSsh,
                                &host,
                            )
                        })
                        .await
                        .map_err(|e| e.to_string())?;
                        if matches!(p, gitkraft_platforms::ssh::Probe::Authenticated { .. }) {
                            Ok(line)
                        } else {
                            Err(line)
                        }
                    },
                    HubMessage::KeysActionDone,
                )
            }
            HubMessage::KeysActionDone(result) => {
                match result {
                    Ok(t) => self.notice = Some((t, true)),
                    Err(e) => self.notice = Some((e, false)),
                }
                self.load_keys()
            }
            HubMessage::PulseLoaded { repo, days, data } => {
                if self.pulse.repo.as_ref().map(RepoId::key).as_deref() == Some(&repo)
                    && self.pulse.days == days
                {
                    self.pulse.data = Some(*data);
                }
                Task::none()
            }
            HubMessage::ContributionsLoaded(c) => {
                if let Some(e) = c.errors.first() {
                    self.notice = Some((e.clone(), false));
                }
                self.pulse.contributions = Some(c);
                Task::none()
            }
            HubMessage::PulseRepo(i) => {
                if let Some(r) = self.watched.get(i).cloned() {
                    self.pulse.repo = Some(r);
                    self.pulse.data = None;
                    return self.load_pulse(self.pulse.days.max(30));
                }
                Task::none()
            }
            HubMessage::PulseDays(d) => {
                self.pulse.data = None;
                self.load_pulse(d)
            }
            HubMessage::ToggleBell => self.toggle_bell(),
            HubMessage::Tick => self.tick(),
            HubMessage::Polled => {
                self.refresh_bell();
                Task::none()
            }
        }
    }

    // ── Loading ──────────────────────────────────────────────────────────────

    fn load_all(&mut self) -> Task<HubMessage> {
        let tasks: Vec<_> = self
            .visible_sections()
            .into_iter()
            .map(|i| self.load_section(i))
            .collect();
        Task::batch(tasks)
    }

    fn load_section(&mut self, idx: usize) -> Task<HubMessage> {
        let (Some(api), Some(section)) = (self.handle(), self.dashboard.sections.get(idx).cloned())
        else {
            return Task::none();
        };
        let Some(slot) = self.slots.get_mut(idx) else {
            return Task::none();
        };
        slot.loading = true;
        slot.generation += 1;
        let generation = slot.generation;
        Task::perform(
            async move { api.load_section(&section).await },
            move |data| HubMessage::SectionLoaded {
                idx,
                generation,
                data: Arc::new(data),
            },
        )
    }

    fn select_section(&mut self, vis: usize) -> Task<HubMessage> {
        let n = self.visible_sections().len();
        if n == 0 {
            return Task::none();
        }
        self.active = vis.min(n - 1);
        self.row = None;
        self.log = None;
        self.detail = Detail::default();
        let mut tasks = Vec::new();
        if let Some(idx) = self.active_index() {
            if self.slots[idx].data.is_none() && !self.slots[idx].loading {
                tasks.push(self.load_section(idx));
            } else if self.active_data().is_some_and(|d| !d.is_empty()) {
                self.row = Some(0);
                tasks.push(self.on_selection());
            }
        }
        Task::batch(tasks)
    }

    fn on_selection(&mut self) -> Task<HubMessage> {
        let (Some(api), Some(row)) = (self.handle(), self.row) else {
            self.detail = Detail::default();
            return Task::none();
        };
        match self.active_data() {
            Some(SectionData::Prs(r)) => {
                let Some(pr) = r.items.get(row).cloned() else {
                    return Task::none();
                };
                let key = format!("{}#{}", pr.repo.key(), pr.number);
                self.begin_detail(&key);
                self.tab = PrTab::Overview;
                let k = key.clone();
                Task::perform(
                    async move {
                        api.pr_detail(&pr.repo, pr.number)
                            .await
                            .map(Arc::new)
                            .map_err(|e| e.to_string())
                    },
                    move |result| HubMessage::PrLoaded {
                        key: k.clone(),
                        result,
                    },
                )
            }
            Some(SectionData::Issues(r)) => {
                let Some(issue) = r.items.get(row).cloned() else {
                    return Task::none();
                };
                let key = format!("{}#{}", issue.repo.key(), issue.number);
                self.begin_detail(&key);
                let k = key.clone();
                Task::perform(
                    async move {
                        match api.backend(&issue.repo.host) {
                            Some(b) => b
                                .comments(&issue.repo, ItemKind::Issue, issue.number)
                                .await
                                .map_err(|e| e.to_string()),
                            None => Err("no account".to_string()),
                        }
                    },
                    move |result| HubMessage::CommentsLoaded {
                        key: k.clone(),
                        result,
                    },
                )
            }
            Some(SectionData::Runs(r)) => {
                let Some(run) = r.items.get(row).cloned() else {
                    return Task::none();
                };
                let key = format!("{}!{}", run.repo.key(), run.id);
                self.begin_detail(&key);
                let k = key.clone();
                Task::perform(
                    async move {
                        match api.ci(&run.repo.host) {
                            Some(ci) => ci
                                .run_detail(&run.repo, run.id)
                                .await
                                .map(Arc::new)
                                .map_err(|e| e.to_string()),
                            None => Err("no account".to_string()),
                        }
                    },
                    move |result| HubMessage::RunLoaded {
                        key: k.clone(),
                        result,
                    },
                )
            }
            _ => {
                self.detail = Detail::default();
                Task::none()
            }
        }
    }

    fn begin_detail(&mut self, key: &str) {
        if self.detail.key.as_deref() != Some(key) {
            self.detail = Detail {
                key: Some(key.to_string()),
                loading: true,
                ..Detail::default()
            };
        }
    }

    fn open_log(&mut self) -> Task<HubMessage> {
        let (Some(api), Some(row)) = (self.handle(), self.row) else {
            return Task::none();
        };
        let Some(SectionData::Runs(r)) = self.active_data() else {
            return Task::none();
        };
        let Some(run) = r.items.get(row).cloned() else {
            return Task::none();
        };
        let Some(job) = self
            .detail
            .run
            .as_ref()
            .and_then(|d| d.jobs.get(self.detail.job))
            .cloned()
        else {
            self.notice = Some(("no job selected yet".into(), false));
            return Task::none();
        };
        let title = format!("{} · {}", run.name, job.name);
        self.notice = Some((format!("loading log of {}…", job.name), true));
        Task::perform(
            async move {
                match api.ci(&run.repo.host) {
                    Some(ci) => ci
                        .job_log(&run.repo, job.id)
                        .await
                        .map(|raw| (title, raw))
                        .map_err(|e| e.to_string()),
                    None => Err("no account".to_string()),
                }
            },
            HubMessage::LogLoaded,
        )
    }

    fn load_pulse(&mut self, days: u32) -> Task<HubMessage> {
        let Some(api) = self.handle() else {
            return Task::none();
        };
        let Some(repo) = self.pulse.repo.clone().or_else(|| self.current_repo()) else {
            self.notice = Some(("no repository to show a pulse for".into(), false));
            return Task::none();
        };
        self.pulse.repo = Some(repo.clone());
        self.pulse.days = days;
        let api2 = api.clone();
        let key = repo.key();
        let pulse = Task::perform(
            async move { api2.repo_pulse(&repo, days).await },
            move |data| HubMessage::PulseLoaded {
                repo: key.clone(),
                days,
                data: Box::new(data),
            },
        );
        let contributions = Task::perform(async move { api.contributions().await }, |c| {
            HubMessage::ContributionsLoaded(Arc::new(c))
        });
        Task::batch([pulse, contributions])
    }

    // ── SSH keys ─────────────────────────────────────────────────────────────

    /// The selected key row.
    pub fn selected_key(&self) -> Option<&gitkraft_hub::keys::KeyRow> {
        self.keys.data.as_ref()?.rows.get(self.keys.row)
    }

    /// The host to act on: the selected key's, else the first account's.
    pub fn keys_host(&self) -> Option<String> {
        self.selected_key()
            .map(|r| r.host.clone())
            .or_else(|| self.keys.data.as_ref()?.hosts.first().cloned())
    }

    fn load_keys(&mut self) -> Task<HubMessage> {
        let Some(api) = self.handle() else {
            return Task::none();
        };
        self.keys.loading = true;
        Task::perform(
            async move {
                let mine = tokio::task::spawn_blocking(|| {
                    gitkraft_hub::keys::local_fingerprints(
                        &ssh_dir(),
                        &gitkraft_platforms::ssh::SystemSsh,
                    )
                })
                .await
                .unwrap_or_default();
                Arc::new(gitkraft_hub::keys::load(&api, &mine).await)
            },
            HubMessage::KeysLoaded,
        )
    }

    // ── Section order ────────────────────────────────────────────────────────

    /// Moves the section at visible index `from` to `to` and saves the order.
    pub fn reorder_sections(&mut self, from: usize, to: usize) -> bool {
        let vis = self.visible_sections();
        let (Some(&a), Some(&b)) = (vis.get(from), vis.get(to)) else {
            return false;
        };
        if !self.dashboard.reorder(a, b) {
            return false;
        }
        reorder(&mut self.slots, a, b);
        self.active = to;
        if let Some(path) = &self.dashboard_file
            && let Ok(text) = self.dashboard.to_toml()
        {
            let _ = files::write_atomic(path, &text);
        }
        true
    }

    // ── Actions ──────────────────────────────────────────────────────────────

    fn target(&self) -> Option<(RepoId, u64, ItemKind, bool)> {
        let row = self.row?;
        match self.active_data()? {
            SectionData::Prs(r) => r.items.get(row).map(|p| {
                (
                    p.repo.clone(),
                    p.number,
                    ItemKind::PullRequest,
                    p.state == ItemState::Open,
                )
            }),
            SectionData::Issues(r) => r.items.get(row).map(|i| {
                (
                    i.repo.clone(),
                    i.number,
                    ItemKind::Issue,
                    i.state == ItemState::Open,
                )
            }),
            _ => None,
        }
    }

    fn act<F, Fut>(&mut self, f: F) -> Task<HubMessage>
    where
        F: FnOnce(Arc<Api>) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        match self.handle() {
            Some(api) => Task::perform(f(api), HubMessage::ActionDone),
            None => Task::none(),
        }
    }

    fn post_comment(&mut self) -> Task<HubMessage> {
        let body = self.note.trim().to_string();
        let Some((repo, number, kind, _)) = self.target() else {
            return Task::none();
        };
        if body.is_empty() {
            self.notice = Some(("a comment needs some text".into(), false));
            return Task::none();
        }
        self.act(move |api| async move {
            let b = api.backend(&repo.host).ok_or("no account")?;
            b.post_comment(&repo, kind, number, &body)
                .await
                .map(|()| "comment posted".to_string())
                .map_err(|e| e.to_string())
        })
    }

    fn review(&mut self, verdict: ReviewVerdict) -> Task<HubMessage> {
        let body = self.note.trim().to_string();
        let Some((repo, number, ItemKind::PullRequest, _)) = self.target() else {
            self.notice = Some(("reviews only apply to pull requests".into(), false));
            return Task::none();
        };
        if verdict == ReviewVerdict::RequestChanges && body.is_empty() {
            self.notice = Some(("requesting changes needs a note".into(), false));
            return Task::none();
        }
        self.act(move |api| async move {
            let b = api.backend(&repo.host).ok_or("no account")?;
            b.submit_review(&repo, number, verdict, &body)
                .await
                .map(|()| "review sent".to_string())
                .map_err(|e| e.to_string())
        })
    }

    fn merge(&mut self, method: MergeMethod) -> Task<HubMessage> {
        let Some((repo, number, ItemKind::PullRequest, _)) = self.target() else {
            return Task::none();
        };
        self.act(move |api| async move {
            let b = api.backend(&repo.host).ok_or("no account")?;
            b.merge(&repo, number, method)
                .await
                .map(|()| format!("#{number} merged"))
                .map_err(|e| e.to_string())
        })
    }

    fn toggle_open(&mut self) -> Task<HubMessage> {
        let Some((repo, number, kind, open)) = self.target() else {
            return Task::none();
        };
        self.act(move |api| async move {
            let b = api.backend(&repo.host).ok_or("no account")?;
            b.set_open(&repo, kind, number, !open)
                .await
                .map(|()| format!("#{number} {}", if open { "closed" } else { "reopened" }))
                .map_err(|e| e.to_string())
        })
    }

    fn selected_run(&self) -> Option<gitkraft_ci::Run> {
        let SectionData::Runs(r) = self.active_data()? else {
            return None;
        };
        r.items.get(self.row?).cloned()
    }

    fn rerun(&mut self, scope: RerunScope) -> Task<HubMessage> {
        let Some(run) = self.selected_run() else {
            return Task::none();
        };
        self.act(move |api| async move {
            let ci = api.ci(&run.repo.host).ok_or("no account")?;
            ci.rerun(&run.repo, &run, scope)
                .await
                .map(|()| "re-run started".to_string())
                .map_err(|e| e.to_string())
        })
    }

    fn artifacts(&mut self) -> Task<HubMessage> {
        let Some(run) = self.selected_run() else {
            self.notice = Some(("select a CI run first".into(), false));
            return Task::none();
        };
        let dir = dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gitkraft-artifacts");
        self.notice = Some((
            format!("downloading artifacts of {} #{}…", run.name, run.number),
            true,
        ));
        self.act(move |api| async move {
            let (saved, skipped) =
                gitkraft_hub::artifacts::download_all(&api, &run.repo, run.id, &dir).await?;
            let mut t = format!("saved {} file(s) to {}", saved.len(), dir.display());
            if !skipped.is_empty() {
                t.push_str(&format!(" · skipped: {}", skipped.join(", ")));
            }
            Ok(t)
        })
    }

    fn dispatch(&mut self) -> Task<HubMessage> {
        let Some(run) = self.selected_run() else {
            self.notice = Some(("select a CI run first".into(), false));
            return Task::none();
        };
        let needs_workflow = !run.workflow.is_empty();
        let text = if self.note.trim().is_empty() {
            if needs_workflow {
                format!("{} {}", run.workflow, run.branch)
            } else {
                run.branch.clone()
            }
        } else {
            self.note.clone()
        };
        let gitkraft_hub::dispatch::Dispatch {
            workflow,
            git_ref,
            inputs,
        } = match gitkraft_hub::dispatch::parse(&text, needs_workflow) {
            Ok(v) => v,
            Err(e) => {
                self.notice = Some((e, false));
                return Task::none();
            }
        };
        self.act(move |api| async move {
            let ci = api.ci(&run.repo.host).ok_or("no account")?;
            ci.dispatch(&run.repo, &workflow, &git_ref, &inputs)
                .await
                .map(|()| {
                    format!(
                        "started {} on {git_ref}",
                        if workflow.is_empty() {
                            "a pipeline"
                        } else {
                            &workflow
                        }
                    )
                })
                .map_err(|e| e.to_string())
        })
    }

    fn cancel_run(&mut self) -> Task<HubMessage> {
        let Some(run) = self.selected_run() else {
            return Task::none();
        };
        self.act(move |api| async move {
            let ci = api.ci(&run.repo.host).ok_or("no account")?;
            ci.cancel(&run.repo, run.id)
                .await
                .map(|()| "run cancelled".to_string())
                .map_err(|e| e.to_string())
        })
    }

    // ── Notifications ────────────────────────────────────────────────────────

    fn toggle_bell(&mut self) -> Task<HubMessage> {
        let Some(repo) = self.repo_for_bell() else {
            self.notice = Some(("no repository selected".into(), false));
            return Task::none();
        };
        let key = repo.key();
        let on = self.notify_config.resolve(&key, None).enabled;
        let result = (|| {
            let path = self
                .notify_file
                .clone()
                .ok_or("no config file in this session")?;
            let text = files::read_optional(&path)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            let edited = set_enabled(&text, Some(&key), !on)?;
            files::write_atomic(&path, &edited).map_err(|e| e.to_string())
        })();
        match result {
            Ok(()) => {
                self.notify_config = read_config(self.notify_file.as_deref());
                self.notice = Some((
                    format!("notifications {} for {key}", if on { "off" } else { "on" }),
                    true,
                ));
                self.refresh_bell();
            }
            Err(e) => self.notice = Some((format!("could not change notifications: {e}"), false)),
        }
        Task::none()
    }

    fn refresh_bell(&mut self) {
        let Some(repo) = self.repo_for_bell() else {
            self.bell = None;
            return;
        };
        let key = repo.key();
        let eff = self.notify_config.resolve(&key, None);
        let Ok(engine) = self.engine.try_lock() else {
            return;
        };
        let st = engine.status.repos.get(&key).cloned().unwrap_or_default();
        let input = StatusInput {
            enabled: eff.enabled,
            muted_until: engine.state.muted_until(&key),
            last_poll: st.last_poll,
            last_error: st.last_error,
            poller_heartbeat: engine.status.heartbeat,
            interval: eff.poll_interval,
        };
        self.bell = Some(Bell {
            unread: engine.inbox.unread_for(&key),
            status: status(&input, Utc::now()),
            key,
        });
    }

    fn tick(&mut self) -> Task<HubMessage> {
        if self.notify_file.is_some() {
            self.notify_config = read_config(self.notify_file.as_deref());
        }
        self.refresh_bell();
        let Some(api) = self.handle() else {
            return Task::none();
        };
        if self.notify_file.is_none() {
            return Task::none();
        }
        let engine = self.engine.clone();
        let watched = self.watched.clone();
        Task::perform(
            async move {
                let config = read_config(files::notify_config_path().ok().as_deref());
                let now = Utc::now();
                let mut e = engine.lock().await;
                if e.status.mode.is_empty() {
                    *e = Engine::load();
                    e.status.mode = "in-app".into();
                }
                let notifier = DesktopNotifier;
                let report = run_cycle(
                    &api,
                    &mut e,
                    &watched,
                    &config,
                    &BTreeMap::<String, NotifySettings>::new(),
                    now,
                    chrono::Local::now().time(),
                    &notifier as &dyn Notifier,
                )
                .await;
                if report.polled > 0 || report.alerts > 0 {
                    let _ = e.save();
                }
            },
            |()| HubMessage::Polled,
        )
    }
}

fn ssh_dir() -> PathBuf {
    dirs::home_dir().map_or_else(|| PathBuf::from(".ssh"), |h| h.join(".ssh"))
}

fn machine_name() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "this machine".into())
}

fn read_config(path: Option<&std::path::Path>) -> NotifyConfig {
    path.and_then(|p| files::read_optional(p).ok().flatten())
        .and_then(|t| NotifyConfig::parse(&t).ok())
        .unwrap_or_default()
}

async fn setup(repos: Vec<Vec<String>>, sections: Vec<String>) -> Result<Setup, String> {
    let built = tokio::task::spawn_blocking(move || {
        let file = files::accounts_path()
            .ok()
            .and_then(|p| files::read_optional(&p).ok().flatten())
            .map(|t| AccountsFile::parse(&t))
            .transpose()
            .map_err(|e| format!("accounts.toml: {e}"))?
            .unwrap_or_default();
        let urls: Vec<String> = repos.iter().flatten().cloned().collect();
        let accounts = discover(&urls, &file, gitkraft_platforms::auth::resolve_token);
        let mut watched: Vec<RepoId> = Vec::new();
        for url in &urls {
            if let Some(r) = gitkraft_platforms::parse_remote_url(url)
                && !watched.contains(&r)
            {
                watched.push(r);
            }
        }
        for key in &sections {
            if let Some(r) = RepoId::from_key(key)
                && !watched.contains(&r)
            {
                watched.push(r);
            }
        }
        Ok::<_, String>((accounts, watched))
    })
    .await
    .map_err(|e| e.to_string())??;
    let (accounts, watched) = built;
    let client: Client = Arc::new(CachedClient::new(
        ReqwestClient::new().map_err(|e| e.to_string())?,
    ));
    let count = accounts.len();
    Ok(Setup {
        api: ApiHandle(Arc::new(Hub::new(accounts, &client))),
        watched,
        accounts: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::model::{PullRequest, User};
    use gitkraft_settings::dashboard::{Involvement, Section, SectionKind, SectionState};

    fn section(name: &str, kind: SectionKind, hidden: bool) -> Section {
        Section {
            name: name.into(),
            kind,
            repos: if kind == SectionKind::Ci {
                vec!["github.com/o/r".into()]
            } else {
                vec![]
            },
            involvement: Some(Involvement::Authored),
            state: SectionState::Open,
            label: None,
            branch: None,
            hidden,
        }
    }

    fn state() -> HubState {
        let mut s = HubState::detached(Dashboard {
            sections: vec![
                section("Mine", SectionKind::Prs, false),
                section("Issues", SectionKind::Issues, false),
                section("CI", SectionKind::Ci, false),
            ],
        });
        s.ready = true;
        let pr = |n| PullRequest {
            repo: RepoId::new("github.com", "o", "r"),
            number: n,
            title: format!("pr {n}"),
            author: User {
                login: "a".into(),
                avatar_url: None,
            },
            ..PullRequest::default()
        };
        s.slots[0].data = Some(Arc::new(SectionData::Prs(SectionResult {
            items: vec![pr(1), pr(2), pr(3)],
            errors: vec![],
        })));
        s
    }

    #[test]
    fn dragging_a_tab_onto_another_moves_the_section_and_its_data() {
        let mut s = state();
        let _ = s.update(HubMessage::DragStart(0));
        let _ = s.update(HubMessage::DragEnter(2));
        assert_eq!(s.drag, Some((0, 2)));
        let _ = s.update(HubMessage::DragDrop(2));
        assert_eq!(
            s.dashboard
                .sections
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
            ["Issues", "CI", "Mine"]
        );
        assert_eq!(s.active, 2);
        assert!(matches!(
            s.slots[2].data.as_deref(),
            Some(SectionData::Prs(_))
        ));
        assert!(s.drag.is_none());
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t == "section moved" && *ok)
        );
    }

    #[test]
    fn releasing_on_the_same_tab_is_a_click_and_elsewhere_cancels() {
        let mut s = state();
        let _ = s.update(HubMessage::DragStart(1));
        let _ = s.update(HubMessage::DragDrop(1));
        assert_eq!(s.active, 1);
        assert_eq!(s.dashboard.sections[0].name, "Mine");
        let _ = s.update(HubMessage::DragStart(0));
        let _ = s.update(HubMessage::DragCancel);
        assert!(s.drag.is_none());
        let _ = s.update(HubMessage::DragDrop(2));
        assert_eq!(
            s.dashboard.sections[0].name, "Mine",
            "a drop without a drag does nothing"
        );
    }

    #[test]
    fn reordering_respects_hidden_sections_and_saves_when_a_file_is_set() {
        let mut s = state();
        s.dashboard.sections[1].hidden = true;
        let dir = tempfile::tempdir().expect("tmp");
        s.dashboard_file = Some(dir.path().join("dashboard.toml"));
        assert!(s.reorder_sections(0, 1));
        assert_eq!(
            s.dashboard
                .sections
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
            ["Issues", "CI", "Mine"]
        );
        let saved = Dashboard::parse(
            &std::fs::read_to_string(dir.path().join("dashboard.toml")).expect("saved"),
        )
        .expect("valid");
        assert_eq!(saved, s.dashboard);
        assert!(!s.reorder_sections(5, 0));
    }

    #[test]
    fn selecting_rows_sections_and_stale_loads() {
        let mut s = state();
        let _ = s.update(HubMessage::SelectRow(1));
        assert_eq!(s.row, Some(1));
        assert_eq!(
            s.current_repo().map(|r| r.key()),
            Some("github.com/o/r".into())
        );
        let _ = s.update(HubMessage::SelectSection(0));
        assert_eq!(
            s.row,
            Some(0),
            "the first row is selected when the section has data"
        );
        s.slots[0].generation = 3;
        let empty = Arc::new(SectionData::Prs(SectionResult {
            items: vec![],
            errors: vec![],
        }));
        let _ = s.update(HubMessage::SectionLoaded {
            idx: 0,
            generation: 2,
            data: empty.clone(),
        });
        assert_eq!(
            s.active_data().map(SectionData::len),
            Some(3),
            "stale load ignored"
        );
        let _ = s.update(HubMessage::SectionLoaded {
            idx: 0,
            generation: 3,
            data: empty,
        });
        assert_eq!(s.active_data().map(SectionData::len), Some(0));
        assert_eq!(s.row, None);
    }

    #[test]
    fn actions_without_a_selection_or_input_explain_themselves() {
        let mut s = state();
        let _ = s.update(HubMessage::PostComment);
        assert!(s.notice.is_none(), "no row selected: nothing to say yet");
        let _ = s.update(HubMessage::SelectRow(0));
        let _ = s.update(HubMessage::NoteInput("  ".into()));
        let _ = s.update(HubMessage::PostComment);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("needs some text") && !ok)
        );
        let _ = s.update(HubMessage::RequestChanges);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, _)| t.contains("needs a note"))
        );
        let _ = s.update(HubMessage::SelectSection(1));
        let _ = s.update(HubMessage::Approve);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, _)| t.contains("only apply to pull requests"))
        );
    }

    #[test]
    fn a_log_arrives_parsed_with_its_first_failure_focused() {
        let mut s = state();
        let _ = s.update(HubMessage::LogLoaded(Ok((
            "CI · build".into(),
            "ok\nerror[E0425]: no x\n --> src/a.rs:1:2".into(),
        ))));
        let log = s.log.as_ref().expect("log");
        assert_eq!(
            (log.lines.len(), log.annotations.len(), log.focus),
            (3, 1, Some(1))
        );
        let _ = s.update(HubMessage::JumpLog(2));
        assert_eq!(s.log.as_ref().and_then(|l| l.focus), Some(2));
        let _ = s.update(HubMessage::CloseLog);
        assert!(s.log.is_none());
        let _ = s.update(HubMessage::LogLoaded(Err("403".into())));
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("log: 403") && !ok)
        );
    }

    #[test]
    fn messages_for_a_different_item_are_ignored() {
        let mut s = state();
        s.detail.key = Some("a#1".into());
        s.detail.loading = true;
        let _ = s.update(HubMessage::CommentsLoaded {
            key: "a#2".into(),
            result: Err("late".into()),
        });
        assert!(s.detail.loading && s.detail.error.is_none());
        let _ = s.update(HubMessage::CommentsLoaded {
            key: "a#1".into(),
            result: Err("real".into()),
        });
        assert_eq!(s.detail.error.as_deref(), Some("real"));
    }

    #[test]
    fn the_bell_follows_the_selected_repo_and_the_config() {
        let mut s = state();
        let _ = s.update(HubMessage::SelectRow(0));
        s.refresh_bell();
        assert!(matches!(
            s.bell.as_ref().map(|b| &b.status),
            Some(NotifyStatus::Off)
        ));
        s.notify_config = NotifyConfig::parse("[defaults]\nenabled = true\n").expect("valid");
        s.refresh_bell();
        assert!(matches!(
            s.bell.as_ref().map(|b| &b.status),
            Some(NotifyStatus::NoPoller)
        ));
    }

    #[test]
    fn toggling_the_bell_edits_the_config_file() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("notifications.toml");
        let mut s = state();
        s.notify_file = Some(path.clone());
        let _ = s.update(HubMessage::SelectRow(0));
        let _ = s.update(HubMessage::ToggleBell);
        assert!(s.notify_config.resolve("github.com/o/r", None).enabled);
        assert!(
            std::fs::read_to_string(&path)
                .expect("written")
                .contains("enabled = true")
        );
        let _ = s.update(HubMessage::ToggleBell);
        assert!(!s.notify_config.resolve("github.com/o/r", None).enabled);
        let mut detached = state();
        let _ = detached.update(HubMessage::SelectRow(0));
        let _ = detached.update(HubMessage::ToggleBell);
        assert!(
            detached
                .notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("no config file") && !ok)
        );
    }

    #[test]
    fn setup_results_fill_the_state_or_explain() {
        let mut s = state();
        s.ready = false;
        let _ = s.update(HubMessage::Ready(Err("accounts.toml: bad".into())));
        assert!(
            s.ready
                && s.notice
                    .as_ref()
                    .is_some_and(|(t, ok)| t.contains("accounts.toml") && !ok)
        );
    }

    fn key_rows() -> Arc<gitkraft_hub::keys::KeysData> {
        use gitkraft_hub::keys::{KeyRow, KeysData};
        use gitkraft_platforms::model::SshKey;
        let row = |host: &str, id: &str, mine| KeyRow {
            host: host.into(),
            key: SshKey {
                id: id.into(),
                title: format!("t{id}"),
                key: String::new(),
                fingerprint: None,
                created_at: None,
            },
            mine,
        };
        Arc::new(KeysData {
            hosts: vec!["github.com".into(), "gitlab.com".into()],
            rows: vec![row("github.com", "1", true), row("gitlab.com", "2", false)],
            errors: vec!["codeberg.org: 403".into()],
        })
    }

    #[test]
    fn keys_arrive_select_and_ask_before_removal() {
        let mut s = state();
        let _ = s.update(HubMessage::AskRevoke);
        assert!(
            !s.keys.confirm_revoke
                && s.notice
                    .as_ref()
                    .is_some_and(|(t, ok)| t.contains("select a key") && !ok)
        );
        s.keys.loading = true;
        let _ = s.update(HubMessage::KeysLoaded(key_rows()));
        assert!(!s.keys.loading);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t == "codeberg.org: 403" && !ok)
        );
        let _ = s.update(HubMessage::SelectKey(1));
        assert_eq!(s.keys_host().as_deref(), Some("gitlab.com"));
        let _ = s.update(HubMessage::AskRevoke);
        assert!(s.keys.confirm_revoke);
        let _ = s.update(HubMessage::SelectKey(0));
        assert!(!s.keys.confirm_revoke, "moving away cancels the question");
        let _ = s.update(HubMessage::AskRevoke);
        let _ = s.update(HubMessage::CancelRevoke);
        assert!(!s.keys.confirm_revoke);
        let _ = s.update(HubMessage::ConfirmRevoke);
        assert!(!s.keys.confirm_revoke);
    }

    #[test]
    fn a_shorter_key_list_keeps_the_selection_in_range_and_results_show_as_notices() {
        let mut s = state();
        s.keys.row = 9;
        let _ = s.update(HubMessage::KeysLoaded(key_rows()));
        assert_eq!(s.keys.row, 1);
        let _ = s.update(HubMessage::KeysActionDone(Ok(
            "removed key 2 from gitlab.com".into(),
        )));
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.starts_with("removed") && *ok)
        );
        let _ = s.update(HubMessage::KeysActionDone(Err("403".into())));
        assert!(s.notice.as_ref().is_some_and(|(t, ok)| t == "403" && !ok));
        let _ = s.update(HubMessage::SetupKeys("github.com".into()));
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("no account") && !ok)
        );
    }

    #[test]
    fn dispatch_without_a_run_or_with_bad_text_explains() {
        let mut s = state();
        let _ = s.update(HubMessage::Dispatch);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("select a CI run") && !ok)
        );
        let run = gitkraft_ci::Run {
            id: 1,
            workflow: "ci.yml".into(),
            branch: "main".into(),
            ..Default::default()
        };
        s.slots[2].data = Some(Arc::new(SectionData::Runs(SectionResult {
            items: vec![run],
            errors: vec![],
        })));
        let _ = s.update(HubMessage::SelectSection(2));
        let _ = s.update(HubMessage::SelectRow(0));
        let _ = s.update(HubMessage::NoteInput("ci.yml".into()));
        let _ = s.update(HubMessage::Dispatch);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, ok)| t.contains("branch or tag") && !ok)
        );
        let _ = s.update(HubMessage::NoteInput("ci.yml main bad".into()));
        let _ = s.update(HubMessage::Dispatch);
        assert!(
            s.notice
                .as_ref()
                .is_some_and(|(t, _)| t.contains("key=value"))
        );
    }
}
