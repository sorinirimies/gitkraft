//! Demo mode: a hub that answers from made-up data, so the apps can be shown (README
//! screenshots, a first look) without an account, a token or a network.
//!
//! `gitkraft-web --demo` uses it. Nothing here is real: the people, repositories and pull
//! requests are invented, and every write goes nowhere.

use chrono::{Duration, Utc};
use gitkraft_platforms::testing::FakeClient;
use gitkraft_platforms::{Account, DynClient, PlatformKind, Secret};
use serde_json::json;

use crate::hub::Hub;

const API: &str = "https://api.github.com";
/// The person the demo is signed in as.
pub const ME: &str = "ada";

/// A hub with one fake GitHub account and a believable day of work.
pub fn hub() -> Hub<DynClient> {
    let fake = FakeClient::new();
    seed(&fake);
    let mut account =
        Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("demo"));
    account.me = Some(ME.into());
    Hub::new(vec![account], &DynClient::new(fake))
}

fn ago(hours: i64) -> String {
    (Utc::now() - Duration::hours(hours)).to_rfc3339()
}

fn hit(
    repo: &str,
    n: u64,
    title: &str,
    by: &str,
    pr: bool,
    hours: i64,
    comments: u32,
) -> serde_json::Value {
    let mut v = json!({
        "number": n, "title": title, "state": "open", "user": {"login": by},
        "comments": comments, "updated_at": ago(hours), "created_at": ago(hours + 30),
        "html_url": format!("https://github.com/{repo}/{}/{n}", if pr { "pull" } else { "issues" }),
        "repository_url": format!("{API}/repos/{repo}"),
        "labels": [{"name": if pr { "ready" } else { "bug" }, "color": "0e8a16"}],
    });
    if pr {
        v["pull_request"] = json!({});
    }
    v
}

fn seed(fake: &FakeClient) {
    let items = vec![
        hit(
            "dodai/gitkraft",
            214,
            "Tray icon and start at login on all three systems",
            "grace",
            true,
            2,
            6,
        ),
        hit(
            "dodai/gitkraft",
            209,
            "Dodai design system themes with a live picker",
            ME,
            true,
            5,
            3,
        ),
        hit(
            "dodai/dodai-ui",
            88,
            "Chamfered frames for iced panels",
            "linus",
            true,
            9,
            11,
        ),
        hit(
            "dodai/topcoat",
            41,
            "Serve the theme variables without a build step",
            ME,
            true,
            26,
            2,
        ),
        hit(
            "dodai/gitkraft",
            201,
            "Calendar day opens the activity of that day",
            "margaret",
            false,
            3,
            4,
        ),
        hit(
            "dodai/dodai-ui",
            93,
            "Glow looks heavy on light themes",
            "linus",
            false,
            12,
            7,
        ),
        hit(
            "dodai/topcoat",
            37,
            "Document the --background flag",
            ME,
            false,
            48,
            1,
        ),
    ];
    fake.push_json(
        &format!("{API}/search/issues"),
        &json!({ "items": items }).to_string(),
    );

    let note = |id: u32, kind: &str, repo: &str, n: u64, title: &str, reason: &str, hours: i64| {
        json!({
            "id": id.to_string(), "unread": true, "reason": reason, "updated_at": ago(hours),
            "subject": {"title": title, "type": kind,
                "url": format!("{API}/repos/{repo}/{}/{n}", if kind == "Issue" { "issues" } else { "pulls" })},
            "repository": {"full_name": repo},
        })
    };
    let notes = vec![
        note(
            1,
            "PullRequest",
            "dodai/gitkraft",
            214,
            "Tray icon and start at login on all three systems",
            "review_requested",
            2,
        ),
        note(
            2,
            "Issue",
            "dodai/dodai-ui",
            93,
            "Glow looks heavy on light themes",
            "mention",
            12,
        ),
        note(
            3,
            "PullRequest",
            "dodai/dodai-ui",
            88,
            "Chamfered frames for iced panels",
            "comment",
            9,
        ),
    ];
    fake.push_json(&format!("{API}/notifications"), &json!(notes).to_string());

    // A year of contributions: busier on weekdays, quiet on some weeks.
    let today = Utc::now().date_naive();
    let weeks: Vec<_> = (0..53)
        .map(|w| {
            let days: Vec<_> = (0..7)
                .filter_map(|d| {
                    let date = today - Duration::days((52 - w) * 7 + (6 - d));
                    (date <= today).then(|| {
                        let seed = (w * 7 + d) as u64;
                        let weekday = d != 0 && d != 6;
                        let n = if (seed * 2654435761) % 11 < 3 {
                            0
                        } else if weekday {
                            (seed * 40503) % 9
                        } else {
                            (seed * 7) % 3
                        };
                        json!({"date": date.to_string(), "contributionCount": n})
                    })
                })
                .collect();
            json!({ "contributionDays": days })
        })
        .collect();
    let calendar = json!({"data": {"user": {"contributionsCollection": {"contributionCalendar": {"weeks": weeks}}}}});
    fake.respond_with("https://api.github.com/graphql", move |req| {
        graphql_answer(&calendar, req)
    });

    // One pull request worth opening.
    let base = format!("{API}/repos/dodai/gitkraft");
    fake.push_json(&format!("{base}/pulls/214"), &json!({
        "number": 214, "title": "Tray icon and start at login on all three systems",
        "body": "Adds a tray icon and a menu, and a start-at-login switch.\n\n- macOS and Windows through `tray-icon`\n- Linux through StatusNotifierItem\n- closing the window can hide it to the tray\n\nOnly one open GitKraft runs the daily agent jobs.",
        "state": "open", "user": {"login": "grace"}, "head": {"ref": "feat/tray", "sha": "c0ffee1"},
        "base": {"ref": "main"}, "additions": 412, "deletions": 38, "comments": 6,
        "created_at": ago(40), "updated_at": ago(2),
        "html_url": "https://github.com/dodai/gitkraft/pull/214",
    }).to_string());
    fake.push_json(
        &format!("{base}/commits/c0ffee1/status"),
        r#"{"state":"success"}"#,
    );
    fake.push_json(&format!("{base}/issues/214/comments"), &json!([
        {"id": 1, "body": "Nice. Does it work when the desktop has no tray?", "user": {"login": "linus"}, "created_at": ago(30)},
        {"id": 2, "body": "Yes, the window opens as usual and a hint goes to the log.", "user": {"login": "grace"}, "created_at": ago(28)},
    ]).to_string());
    fake.push_json(
        &format!("{base}/pulls/214/reviews"),
        &json!([
            {"state": "APPROVED", "user": {"login": "margaret"}},
            {"state": "COMMENTED", "user": {"login": "linus"}},
        ])
        .to_string(),
    );
    fake.push_status(&format!("{base}/pulls/214.diff"), 200,
        "diff --git a/src/tray.rs b/src/tray.rs\n--- a/src/tray.rs\n+++ b/src/tray.rs\n@@ -10,6 +10,9 @@ pub fn start() -> bool {\n     if cfg!(test) {\n         return false;\n     }\n+    if !host_available() {\n+        return false;\n+    }\n     imp::start()\n }\n");
}

/// The demo's GitHub GraphQL: the contribution calendar for the calendar query, and for a day
/// query (`from` in the variables) what the calendar says was done that day.
fn graphql_answer(
    calendar: &serde_json::Value,
    req: &gitkraft_platforms::http::Request,
) -> gitkraft_platforms::http::Response {
    let body: serde_json::Value = req
        .body
        .as_deref()
        .and_then(|b| serde_json::from_slice(b).ok())
        .unwrap_or_default();
    let from = body["variables"]["from"].as_str().unwrap_or_default();
    let reply = match from.get(..10) {
        None => calendar.clone(),
        Some(date) => day_reply(calendar, date),
    };
    gitkraft_platforms::http::Response {
        status: 200,
        headers: vec![("content-type".into(), "application/json".into())],
        body: reply.to_string().into_bytes(),
    }
}

/// The answer to the day query for `date`: the calendar's count for that day, split into commits
/// in two repositories plus, on busier days, one pull request and one review.
fn day_reply(calendar: &serde_json::Value, date: &str) -> serde_json::Value {
    let total =
        calendar["data"]["user"]["contributionsCollection"]["contributionCalendar"]["weeks"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|w| w["contributionDays"].as_array().into_iter().flatten())
            .find(|d| d["date"] == date)
            .and_then(|d| d["contributionCount"].as_u64())
            .unwrap_or(0);
    let commits = total.saturating_sub(if total > 5 { 2 } else { 0 });
    let first = commits.div_ceil(2);
    let repos: Vec<_> = [("dodai/gitkraft", first), ("dodai/dodai-ui", commits - first)]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .map(|(name, n)| json!({"repository": {"nameWithOwner": name, "url": format!("https://github.com/{name}")}, "contributions": {"totalCount": n}}))
        .collect();
    let pr = |n: u64, title: &str| json!({"pullRequest": {"number": n, "title": title, "url": format!("https://github.com/dodai/gitkraft/pull/{n}"), "repository": {"nameWithOwner": "dodai/gitkraft"}}});
    let (prs, reviews) = if total > 5 {
        (
            vec![pr(214, "Tray icon and start at login")],
            vec![pr(209, "Fix the CI log scroll")],
        )
    } else {
        (vec![], vec![])
    };
    json!({"data": {"user": {"contributionsCollection": {
        "contributionCalendar": {"totalContributions": total},
        "commitContributionsByRepository": repos,
        "pullRequestContributions": {"nodes": prs},
        "issueContributions": {"nodes": []},
        "pullRequestReviewContributions": {"nodes": reviews},
    }}}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_settings::dashboard::Dashboard;

    #[tokio::test]
    async fn a_busy_calendar_day_opens_with_what_was_done_that_day() {
        let hub = hub();
        let calendar = hub.contributions().await;
        let busy = calendar
            .items
            .iter()
            .find(|d| d.count > 5)
            .expect("the demo year has a busy day");
        let day = hub.day_activity(busy.date).await;
        assert!(day.errors.is_empty(), "{:?}", day.errors);
        assert_eq!(day.activity.total, busy.count);
        assert!(!day.activity.entries.is_empty());
    }

    #[tokio::test]
    async fn the_demo_hub_fills_every_starter_section_without_a_network() {
        let hub = hub();
        let mut counts = Vec::new();
        for section in &Dashboard::starter().sections {
            let data = hub.load_section(section).await;
            counts.push(data.len());
            assert!(data.errors().is_empty(), "{:?}", data.errors());
        }
        assert!(counts.iter().all(|&n| n > 0), "{counts:?}");
    }
}
