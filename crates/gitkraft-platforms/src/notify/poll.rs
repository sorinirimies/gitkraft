use super::alert::Alert;
use super::diff::{diff_snapshots, inbox_alerts, snapshot};
use super::state::NotifyState;
use crate::backend::Backend;
use crate::error::Result;
use crate::http::HttpClient;
use crate::model::{ListQuery, StateFilter};
use crate::platform::RepoId;

/// Polls one project: lists PRs and issues, diffs against the last snapshot,
/// stores the new one and returns the raw (unfiltered) alerts.
pub async fn poll_repo<C: HttpClient>(
    backend: &Backend<C>,
    repo: &RepoId,
    me: &str,
    state: &mut NotifyState,
) -> Result<Vec<Alert>> {
    let q = ListQuery {
        state: StateFilter::All,
        ..ListQuery::default()
    };
    let prs = backend.list_pull_requests(repo, &q).await?;
    let issues = backend.list_issues(repo, &q).await?;
    let cur = snapshot(&prs, &issues, me);
    let key = repo.key();
    let alerts = diff_snapshots(repo, state.snapshots.get(&key), &cur);
    state.snapshots.insert(key, cur);
    Ok(alerts)
}

/// Polls the account-wide inbox of `host` and returns alerts for new unread entries.
/// Alerts for projects whose `effective` says disabled are dropped by [`super::deliver`].
pub async fn poll_inbox<C: HttpClient>(
    backend: &Backend<C>,
    host: &str,
    state: &mut NotifyState,
) -> Result<Vec<Alert>> {
    let current = backend.notifications().await?;
    let first = !state.seen_notifications.contains_key(host);
    let seen = state
        .seen_notifications
        .entry(host.to_string())
        .or_default();
    Ok(inbox_alerts(seen, &current, first))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Account;
    use crate::platform::PlatformKind;
    use crate::testing::FakeClient;

    fn backend(fake: &FakeClient) -> Backend<FakeClient> {
        Backend::new(
            &Account::new(PlatformKind::GitHub, "github.com"),
            fake.clone(),
        )
    }

    fn repo() -> RepoId {
        RepoId::new("github.com", "o", "r")
    }

    const PULLS_1: &str =
        r#"[{"number":1,"title":"a","state":"open","user":{"login":"x"},"html_url":"u1"}]"#;
    const PULLS_2: &str = r#"[{"number":1,"title":"a","state":"open","user":{"login":"x"},"html_url":"u1"},{"number":2,"title":"b","state":"open","user":{"login":"y"},"html_url":"u2"}]"#;

    #[tokio::test]
    async fn second_poll_reports_the_new_pr_only() {
        let fake = FakeClient::new();
        fake.push_json("https://api.github.com/repos/o/r/issues", "[]");
        fake.push_json("https://api.github.com/repos/o/r/pulls", PULLS_1);
        fake.push_json("https://api.github.com/repos/o/r/pulls", PULLS_2);
        let b = backend(&fake);
        let mut state = NotifyState::default();
        let first = poll_repo(&b, &repo(), "me", &mut state)
            .await
            .unwrap_or_default();
        assert!(first.is_empty());
        let second = poll_repo(&b, &repo(), "me", &mut state)
            .await
            .unwrap_or_default();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].number, Some(2));
        let third = poll_repo(&b, &repo(), "me", &mut state)
            .await
            .unwrap_or_default();
        assert!(third.is_empty(), "nothing changed");
    }

    #[tokio::test]
    async fn a_failed_poll_leaves_the_snapshot_alone() {
        let fake = FakeClient::new();
        fake.push_json("https://api.github.com/repos/o/r/issues", "[]");
        fake.push_json("https://api.github.com/repos/o/r/pulls", PULLS_1);
        fake.push_status("https://api.github.com/repos/o/r/pulls", 500, "boom");
        let b = backend(&fake);
        let mut state = NotifyState::default();
        let _ = poll_repo(&b, &repo(), "me", &mut state).await;
        let before = state.clone();
        assert!(poll_repo(&b, &repo(), "me", &mut state).await.is_err());
        assert_eq!(state, before);
    }

    #[tokio::test]
    async fn inbox_poll_seeds_then_reports() {
        let fake = FakeClient::new();
        let note = |id: &str| {
            format!(
                r#"{{"id":"{id}","unread":true,"reason":"mention","subject":{{"title":"t","url":"https://api.github.com/repos/o/r/issues/3","type":"Issue"}},"repository":{{"full_name":"o/r"}}}}"#
            )
        };
        fake.push_json(
            "https://api.github.com/notifications",
            &format!("[{}]", note("1")),
        );
        fake.push_json(
            "https://api.github.com/notifications",
            &format!("[{},{}]", note("1"), note("2")),
        );
        let b = backend(&fake);
        let mut state = NotifyState::default();
        assert!(
            poll_inbox(&b, "github.com", &mut state)
                .await
                .unwrap_or_default()
                .is_empty()
        );
        let alerts = poll_inbox(&b, "github.com", &mut state)
            .await
            .unwrap_or_default();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].number, Some(3));
    }
}
