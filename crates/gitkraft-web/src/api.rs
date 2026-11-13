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
