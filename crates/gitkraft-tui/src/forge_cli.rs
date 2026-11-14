//! The command line: `gitkraft-tui` without a command opens the TUI; with one it talks to your
//! forges and prints, so a script, a shell alias or an agent without MCP can use the same accounts.
//!
//! Every read command takes `--json`. Commands that change something say what they did and refuse
//! nothing silently; a forced merge is not offered here (use the UI, where it is a deliberate act).

use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use clap::{Args, CommandFactory, Parser, Subcommand};
use gitkraft_ci::{RerunScope, RunQuery};
use gitkraft_hub::{Hub, SectionData, args};
use gitkraft_platforms::model::{ItemState, ListQuery, MergeOptions, Page, StateFilter};
use gitkraft_platforms::{DynClient, ItemKind, RepoId};
use gitkraft_settings::dashboard::SectionKind;
use serde::Serialize;

/// GitKraft: a Git IDE and an aggregate client for GitHub, GitLab, Gitea, Forgejo and Codeberg.
#[derive(Debug, Parser)]
#[command(name = "gitkraft-tui", version, args_conflicts_with_subcommands = true)]
pub struct Cli {
    /// Repository to open in the TUI (default: the current directory, or the last one).
    pub repo_path: Option<PathBuf>,
    /// Colour theme for the TUI; remembered.
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,
    /// Print the theme names and exit.
    #[arg(long)]
    pub list_themes: bool,
    /// What to do instead of opening the TUI.
    #[command(subcommand)]
    pub command: Option<Cmd>,
}

/// Options every forge command shares.
#[derive(Debug, Args, Clone, Default)]
pub struct Common {
    /// Repository as `host/owner/name` or `owner/name` (default: the current checkout).
    #[arg(short = 'R', long, global = true, value_name = "REPO")]
    pub repo: Option<String>,
    /// Print JSON instead of text.
    #[arg(long, global = true)]
    pub json: bool,
}

/// The commands.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Notifications and polling in the background (`notify --help` lists its commands).
    Notify {
        /// Arguments of the notify command.
        #[arg(num_args = 0.., trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Log in to a host (device flow, or `--token`).
    Login {
        /// `HOST [--token ..]`.
        #[arg(num_args = 0.., trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Forget a host's stored token.
    Logout {
        /// `HOST`.
        #[arg(num_args = 0.., trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Clone a repository by `owner/name` or URL, over the transport your account prefers.
    Clone {
        /// `SPEC [--via ssh|https] [DIR]`.
        #[arg(num_args = 0.., trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// SSH keys: status, setup, upload, remove.
    Ssh {
        /// `status|setup|…`.
        #[arg(num_args = 0.., trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Store an AI API key in the system keyring (read from stdin, never from an argument).
    AiKey {
        /// anthropic or openai.
        provider: String,
    },
    /// Draft a commit message for what is staged, in the project's style (`.gitkraft/ai.toml`).
    CommitMessage {
        /// Print the redacted prompt instead of sending it.
        #[arg(long)]
        dry_run: bool,
        /// Write a starter `.gitkraft/ai.toml` and exit.
        #[arg(long)]
        init: bool,
        /// Print JSON (message and warnings).
        #[arg(long)]
        json: bool,
    },
    /// Print a shell completion script.
    Completions {
        /// bash, zsh, fish, elvish or powershell.
        shell: clap_complete::Shell,
    },
    #[command(flatten)]
    Forge(Forge),
}

/// What talks to the forges.
#[derive(Debug, Subcommand)]
pub enum Forge {
    /// The connected accounts and what each supports.
    Accounts(Common),
    /// Notifications of every account.
    Inbox(Common),
    /// Pull requests: yours, or those of a repository with `-R`.
    Prs(ListArgs),
    /// Issues: yours, or those of a repository with `-R`.
    Issues(ListArgs),
    /// One pull request, or an issue with `--issue`.
    Show(ShowArgs),
    /// CI runs of a repository.
    Runs(RunsArgs),
    /// The jobs of one CI run.
    Run(RunArgs),
    /// The log of one CI job.
    Log(LogArgs),
    /// What you did on a day (default today), like GitHub's panel under the calendar.
    Day(DayArgs),
    /// Activity of a repository over the last days.
    Pulse(PulseArgs),
    /// GitHub discussions of a repository, or one with its comments.
    Discussions(DiscussionArgs),
    /// Kanban boards: GitHub Projects v2, GitLab issue boards.
    Boards(Common),
    /// A wiki: the page list, or one page.
    Wiki(WikiArgs),
    /// Comment on a pull request (or an issue with `--issue`).
    Comment(CommentArgs),
    /// Answer a comment of a discussion (find the comment id with `discussions NUMBER --json`).
    Reply(ReplyArgs),
    /// Create or edit a wiki page from a file, stdin or your editor.
    WikiSave(WikiSaveArgs),
    /// Move a board card to another column.
    BoardMove(BoardMoveArgs),
    /// Merge a pull request.
    Merge(MergeArgs),
    /// Ask an AI why a CI run failed (needs a key, see `ai-key`; `--dry-run` shows what would be sent).
    Explain(ExplainArgs),
    /// Run a CI run again (failed jobs by default).
    Rerun(RerunArgs),
}

/// `prs` / `issues`.
#[derive(Debug, Args)]
pub struct ListArgs {
    #[command(flatten)]
    pub common: Common,
    /// With `-R`: filter by author. Without: `authored` (default), `assigned` or `review`.
    #[arg(long, value_name = "WHO")]
    pub who: Option<String>,
    /// open (default), closed or all.
    #[arg(long, default_value = "open")]
    pub state: String,
    /// Only items with this label (with `-R`).
    #[arg(long)]
    pub label: Option<String>,
    /// How many (with `-R`).
    #[arg(short = 'n', long, default_value_t = 30)]
    pub limit: u32,
}

/// `show`.
#[derive(Debug, Args)]
pub struct ShowArgs {
    #[command(flatten)]
    pub common: Common,
    /// Number on the platform.
    pub number: u64,
    /// It is an issue.
    #[arg(long)]
    pub issue: bool,
    /// Include the diff (pull requests).
    #[arg(long)]
    pub diff: bool,
}

/// `runs`.
#[derive(Debug, Args)]
pub struct RunsArgs {
    #[command(flatten)]
    pub common: Common,
    /// Only this branch.
    #[arg(long)]
    pub branch: Option<String>,
    /// queued, running, success, failed or cancelled.
    #[arg(long)]
    pub status: Option<String>,
    /// How many.
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: u32,
}

/// `run`.
#[derive(Debug, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub common: Common,
    /// Run id (from `runs`).
    pub run_id: u64,
}

/// `log`.
#[derive(Debug, Args)]
pub struct LogArgs {
    #[command(flatten)]
    pub common: Common,
    /// Job id (from `run`).
    pub job_id: u64,
    /// Last N lines.
    #[arg(short = 'n', long, default_value_t = 150)]
    pub tail: usize,
    /// Only lines containing this text.
    #[arg(long)]
    pub grep: Option<String>,
}

/// `day`.
#[derive(Debug, Args)]
pub struct DayArgs {
    #[command(flatten)]
    pub common: Common,
    /// `YYYY-MM-DD` (UTC), within the last year.
    pub date: Option<String>,
}

/// `pulse`.
#[derive(Debug, Args)]
pub struct PulseArgs {
    #[command(flatten)]
    pub common: Common,
    /// Days to look back.
    #[arg(long, default_value_t = 7)]
    pub days: u32,
}

/// `discussions`.
#[derive(Debug, Args)]
pub struct DiscussionArgs {
    #[command(flatten)]
    pub common: Common,
    /// With a number: that discussion and its comments.
    pub number: Option<u64>,
    /// How many to list.
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: u32,
}

/// `wiki`.
#[derive(Debug, Args)]
pub struct WikiArgs {
    #[command(flatten)]
    pub common: Common,
    /// Page slug; without it the page list.
    pub page: Option<String>,
}

/// `comment`.
#[derive(Debug, Args)]
pub struct CommentArgs {
    #[command(flatten)]
    pub common: Common,
    /// Number on the platform.
    pub number: u64,
    /// The comment (Markdown).
    pub body: String,
    /// It is an issue.
    #[arg(long)]
    pub issue: bool,
}

/// `reply`.
#[derive(Debug, Args)]
pub struct ReplyArgs {
    #[command(flatten)]
    pub common: Common,
    /// Discussion number.
    pub number: u64,
    /// Id of the comment to answer.
    pub comment_id: u64,
    /// The reply (Markdown).
    pub body: String,
}

/// `wiki-save`.
#[derive(Debug, Args)]
pub struct WikiSaveArgs {
    #[command(flatten)]
    pub common: Common,
    /// The page to edit (its slug); without it a new page is created and `--title` is needed.
    pub page: Option<String>,
    /// Title; the page's own when editing.
    #[arg(long)]
    pub title: Option<String>,
    /// Read the content from this file; `-` is stdin (the default).
    #[arg(
        long,
        value_name = "FILE",
        default_value = "-",
        conflicts_with = "edit"
    )]
    pub file: String,
    /// Edit the page in $VISUAL / $EDITOR instead.
    #[arg(long)]
    pub edit: bool,
    /// The change message (default `Update <title>`).
    #[arg(short, long)]
    pub message: Option<String>,
}

/// `board-move`.
#[derive(Debug, Args)]
pub struct BoardMoveArgs {
    #[command(flatten)]
    pub common: Common,
    /// Issue or pull request number of the card.
    pub number: u64,
    /// Target column: its name, or a unique start or part of it.
    pub column: String,
    /// The board, when the card is on several.
    #[arg(long)]
    pub board: Option<String>,
}

/// `merge`.
#[derive(Debug, Args)]
pub struct MergeArgs {
    #[command(flatten)]
    pub common: Common,
    /// Pull request number.
    pub number: u64,
    /// merge (default), squash or rebase.
    #[arg(long, default_value = "merge")]
    pub method: String,
    /// Merge when checks pass instead of now.
    #[arg(long, conflicts_with = "queue")]
    pub when_green: bool,
    /// Add to the merge queue.
    #[arg(long)]
    pub queue: bool,
}

/// `explain`.
#[derive(Debug, Args)]
pub struct ExplainArgs {
    #[command(flatten)]
    pub common: Common,
    /// Run id (from `runs`).
    pub run_id: u64,
    /// A job of that run; default: the first failed one.
    pub job_id: Option<u64>,
    /// Print the redacted prompt instead of sending it.
    #[arg(long)]
    pub dry_run: bool,
}

/// `rerun`.
#[derive(Debug, Args)]
pub struct RerunArgs {
    #[command(flatten)]
    pub common: Common,
    /// Run id.
    pub run_id: u64,
    /// Every job, not only the failed ones.
    #[arg(long)]
    pub all: bool,
}

/// Prints the completion script of `shell`.
#[must_use]
pub fn completions(shell: clap_complete::Shell) -> String {
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "gitkraft-tui", &mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

impl Forge {
    /// The `-R` value, if given.
    #[must_use]
    pub fn repo_flag(&self) -> Option<&str> {
        self.common().repo.as_deref()
    }

    fn common(&self) -> &Common {
        match self {
            Self::Accounts(c) | Self::Inbox(c) | Self::Boards(c) => c,
            Self::Prs(a) | Self::Issues(a) => &a.common,
            Self::Show(a) => &a.common,
            Self::Runs(a) => &a.common,
            Self::Run(a) => &a.common,
            Self::Log(a) => &a.common,
            Self::Day(a) => &a.common,
            Self::Pulse(a) => &a.common,
            Self::Discussions(a) => &a.common,
            Self::Wiki(a) => &a.common,
            Self::Comment(a) => &a.common,
            Self::Reply(a) => &a.common,
            Self::WikiSave(a) => &a.common,
            Self::BoardMove(a) => &a.common,
            Self::Merge(a) => &a.common,
            Self::Rerun(a) => &a.common,
            Self::Explain(a) => &a.common,
        }
    }
}

/// The accounts and the repository a command runs against.
#[derive(Debug)]
pub struct Session {
    hub: Hub<DynClient>,
    default_repo: Option<RepoId>,
    client: DynClient,
}

fn json_of<T: Serialize>(v: &T) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(v)?))
}

fn state_word(s: ItemState) -> &'static str {
    match s {
        ItemState::Open => "open",
        ItemState::Closed => "closed",
        ItemState::Merged => "merged",
    }
}

fn day(t: Option<chrono::DateTime<chrono::Utc>>) -> String {
    t.map_or_else(String::new, |t| t.format("%Y-%m-%d").to_string())
}

fn errors(out: &mut String, errs: &[String]) {
    for e in errs {
        let _ = writeln!(out, "! {e}");
    }
}

impl Session {
    /// A session over `hub`; `default_repo` answers commands given no `-R`.
    #[must_use]
    pub fn new(hub: Hub<DynClient>, default_repo: Option<RepoId>, client: DynClient) -> Self {
        Self {
            hub,
            default_repo,
            client,
        }
    }

    /// Finds the accounts the way the TUI does (env, keyring, `gh`/`glab`, git credentials).
    ///
    /// # Errors
    /// When the HTTP client cannot be built or the repo flag is not `host/owner/name`.
    pub async fn connect(cwd: PathBuf, repo: Option<&str>) -> Result<Self> {
        let default_repo = match repo.and_then(RepoId::from_key) {
            Some(r) => Some(r),
            None => gitkraft_hub::accounts::repo_of_dir(&cwd),
        };
        let extra: Vec<String> = default_repo.iter().map(RepoId::key).collect();
        let accounts =
            tokio::task::spawn_blocking(move || gitkraft_hub::accounts::discover_for(&cwd, &extra))
                .await?
                .map_err(|e| anyhow!(e))?;
        let client = DynClient::new(std::sync::Arc::new(gitkraft_platforms::CachedClient::new(
            gitkraft_platforms::ReqwestClient::new().map_err(|e| anyhow!(e.to_string()))?,
        )));
        Ok(Self::new(Hub::new(accounts, &client), default_repo, client))
    }

    fn repo(&self, given: Option<&str>) -> Result<RepoId> {
        args::resolve_repo(
            given,
            self.default_repo.as_ref(),
            &self.hub.hosts(),
            "not in a git checkout with a known remote: pass -R host/owner/name",
        )
        .map_err(|e| anyhow!(e))
    }

    fn backend(&self, repo: &RepoId) -> Result<&gitkraft_platforms::Backend<DynClient>> {
        self.hub.backend(&repo.host).ok_or_else(|| {
            anyhow!(
                "no account for {}: log in with `gitkraft-tui login {}` or set a token",
                repo.host,
                repo.host
            )
        })
    }

    fn ci(&self, repo: &RepoId) -> Result<&gitkraft_ci::CiBackend<DynClient>> {
        self.hub
            .ci(&repo.host)
            .ok_or_else(|| anyhow!("no account for {}", repo.host))
    }

    /// Runs one command and returns what to print.
    ///
    /// # Errors
    /// Anything the platform refuses, and arguments that make no sense.
    pub async fn run(&self, cmd: &Forge) -> Result<String> {
        let json = cmd.common().json;
        let repo_flag = cmd.common().repo.as_deref();
        match cmd {
            Forge::Accounts(_) => self.accounts(json).await,
            Forge::Inbox(_) => self.inbox(json).await,
            Forge::Prs(a) => self.items(a, true, json).await,
            Forge::Issues(a) => self.items(a, false, json).await,
            Forge::Show(a) => self.show(a, json).await,
            Forge::Runs(a) => {
                let repo = self.repo(repo_flag)?;
                let q = RunQuery {
                    branch: a.branch.clone(),
                    status: a
                        .status
                        .as_deref()
                        .map(args::status)
                        .transpose()
                        .map_err(|e| anyhow!(e))?,
                    page: 1,
                    per_page: a.limit.clamp(1, 100),
                };
                let runs = self.ci(&repo)?.list_runs(&repo, &q).await?;
                if json {
                    return json_of(&runs);
                }
                let mut o = String::new();
                for r in &runs {
                    let _ = writeln!(
                        o,
                        "{:<10} {:<9} {:<20} {} {}",
                        r.id,
                        format!("{:?}", r.status),
                        r.branch,
                        day(r.created_at),
                        r.name
                    );
                }
                Ok(o)
            }
            Forge::Run(a) => {
                let repo = self.repo(repo_flag)?;
                let d = self.ci(&repo)?.run_detail(&repo, a.run_id).await?;
                if json {
                    return json_of(&serde_json::json!({"run": d.run, "jobs": d.jobs}));
                }
                let mut o = format!(
                    "{} #{} {:?} on {}\n",
                    d.run.name, d.run.number, d.run.status, d.run.branch
                );
                for j in &d.jobs {
                    let _ = writeln!(
                        o,
                        "  {:<12} {:<9} {}",
                        j.id,
                        format!("{:?}", j.status),
                        j.name
                    );
                }
                Ok(o)
            }
            Forge::Log(a) => {
                let repo = self.repo(repo_flag)?;
                let raw = self.ci(&repo)?.job_log(&repo, a.job_id).await?;
                let x = args::log_excerpt(&raw, a.tail, a.grep.as_deref(), usize::MAX);
                let picked = x.lines;
                if json {
                    return json_of(
                        &serde_json::json!({"total_lines": x.total_lines, "lines": picked}),
                    );
                }
                Ok(picked.iter().fold(String::new(), |mut o, l| {
                    o.push_str(l);
                    o.push('\n');
                    o
                }))
            }
            Forge::Day(a) => self.day(a, json).await,
            Forge::Pulse(a) => {
                let repo = self.repo(repo_flag)?;
                self.backend(&repo)?;
                let p = self.hub.repo_pulse(&repo, a.days.clamp(1, 90)).await;
                if json {
                    return json_of(&serde_json::json!({
                        "repo": repo.key(), "days": p.days, "prs_opened": p.prs_opened,
                        "prs_merged": p.prs_merged, "prs_closed": p.prs_closed,
                        "prs_open_now": p.prs_open_now, "issues_opened": p.issues_opened,
                        "issues_closed": p.issues_closed, "ci_green": p.ci_green,
                        "ci_red": p.ci_red, "top_authors": p.top_authors,
                    }));
                }
                Ok(format!(
                    "{} over {} days\nPRs      {} opened, {} merged, {} closed, {} open now\nIssues   {} opened, {} closed\nCI       {} green, {} red\n",
                    repo.key(),
                    p.days,
                    p.prs_opened,
                    p.prs_merged,
                    p.prs_closed,
                    p.prs_open_now,
                    p.issues_opened,
                    p.issues_closed,
                    p.ci_green,
                    p.ci_red
                ))
            }
            Forge::Discussions(a) => {
                let repo = self.repo(repo_flag)?;
                let b = self.backend(&repo)?;
                match a.number {
                    Some(n) => {
                        let d = b.discussion(&repo, n).await?;
                        if json {
                            return json_of(&d);
                        }
                        let mut o = format!(
                            "#{} {} [{}]\n{}\n",
                            d.discussion.number,
                            d.discussion.title,
                            d.discussion.category,
                            d.discussion.body
                        );
                        for c in &d.comments {
                            let _ = write!(o, "\n— {}\n{}\n", c.author.login, c.body);
                        }
                        Ok(o)
                    }
                    None => {
                        let list = b.discussions(&repo, a.limit.clamp(1, 100)).await?;
                        if json {
                            return json_of(&list);
                        }
                        Ok(list.iter().fold(String::new(), |mut o, d| {
                            let _ = writeln!(
                                o,
                                "#{:<7} {:<14} {}{} ({} comments, {})",
                                d.number,
                                d.category,
                                if d.answered { "✓ " } else { "" },
                                d.title,
                                d.comments,
                                d.author.login
                            );
                            o
                        }))
                    }
                }
            }
            Forge::Boards(_) => {
                let repo = self.repo(repo_flag)?;
                let boards = self.backend(&repo)?.boards(&repo).await?;
                if json {
                    return json_of(&boards);
                }
                let mut o = String::new();
                for b in &boards {
                    let _ = writeln!(o, "{}", b.name);
                    for c in &b.columns {
                        let _ = writeln!(o, "  {} ({})", c.name, c.cards.len());
                        for card in &c.cards {
                            let _ = writeln!(
                                o,
                                "    {} {}",
                                card.number.map(|n| format!("#{n}")).unwrap_or_default(),
                                card.title
                            );
                        }
                    }
                }
                Ok(o)
            }
            Forge::Wiki(a) => {
                let repo = self.repo(repo_flag)?;
                let b = self.backend(&repo)?;
                match &a.page {
                    Some(slug) => {
                        let p = b.wiki_page(&repo, slug).await?;
                        if json {
                            return json_of(&p);
                        }
                        Ok(format!("{}\n", p.content))
                    }
                    None => {
                        let pages = b.wiki_pages(&repo).await?;
                        if json {
                            return json_of(&pages);
                        }
                        Ok(pages.iter().fold(String::new(), |mut o, p| {
                            let _ = writeln!(o, "{:<30} {}", p.slug, p.title);
                            o
                        }))
                    }
                }
            }
            Forge::Comment(a) => {
                let repo = self.repo(repo_flag)?;
                if a.body.trim().is_empty() {
                    bail!("the comment is empty");
                }
                let kind = if a.issue {
                    ItemKind::Issue
                } else {
                    ItemKind::PullRequest
                };
                self.backend(&repo)?
                    .post_comment(&repo, kind, a.number, &a.body)
                    .await?;
                Ok(format!("commented on {}#{}\n", repo.key(), a.number))
            }
            Forge::Reply(a) => {
                let repo = self.repo(repo_flag)?;
                let done = gitkraft_hub::community::reply(
                    &self.hub,
                    &repo,
                    a.number,
                    a.comment_id,
                    &a.body,
                )
                .await
                .map_err(|e| anyhow!(e))?;
                Ok(format!("{done}\n"))
            }
            Forge::BoardMove(a) => {
                let repo = self.repo(repo_flag)?;
                let done = gitkraft_hub::community::move_card(
                    &self.hub,
                    &repo,
                    a.board.as_deref(),
                    a.number,
                    &a.column,
                )
                .await
                .map_err(|e| anyhow!(e))?;
                Ok(format!("{done}\n"))
            }
            Forge::WikiSave(a) => self.wiki_save(a, repo_flag).await,
            Forge::Merge(a) => {
                let repo = self.repo(repo_flag)?;
                let options = MergeOptions {
                    method: args::merge_method(Some(&a.method)).map_err(|e| anyhow!(e))?,
                    timing: args::merge_timing(Some(if a.queue {
                        "queue"
                    } else if a.when_green {
                        "when_green"
                    } else {
                        "now"
                    }))
                    .map_err(|e| anyhow!(e))?,
                    force: false,
                };
                let outcome = self
                    .backend(&repo)?
                    .merge_with(&repo, a.number, &options)
                    .await?;
                Ok(format!("{}\n", outcome.describe(a.number)))
            }
            Forge::Explain(a) => self.explain(a, repo_flag, json).await,
            Forge::Rerun(a) => {
                let repo = self.repo(repo_flag)?;
                let ci = self.ci(&repo)?;
                let run = ci.run_detail(&repo, a.run_id).await?.run;
                let scope = if a.all {
                    RerunScope::All
                } else {
                    RerunScope::Failed
                };
                ci.rerun(&repo, &run, scope).await?;
                Ok(format!("run {} is starting again\n", a.run_id))
            }
        }
    }

    async fn wiki_save(&self, a: &WikiSaveArgs, repo_flag: Option<&str>) -> Result<String> {
        use gitkraft_hub::community::{WikiDraft, save_wiki, upload_draft};
        let repo = self.repo(repo_flag)?;
        // The page as it is now, for its title and as the editor's starting text.
        let current = match &a.page {
            Some(slug) => Some(self.backend(&repo)?.wiki_page(&repo, slug).await?),
            None => None,
        };
        let title = a
            .title
            .clone()
            .or_else(|| current.as_ref().map(|p| p.page.title.clone()))
            .ok_or_else(|| anyhow!("a new page needs --title"))?;
        if a.edit {
            let draft = WikiDraft::write(
                &repo,
                a.page.as_deref(),
                &title,
                current.as_ref().map_or("", |p| p.content.as_str()),
            )
            .map_err(|e| anyhow!(e))?;
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".into());
            let mut parts = editor.split_whitespace();
            let program = parts.next().unwrap_or("vi");
            let status = std::process::Command::new(program)
                .args(parts)
                .arg(&draft.path)
                .status()
                .map_err(|e| anyhow!("cannot run {program}: {e}"))?;
            if !status.success() {
                bail!("{program} did not finish cleanly: nothing was saved");
            }
            return Ok(format!(
                "{}\n",
                upload_draft(&self.hub, &draft)
                    .await
                    .map_err(|e| anyhow!(e))?
            ));
        }
        let content = if a.file == "-" {
            std::io::read_to_string(std::io::stdin())?
        } else {
            std::fs::read_to_string(&a.file).map_err(|e| anyhow!("{}: {e}", a.file))?
        };
        let done = save_wiki(
            &self.hub,
            &repo,
            a.page.as_deref(),
            &title,
            &content,
            a.message.as_deref().unwrap_or_default(),
        )
        .await
        .map_err(|e| anyhow!(e))?;
        Ok(format!("{done}\n"))
    }

    async fn explain(
        &self,
        a: &ExplainArgs,
        repo_flag: Option<&str>,
        json: bool,
    ) -> Result<String> {
        use gitkraft_hub::ai::{self, AiConfig, Remote};
        let repo = self.repo(repo_flag)?;
        let ci = self.ci(&repo)?;
        let detail = ci.run_detail(&repo, a.run_id).await?;
        let job = match a.job_id {
            Some(id) => detail.jobs.iter().find(|j| j.id == id),
            None => ai::failed_job(&detail.jobs, None),
        }
        .ok_or_else(|| anyhow!("no failed job in run {}: pass a job id", a.run_id))?;
        let ctx = ai::failure_context(ci, &detail.run, job)
            .await
            .map_err(|e| anyhow!(e))?;
        if a.dry_run {
            return Ok(format!("{}\n", ai::failure_prompt(&ctx)));
        }
        let env = |n: &str| std::env::var(n).ok();
        let config = AiConfig::from_env(&env);
        let key = config.key(&env, &gitkraft_platforms::credentials::SystemKeyring)?;
        let llm = Remote::new(self.client.clone(), config, key);
        let text = ai::explain_failure(&llm, &ctx).await?;
        if json {
            return json_of(&serde_json::json!({"job": job.name, "explanation": text}));
        }
        Ok(format!("{} / {}\n\n{text}\n", ctx.workflow, ctx.job))
    }

    async fn accounts(&self, json: bool) -> Result<String> {
        let mut rows = Vec::new();
        for host in self.hub.hosts() {
            let me = self.hub.me(host).await.map_err(|e| e.to_string());
            rows.push(serde_json::json!({
                "host": host,
                "kind": self.hub.backend(host).map(|b| format!("{:?}", b.kind())),
                "me": me.as_ref().ok(),
                "error": me.as_ref().err(),
                "ci": self.hub.ci(host).map(|c| c.system_name()),
            }));
        }
        if json {
            return json_of(&rows);
        }
        if rows.is_empty() {
            return Ok(
                "no accounts: set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or run `gitkraft-tui login HOST`\n"
                    .into(),
            );
        }
        Ok(rows.iter().fold(String::new(), |mut o, r| {
            let _ = writeln!(
                o,
                "{:<20} {:<8} {}",
                r["host"].as_str().unwrap_or_default(),
                r["kind"].as_str().unwrap_or_default(),
                r["me"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| r["error"].as_str().map(|e| format!("! {e}")))
                    .unwrap_or_default()
            );
            o
        }))
    }

    async fn inbox(&self, json: bool) -> Result<String> {
        let SectionData::Inbox(r) = self
            .hub
            .load_section(&args::section(
                "cli",
                SectionKind::Inbox,
                None,
                StateFilter::Open,
            ))
            .await
        else {
            bail!("unexpected section data");
        };
        if json {
            return json_of(&serde_json::json!({"items": r.items, "errors": r.errors}));
        }
        let mut o = String::new();
        for n in &r.items {
            let _ = writeln!(
                o,
                "{} {:<28} {:<16} {}",
                if n.unread { "●" } else { " " },
                n.repo.as_ref().map(RepoId::key).unwrap_or_default(),
                format!("{:?}", n.reason),
                n.title
            );
        }
        errors(&mut o, &r.errors);
        Ok(o)
    }

    async fn items(&self, a: &ListArgs, prs: bool, json: bool) -> Result<String> {
        let state = args::state_filter(Some(&a.state)).map_err(|e| anyhow!(e))?;
        if a.common.repo.is_some() {
            let repo = self.repo(a.common.repo.as_deref())?;
            let q = ListQuery {
                state,
                author: a.who.clone(),
                assignee: None,
                label: a.label.clone(),
                page: Page {
                    page: 1,
                    per_page: a.limit.clamp(1, 100),
                },
            };
            let b = self.backend(&repo)?;
            let rows: Vec<(u64, String, String, String, String)> = if prs {
                b.list_pull_requests(&repo, &q)
                    .await?
                    .iter()
                    .map(|p| row(p.number, p.state, &p.title, &p.author.login, p.updated_at))
                    .collect()
            } else {
                b.list_issues(&repo, &q)
                    .await?
                    .iter()
                    .map(|i| row(i.number, i.state, &i.title, &i.author.login, i.updated_at))
                    .collect()
            };
            // JSON keeps the full models
            if json {
                return if prs {
                    json_of(&b.list_pull_requests(&repo, &q).await?)
                } else {
                    json_of(&b.list_issues(&repo, &q).await?)
                };
            }
            return Ok(rows.iter().fold(String::new(), |mut o, r| {
                let _ = writeln!(o, "#{:<7} {:<7} {:<16} {}  {}", r.0, r.1, r.3, r.2, r.4);
                o
            }));
        }
        let who = args::involvement(a.who.as_deref()).map_err(|e| anyhow!(e))?;
        let kind = if prs {
            SectionKind::Prs
        } else {
            SectionKind::Issues
        };
        let data = self
            .hub
            .load_section(&args::section("cli", kind, Some(who), state))
            .await;
        match data {
            SectionData::Prs(r) => self.mine(json, &r.items, &r.errors, |p| {
                (&p.repo, p.number, p.state, &p.title)
            }),
            SectionData::Issues(r) => self.mine(json, &r.items, &r.errors, |i| {
                (&i.repo, i.number, i.state, &i.title)
            }),
            _ => bail!("unexpected section data"),
        }
    }

    #[allow(clippy::unused_self, clippy::type_complexity)]
    fn mine<T: Serialize>(
        &self,
        json: bool,
        items: &[T],
        errs: &[String],
        f: impl Fn(&T) -> (&RepoId, u64, ItemState, &String),
    ) -> Result<String> {
        if json {
            return json_of(&serde_json::json!({"items": items, "errors": errs}));
        }
        let mut o = String::new();
        for i in items {
            let (repo, n, s, title) = f(i);
            let _ = writeln!(o, "{}#{n:<6} {:<7} {title}", repo.key(), state_word(s));
        }
        errors(&mut o, errs);
        Ok(o)
    }

    async fn show(&self, a: &ShowArgs, json: bool) -> Result<String> {
        let repo = self.repo(a.common.repo.as_deref())?;
        if a.issue {
            let b = self.backend(&repo)?;
            let issue = b.issue(&repo, a.number).await?;
            let comments = b.comments(&repo, ItemKind::Issue, a.number).await?;
            if json {
                return json_of(&serde_json::json!({"issue": issue, "comments": comments}));
            }
            let mut o = format!(
                "#{} {} [{}] by {}\n\n{}\n",
                issue.number,
                issue.title,
                state_word(issue.state),
                issue.author.login,
                issue.body
            );
            for c in &comments {
                let _ = write!(o, "\n— {}\n{}\n", c.author.login, c.body);
            }
            return Ok(o);
        }
        self.backend(&repo)?;
        let d = self.hub.pr_detail(&repo, a.number).await?;
        if json {
            return json_of(&serde_json::json!({
                "pull_request": d.pr, "reviews": d.reviews, "comments": d.comments,
                "diff": a.diff.then_some(&d.diff), "errors": d.errors,
            }));
        }
        let p = &d.pr;
        let mut o = format!(
            "#{} {} [{}{}] {} → {} by {}\nchecks: {}   mergeable: {}{}\n\n{}\n",
            p.number,
            p.title,
            state_word(p.state),
            if p.draft { ", draft" } else { "" },
            p.source_branch,
            p.target_branch,
            p.author.login,
            p.checks.map_or("none".into(), |c| format!("{c:?}")),
            p.mergeable.map_or("unknown".into(), |m| m.to_string()),
            if p.behind == Some(true) {
                "   behind its target"
            } else {
                ""
            },
            p.body
        );
        for r in &d.reviews {
            let _ = writeln!(o, "review: {} {:?}", r.author.login, r.verdict);
        }
        for c in &d.comments {
            let _ = write!(o, "\n— {}\n{}\n", c.author.login, c.body);
        }
        if a.diff {
            let _ = write!(o, "\n{}", d.diff);
        }
        errors(&mut o, &d.errors);
        Ok(o)
    }

    async fn day(&self, a: &DayArgs, json: bool) -> Result<String> {
        let today = chrono::Utc::now().date_naive();
        let date = match a.date.as_deref() {
            None => today,
            Some(t) => t
                .parse::<chrono::NaiveDate>()
                .map_err(|_| anyhow!("date must be YYYY-MM-DD"))?,
        };
        if date > today || (today - date).num_days() > 370 {
            bail!("date must be within the last year and not in the future");
        }
        let r = self.hub.day_activity(date).await;
        let groups: Vec<_> = r.activity.groups();
        if json {
            let g: Vec<_> = groups
                .iter()
                .map(|(k, n, items)| serde_json::json!({"heading": k.heading(), "count": n, "items": items}))
                .collect();
            return json_of(&serde_json::json!({
                "headline": r.activity.headline(), "total": r.activity.total,
                "groups": g, "errors": r.errors,
            }));
        }
        let mut o = format!("{}\n", r.activity.headline());
        for (kind, n, items) in &groups {
            let _ = writeln!(o, "\n{} ({n})", kind.heading());
            for e in items {
                let _ = writeln!(
                    o,
                    "  {} {} {}",
                    e.repo,
                    e.number.map(|n| format!("#{n}")).unwrap_or_default(),
                    e.title
                );
            }
        }
        errors(&mut o, &r.errors);
        Ok(o)
    }
}

fn row(
    n: u64,
    s: ItemState,
    title: &str,
    author: &str,
    at: Option<chrono::DateTime<chrono::Utc>>,
) -> (u64, String, String, String, String) {
    (
        n,
        state_word(s).into(),
        title.into(),
        author.into(),
        day(at),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, PlatformKind, Secret};

    const API: &str = "https://api.github.com/repos/o/r";

    fn rig() -> (Session, FakeClient) {
        let fake = FakeClient::new();
        let mut account =
            Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
        account.me = Some("me".into());
        let hub = Hub::new(vec![account], &DynClient::new(fake.clone()));
        (
            Session::new(
                hub,
                Some(RepoId::new("github.com", "o", "r")),
                DynClient::new(fake.clone()),
            ),
            fake,
        )
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("gitkraft-tui").chain(args.iter().copied()))
    }

    fn forge(args: &[&str]) -> Forge {
        match parse(args).map(|c| c.command) {
            Ok(Some(Cmd::Forge(f))) => f,
            other => panic!("not a forge command: {other:?}"),
        }
    }

    #[test]
    fn no_arguments_open_the_tui_and_a_path_is_the_repo() {
        let c = parse(&[]).expect("parse");
        assert!(c.command.is_none() && c.repo_path.is_none());
        let c = parse(&["/tmp/r", "--theme", "Nord"]).expect("parse");
        assert_eq!(c.repo_path.as_deref(), Some(std::path::Path::new("/tmp/r")));
        assert_eq!(c.theme.as_deref(), Some("Nord"));
        assert!(parse(&["--list-themes"]).expect("parse").list_themes);
        assert!(parse(&["--bogus"]).is_err());
    }

    #[test]
    fn legacy_commands_keep_their_own_arguments_and_flags() {
        match parse(&["notify", "status", "--json"]).map(|c| c.command) {
            Ok(Some(Cmd::Notify { args })) => assert_eq!(args, ["status", "--json"]),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            parse(&["ssh", "setup", "--yes"]).map(|c| c.command),
            Ok(Some(Cmd::Ssh { .. }))
        ));
    }

    #[test]
    fn the_repo_and_json_flags_work_in_any_order_after_the_command() {
        for args in [
            ["runs", "-R", "o/r", "--json"],
            ["runs", "--json", "--repo", "o/r"],
        ] {
            let f = forge(&args);
            assert_eq!(f.common().repo.as_deref(), Some("o/r"));
            assert!(f.common().json);
        }
        assert!(parse(&["merge", "7", "--when-green", "--queue"]).is_err());
    }

    #[test]
    fn completions_name_the_commands() {
        let s = completions(clap_complete::Shell::Bash);
        assert!(s.contains("prs") && s.contains("merge") && s.contains("gitkraft-tui"));
    }

    #[tokio::test]
    async fn runs_lists_text_and_json() {
        let (s, fake) = rig();
        let body = r#"{"workflow_runs":[{"id":5,"name":"CI","status":"completed","conclusion":"failure","head_branch":"main","run_number":3,"event":"push","created_at":"2026-05-01T10:00:00Z","html_url":"u","actor":{"login":"a"}}]}"#;
        fake.push_json(&format!("{API}/actions/runs"), body);
        let text = s.run(&forge(&["runs"])).await.expect("runs");
        assert!(text.contains("5") && text.contains("Failed") && text.contains("main"));
        let fake2 = FakeClient::new();
        fake2.push_json(&format!("{API}/actions/runs"), body);
        let mut account =
            Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
        account.me = Some("me".into());
        let s2 = Session::new(
            Hub::new(vec![account], &DynClient::new(fake2)),
            Some(RepoId::new("github.com", "o", "r")),
            DynClient::new(FakeClient::new()),
        );
        let j = s2.run(&forge(&["runs", "--json"])).await.expect("json");
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid json");
        assert_eq!(v[0]["id"], 5);
    }

    #[tokio::test]
    async fn comment_posts_and_an_empty_one_is_refused() {
        let (s, fake) = rig();
        fake.push_json(&format!("{API}/issues/7/comments"), r#"{"id":1}"#);
        let out = s
            .run(&forge(&["comment", "7", "looks good"]))
            .await
            .expect("comment");
        assert!(out.contains("o/r#7") || out.contains("github.com/o/r#7"));
        let posted = fake.requests();
        assert!(
            String::from_utf8_lossy(posted[0].body.as_deref().unwrap_or_default())
                .contains("looks good")
        );
        assert!(s.run(&forge(&["comment", "7", "  "])).await.is_err());
    }

    #[tokio::test]
    async fn bad_arguments_and_a_missing_repo_are_errors_not_panics() {
        let (s, _) = rig();
        assert!(
            s.run(&forge(&["merge", "1", "--method", "fast"]))
                .await
                .is_err()
        );
        assert!(
            s.run(&forge(&["prs", "-R", "o/r", "--state", "nope"]))
                .await
                .is_err()
        );
        assert!(s.run(&forge(&["day", "tomorrow"])).await.is_err());
        let hub = Hub::new(vec![], &DynClient::new(FakeClient::new()));
        let none = Session::new(hub, None, DynClient::new(FakeClient::new()));
        assert!(none.run(&forge(&["runs"])).await.is_err());
        assert!(none.run(&forge(&["runs", "-R", "o/r"])).await.is_err());
    }

    fn failed_run(fake: &FakeClient) {
        fake.push_json(
            &format!("{API}/actions/runs/101"),
            r#"{"id":101,"name":"CI","status":"completed","conclusion":"failure","head_branch":"main","created_at":"2026-03-01T10:00:00Z","updated_at":"2026-03-01T10:05:05Z"}"#,
        );
        fake.push_json(
            &format!("{API}/actions/runs/101/jobs"),
            r#"{"jobs":[{"id":7,"name":"lint","status":"completed","conclusion":"success","steps":[]},{"id":8,"name":"test","status":"completed","conclusion":"failure","steps":[]}]}"#,
        );
        fake.push_status(
            &format!("{API}/actions/jobs/8/logs"),
            200,
            "running tests\nerror: boom with ghp_abcdefghijklmnopqrstuvwxyz0123\n",
        );
    }

    #[tokio::test]
    async fn explain_dry_run_shows_the_redacted_prompt_of_the_first_failed_job() {
        let (s, fake) = rig();
        failed_run(&fake);
        let out = s
            .run(&forge(&["explain", "101", "--dry-run"]))
            .await
            .expect("prompt");
        assert!(out.contains("Job: test") && out.contains("error: boom"));
        assert!(!out.contains("ghp_abc"), "{out}");
        assert!(
            fake.requests().iter().all(|r| !r.url.contains("anthropic")),
            "a dry run sends nothing to the model"
        );
    }

    #[tokio::test]
    async fn a_run_without_a_failed_job_says_so() {
        let (s, fake) = rig();
        fake.push_json(
            &format!("{API}/actions/runs/5"),
            r#"{"id":5,"name":"CI","status":"completed","conclusion":"success","created_at":"2026-03-01T10:00:00Z"}"#,
        );
        fake.push_json(&format!("{API}/actions/runs/5/jobs"), r#"{"jobs":[]}"#);
        let e = s.run(&forge(&["explain", "5"])).await.expect_err("none");
        assert!(e.to_string().contains("no failed job"));
    }

    #[tokio::test]
    async fn reply_and_board_move_go_through_the_shared_helpers_and_report_their_errors() {
        let (s, fake) = rig();
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"repository":{"discussion":{"id":"D","comments":{"nodes":[{"databaseId":9,"id":"C_9","replies":{"nodes":[]}}]}}}}}"#,
        );
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"addDiscussionComment":{"comment":{"id":"n"}}}}"#,
        );
        let out = s
            .run(&forge(&["reply", "5", "9", "thanks"]))
            .await
            .expect("reply");
        assert!(out.contains("comment 9"), "{out}");
        let err = s
            .run(&forge(&["reply", "5", "9", " "]))
            .await
            .expect_err("empty");
        assert!(err.to_string().contains("some text"));

        let (s, fake) = rig();
        fake.push_json(
            "https://api.github.com/graphql",
            r#"{"data":{"repository":{"projectsV2":{"nodes":[]}}}}"#,
        );
        let err = s
            .run(&forge(&["board-move", "1", "Done"]))
            .await
            .expect_err("no card");
        assert!(err.to_string().contains("no board"), "{err}");
        assert!(
            parse(&["board-move", "1"]).is_err(),
            "the column is required"
        );
    }

    #[tokio::test]
    async fn wiki_save_reads_a_file_and_needs_a_title_for_a_new_page() {
        let (s, _) = rig();
        let err = s.run(&forge(&["wiki-save"])).await.expect_err("no title");
        assert!(err.to_string().contains("--title"), "{err}");
        assert!(parse(&["wiki-save", "--edit", "--file", "x"]).is_err());
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("p.md");
        std::fs::write(&file, "# Hi").expect("write");
        let err = s
            .run(&forge(&[
                "wiki-save",
                "--title",
                "Home",
                "--file",
                &file.display().to_string(),
            ]))
            .await
            .expect_err("github has no wiki api");
        assert!(err.to_string().contains("git"), "{err}");
    }
}
