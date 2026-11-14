//! One job, start to report: pick the repositories, scan, and when something is red let the
//! chosen harness look at it. Nothing is pushed, merged or re-run.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use gitkraft_platforms::HttpClient;
use gitkraft_platforms::RepoId;
use gitkraft_platforms::model::RepoInfo;
use gitkraft_platforms::notify::{Alert, Inbox, Notifier};
use gitkraft_settings::notify::NotifyEvent;

use super::config::{HarnessKind, Job};
use super::harness::{Exec, invocation};
use super::scan::{Finding, FindingKind, Scan, scan};
use super::store::Report;
use crate::ai::{Llm, explain_failure, failed_job, failure_context};
use crate::hub::Hub;
use crate::review_ai::{Action, Policy};
use crate::wiki_git::blocking;

/// Findings the built-in model explains per run (each costs a model call).
const MAX_EXPLAINED: usize = 5;

/// The instructions every harness gets unless the job brings its own.
pub const DEFAULT_PROMPT: &str = "You watch the CI health of the repositories below. For each red item, read the failed log \
with the forge's own read-only tools (gh, glab, tea or its web page), then say the cause in one or two sentences, the fix, \
and whether it looks flaky. Do NOT push, merge, re-run, comment, change settings or touch any file: report only. \
Answer in short markdown, one bullet per item, and end with a line `FINDINGS: N` for the items that still need a person.";

/// What a run needs from outside.
pub struct Deps<'a, L: Llm> {
    /// The built-in model, when one is configured.
    pub llm: Option<&'a L>,
    /// How process harnesses are started.
    pub exec: Arc<dyn Exec>,
    /// Which hosts a model may see.
    pub policy: &'a Policy,
    /// A writable folder for prompt files.
    pub scratch: PathBuf,
}

impl<L: Llm> std::fmt::Debug for Deps<'_, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deps")
            .field("scratch", &self.scratch)
            .finish_non_exhaustive()
    }
}

/// A finished run: the report to keep and what to announce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The report.
    pub report: Report,
    /// One alert per finding for the inbox (empty when the job has the inbox off).
    pub inbox: Vec<Alert>,
    /// The one alert for the system notification (None when green or switched off).
    pub system: Option<Alert>,
}

fn alert(f: &Finding) -> Alert {
    let (host, rest) = f.repo.split_once('/').unwrap_or((&f.repo, ""));
    let (owner, name) = rest.rsplit_once('/').unwrap_or(("", rest));
    Alert {
        repo: RepoId::new(host, owner, name),
        event: NotifyEvent::CiFailed,
        number: f.number,
        title: match f.kind {
            FindingKind::FailedRun => format!("{} failed on the default branch", f.what),
            FindingKind::FailingPr => format!(
                "PR #{} {}: a check fails",
                f.number.unwrap_or_default(),
                f.what
            ),
        },
        url: f.url.clone(),
        branch: f.branch.clone(),
        labels: vec!["agent-watch".into()],
        author: None,
        mine: true,
        involved: true,
    }
}

fn outcome(job: &Job, report: Report) -> Outcome {
    let inbox = if job.notify.inbox {
        report.findings.iter().map(alert).collect()
    } else {
        Vec::new()
    };
    let system = (job.notify.system && report.needs_you()).then(|| {
        let mut a = report.findings.first().map_or_else(
            || Alert {
                repo: RepoId::new("agent-watch", "gitkraft", &job.name),
                event: NotifyEvent::CiFailed,
                number: None,
                title: String::new(),
                url: String::new(),
                branch: None,
                labels: vec![],
                author: None,
                mine: true,
                involved: true,
            },
            alert,
        );
        a.title = match &report.failed {
            Some(e) => format!("{}: the run failed: {e}", job.name),
            None => format!(
                "{}: {} red in {} repos",
                job.name,
                report.findings.len(),
                report.repos
            ),
        };
        a
    });
    Outcome {
        report,
        inbox,
        system,
    }
}

/// Puts the outcome's alerts in `inbox` and shows the summary through `notifier`.
pub fn announce(outcome: &Outcome, inbox: &mut Inbox, notifier: &dyn Notifier, at: DateTime<Utc>) {
    for a in &outcome.inbox {
        inbox.push(a.clone(), at);
    }
    if let Some(a) = &outcome.system {
        notifier.show(a, true);
    }
}

/// Runs `job` and keeps its report in `store`; the caller announces the returned outcome (an app
/// that holds the inbox in memory does that itself, see [`announce`]).
///
/// # Errors
/// The report could not be written.
pub async fn run_and_file<C: HttpClient + Clone, L: Llm>(
    hub: &Hub<C>,
    job: &Job,
    universe: &[RepoInfo],
    deps: &Deps<'_, L>,
    store: &super::store::Store,
) -> Result<Outcome, String> {
    let out = run_job(hub, job, universe, deps, Utc::now()).await;
    store.save(&out.report)?;
    Ok(out)
}

/// Runs `job` and files everything: the report in `store`, the alerts in the inbox file at
/// `inbox_path` and one notification through `notifier`. What every front-end and the service call.
///
/// # Errors
/// The report or the inbox could not be written.
pub async fn execute<C: HttpClient + Clone, L: Llm>(
    hub: &Hub<C>,
    job: &Job,
    universe: &[RepoInfo],
    deps: &Deps<'_, L>,
    store: &super::store::Store,
    inbox_path: &std::path::Path,
    notifier: &dyn Notifier,
) -> Result<Outcome, String> {
    use gitkraft_settings::files;
    let out = run_and_file(hub, job, universe, deps, store).await?;
    let mut inbox = files::read_optional(inbox_path)
        .map_err(|e| e.to_string())?
        .map(|t| Inbox::from_json(&t))
        .unwrap_or_default();
    announce(&out, &mut inbox, notifier, Utc::now());
    files::write_atomic(inbox_path, &inbox.to_json()).map_err(|e| e.to_string())?;
    Ok(out)
}

/// The prompt for a process harness: the instructions and the red items.
#[must_use]
pub fn build_prompt(job: &Job, findings: &[&Finding]) -> String {
    let mut s = job
        .prompt
        .clone()
        .unwrap_or_else(|| DEFAULT_PROMPT.to_string());
    s.push_str("\n\nRed right now:\n");
    for f in findings {
        s.push_str(&format!("- {} {}\n", f.headline(), f.url));
    }
    s
}

fn host_of(f: &Finding) -> &str {
    f.repo.split('/').next().unwrap_or_default()
}

/// Runs `job` over `universe` (every repository the accounts can see).
pub async fn run_job<C: HttpClient + Clone, L: Llm>(
    hub: &Hub<C>,
    job: &Job,
    universe: &[RepoInfo],
    deps: &Deps<'_, L>,
    now: DateTime<Utc>,
) -> Outcome {
    let picked = job.projects.pick(universe);
    let mut found: Scan = scan(hub, &picked).await;
    let mut report = Report {
        job: job.name.clone(),
        started: now,
        finished: now,
        harness: "none".into(),
        model: None,
        repos: found.repos,
        workflows: found.workflows,
        findings: Vec::new(),
        errors: std::mem::take(&mut found.errors),
        summary: String::new(),
        failed: None,
        seen: false,
    };
    if found.is_green() {
        report.summary = format!(
            "All green: {} repos, {} workflows on their default branch, no open pull request with a failing check.",
            found.repos, found.workflows
        );
        report.finished = Utc::now();
        return outcome(job, report);
    }
    report.harness = job.harness.name().to_string();
    report.model = job.model.clone();
    report.findings = found.findings;

    let allowed: Vec<usize> = (0..report.findings.len())
        .filter(|i| {
            deps.policy
                .allows(host_of(&report.findings[*i]), Action::Watch)
        })
        .collect();
    let blocked = report.findings.len() - allowed.len();
    let mut notes = Vec::new();
    if blocked > 0 {
        notes.push(format!(
            "{blocked} red item(s) on hosts that have not allowed `watch` were not shown to an agent (`gitkraft-tui ai-allow HOST watch`)."
        ));
    }
    if allowed.is_empty() {
        report.summary = notes.join("\n");
    } else if job.harness == HarnessKind::Builtin {
        let Some(llm) = deps.llm else {
            report.failed = Some("no model configured: set GITKRAFT_AI_KEY (or GITKRAFT_AI_PROVIDER/MODEL/URL for a local one), or pick another harness".into());
            report.finished = Utc::now();
            return outcome(job, report);
        };
        let todo: Vec<usize> = allowed
            .into_iter()
            .filter(|i| report.findings[*i].run_id.is_some())
            .take(MAX_EXPLAINED)
            .collect();
        for i in todo {
            let f = report.findings[i].clone();
            let (Some(ci), Some(run_id)) = (hub.ci(host_of(&f)), f.run_id) else {
                continue;
            };
            let repo = repo_of(&f);
            let explained = async {
                let d = ci
                    .run_detail(&repo, run_id)
                    .await
                    .map_err(|e| e.to_string())?;
                let job = failed_job(&d.jobs, None).ok_or("no failed job")?;
                let ctx = failure_context(ci, &d.run, job).await?;
                explain_failure(llm, &ctx).await.map_err(|e| e.to_string())
            }
            .await;
            report.findings[i].explanation =
                Some(explained.unwrap_or_else(|e| format!("could not explain: {e}")));
        }
        notes.push(
            "Failing pull requests are listed without explanation: open them to ask for one."
                .into(),
        );
        report.summary = notes.join("\n");
    } else {
        let shown: Vec<&Finding> = allowed.iter().map(|i| &report.findings[*i]).collect();
        let prompt = build_prompt(job, &shown);
        match invocation(job.harness, job.model.as_deref(), &prompt, &deps.scratch) {
            None => {}
            Some(inv) => {
                if let Some(file) = &inv.prompt_file
                    && let Err(e) = std::fs::write(file, &prompt)
                {
                    report.failed = Some(format!("{}: {e}", file.display()));
                }
                if report.failed.is_none() {
                    let exec = deps.exec.clone();
                    let (program, args) = (inv.program.clone(), inv.args.clone());
                    let answer = blocking(move || exec.run(&program, &args))
                        .await
                        .and_then(|r| r);
                    match answer {
                        Ok(text) if job.harness.headless() => notes.push(text),
                        Ok(_) => notes.push(format!(
                            "Opened in {}: read the answer there.",
                            job.harness.name()
                        )),
                        Err(e) => report.failed = Some(e),
                    }
                }
            }
        }
        report.summary = notes.join("\n\n");
    }
    report.finished = Utc::now();
    outcome(job, report)
}

fn repo_of(f: &Finding) -> RepoId {
    let (host, rest) = f.repo.split_once('/').unwrap_or((&f.repo, ""));
    let (owner, name) = rest.rsplit_once('/').unwrap_or(("", rest));
    RepoId::new(host, owner, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_watch::harness::ScriptedExec;
    use crate::ai::FakeLlm;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{DynClient, PlatformKind};

    use gitkraft_platforms::{Account, Secret};

    fn info(name: &str, private: bool) -> RepoInfo {
        RepoInfo {
            repo: RepoId::new("github.com", "o", name),
            private,
            default_branch: Some("main".into()),
            ..RepoInfo::default()
        }
    }

    fn runs_json(id: u64, name: &str, conclusion: &str) -> String {
        format!(
            r#"{{"workflow_runs":[{{"id":{id},"name":"{name}","display_title":"t","run_number":1,"status":"completed","conclusion":"{conclusion}","head_branch":"main","head_sha":"abc","event":"push","created_at":"2026-03-01T08:00:00Z","html_url":"https://github.com/o/r/actions/runs/{id}"}}]}}"#
        )
    }

    /// A hub whose only repository `r` has one workflow ending in `conclusion`.
    fn hub_with(conclusion: &str) -> Hub<DynClient> {
        let fake = FakeClient::new();
        fake.push_json(
            "https://api.github.com/repos/o/r/actions/runs",
            &runs_json(9, "CI", conclusion),
        );
        fake.push_json("https://api.github.com/repos/o/r/pulls", "[]");
        let a = Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
        Hub::new(vec![a], &DynClient::new(fake))
    }

    fn deps<'a>(
        llm: Option<&'a FakeLlm>,
        exec: Arc<dyn Exec>,
        policy: &'a Policy,
    ) -> Deps<'a, FakeLlm> {
        Deps {
            llm,
            exec,
            policy,
            scratch: std::env::temp_dir(),
        }
    }

    fn allowed() -> Policy {
        let mut p = Policy::default();
        p.allow("github.com", &[Action::Watch]);
        p
    }

    fn pi_job() -> Job {
        let mut j = Job::new("j", "daily 08:00");
        j.harness = HarnessKind::Pi;
        j.model = Some("m/x".into());
        j
    }

    #[tokio::test]
    async fn executing_a_red_job_files_the_report_the_inbox_and_one_notification() {
        use gitkraft_platforms::notify::RecordingNotifier;
        let tmp = tempfile::tempdir().expect("tmp");
        let store = super::super::store::Store::new(tmp.path().join("reports"));
        let inbox = tmp.path().join("inbox.json");
        let n = RecordingNotifier::default();
        let policy = allowed();
        let exec = Arc::new(ScriptedExec::default());
        let out = execute(
            &hub_with("failure"),
            &pi_job(),
            &[info("r", false)],
            &deps(None, exec, &policy),
            &store,
            &inbox,
            &n,
        )
        .await
        .expect("execute");
        assert_eq!(out.report.findings.len(), 1);
        assert_eq!(store.latest("j").expect("saved").findings.len(), 1);
        let saved = std::fs::read_to_string(&inbox).expect("inbox");
        assert_eq!(Inbox::from_json(&saved).entries().len(), 1);
        assert_eq!(n.shown().len(), 1);
    }

    #[tokio::test]
    async fn a_green_scan_writes_a_report_without_calling_any_agent() {
        let exec = Arc::new(ScriptedExec {
            reply: Ok("never".into()),
            ..ScriptedExec::default()
        });
        let policy = allowed();
        let d = deps(None, exec.clone(), &policy);
        let out = run_job(
            &hub_with("success"),
            &pi_job(),
            &[info("r", false)],
            &d,
            Utc::now(),
        )
        .await;
        assert!(!out.report.needs_you() && out.report.harness == "none");
        assert!(
            out.report
                .summary
                .starts_with("All green: 1 repos, 1 workflows"),
            "{}",
            out.report.summary
        );
        assert!(out.inbox.is_empty() && out.system.is_none());
        assert!(
            exec.calls.lock().expect("lock").is_empty(),
            "no model, no cost"
        );
    }

    #[tokio::test]
    async fn a_red_workflow_goes_to_the_chosen_harness_which_is_told_not_to_act() {
        let exec = Arc::new(ScriptedExec {
            reply: Ok("cause: flaky network\nFINDINGS: 1".into()),
            ..ScriptedExec::default()
        });
        let policy = allowed();
        let d = deps(None, exec.clone(), &policy);
        let out = run_job(
            &hub_with("failure"),
            &pi_job(),
            &[info("r", true)],
            &d,
            Utc::now(),
        )
        .await;
        assert_eq!(out.report.findings.len(), 1);
        assert_eq!(out.report.findings[0].headline(), "github.com/o/r CI");
        assert!(out.report.summary.contains("flaky network"));
        let calls = exec.calls.lock().expect("lock").clone();
        assert!(
            calls[0].starts_with("pi -p --model m/x ") && calls[0].contains("Do NOT push"),
            "{calls:?}"
        );
        assert!(calls[0].contains("github.com/o/r CI https://github.com/o/r/actions/runs/9"));
        assert_eq!(out.inbox.len(), 1);
        assert_eq!(
            out.system.as_ref().map(|a| a.title.as_str()),
            Some("j: 1 red in 1 repos")
        );
    }

    #[tokio::test]
    async fn a_host_that_has_not_allowed_watch_is_listed_but_never_shown_to_an_agent() {
        let exec = Arc::new(ScriptedExec {
            reply: Ok("x".into()),
            ..ScriptedExec::default()
        });
        let policy = Policy::default();
        let d = deps(None, exec.clone(), &policy);
        let out = run_job(
            &hub_with("failure"),
            &pi_job(),
            &[info("r", false)],
            &d,
            Utc::now(),
        )
        .await;
        assert_eq!(out.report.findings.len(), 1, "still reported");
        assert!(
            out.report.summary.contains("not shown to an agent"),
            "{}",
            out.report.summary
        );
        assert!(exec.calls.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn a_failing_harness_fails_the_report_and_the_builtin_one_needs_a_model() {
        let exec = Arc::new(ScriptedExec {
            reply: Err("pi: not found".into()),
            ..ScriptedExec::default()
        });
        let policy = allowed();
        let d = deps(None, exec, &policy);
        let out = run_job(
            &hub_with("failure"),
            &pi_job(),
            &[info("r", false)],
            &d,
            Utc::now(),
        )
        .await;
        assert_eq!(out.report.failed.as_deref(), Some("pi: not found"));
        assert!(
            out.system
                .is_some_and(|a| a.title.contains("the run failed"))
        );

        let exec = Arc::new(ScriptedExec::default());
        let d = deps(None, exec, &policy);
        let out = run_job(
            &hub_with("failure"),
            &Job::new("b", "daily"),
            &[info("r", false)],
            &d,
            Utc::now(),
        )
        .await;
        assert!(
            out.report
                .failed
                .is_some_and(|e| e.contains("no model configured"))
        );
    }

    #[tokio::test]
    async fn the_selector_decides_what_is_scanned() {
        let exec = Arc::new(ScriptedExec {
            reply: Ok("x".into()),
            ..ScriptedExec::default()
        });
        let policy = allowed();
        let d = deps(None, exec, &policy);
        let mut j = pi_job();
        j.projects.visibility = crate::agent_watch::select::Visibility::Public;
        let out = run_job(&hub_with("failure"), &j, &[info("r", true)], &d, Utc::now()).await;
        assert_eq!(out.report.repos, 0, "the only repository is private");
        assert!(!out.report.needs_you());
    }

    #[test]
    fn announcing_fills_the_inbox_and_shows_one_system_notification() {
        use gitkraft_platforms::notify::RecordingNotifier;
        let f = |n: u64| Finding {
            repo: "h/o/r".into(),
            kind: FindingKind::FailingPr,
            what: "p".into(),
            url: "u".into(),
            run_id: None,
            branch: None,
            number: Some(n),
            explanation: None,
        };
        let report = Report {
            job: "j".into(),
            started: Utc::now(),
            finished: Utc::now(),
            harness: "pi".into(),
            model: None,
            repos: 1,
            workflows: 1,
            findings: vec![f(1), f(2)],
            errors: vec![],
            summary: String::new(),
            failed: None,
            seen: false,
        };
        let out = outcome(&Job::new("j", "daily"), report);
        let (mut inbox, n) = (Inbox::default(), RecordingNotifier::default());
        announce(&out, &mut inbox, &n, Utc::now());
        announce(&out, &mut inbox, &n, Utc::now());
        assert_eq!(
            inbox.entries().len(),
            2,
            "the same item is not added twice while unread"
        );
        assert_eq!(n.shown().len(), 2, "one system notification per run");
    }

    #[test]
    fn the_prompt_lists_the_red_items_and_forbids_acting() {
        let f = Finding {
            repo: "github.com/o/r".into(),
            kind: FindingKind::FailedRun,
            what: "CI".into(),
            url: "https://x/9".into(),
            run_id: Some(9),
            branch: None,
            number: None,
            explanation: None,
        };
        let p = build_prompt(&Job::new("j", "daily"), &[&f]);
        assert!(
            p.contains("Do NOT push") && p.contains("github.com/o/r CI https://x/9"),
            "{p}"
        );
        let mut custom = Job::new("j", "daily");
        custom.prompt = Some("mine".into());
        assert!(build_prompt(&custom, &[&f]).starts_with("mine"));
    }

    #[test]
    fn alerts_name_the_repo_and_the_system_one_is_a_single_summary() {
        let mut report = Report {
            job: "j".into(),
            started: Utc::now(),
            finished: Utc::now(),
            harness: "pi".into(),
            model: None,
            repos: 4,
            workflows: 8,
            findings: vec![],
            errors: vec![],
            summary: String::new(),
            failed: None,
            seen: false,
        };
        for n in 0..3 {
            report.findings.push(Finding {
                repo: "git.lan/team/sub/svc".into(),
                kind: FindingKind::FailingPr,
                what: format!("p{n}"),
                url: "u".into(),
                run_id: None,
                branch: None,
                number: Some(n),
                explanation: None,
            });
        }
        let out = outcome(&Job::new("j", "daily"), report.clone());
        assert_eq!(out.inbox.len(), 3);
        assert_eq!(out.inbox[0].repo, RepoId::new("git.lan", "team/sub", "svc"));
        assert_eq!(
            out.system.as_ref().map(|a| a.title.as_str()),
            Some("j: 3 red in 4 repos")
        );
        let mut quiet = Job::new("j", "daily");
        quiet.notify.inbox = false;
        quiet.notify.system = false;
        let out = outcome(&quiet, report);
        assert!(out.inbox.is_empty() && out.system.is_none());
    }
}
