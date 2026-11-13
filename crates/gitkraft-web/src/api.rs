//! JSON and static routes.

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        content::{Css, Json},
        error::bad_request,
        route,
    },
};

use crate::actions::{ActionRequest, perform};
use crate::session::Session;
use crate::style::CSS;

/// Liveness probe.
#[route(GET "/api/health")]
async fn health() -> Result<Json<&'static str>> {
    Ok(Json("ok"))
}

/// The dashboard sections, without their data.
#[route(GET "/api/sections")]
async fn sections(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let session: &Session = app_context(cx);
    let items: Vec<_> = session
        .dashboard()
        .visible()
        .map(|(i, s)| serde_json::json!({ "index": i, "name": s.name, "kind": s.kind, "repos": s.repos }))
        .collect();
    Ok(Json(serde_json::json!(items)))
}

/// Contributions per day over every account.
#[route(GET "/api/contributions")]
async fn contributions(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let session: &Session = app_context(cx);
    let c = session.hub().contributions().await;
    Ok(Json(
        serde_json::json!({ "days": c.items, "errors": c.errors }),
    ))
}

/// `?date=YYYY-MM-DD`.
#[derive(Debug)]
#[topcoat::router::query_params(error = bad_request("date must be YYYY-MM-DD"))]
struct DayParams {
    date: String,
}

/// What you did on one day, over every account: the panel under the contribution calendar.
#[route(GET "/api/day")]
async fn day(cx: &Cx) -> Result<Json<serde_json::Value>> {
    let session: &Session = app_context(cx);
    let params = topcoat::router::query_params::<DayParams>(cx)?;
    let date = parse_day(&params.date, chrono::Utc::now().date_naive()).map_err(bad_request)?;
    let r = session.hub().day_activity(date).await;
    let groups: Vec<_> = r
        .activity
        .groups()
        .into_iter()
        .map(|(kind, count, items)| {
            serde_json::json!({ "heading": kind.heading(), "count": count, "items": items })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "date": r.activity.date,
        "total": r.activity.total,
        "headline": r.activity.headline(),
        "groups": groups,
        "errors": r.errors,
    })))
}

/// A day the calendar can show: not in the future, not older than the calendar's year.
pub(crate) fn parse_day(
    text: &str,
    today: chrono::NaiveDate,
) -> Result<chrono::NaiveDate, &'static str> {
    let date: chrono::NaiveDate = text.parse().map_err(|_| "date must be YYYY-MM-DD")?;
    if date > today {
        return Err("that day has not happened yet");
    }
    if (today - date).num_days() > 370 {
        return Err("the calendar only reaches back one year");
    }
    Ok(date)
}

/// An action with the page's token.
#[derive(Debug, serde::Deserialize)]
struct Envelope {
    #[serde(default)]
    token: String,
    #[serde(flatten)]
    req: ActionRequest,
}

/// Runs an action: `{"action": "comment", "repo": "github.com/o/r", "number": 1, "kind": "pr", "body": "…"}`.
#[route(POST "/api/action")]
async fn action(cx: &Cx, Json(env): Json<Envelope>) -> Result<Json<serde_json::Value>> {
    let session: &Session = app_context(cx);
    if env.token != session.token() {
        return Err(bad_request("missing or wrong token: reload the page").into());
    }
    match perform(session, env.req).await {
        Ok(message) => Ok(Json(serde_json::json!({ "ok": true, "message": message }))),
        Err(e) => Err(bad_request(e).into()),
    }
}

#[route(GET "/style.css")]
async fn style() -> Result<Css<&'static str>> {
    Ok(Css(CSS))
}

#[cfg(test)]
mod tests {
    use super::parse_day;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("date")
    }

    #[test]
    fn only_days_the_calendar_shows_are_accepted() {
        let today = d(2026, 3, 10);
        assert_eq!(parse_day("2026-03-10", today), Ok(today));
        assert_eq!(parse_day("2025-03-10", today), Ok(d(2025, 3, 10)));
        assert!(parse_day("2026-03-11", today).is_err(), "future");
        assert!(parse_day("2024-01-01", today).is_err(), "too old");
        assert!(parse_day("yesterday", today).is_err());
        assert!(parse_day("", today).is_err());
    }
}
