//! The hub screen's state and everything that changes it: selection, section
//! order, background loading, actions and the in-app notification poller.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use chrono::{Local, Utc};
use gitkraft_ci::{RerunScope, Run, RunDetail};
use gitkraft_hub::accounts::{AccountsFile, discover};
use gitkraft_hub::poller::{Engine, run_cycle};
use gitkraft_hub::pulse::Pulse;
use gitkraft_hub::{Hub, PrDetail, SectionData, SectionResult};
use gitkraft_platforms::model::{
    ActivityDay, Comment, Issue, ItemState, MergeMethod, Notification, PullRequest, ReviewVerdict,
};
use gitkraft_platforms::notify::{Notifier, desktop::DesktopNotifier};
use gitkraft_platforms::{CachedClient, ItemKind, RepoId, ReqwestClient};
use gitkraft_settings::dashboard::{Dashboard, reorder};
use gitkraft_settings::files;
use gitkraft_settings::notify::{
    NotifyConfig, NotifySettings, NotifyStatus, StatusInput, set_enabled, status,
};
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;

use super::drag::{Drag, DragOutcome, DragTarget};
use super::images::{Images, decode};
use super::list;
use super::logview::LogView;

/// The HTTP client every account shares.
pub type Client = Arc<CachedClient<ReqwestClient>>;
/// The hub over that client.
pub type Api = Hub<Client>;

/// Top-level views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Sections with list and detail.
    Dashboard,
    /// Activity graphs.
    Pulse,
    /// SSH keys of every account.
    Keys,
}

/// Which pane keys act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// The item list.
    List,
    /// The detail pane.
    Detail,
}

/// Tabs of the PR detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    /// Description, reviews, checks.
    Overview,
    /// The diff.
    Diff,
    /// Comments.
    Comments,
}

impl DetailTab {
    /// The next tab, wrapping.
    pub fn next(self) -> Self {
        match self {
            Self::Overview => Self::Diff,
            Self::Diff => Self::Comments,
            Self::Comments => Self::Overview,
        }
    }
}

/// Something that reacts to the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A view tab.
    ViewTab(View),
    /// A section tab (visible index).
    Section(usize),
    /// A list row (absolute index).
    Row(usize),
    /// The divider between list and detail.
    Divider,
    /// The notification bell.
    Bell,
    /// A PR detail tab.
    DetailTab(DetailTab),
    /// A job row in a run's detail.
    Job(usize),
    /// The list area (for the scroll wheel).
    ListArea,
    /// The detail area (for the scroll wheel).
    DetailArea,
    /// A row of the log viewer (visible index).
    LogRow(usize),
    /// A row of the annotation list.
    Annotation(usize),
}

/// A clickable region of the last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// Screen area.
    pub rect: Rect,
    /// What it does.
    pub target: Target,
}

/// One dashboard section's data.
#[derive(Debug, Default)]
pub struct Slot {
    /// Loaded data, if any.
    pub data: Option<SectionData>,
    /// A load is running.
    pub loading: bool,
    generation: u64,
}

/// What the detail pane holds.
#[derive(Debug, Default)]
pub struct Detail {
    /// Key of the item shown (`repo#number` or `repo run id`).
    pub key: Option<String>,
    /// PR with comments, reviews, diff.
    pub pr: Option<PrDetail>,
    /// Issue comments.
    pub comments: Vec<Comment>,
    /// Run with jobs.
    pub run: Option<RunDetail>,
    /// A request is running.
    pub loading: bool,
    /// Why it failed.
    pub error: Option<String>,
    /// Active PR tab.
    pub tab: Option<DetailTab>,
    /// Vertical scroll of the text.
    pub scroll: u16,
    /// Selected job of a run.
    pub job: usize,
    /// Cursor line of the diff tab (index into the diff's lines).
    pub diff_cursor: usize,
    /// Rows the text area had when last drawn.
    pub body_rows: u16,
}

/// Text prompt (comment, review body).
#[derive(Debug, Clone)]
pub struct Prompt {
    /// What the text is for.
    pub purpose: PromptPurpose,
    /// What was typed.
    pub buffer: String,
}

/// Reasons for a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptPurpose {
    /// A comment on the item.
    Comment,
    /// Approve with an optional note.
    Approve,
    /// Request changes with a note.
    RequestChanges,
    /// Search text in the log viewer.
    LogSearch,
    /// Start a workflow: `WORKFLOW REF [key=value …]`.
    Dispatch,
    /// A comment on the diff line in [`HubState::inline_anchor`].
    Inline,
}

/// Actions that ask for confirmation first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Merge the PR.
    Merge(MergeMethod),
    /// Close an open item or reopen a closed one.
    ToggleOpen,
    /// Re-run CI.
    Rerun(RerunScope),
    /// Cancel a CI run.
    CancelRun,
    /// Remove the selected SSH key from its account.
    RevokeKey,
}

/// A pending confirmation.
#[derive(Debug, Clone)]
pub struct Confirm {
    /// What will happen.
    pub action: Action,
    /// The question.
    pub text: String,
}

/// State of the Pulse view.
#[derive(Debug, Default)]
pub struct PulseState {
    /// Repo the numbers are for.
    pub repo: Option<RepoId>,
    /// The numbers.
    pub data: Option<Pulse>,
    /// Contribution calendar over all accounts.
    pub contributions: Option<SectionResult<ActivityDay>>,
    /// Days covered by `data`.
    pub days: u32,
    /// Something is loading.
    pub loading: bool,
}

/// State of the SSH keys view.
#[derive(Debug, Default)]
pub struct KeysState {
    /// What was loaded.
    pub data: Option<gitkraft_hub::keys::KeysData>,
    /// Selected row.
    pub row: usize,
    /// A load is running.
    pub loading: bool,
}

/// A repo to watch, with its local checkout when there is one.
#[derive(Debug, Clone)]
pub struct Watched {
    /// The project.
    pub repo: RepoId,
    /// Checkout, for the repo's own `.gitkraft/notifications.toml`.
    pub path: Option<PathBuf>,
}

enum Msg {
    Ready {
        api: Arc<Api>,
        client: Client,
        watched: Vec<Watched>,
        accounts: usize,
    },
    Image {
        url: String,
        result: Result<image::DynamicImage, String>,
    },
    SetupFailed(String),
    Section {
        idx: usize,
        generation: u64,
        data: SectionData,
    },
    Pr {
        key: String,
        result: Result<PrDetail, String>,
    },
    Run {
        key: String,
        result: Result<RunDetail, String>,
    },
    Comments {
        key: String,
        result: Result<Vec<Comment>, String>,
    },
    Log {
        title: String,
        result: Result<String, String>,
    },
    Pulse {
        repo: String,
        days: u32,
        data: Pulse,
    },
    Contributions(SectionResult<ActivityDay>),
    Keys(gitkraft_hub::keys::KeysData),
    Done {
        text: String,
        ok: bool,
        reload: bool,
    },
}

/// Everything the hub screen needs.
#[derive(Debug)]
pub struct HubState {
    /// Active view.
    pub view: View,
    /// Focused pane.
    pub focus: Focus,
    /// The dashboard being shown.
    pub dashboard: Dashboard,
    /// Data per section, parallel to `dashboard.sections`.
    pub slots: Vec<Slot>,
    /// Active section (visible index).
    pub active: usize,
    /// Selected row of the active section.
    pub row: usize,
    /// First visible row.
    pub offset: usize,
    /// List pane width in percent.
    pub list_percent: u16,
    /// Detail pane content.
    pub detail: Detail,
    /// Log viewer, when open.
    pub log: Option<LogView>,
    /// Text prompt, when open.
    pub prompt: Option<Prompt>,
    /// Confirmation, when asked.
    pub confirm: Option<Confirm>,
    /// Drag state.
    pub drag: Drag,
    /// Hit regions of the last frame.
    pub hits: Vec<Hit>,
    /// The last left click, to recognise a double click.
    pub last_click: Option<(Instant, Target)>,
    /// Pulse view.
    pub pulse: PulseState,
    /// SSH keys view.
    pub keys: KeysState,
    /// The command palette, while open.
    pub palette: Option<super::palette::Palette>,
    /// Where the pending inline comment goes: `(path, line)`.
    pub inline_anchor: Option<(String, u32)>,
    /// Image overlay.
    pub images: Images,
    /// Terminal graphics protocol for images.
    pub picker: Picker,
    /// Message for the status line.
    pub notice: Option<(String, bool, Instant)>,
    /// Setup finished: accounts are ready.
    pub ready: bool,
    /// Number of accounts found.
    pub accounts: usize,
    /// Watched projects.
    pub watched: Vec<Watched>,
    /// Shared poller state (bell, inbox).
    pub engine: Arc<tokio::sync::Mutex<Engine>>,
    /// Parsed `notifications.toml`, refreshed every few seconds.
    pub notify_config: NotifyConfig,
    config_loaded: Instant,
    /// Where the dashboard order is saved; `None` in tests.
    dashboard_file: Option<PathBuf>,
    /// The notification config file; `None` in tests.
    notify_file: Option<PathBuf>,
    /// Save the list width to the TUI settings.
    save_layout_on_resize: bool,
    api: Option<Arc<Api>>,
    client: Option<Client>,
    runtime: Option<tokio::runtime::Runtime>,
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Msg>,
}

impl Drop for HubState {
    fn drop(&mut self) {
        // Do not wait for in-flight requests when the user leaves.
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

const CONFIG_REFRESH: Duration = Duration::from_secs(3);
const NOTICE_FOR: Duration = Duration::from_secs(6);

impl HubState {
    /// A state with no network: for tests and before setup finishes.
    pub fn detached(dashboard: Dashboard) -> Self {
        let (tx, rx) = mpsc::channel();
        let slots = dashboard.sections.iter().map(|_| Slot::default()).collect();
        Self {
            view: View::Dashboard,
            focus: Focus::List,
            dashboard,
            slots,
            active: 0,
            row: 0,
            offset: 0,
            list_percent: 45,
            detail: Detail::default(),
            log: None,
            prompt: None,
            confirm: None,
            drag: Drag::default(),
            hits: Vec::new(),
            last_click: None,
            pulse: PulseState::default(),
            keys: KeysState::default(),
            palette: None,
            inline_anchor: None,
            images: Images::default(),
            picker: Picker::halfblocks(),
            client: None,
            notice: None,
            ready: false,
            accounts: 0,
            watched: Vec::new(),
            engine: Arc::new(tokio::sync::Mutex::new(Engine::default())),
            notify_config: NotifyConfig::default(),
            config_loaded: Instant::now(),
            dashboard_file: None,
            notify_file: None,
            save_layout_on_resize: false,
            api: None,
            runtime: None,
            tx,
            rx,
        }
    }

    /// Opens the hub: loads the dashboard file, then discovers accounts in the background.
    pub fn open(repos: Vec<(PathBuf, Vec<String>)>) -> Self {
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
        // Ask the terminal which graphics protocol it speaks (once, in raw mode, before events are read).
        state.picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        state.dashboard_file = files::dashboard_path().ok();
        state.notify_file = files::notify_config_path().ok();
        state.save_layout_on_resize = true;
        if let Some(msg) = notice {
            state.set_notice(msg, false);
        }
        if let Some(pct) = gitkraft_settings::persistence::load_tui_settings()
            .ok()
            .and_then(|s| s.layout)
            .and_then(|l| l.hub_list_percent)
        {
            state.list_percent = (pct as u16).clamp(20, 80);
        }
        state.notify_config = load_notify_config_from(state.notify_file.as_deref());
        state.runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("gitkraft-hub")
            .enable_all()
            .build()
            .ok();
        if state.runtime.is_none() {
            state.set_notice("could not start the network runtime".into(), false);
            return state;
        }
        state.spawn_setup(repos);
        state
    }

    // ── Setup ────────────────────────────────────────────────────────────────

    fn handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime.as_ref().map(|r| r.handle().clone())
    }

    fn spawn_setup(&mut self, repos: Vec<(PathBuf, Vec<String>)>) {
        let Some(rt) = self.handle() else { return };
        let tx = self.tx.clone();
        let sections: Vec<String> = self
            .dashboard
            .sections
            .iter()
            .flat_map(|s| s.repos.clone())
            .collect();
        rt.spawn(async move {
            // Token lookup may run `gh`, so keep it off the async workers.
            let built = tokio::task::spawn_blocking(move || {
                let file = files::accounts_path()
                    .ok()
                    .and_then(|p| files::read_optional(&p).ok().flatten())
                    .map(|t| AccountsFile::parse(&t))
                    .transpose()?
                    .unwrap_or_default();
                let urls: Vec<String> = repos.iter().flat_map(|(_, u)| u.clone()).collect();
                let accounts = discover(&urls, &file, gitkraft_platforms::auth::resolve_token);
                let mut watched: Vec<Watched> = Vec::new();
                for (path, urls) in &repos {
                    for url in urls {
                        if let Some(repo) = gitkraft_platforms::parse_remote_url(url)
                            && !watched.iter().any(|w| w.repo == repo)
                        {
                            watched.push(Watched {
                                repo,
                                path: Some(path.clone()),
                            });
                        }
                    }
                }
                for key in &sections {
                    if let Some(repo) = RepoId::from_key(key)
                        && !watched.iter().any(|w| w.repo == repo)
                    {
                        watched.push(Watched { repo, path: None });
                    }
                }
                Ok::<_, String>((accounts, watched))
            })
            .await;
            let (accounts, watched) = match built {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => {
                    return drop(tx.send(Msg::SetupFailed(format!("accounts.toml: {e}"))));
                }
                Err(e) => return drop(tx.send(Msg::SetupFailed(e.to_string()))),
            };
            let client = match ReqwestClient::new() {
                Ok(c) => Arc::new(CachedClient::new(c)),
                Err(e) => return drop(tx.send(Msg::SetupFailed(e.to_string()))),
            };
            let count = accounts.len();
            let api = Arc::new(Hub::new(accounts, &client));
            let _ = tx.send(Msg::Ready {
                api,
                client,
                watched,
                accounts: count,
            });
        });
    }

    fn spawn_poller(&self, api: Arc<Api>, watched: Vec<Watched>) {
        let Some(rt) = self.handle() else { return };
        let engine = self.engine.clone();
        rt.spawn(async move {
            {
                let mut e = engine.lock().await;
                *e = Engine::load();
                e.status.mode = "in-app".into();
            }
            let notifier = DesktopNotifier;
            loop {
                let config = load_notify_config();
                let repos: Vec<RepoId> = watched.iter().map(|w| w.repo.clone()).collect();
                let locals: BTreeMap<String, NotifySettings> = watched
                    .iter()
                    .filter_map(|w| {
                        let text = files::read_optional(
                            &w.path
                                .as_ref()?
                                .join(".gitkraft")
                                .join("notifications.toml"),
                        )
                        .ok()??;
                        let s = NotifySettings::parse(&text).ok()?;
                        Some((w.repo.key(), s))
                    })
                    .collect();
                let now = Utc::now();
                let mut e = engine.lock().await;
                let report = run_cycle(
                    &api,
                    &mut e,
                    &repos,
                    &config,
                    &locals,
                    now,
                    Local::now().time(),
                    &notifier as &dyn Notifier,
                )
                .await;
                if report.polled > 0 || report.alerts > 0 {
                    let _ = e.save();
                } else if e.status.repos.is_empty() {
                    // Nothing enabled: still tell the UIs a poller is alive.
                    let _ = files::write_atomic(
                        &files::notify_status_path().unwrap_or_default(),
                        &e.status.to_json(),
                    );
                }
                drop(e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }

    // ── Selection and sections ───────────────────────────────────────────────

    /// Indexes into `dashboard.sections` of the shown sections.
    pub fn visible_sections(&self) -> Vec<usize> {
        self.dashboard.visible().map(|(i, _)| i).collect()
    }

    /// The active section's index in `dashboard.sections`.
    pub fn active_index(&self) -> Option<usize> {
        self.visible_sections().get(self.active).copied()
    }

    /// Data of the active section.
    pub fn active_data(&self) -> Option<&SectionData> {
        self.slots.get(self.active_index()?)?.data.as_ref()
    }

    /// Number of rows in the active section.
    pub fn row_count(&self) -> usize {
        self.active_data().map_or(0, SectionData::len)
    }

    /// Switches to section `vis` (visible index) and loads it if needed.
    pub fn select_section(&mut self, vis: usize) {
        let n = self.visible_sections().len();
        if n == 0 {
            return;
        }
        self.active = vis.min(n - 1);
        self.row = 0;
        self.offset = 0;
        self.focus = Focus::List;
        self.detail = Detail::default();
        if let Some(idx) = self.active_index()
            && self.slots[idx].data.is_none()
            && !self.slots[idx].loading
        {
            self.load_section(idx);
        }
        self.on_selection_changed();
    }

    /// Next section, wrapping.
    pub fn next_section(&mut self, delta: isize) {
        let n = self.visible_sections().len();
        if n > 0 {
            self.select_section((self.active as isize + delta).rem_euclid(n as isize) as usize);
        }
    }

    /// Moves the selected row.
    pub fn move_row(&mut self, delta: isize) {
        self.set_row(list::step(self.row, self.row_count(), delta));
    }

    /// Selects row `row`.
    pub fn set_row(&mut self, row: usize) {
        let row = row.min(self.row_count().saturating_sub(1));
        if row != self.row || self.detail.key.is_none() {
            self.row = row;
            self.detail.scroll = 0;
            self.detail.diff_cursor = 0;
            self.detail.job = 0;
            self.on_selection_changed();
        }
    }

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
        self.save_dashboard();
        true
    }

    fn save_dashboard(&mut self) {
        let Some(path) = self.dashboard_file.clone() else {
            return;
        };
        let result = self
            .dashboard
            .to_toml()
            .and_then(|t| files::write_atomic(&path, &t).map_err(|e| e.to_string()));
        if let Err(e) = result {
            self.set_notice(format!("could not save the section order: {e}"), false);
        }
    }

    /// Applies a finished drag.
    pub fn apply_drag(&mut self, outcome: DragOutcome, area: Rect) {
        match outcome {
            DragOutcome::Click(DragTarget::Section(i)) => self.select_section(i),
            DragOutcome::Click(DragTarget::Divider)
            | DragOutcome::None
            | DragOutcome::Cancelled => {}
            DragOutcome::Drop { from, to } => {
                if self.reorder_sections(from, to) {
                    self.set_notice("section moved".into(), true);
                }
            }
            DragOutcome::Resized { x } => {
                self.list_percent = list::percent_at(x, area.x, area.width);
                self.save_layout();
            }
        }
    }

    /// Live resize while the divider is dragged.
    pub fn resize_to(&mut self, x: u16, area: Rect) {
        self.list_percent = list::percent_at(x, area.x, area.width);
    }

    fn save_layout(&mut self) {
        if !self.save_layout_on_resize {
            return;
        }
        let pct = f32::from(self.list_percent);
        let _ = gitkraft_settings::persistence::update_tui_settings(|s| {
            s.layout
                .get_or_insert_with(Default::default)
                .hub_list_percent = Some(pct);
        });
    }

    // ── What is selected ─────────────────────────────────────────────────────

    /// The selected PR.
    pub fn selected_pr(&self) -> Option<&PullRequest> {
        match self.active_data()? {
            SectionData::Prs(r) => r.items.get(self.row),
            _ => None,
        }
    }

    /// The selected issue.
    pub fn selected_issue(&self) -> Option<&Issue> {
        match self.active_data()? {
            SectionData::Issues(r) => r.items.get(self.row),
            _ => None,
        }
    }

    /// The selected CI run.
    pub fn selected_run(&self) -> Option<&Run> {
        match self.active_data()? {
            SectionData::Runs(r) => r.items.get(self.row),
            _ => None,
        }
    }

    /// The selected notification.
    pub fn selected_note(&self) -> Option<&Notification> {
        match self.active_data()? {
            SectionData::Inbox(r) => r.items.get(self.row),
            _ => None,
        }
    }

    /// Repo of the selected item, else the first watched repo.
    pub fn current_repo(&self) -> Option<RepoId> {
        self.selected_pr()
            .map(|p| p.repo.clone())
            .or_else(|| self.selected_issue().map(|i| i.repo.clone()))
            .or_else(|| self.selected_run().map(|r| r.repo.clone()))
            .or_else(|| self.selected_note().and_then(|n| n.repo.clone()))
            .or_else(|| self.watched.first().map(|w| w.repo.clone()))
    }

    /// Web URL of the selected item.
    pub fn selected_url(&self) -> Option<String> {
        self.selected_pr()
            .map(|p| p.url.clone())
            .or_else(|| self.selected_issue().map(|i| i.url.clone()))
            .or_else(|| self.selected_run().map(|r| r.url.clone()))
            .or_else(|| self.selected_note().map(|n| n.url.clone()))
            .filter(|u| !u.is_empty())
    }

    // ── Loading ──────────────────────────────────────────────────────────────

    /// Loads (or reloads) section `idx`.
    pub fn load_section(&mut self, idx: usize) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let Some(section) = self.dashboard.sections.get(idx).cloned() else {
            return;
        };
        let Some(slot) = self.slots.get_mut(idx) else {
            return;
        };
        slot.loading = true;
        slot.generation += 1;
        let generation = slot.generation;
        let tx = self.tx.clone();
        rt.spawn(async move {
            let data = api.load_section(&section).await;
            let _ = tx.send(Msg::Section {
                idx,
                generation,
                data,
            });
        });
    }

    /// Reloads every visible section.
    pub fn load_all(&mut self) {
        for idx in self.visible_sections() {
            self.load_section(idx);
        }
    }

    fn on_selection_changed(&mut self) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let tx = self.tx.clone();
        if let Some(pr) = self.selected_pr().cloned() {
            let key = format!("{}#{}", pr.repo.key(), pr.number);
            self.begin_detail(&key);
            self.detail.tab = Some(DetailTab::Overview);
            rt.spawn(async move {
                let result = api
                    .pr_detail(&pr.repo, pr.number)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx.send(Msg::Pr { key, result });
            });
        } else if let Some(issue) = self.selected_issue().cloned() {
            let key = format!("{}#{}", issue.repo.key(), issue.number);
            self.begin_detail(&key);
            rt.spawn(async move {
                let result = match api.backend(&issue.repo.host) {
                    Some(b) => b
                        .comments(&issue.repo, ItemKind::Issue, issue.number)
                        .await
                        .map_err(|e| e.to_string()),
                    None => Err("no account".to_string()),
                };
                let _ = tx.send(Msg::Comments { key, result });
            });
        } else if let Some(run) = self.selected_run().cloned() {
            let key = format!("{}!{}", run.repo.key(), run.id);
            self.begin_detail(&key);
            rt.spawn(async move {
                let result = match api.ci(&run.repo.host) {
                    Some(ci) => ci
                        .run_detail(&run.repo, run.id)
                        .await
                        .map_err(|e| e.to_string()),
                    None => Err("no account".to_string()),
                };
                let _ = tx.send(Msg::Run { key, result });
            });
        } else {
            self.detail = Detail::default();
        }
    }

    fn begin_detail(&mut self, key: &str) {
        if self.detail.key.as_deref() == Some(key) {
            return;
        }
        self.detail = Detail {
            key: Some(key.to_string()),
            loading: true,
            ..Detail::default()
        };
    }

    /// Opens the log of the selected job of the shown run.
    pub fn open_job_log(&mut self) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let Some(run) = self.selected_run().cloned() else {
            return;
        };
        let Some(job) = self
            .detail
            .run
            .as_ref()
            .and_then(|d| d.jobs.get(self.detail.job))
            .cloned()
        else {
            self.set_notice("no job selected yet".into(), false);
            return;
        };
        let tx = self.tx.clone();
        let title = format!("{} · {}", run.name, job.name);
        self.set_notice(format!("loading log of {}…", job.name), true);
        rt.spawn(async move {
            let result = match api.ci(&run.repo.host) {
                Some(ci) => ci
                    .job_log(&run.repo, job.id)
                    .await
                    .map_err(|e| e.to_string()),
                None => Err("no account".to_string()),
            };
            let _ = tx.send(Msg::Log { title, result });
        });
    }

    /// Loads the Pulse view's numbers for the current repo.
    pub fn load_pulse(&mut self, days: u32) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let Some(repo) = self.pulse.repo.clone().or_else(|| self.current_repo()) else {
            self.set_notice("no repository to show a pulse for".into(), false);
            return;
        };
        self.pulse.loading = true;
        self.pulse.repo = Some(repo.clone());
        self.pulse.days = days;
        let tx = self.tx.clone();
        let api2 = api.clone();
        rt.spawn(async move {
            let data = api2.repo_pulse(&repo, days).await;
            let _ = tx.send(Msg::Pulse {
                repo: repo.key(),
                days,
                data,
            });
        });
        let tx = self.tx.clone();
        rt.spawn(async move {
            let c = api.contributions().await;
            let _ = tx.send(Msg::Contributions(c));
        });
    }

    // ── Artifacts ────────────────────────────────────────────────────────────

    /// Downloads the selected run's artifacts into the downloads folder.
    pub fn download_artifacts(&mut self) {
        let Some(run) = self.selected_run().cloned() else {
            self.set_notice("select a CI run first".into(), false);
            return;
        };
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let dir = dirs::download_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gitkraft-artifacts");
        self.set_notice(
            format!("downloading artifacts of {} #{}…", run.name, run.number),
            true,
        );
        let tx = self.tx.clone();
        rt.spawn(async move {
            let (text, ok) =
                match gitkraft_hub::artifacts::download_all(&api, &run.repo, run.id, &dir).await {
                    Ok((saved, skipped)) => {
                        let mut t = format!("saved {} file(s) to {}", saved.len(), dir.display());
                        if !skipped.is_empty() {
                            t.push_str(&format!(" · skipped: {}", skipped.join(", ")));
                        }
                        (t, true)
                    }
                    Err(e) => (e, false),
                };
            let _ = tx.send(Msg::Done {
                text,
                ok,
                reload: false,
            });
        });
    }

    // ── Inline review comments ───────────────────────────────────────────────

    /// Moves the diff cursor.
    pub fn move_diff_cursor(&mut self, delta: isize) {
        let n = self
            .detail
            .pr
            .as_ref()
            .map_or(0, |d| d.diff.lines().count());
        self.detail.diff_cursor = super::list::step(self.detail.diff_cursor, n, delta);
    }

    /// Opens a prompt for a comment on the diff line under the cursor.
    pub fn open_inline_prompt(&mut self) {
        let diff = self
            .detail
            .pr
            .as_ref()
            .map(|d| d.diff.as_str())
            .unwrap_or_default();
        let pos = gitkraft_platforms::diffpos::positions(diff);
        let Some((path, line)) = pos.get(self.detail.diff_cursor).and_then(|p| p.anchor()) else {
            self.set_notice(
                "move the cursor (j/k) onto an added or unchanged line first".into(),
                false,
            );
            return;
        };
        self.inline_anchor = Some((path.to_string(), line));
        self.prompt = Some(Prompt {
            purpose: PromptPurpose::Inline,
            buffer: String::new(),
        });
    }

    // ── Dispatch ─────────────────────────────────────────────────────────────

    /// Opens the dispatch prompt, prefilled from the selected run.
    pub fn open_dispatch_prompt(&mut self) {
        let Some(run) = self.selected_run().cloned() else {
            self.set_notice(
                "select a CI run first: dispatching starts the same workflow again".into(),
                false,
            );
            return;
        };
        let buffer = if run.workflow.is_empty() {
            format!("{} ", run.branch)
        } else {
            format!("{} {} ", run.workflow, run.branch)
        };
        self.prompt = Some(Prompt {
            purpose: PromptPurpose::Dispatch,
            buffer,
        });
    }

    fn dispatch_from(&mut self, text: &str) {
        let Some(run) = self.selected_run().cloned() else {
            return;
        };
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let needs_workflow = !run.workflow.is_empty();
        let gitkraft_hub::dispatch::Dispatch {
            workflow,
            git_ref,
            inputs,
        } = match gitkraft_hub::dispatch::parse(text, needs_workflow) {
            Ok(v) => v,
            Err(e) => {
                self.set_notice(e, false);
                return;
            }
        };
        let tx = self.tx.clone();
        rt.spawn(async move {
            let r = match api.ci(&run.repo.host) {
                Some(ci) => ci
                    .dispatch(&run.repo, &workflow, &git_ref, &inputs)
                    .await
                    .map_err(|e| e.to_string()),
                None => Err("no account".into()),
            };
            done(
                &tx,
                r,
                format!(
                    "started {} on {git_ref}",
                    if workflow.is_empty() {
                        "a pipeline"
                    } else {
                        &workflow
                    }
                ),
                true,
            );
        });
    }

    // ── Worktree ─────────────────────────────────────────────────────────────

    /// Checks the selected pull request out into a worktree next to its local checkout.
    pub fn checkout_pr_worktree(&mut self) {
        let Some(pr) = self.selected_pr().cloned() else {
            self.set_notice("select a pull request first".into(), false);
            return;
        };
        let Some(path) = self
            .watched
            .iter()
            .find(|w| w.repo == pr.repo)
            .and_then(|w| w.path.clone())
        else {
            self.set_notice(
                format!(
                    "no local checkout of {} is open: open it in a tab first",
                    pr.repo.full_name()
                ),
                false,
            );
            return;
        };
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let Some(kind) = api.backend(&pr.repo.host).map(|b| b.kind()) else {
            self.set_notice(format!("{}: no account", pr.repo.host), false);
            return;
        };
        self.set_notice(format!("checking out #{}…", pr.number), true);
        let tx = self.tx.clone();
        rt.spawn(async move {
            let r = tokio::task::spawn_blocking(move || {
                checkout_worktree(kind, &pr.repo, pr.number, &path)
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
            let (text, ok) = match r {
                Ok(t) => (t, true),
                Err(e) => (e, false),
            };
            let _ = tx.send(Msg::Done {
                text,
                ok,
                reload: false,
            });
        });
    }

    // ── SSH keys ─────────────────────────────────────────────────────────────

    /// The selected key row.
    pub fn selected_key(&self) -> Option<&gitkraft_hub::keys::KeyRow> {
        self.keys.data.as_ref()?.rows.get(self.keys.row)
    }

    /// Host to act on: the selected key's, else the first account's.
    pub fn keys_host(&self) -> Option<String> {
        self.selected_key()
            .map(|r| r.host.clone())
            .or_else(|| self.keys.data.as_ref()?.hosts.first().cloned())
    }

    /// Moves the key selection.
    pub fn move_key(&mut self, delta: isize) {
        let n = self.keys.data.as_ref().map_or(0, |d| d.rows.len());
        if n > 0 {
            self.keys.row = (self.keys.row as isize + delta).clamp(0, n as isize - 1) as usize;
        }
    }

    /// Loads the keys of every account.
    pub fn load_keys(&mut self) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        self.keys.loading = true;
        let tx = self.tx.clone();
        rt.spawn(async move {
            let _ = tx.send(Msg::Keys(reload_keys(&api).await));
        });
    }

    fn revoke_selected_key(&mut self) {
        let (Some(api), Some(rt), Some(row)) = (
            self.api.clone(),
            self.handle(),
            self.selected_key().cloned(),
        ) else {
            return;
        };
        let tx = self.tx.clone();
        rt.spawn(async move {
            let r = gitkraft_hub::keys::revoke(&api, &row.host, &row.key.id).await;
            let (text, ok) = match r {
                Ok(t) => (t, true),
                Err(e) => (e, false),
            };
            let _ = tx.send(Msg::Done {
                text,
                ok,
                reload: false,
            });
            let _ = tx.send(Msg::Keys(reload_keys(&api).await));
        });
    }

    /// Creates or reuses this machine's key for the selected account and registers it.
    pub fn setup_keys_host(&mut self) {
        let (Some(api), Some(rt), Some(host)) = (self.api.clone(), self.handle(), self.keys_host())
        else {
            self.set_notice("no account to set up".into(), false);
            return;
        };
        self.set_notice(format!("setting up ssh for {host}…"), true);
        let tx = self.tx.clone();
        rt.spawn(async move {
            let dir = ssh_dir();
            let r = gitkraft_hub::keys::setup_host(
                &api,
                &host,
                &dir,
                &gitkraft_platforms::ssh::SystemSsh,
                &machine_name(),
            )
            .await;
            let (text, ok) = match r {
                Ok(t) => (t.lines().collect::<Vec<_>>().join(" · "), true),
                Err(e) => (e, false),
            };
            let _ = tx.send(Msg::Done {
                text,
                ok,
                reload: false,
            });
            let _ = tx.send(Msg::Keys(reload_keys(&api).await));
        });
    }

    /// Tests the connection to the selected account over ssh.
    pub fn test_keys_host(&mut self) {
        let (Some(rt), Some(host)) = (self.handle(), self.keys_host()) else {
            return;
        };
        self.set_notice(format!("testing ssh to {host}…"), true);
        let tx = self.tx.clone();
        rt.spawn(async move {
            let (p, line) = tokio::task::spawn_blocking(move || {
                gitkraft_hub::keys::test_host(&gitkraft_platforms::ssh::SystemSsh, &host)
            })
            .await
            .unwrap_or_else(|e| {
                (
                    gitkraft_platforms::ssh::Probe::Other(e.to_string()),
                    e.to_string(),
                )
            });
            let ok = matches!(p, gitkraft_platforms::ssh::Probe::Authenticated { .. });
            let _ = tx.send(Msg::Done {
                text: line,
                ok,
                reload: false,
            });
        });
    }

    // ── Images ───────────────────────────────────────────────────────────────

    /// Image URLs in the selected item's description and comments.
    pub fn item_image_urls(&self) -> Vec<String> {
        let mut text = String::new();
        if let Some(pr) = self.selected_pr() {
            text.push_str(
                self.detail
                    .pr
                    .as_ref()
                    .map_or(pr.body.as_str(), |d| d.pr.body.as_str()),
            );
            for c in self.detail.pr.iter().flat_map(|d| d.comments.iter()) {
                text.push('\n');
                text.push_str(&c.body);
            }
        } else if let Some(issue) = self.selected_issue() {
            text.push_str(&issue.body);
            for c in &self.detail.comments {
                text.push('\n');
                text.push_str(&c.body);
            }
        }
        gitkraft_hub::markdown::image_urls(&text)
    }

    /// Opens the image overlay for the selected item.
    pub fn open_images(&mut self) {
        let urls = self.item_image_urls();
        if urls.is_empty() {
            self.set_notice("no images in this item".into(), false);
            return;
        }
        self.images.open_on(urls);
        self.fetch_current_image();
    }

    /// Starts fetching the image on screen, unless it is cached.
    pub fn fetch_current_image(&mut self) {
        let Some(url) = self.images.current().map(str::to_string) else {
            return;
        };
        if !self.images.needs_fetch(&url) {
            return;
        }
        let (Some(client), Some(rt)) = (self.client.clone(), self.handle()) else {
            self.images.set_failed(&url, "not connected".into());
            return;
        };
        self.images.mark_loading(&url);
        let tx = self.tx.clone();
        rt.spawn(async move {
            let result = fetch_image(&client, &url).await;
            let _ = tx.send(Msg::Image { url, result });
        });
    }

    /// Puts an already decoded image into the overlay's cache (tests and previews).
    pub fn set_image(&mut self, url: &str, img: image::DynamicImage) {
        let protocol = self.picker.new_resize_protocol(img);
        self.images.set_ready(url, protocol);
    }

    // ── Actions ──────────────────────────────────────────────────────────────

    /// Asks before doing `action` on the selected item.
    pub fn ask(&mut self, action: Action) {
        if action == Action::RevokeKey {
            match self.selected_key() {
                Some(r) => {
                    let text = format!(
                        "Remove “{}” from {}?{} y/n",
                        r.key.title,
                        r.host,
                        if r.mine {
                            " It is this machine's key: git over ssh stops working until you run setup again."
                        } else {
                            ""
                        }
                    );
                    self.confirm = Some(Confirm { action, text });
                }
                None => self.set_notice("select a key first".into(), false),
            }
            return;
        }
        let text = match (
            action,
            self.selected_pr().is_some(),
            self.selected_run().is_some(),
        ) {
            (Action::Merge(m), true, _) => format!("Merge PR ({m:?})? y/n"),
            (Action::ToggleOpen, _, false)
                if self.selected_pr().is_some() || self.selected_issue().is_some() =>
            {
                let open = self
                    .selected_pr()
                    .map(|p| p.state == ItemState::Open)
                    .or_else(|| self.selected_issue().map(|i| i.state == ItemState::Open));
                if open == Some(true) {
                    "Close this item? y/n".into()
                } else {
                    "Reopen this item? y/n".into()
                }
            }
            (Action::Rerun(RerunScope::All), _, true) => "Re-run every job? y/n".into(),
            (Action::Rerun(RerunScope::Failed), _, true) => "Re-run the failed jobs? y/n".into(),
            (Action::CancelRun, _, true) => "Cancel this run? y/n".into(),
            _ => {
                self.set_notice(
                    "that action does not apply to the selected item".into(),
                    false,
                );
                return;
            }
        };
        self.confirm = Some(Confirm { action, text });
    }

    /// Runs a confirmed action.
    pub fn perform(&mut self, action: Action) {
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let tx = self.tx.clone();
        match action {
            Action::Merge(method) => {
                let Some(pr) = self.selected_pr().cloned() else {
                    return;
                };
                rt.spawn(async move {
                    let r = match api.backend(&pr.repo.host) {
                        Some(b) => b
                            .merge(&pr.repo, pr.number, method)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(&tx, r, format!("merged #{}", pr.number), true);
                });
            }
            Action::ToggleOpen => {
                let target = self
                    .selected_pr()
                    .map(|p| {
                        (
                            p.repo.clone(),
                            p.number,
                            ItemKind::PullRequest,
                            p.state == ItemState::Open,
                        )
                    })
                    .or_else(|| {
                        self.selected_issue().map(|i| {
                            (
                                i.repo.clone(),
                                i.number,
                                ItemKind::Issue,
                                i.state == ItemState::Open,
                            )
                        })
                    });
                let Some((repo, number, kind, open)) = target else {
                    return;
                };
                rt.spawn(async move {
                    let r = match api.backend(&repo.host) {
                        Some(b) => b
                            .set_open(&repo, kind, number, !open)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(
                        &tx,
                        r,
                        format!("#{number} {}", if open { "closed" } else { "reopened" }),
                        true,
                    );
                });
            }
            Action::Rerun(scope) => {
                let Some(run) = self.selected_run().cloned() else {
                    return;
                };
                rt.spawn(async move {
                    let r = match api.ci(&run.repo.host) {
                        Some(ci) => ci
                            .rerun(&run.repo, &run, scope)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(&tx, r, "re-run started".into(), true);
                });
            }
            Action::RevokeKey => self.revoke_selected_key(),
            Action::CancelRun => {
                let Some(run) = self.selected_run().cloned() else {
                    return;
                };
                rt.spawn(async move {
                    let r = match api.ci(&run.repo.host) {
                        Some(ci) => ci
                            .cancel(&run.repo, run.id)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(&tx, r, "run cancelled".into(), true);
                });
            }
        }
    }

    fn submit_inline(&mut self, body: &str) {
        let (Some(api), Some(rt), Some((path, line)), Some(pr)) = (
            self.api.clone(),
            self.handle(),
            self.inline_anchor.take(),
            self.selected_pr().cloned(),
        ) else {
            return;
        };
        if body.is_empty() {
            return;
        }
        let (tx, body) = (self.tx.clone(), body.to_string());
        rt.spawn(async move {
            let r = match api.backend(&pr.repo.host) {
                Some(b) => b
                    .comment_on_line(&pr.repo, pr.number, &path, line, &body)
                    .await
                    .map_err(|e| e.to_string()),
                None => Err("no account".into()),
            };
            done(&tx, r, format!("commented on {path}:{line}"), true);
        });
    }

    /// Sends the prompt's text.
    pub fn submit_prompt(&mut self) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        if prompt.purpose == PromptPurpose::LogSearch {
            if let Some(log) = &mut self.log {
                log.set_query(&prompt.buffer);
            }
            return;
        }
        if prompt.purpose == PromptPurpose::Dispatch {
            self.dispatch_from(&prompt.buffer);
            return;
        }
        if prompt.purpose == PromptPurpose::Inline {
            self.submit_inline(prompt.buffer.trim());
            return;
        }
        let (Some(api), Some(rt)) = (self.api.clone(), self.handle()) else {
            return;
        };
        let tx = self.tx.clone();
        let body = prompt.buffer.trim().to_string();
        let target = self
            .selected_pr()
            .map(|p| (p.repo.clone(), p.number, ItemKind::PullRequest))
            .or_else(|| {
                self.selected_issue()
                    .map(|i| (i.repo.clone(), i.number, ItemKind::Issue))
            });
        let Some((repo, number, kind)) = target else {
            return;
        };
        match prompt.purpose {
            PromptPurpose::Comment if !body.is_empty() => {
                rt.spawn(async move {
                    let r = match api.backend(&repo.host) {
                        Some(b) => b
                            .post_comment(&repo, kind, number, &body)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(&tx, r, "comment posted".into(), true);
                });
            }
            PromptPurpose::Approve | PromptPurpose::RequestChanges
                if kind == ItemKind::PullRequest =>
            {
                let verdict = if prompt.purpose == PromptPurpose::Approve {
                    ReviewVerdict::Approve
                } else {
                    ReviewVerdict::RequestChanges
                };
                if verdict == ReviewVerdict::RequestChanges && body.is_empty() {
                    self.set_notice("requesting changes needs a note".into(), false);
                    return;
                }
                rt.spawn(async move {
                    let r = match api.backend(&repo.host) {
                        Some(b) => b
                            .submit_review(&repo, number, verdict, &body)
                            .await
                            .map_err(|e| e.to_string()),
                        None => Err("no account".into()),
                    };
                    done(&tx, r, "review sent".into(), true);
                });
            }
            _ => self.set_notice("nothing to send".into(), false),
        }
    }

    // ── Notifications ────────────────────────────────────────────────────────

    fn refresh_config(&mut self) {
        if self.config_loaded.elapsed() >= CONFIG_REFRESH && self.notify_file.is_some() {
            self.notify_config = load_notify_config_from(self.notify_file.as_deref());
            self.config_loaded = Instant::now();
        }
    }

    /// The bell for the current repo: `(repo key, status, unread count)`.
    pub fn bell(&self) -> Option<(String, NotifyStatus, usize)> {
        let repo = self.current_repo()?;
        let key = repo.key();
        let eff = self.notify_config.resolve(&key, None);
        // Never block the UI on the poller: if it holds the lock, skip this frame's detail.
        let engine = self.engine.try_lock().ok()?;
        let st = engine.status.repos.get(&key).cloned().unwrap_or_default();
        let input = StatusInput {
            enabled: eff.enabled,
            muted_until: engine.state.muted_until(&key),
            last_poll: st.last_poll,
            last_error: st.last_error,
            poller_heartbeat: engine.status.heartbeat,
            interval: eff.poll_interval,
        };
        Some((
            key.clone(),
            status(&input, Utc::now()),
            engine.inbox.unread_for(&key),
        ))
    }

    /// Turns notifications on or off for the current repo (or everything).
    pub fn toggle_notifications(&mut self, global: bool) {
        let repo_key = if global {
            None
        } else {
            self.current_repo().map(|r| r.key())
        };
        if !global && repo_key.is_none() {
            self.set_notice("no repository selected".into(), false);
            return;
        }
        let key = repo_key.clone().unwrap_or_default();
        let now_on = if global {
            self.notify_config.defaults.enabled.unwrap_or(false)
        } else {
            self.notify_config.resolve(&key, None).enabled
        };
        let result = (|| {
            let path = self
                .notify_file
                .clone()
                .ok_or("no config file in this session")?;
            let text = files::read_optional(&path)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            let edited = set_enabled(&text, repo_key.as_deref(), !now_on)?;
            files::write_atomic(&path, &edited).map_err(|e| e.to_string())
        })();
        match result {
            Ok(()) => {
                self.notify_config = load_notify_config_from(self.notify_file.as_deref());
                self.config_loaded = Instant::now();
                let what = if global {
                    "all projects".to_string()
                } else {
                    key
                };
                self.set_notice(
                    format!(
                        "notifications {} for {what}",
                        if now_on { "off" } else { "on" }
                    ),
                    true,
                );
            }
            Err(e) => self.set_notice(format!("could not change notifications: {e}"), false),
        }
    }

    /// Snoozes the current repo for an hour, or ends the snooze.
    pub fn toggle_snooze(&mut self) {
        let Some(repo) = self.current_repo() else {
            return;
        };
        let key = repo.key();
        let engine = self.engine.clone();
        let Ok(mut e) = engine.try_lock() else {
            self.set_notice("poller is busy, try again".into(), false);
            return;
        };
        if e.state.muted_until(&key).is_some_and(|u| u > Utc::now()) {
            e.state.unmute(Some(&key));
            self.set_notice(format!("{key}: snooze ended"), true);
        } else {
            e.state
                .mute(Some(&key), Utc::now() + chrono::Duration::hours(1));
            self.set_notice(format!("{key}: snoozed for 1 h"), true);
        }
        if self.notify_file.is_some() {
            let _ = e.save();
        }
    }

    // ── Messages ─────────────────────────────────────────────────────────────

    /// Sets the line shown in the status bar.
    pub fn set_notice(&mut self, text: String, ok: bool) {
        self.notice = Some((text, ok, Instant::now()));
    }

    /// The notice while it is fresh.
    pub fn fresh_notice(&self) -> Option<(&str, bool)> {
        self.notice
            .as_ref()
            .filter(|(_, _, t)| t.elapsed() < NOTICE_FOR)
            .map(|(s, ok, _)| (s.as_str(), *ok))
    }

    /// Handles finished background work. Call once per tick.
    pub fn poll(&mut self) {
        self.refresh_config();
        while let Ok(msg) = self.rx.try_recv() {
            self.apply(msg);
        }
    }

    fn apply(&mut self, msg: Msg) {
        match msg {
            Msg::Ready {
                api,
                client,
                watched,
                accounts,
            } => {
                self.client = Some(client);
                self.ready = true;
                self.accounts = accounts;
                self.watched = watched.clone();
                self.api = Some(api.clone());
                if accounts == 0 {
                    self.set_notice("no accounts: set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or log in with `gh`".into(), false);
                }
                self.spawn_poller(api, watched);
                for idx in self.visible_sections() {
                    self.load_section(idx);
                }
            }
            Msg::SetupFailed(e) => {
                self.ready = true;
                self.set_notice(e, false);
            }
            Msg::Section {
                idx,
                generation,
                data,
            } => {
                let Some(slot) = self.slots.get_mut(idx) else {
                    return;
                };
                if slot.generation != generation {
                    return; // a newer load is on its way
                }
                slot.loading = false;
                if let Some(first) = data.errors().first() {
                    let n = data.errors().len();
                    let text = if n > 1 {
                        format!("{first} (+{} more)", n - 1)
                    } else {
                        first.clone()
                    };
                    self.set_notice(text, false);
                }
                let was_active = self.active_index() == Some(idx);
                self.slots[idx].data = Some(data);
                if was_active {
                    self.row = self.row.min(self.row_count().saturating_sub(1));
                    self.detail.key = None;
                    self.on_selection_changed();
                }
            }
            Msg::Pr { key, result } => {
                if self.detail.key.as_deref() != Some(&key) {
                    return;
                }
                self.detail.loading = false;
                match result {
                    Ok(d) => {
                        if let Some(e) = d.errors.first() {
                            self.set_notice(e.clone(), false);
                        }
                        self.detail.pr = Some(d);
                    }
                    Err(e) => self.detail.error = Some(e),
                }
            }
            Msg::Comments { key, result } => {
                if self.detail.key.as_deref() != Some(&key) {
                    return;
                }
                self.detail.loading = false;
                match result {
                    Ok(c) => self.detail.comments = c,
                    Err(e) => self.detail.error = Some(e),
                }
            }
            Msg::Run { key, result } => {
                if self.detail.key.as_deref() != Some(&key) {
                    return;
                }
                self.detail.loading = false;
                match result {
                    Ok(d) => self.detail.run = Some(d),
                    Err(e) => self.detail.error = Some(e),
                }
            }
            Msg::Image { url, result } => match result {
                Ok(img) => {
                    let protocol = self.picker.new_resize_protocol(img);
                    self.images.set_ready(&url, protocol);
                }
                Err(e) => self.images.set_failed(&url, e),
            },
            Msg::Log { title, result } => match result {
                Ok(raw) => {
                    self.log = Some(LogView::new(title, &raw));
                    self.notice = None;
                }
                Err(e) => self.set_notice(format!("log: {e}"), false),
            },
            Msg::Pulse { repo, days, data } => {
                if self.pulse.repo.as_ref().map(RepoId::key).as_deref() == Some(&repo)
                    && self.pulse.days == days
                {
                    self.pulse.data = Some(data);
                    self.pulse.loading = false;
                }
            }
            Msg::Keys(data) => {
                self.keys.loading = false;
                if let Some(e) = data.errors.first() {
                    self.set_notice(e.clone(), false);
                }
                self.keys.row = self.keys.row.min(data.rows.len().saturating_sub(1));
                self.keys.data = Some(data);
            }
            Msg::Contributions(c) => {
                if let Some(e) = c.errors.first() {
                    self.set_notice(e.clone(), false);
                }
                self.pulse.contributions = Some(c);
            }
            Msg::Done { text, ok, reload } => {
                self.set_notice(text, ok);
                if ok && reload {
                    if let Some(idx) = self.active_index() {
                        self.load_section(idx);
                    }
                    self.detail.key = None;
                }
            }
        }
    }

    /// Switches the Pulse view to the next or previous watched repo and reloads it.
    pub fn cycle_pulse_repo(&mut self, delta: isize) {
        if self.watched.is_empty() {
            return;
        }
        let cur = self
            .pulse
            .repo
            .as_ref()
            .and_then(|r| self.watched.iter().position(|w| &w.repo == r))
            .unwrap_or(0);
        let next = (cur as isize + delta).rem_euclid(self.watched.len() as isize) as usize;
        self.pulse.repo = Some(self.watched[next].repo.clone());
        self.pulse.data = None;
        let days = self.pulse.days.max(30);
        self.load_pulse(days);
    }

    /// Records whether this click completes a double click on `target`.
    pub fn register_click(&mut self, target: Target) -> bool {
        let double = self
            .last_click
            .is_some_and(|(t, last)| last == target && t.elapsed() < Duration::from_millis(450));
        self.last_click = if double {
            None
        } else {
            Some((Instant::now(), target))
        };
        double
    }

    /// Records the hit regions of the frame just drawn.
    pub fn set_hits(&mut self, hits: Vec<Hit>) {
        self.hits = hits;
    }

    /// The topmost target under `(x, y)`; later regions win.
    pub fn hit_at(&self, x: u16, y: u16) -> Option<Target> {
        self.hits
            .iter()
            .rev()
            .find(|h| list::contains(h.rect, x, y))
            .map(|h| h.target)
    }
}

async fn fetch_image(client: &Client, url: &str) -> Result<image::DynamicImage, String> {
    use gitkraft_platforms::http::{HttpClient, Request};
    let res = client
        .send(Request::get(url))
        .await
        .map_err(|e| e.to_string())?;
    if !(200..300).contains(&res.status) {
        return Err(format!("HTTP {}", res.status));
    }
    tokio::task::spawn_blocking(move || decode(&res.body))
        .await
        .map_err(|e| e.to_string())?
}

/// `(name, url)` of every remote of the checkout at `path`.
pub(crate) fn remotes_of(path: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let repo = git2::Repository::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let remotes =
        gitkraft_core::features::remotes::list_remotes(&repo).map_err(|e| e.to_string())?;
    Ok(remotes
        .into_iter()
        .filter_map(|r| Some((r.name, r.url?)))
        .collect())
}

/// Fetches PR `number` of `repo` and adds (or updates) its worktree; returns what to tell the user.
pub(crate) fn checkout_worktree(
    kind: gitkraft_platforms::PlatformKind,
    repo: &RepoId,
    number: u64,
    workdir: &std::path::Path,
) -> Result<String, String> {
    let remotes = remotes_of(workdir)?;
    let plan =
        gitkraft_hub::checkout::plan(kind, repo, number, &remotes, workdir).ok_or_else(|| {
            format!(
                "{} has no remote that points at {}",
                workdir.display(),
                repo.key()
            )
        })?;
    gitkraft_core::features::remotes::worktree::add_from_remote_ref(
        workdir,
        &plan.remote,
        &plan.remote_ref,
        &plan.branch,
        &plan.dest,
    )
    .map_err(|e| format!("{e:#}"))?;
    Ok(format!(
        "#{number} is checked out in {}",
        plan.dest.display()
    ))
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

async fn reload_keys(api: &Api) -> gitkraft_hub::keys::KeysData {
    let mine = tokio::task::spawn_blocking(|| {
        gitkraft_hub::keys::local_fingerprints(&ssh_dir(), &gitkraft_platforms::ssh::SystemSsh)
    })
    .await
    .unwrap_or_default();
    gitkraft_hub::keys::load(api, &mine).await
}

fn done(tx: &mpsc::Sender<Msg>, result: Result<(), String>, ok_text: String, reload: bool) {
    let msg = match result {
        Ok(()) => Msg::Done {
            text: ok_text,
            ok: true,
            reload,
        },
        Err(e) => Msg::Done {
            text: e,
            ok: false,
            reload: false,
        },
    };
    let _ = tx.send(msg);
}

/// Reads `notifications.toml`; a missing or broken file means defaults.
pub fn load_notify_config() -> NotifyConfig {
    load_notify_config_from(files::notify_config_path().ok().as_deref())
}

fn load_notify_config_from(path: Option<&std::path::Path>) -> NotifyConfig {
    path.and_then(|p| files::read_optional(p).ok().flatten())
        .and_then(|t| NotifyConfig::parse(&t).ok())
        .unwrap_or_default()
}

/// Fixtures for the hub tests.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use gitkraft_platforms::model::User;
    use gitkraft_settings::dashboard::{Involvement, Section, SectionKind, SectionState};

    pub(crate) fn section(name: &str, kind: SectionKind, hidden: bool) -> Section {
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

    pub(crate) fn pr(n: u64, title: &str) -> PullRequest {
        PullRequest {
            repo: RepoId::new("github.com", "o", "r"),
            number: n,
            title: title.into(),
            author: User {
                login: "alice".into(),
                avatar_url: None,
            },
            source_branch: "feat".into(),
            target_branch: "master".into(),
            url: format!("https://github.com/o/r/pull/{n}"),
            ..PullRequest::default()
        }
    }

    /// Three sections (PRs, issues, CI) with PR rows in the first.
    pub(crate) fn state() -> HubState {
        let dashboard = Dashboard {
            sections: vec![
                section("Mine", SectionKind::Prs, false),
                section("Issues", SectionKind::Issues, false),
                section("CI", SectionKind::Ci, false),
            ],
        };
        let mut s = HubState::detached(dashboard);
        s.ready = true;
        s.slots[0].data = Some(SectionData::Prs(SectionResult {
            items: (1..=5).map(|n| pr(n, &format!("pr {n}"))).collect(),
            errors: vec![],
        }));
        s.slots[1].data = Some(SectionData::Issues(SectionResult {
            items: vec![],
            errors: vec![],
        }));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use gitkraft_settings::dashboard::SectionKind;

    #[test]
    fn rows_clamp_and_selection_follows() {
        let mut s = state();
        s.move_row(2);
        assert_eq!(s.selected_pr().map(|p| p.number), Some(3));
        s.move_row(99);
        assert_eq!(s.row, 4);
        s.move_row(-99);
        assert_eq!(s.row, 0);
        s.set_row(usize::MAX);
        assert_eq!(s.row, 4);
        assert_eq!(s.row_count(), 5);
    }

    #[test]
    fn switching_sections_wraps_and_resets_the_row() {
        let mut s = state();
        s.move_row(3);
        s.next_section(1);
        assert_eq!((s.active, s.row), (1, 0));
        s.next_section(-1);
        s.next_section(-1);
        assert_eq!(s.active, 2, "wraps backwards");
        s.select_section(99);
        assert_eq!(s.active, 2, "clamped");
    }

    #[test]
    fn reordering_moves_the_data_with_the_section_and_follows_it() {
        let mut s = state();
        assert!(s.reorder_sections(0, 2));
        assert_eq!(
            s.dashboard
                .sections
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
            ["Issues", "CI", "Mine"]
        );
        assert_eq!(s.active, 2);
        assert!(
            matches!(&s.slots[2].data, Some(SectionData::Prs(_))),
            "the PR data moved along"
        );
        assert!(matches!(&s.slots[0].data, Some(SectionData::Issues(_))));
        assert!(!s.reorder_sections(1, 1));
        assert!(!s.reorder_sections(0, 9));
    }

    #[test]
    fn reordering_uses_visible_positions_when_a_section_is_hidden() {
        let mut s = state();
        s.dashboard.sections[1].hidden = true;
        assert_eq!(s.visible_sections(), vec![0, 2]);
        assert!(s.reorder_sections(0, 1));
        // Visible 0 → 1 means dashboard index 0 → 2.
        assert_eq!(
            s.dashboard
                .sections
                .iter()
                .map(|x| x.name.as_str())
                .collect::<Vec<_>>(),
            ["Issues", "CI", "Mine"]
        );
        assert!(
            s.dashboard.sections[0].hidden,
            "the hidden section keeps its flag"
        );
    }

    #[test]
    fn the_new_order_is_saved_when_a_file_is_set() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("dashboard.toml");
        let mut s = state();
        s.dashboard_file = Some(path.clone());
        assert!(s.reorder_sections(2, 0));
        let text = std::fs::read_to_string(&path).expect("saved");
        let back = Dashboard::parse(&text).expect("valid");
        assert_eq!(back, s.dashboard);
        assert_eq!(back.sections[0].name, "CI");
    }

    #[test]
    fn resizing_is_clamped_and_never_persists_in_tests() {
        let mut s = state();
        let area = Rect::new(0, 3, 100, 20);
        s.apply_drag(DragOutcome::Resized { x: 5 }, area);
        assert_eq!(s.list_percent, 20);
        s.apply_drag(DragOutcome::Resized { x: 99 }, area);
        assert_eq!(s.list_percent, 80);
        s.resize_to(50, area);
        assert_eq!(s.list_percent, 50);
    }

    #[test]
    fn a_drop_reorders_and_a_cancel_does_not() {
        let mut s = state();
        s.apply_drag(DragOutcome::Cancelled, Rect::default());
        assert_eq!(s.dashboard.sections[0].name, "Mine");
        s.apply_drag(DragOutcome::Drop { from: 0, to: 1 }, Rect::default());
        assert_eq!(s.dashboard.sections[1].name, "Mine");
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t == "section moved" && ok)
        );
    }

    #[test]
    fn double_click_needs_the_same_target_quickly() {
        let mut s = state();
        assert!(!s.register_click(Target::Row(1)));
        assert!(s.register_click(Target::Row(1)));
        assert!(!s.register_click(Target::Row(1)), "the pair is consumed");
        assert!(!s.register_click(Target::Row(2)));
        assert!(
            !s.register_click(Target::Row(3)),
            "a different target is not a double click"
        );
    }

    #[test]
    fn hit_testing_prefers_the_latest_region() {
        let mut s = state();
        s.set_hits(vec![
            Hit {
                rect: Rect::new(0, 0, 10, 10),
                target: Target::ListArea,
            },
            Hit {
                rect: Rect::new(2, 2, 4, 1),
                target: Target::Row(0),
            },
        ]);
        assert_eq!(s.hit_at(3, 2), Some(Target::Row(0)));
        assert_eq!(s.hit_at(3, 5), Some(Target::ListArea));
        assert_eq!(s.hit_at(30, 5), None);
    }

    #[test]
    fn actions_need_a_matching_selection() {
        let mut s = state();
        s.ask(Action::Rerun(RerunScope::Failed));
        assert!(s.confirm.is_none(), "a PR is not a CI run");
        assert!(s.fresh_notice().is_some_and(|(_, ok)| !ok));
        s.ask(Action::Merge(MergeMethod::Squash));
        assert!(
            s.confirm
                .as_ref()
                .is_some_and(|c| c.text.contains("Squash"))
        );
        s.confirm = None;
        s.ask(Action::ToggleOpen);
        assert!(
            s.confirm
                .as_ref()
                .is_some_and(|c| c.text.starts_with("Close"))
        );
        s.confirm = None;
        s.select_section(2);
        s.ask(Action::Merge(MergeMethod::Merge));
        assert!(s.confirm.is_none());
    }

    #[test]
    fn current_repo_comes_from_the_selection_then_the_watch_list() {
        let mut s = state();
        assert_eq!(
            s.current_repo().map(|r| r.key()),
            Some("github.com/o/r".into())
        );
        s.select_section(1);
        assert_eq!(s.current_repo(), None);
        s.watched = vec![Watched {
            repo: RepoId::new("codeberg.org", "a", "b"),
            path: None,
        }];
        assert_eq!(
            s.current_repo().map(|r| r.key()),
            Some("codeberg.org/a/b".into())
        );
    }

    #[test]
    fn toggling_notifications_edits_the_file_and_flips_the_state() {
        let dir = tempfile::tempdir().expect("tmp");
        let path = dir.path().join("notifications.toml");
        std::fs::write(&path, "# mine\n[defaults]\npoll_interval = \"2m\"\n").expect("write");
        let mut s = state();
        s.notify_file = Some(path.clone());
        s.toggle_notifications(false);
        assert!(s.notify_config.resolve("github.com/o/r", None).enabled);
        assert!(
            !s.notify_config.resolve("github.com/o/other", None).enabled,
            "only this project"
        );
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("# mine") && text.contains("2m"), "{text}");
        s.toggle_notifications(false);
        assert!(!s.notify_config.resolve("github.com/o/r", None).enabled);
        s.toggle_notifications(true);
        assert!(
            s.notify_config.resolve("github.com/o/other", None).enabled,
            "global switch"
        );
    }

    #[test]
    fn toggling_without_a_repo_or_file_explains_itself() {
        let mut s = state();
        s.toggle_notifications(false);
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("no config file") && !ok),
            "{:?}",
            s.fresh_notice()
        );
        s.select_section(1);
        s.notify_file = None;
        s.toggle_notifications(false);
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, _)| t.contains("no repository"))
        );
    }

    #[test]
    fn the_bell_reflects_config_and_a_missing_poller() {
        let mut s = state();
        let (key, st, unread) = s.bell().expect("bell");
        assert_eq!(
            (key.as_str(), st, unread),
            ("github.com/o/r", NotifyStatus::Off, 0)
        );
        s.notify_config = NotifyConfig::parse("[defaults]\nenabled = true\n").expect("valid");
        assert_eq!(s.bell().map(|b| b.1), Some(NotifyStatus::NoPoller));
        s.engine.try_lock().expect("free").status.heartbeat = Some(Utc::now());
        assert!(matches!(
            s.bell().map(|b| b.1),
            Some(NotifyStatus::On { .. })
        ));
    }

    #[test]
    fn snooze_toggles_per_repo() {
        let mut s = state();
        s.toggle_snooze();
        assert!(
            s.engine
                .try_lock()
                .expect("free")
                .state
                .muted_until("github.com/o/r")
                .is_some()
        );
        s.toggle_snooze();
        assert!(
            s.engine
                .try_lock()
                .expect("free")
                .state
                .muted_until("github.com/o/r")
                .is_none()
        );
    }

    #[test]
    fn messages_for_a_stale_selection_are_dropped() {
        let mut s = state();
        s.detail.key = Some("github.com/o/r#2".into());
        s.detail.loading = true;
        s.apply(Msg::Pr {
            key: "github.com/o/r#1".into(),
            result: Err("late".into()),
        });
        assert!(s.detail.loading && s.detail.error.is_none());
        s.apply(Msg::Pr {
            key: "github.com/o/r#2".into(),
            result: Err("real".into()),
        });
        assert_eq!(s.detail.error.as_deref(), Some("real"));
        assert!(!s.detail.loading);
    }

    #[test]
    fn an_older_section_load_never_overwrites_a_newer_one() {
        let mut s = state();
        s.slots[0].generation = 5;
        s.slots[0].loading = true;
        let stale = SectionData::Prs(SectionResult {
            items: vec![],
            errors: vec![],
        });
        s.apply(Msg::Section {
            idx: 0,
            generation: 4,
            data: stale,
        });
        assert!(s.slots[0].loading, "still waiting for generation 5");
        assert_eq!(s.row_count(), 5);
        let fresh = SectionData::Prs(SectionResult {
            items: vec![pr(9, "new")],
            errors: vec!["codeberg.org: down".into()],
        });
        s.apply(Msg::Section {
            idx: 0,
            generation: 5,
            data: fresh,
        });
        assert_eq!((s.row_count(), s.slots[0].loading), (1, false));
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("codeberg.org") && !ok)
        );
    }

    #[test]
    fn setup_failure_and_no_accounts_are_visible_not_silent() {
        let mut s = state();
        s.ready = false;
        s.apply(Msg::SetupFailed("accounts.toml: bad".into()));
        assert!(s.ready);
        assert!(
            s.fresh_notice()
                .is_some_and(|(t, ok)| t.contains("accounts.toml") && !ok)
        );
    }

    #[test]
    fn hidden_only_dashboards_have_nothing_selected() {
        let mut s = HubState::detached(Dashboard {
            sections: vec![fixtures::section("x", SectionKind::Inbox, true)],
        });
        assert!(s.active_index().is_none());
        s.select_section(0);
        s.move_row(1);
        assert_eq!(s.row_count(), 0);
        assert!(s.selected_url().is_none());
    }
}

#[cfg(test)]
mod worktree_tests {
    use super::*;

    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn remotes_are_listed_and_a_repo_without_a_matching_remote_is_an_error() {
        let root = tempfile::tempdir().expect("tmp");
        let main = root.path().join("main");
        std::fs::create_dir_all(&main).expect("mk");
        git(&main, &["init", "-q"]);
        git(
            &main,
            &["remote", "add", "origin", "git@github.com:o/r.git"],
        );
        git(&main, &["remote", "add", "local", "/srv/git/r"]);
        assert_eq!(
            remotes_of(&main).expect("remotes"),
            [
                ("local".to_string(), "/srv/git/r".to_string()),
                ("origin".to_string(), "git@github.com:o/r.git".to_string())
            ]
        );
        let other = RepoId::new("github.com", "x", "y");
        let err = checkout_worktree(gitkraft_platforms::PlatformKind::GitHub, &other, 3, &main)
            .expect_err("no remote");
        assert!(
            err.contains("no remote that points at github.com/x/y"),
            "{err}"
        );
        assert!(remotes_of(&root.path().join("nope")).is_err());
    }
}
