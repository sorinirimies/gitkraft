//! Signing in without pasting a token: the OAuth device flow (GitHub and GitLab).
//!
//! The caller needs an OAuth application's client id (public, no secret): register one on the
//! platform and put the id in `accounts.toml` (`oauth_client_id`) or `GITKRAFT_OAUTH_CLIENT_ID`.

use std::time::Duration;

use serde::Deserialize;

use crate::account::Secret;
use crate::error::{PlatformError, Result};
use crate::http::{HttpClient, Method, Request};
use crate::platform::PlatformKind;

/// Scopes gitkraft asks for: everything the hub does, including registering SSH keys.
pub fn default_scopes(kind: PlatformKind) -> &'static str {
    match kind {
        PlatformKind::GitHub => "repo read:user notifications workflow admin:public_key read:org",
        PlatformKind::GitLab => "api",
        PlatformKind::Gitea => "",
    }
}

/// What to show the user while they approve in the browser.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DeviceCode {
    /// Secret handle the poll uses.
    pub device_code: String,
    /// The code the user types on the platform.
    pub user_code: String,
    /// Where to type it.
    pub verification_uri: String,
    /// Seconds until the code dies.
    #[serde(default = "default_expiry")]
    pub expires_in: u64,
    /// Seconds between polls.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_expiry() -> u64 {
    900
}

fn default_interval() -> u64 {
    5
}

/// One poll's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    /// The user has not approved yet.
    Pending,
    /// Asked too fast: wait longer.
    SlowDown,
    /// Approved.
    Done(String),
    /// The user said no.
    Denied,
    /// The code expired.
    Expired,
}

fn endpoints(kind: PlatformKind, host: &str) -> Result<(String, String)> {
    match kind {
        PlatformKind::GitHub => Ok((
            format!("https://{host}/login/device/code"),
            format!("https://{host}/login/oauth/access_token"),
        )),
        PlatformKind::GitLab => Ok((
            format!("https://{host}/oauth/authorize_device"),
            format!("https://{host}/oauth/token"),
        )),
        PlatformKind::Gitea => Err(PlatformError::Unsupported(
            "Gitea, Forgejo and Codeberg have no device flow: create a token under Settings → Applications and use `login --token`",
        )),
    }
}

fn form(pairs: &[(&str, &str)]) -> Vec<u8> {
    pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                crate::platform::url_encode(k),
                crate::platform::url_encode(v)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
        .into_bytes()
}

fn post_form(url: String, pairs: &[(&str, &str)]) -> Request {
    Request {
        method: Method::Post,
        url,
        headers: vec![
            ("Accept".into(), "application/json".into()),
            (
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            ),
        ],
        body: Some(form(pairs)),
    }
}

/// Starts the flow.
pub async fn start<C: HttpClient>(
    client: &C,
    kind: PlatformKind,
    host: &str,
    client_id: &str,
    scopes: &str,
) -> Result<DeviceCode> {
    let (url, _) = endpoints(kind, host)?;
    let res = client
        .send(post_form(
            url,
            &[("client_id", client_id), ("scope", scopes)],
        ))
        .await?;
    if !(200..300).contains(&res.status) {
        return Err(PlatformError::Http {
            status: res.status,
            message: res.text().chars().take(200).collect(),
        });
    }
    serde_json::from_slice::<DeviceCode>(&res.body).map_err(|e| {
        PlatformError::Parse(format!(
            "device code: {e}: {}",
            res.text().chars().take(120).collect::<String>()
        ))
    })
}

#[derive(Debug, Deserialize)]
struct TokenAnswer {
    access_token: Option<String>,
    error: Option<String>,
}

/// Asks once whether the user has approved.
pub async fn poll<C: HttpClient>(
    client: &C,
    kind: PlatformKind,
    host: &str,
    client_id: &str,
    device_code: &str,
) -> Result<Poll> {
    let (_, url) = endpoints(kind, host)?;
    let res = client
        .send(post_form(
            url,
            &[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ],
        ))
        .await?;
    // GitHub answers 200 with an `error` field; GitLab answers 400 with one.
    let answer: TokenAnswer = serde_json::from_slice(&res.body)
        .map_err(|e| PlatformError::Parse(format!("token answer: {e}")))?;
    if let Some(t) = answer.access_token.filter(|t| !t.is_empty()) {
        return Ok(Poll::Done(t));
    }
    match answer.error.as_deref() {
        Some("authorization_pending") => Ok(Poll::Pending),
        Some("slow_down") => Ok(Poll::SlowDown),
        Some("access_denied") => Ok(Poll::Denied),
        Some("expired_token") => Ok(Poll::Expired),
        Some(other) => Err(PlatformError::Auth(format!("sign-in failed: {other}"))),
        None => Err(PlatformError::Http {
            status: res.status,
            message: res.text().chars().take(200).collect(),
        }),
    }
}

/// Polls until the user approves, says no, or the code expires. `interval` of the code is
/// honoured (and grows by 5 s on `slow_down`); `scale` shrinks the wait for tests.
pub async fn wait<C: HttpClient>(
    client: &C,
    kind: PlatformKind,
    host: &str,
    client_id: &str,
    code: &DeviceCode,
    scale: f64,
) -> Result<Secret> {
    let mut interval = code.interval.max(1);
    let mut waited = 0u64;
    loop {
        match poll(client, kind, host, client_id, &code.device_code).await? {
            Poll::Done(t) => return Ok(Secret::new(t)),
            Poll::Denied => return Err(PlatformError::Auth("you declined the sign-in".into())),
            Poll::Expired => {
                return Err(PlatformError::Auth(
                    "the code expired; run login again".into(),
                ));
            }
            Poll::SlowDown => interval += 5,
            Poll::Pending => {}
        }
        if waited >= code.expires_in {
            return Err(PlatformError::Auth(
                "the code expired; run login again".into(),
            ));
        }
        tokio::time::sleep(Duration::from_secs_f64(interval as f64 * scale)).await;
        waited += interval;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeClient;

    const CODE: &str = r#"{"device_code":"dc","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#;

    #[tokio::test]
    async fn start_sends_a_form_and_parses_the_code() {
        let c = FakeClient::new();
        c.push_json("https://github.com/login/device/code", CODE);
        let code = start(
            &c,
            PlatformKind::GitHub,
            "github.com",
            "Iv1.abc",
            "repo read:user",
        )
        .await
        .expect("code");
        assert_eq!((code.user_code.as_str(), code.interval), ("ABCD-1234", 5));
        let req = &c.requests()[0];
        assert_eq!(req.method, Method::Post);
        assert!(
            req.headers
                .iter()
                .any(|(k, v)| k == "Content-Type" && v.contains("urlencoded"))
        );
        let body = String::from_utf8(req.body.clone().expect("body")).expect("utf8");
        assert_eq!(body, "client_id=Iv1.abc&scope=repo%20read%3Auser");
    }

    #[tokio::test]
    async fn gitlab_uses_its_own_endpoints_and_defaults_missing_fields() {
        let c = FakeClient::new();
        c.push_json("https://gitlab.example.com/oauth/authorize_device", r#"{"device_code":"d","user_code":"U","verification_uri":"https://gitlab.example.com/oauth/device"}"#);
        let code = start(&c, PlatformKind::GitLab, "gitlab.example.com", "id", "api")
            .await
            .expect("code");
        assert_eq!((code.expires_in, code.interval), (900, 5));
        c.push_json(
            "https://gitlab.example.com/oauth/token",
            r#"{"access_token":"glpat"}"#,
        );
        assert_eq!(
            poll(&c, PlatformKind::GitLab, "gitlab.example.com", "id", "d")
                .await
                .expect("poll"),
            Poll::Done("glpat".into())
        );
    }

    #[tokio::test]
    async fn gitea_has_no_device_flow_and_http_errors_surface() {
        let c = FakeClient::new();
        assert!(matches!(
            start(&c, PlatformKind::Gitea, "codeberg.org", "id", "").await,
            Err(PlatformError::Unsupported(_))
        ));
        c.push_status("https://github.com/login/device/code", 404, "not found");
        assert!(matches!(
            start(&c, PlatformKind::GitHub, "github.com", "bad", "").await,
            Err(PlatformError::Http { status: 404, .. })
        ));
        let d = FakeClient::new();
        d.push_json("https://github.com/login/device/code", "{}");
        assert!(matches!(
            start(&d, PlatformKind::GitHub, "github.com", "id", "").await,
            Err(PlatformError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn every_poll_answer_is_told_apart() {
        for (body, want) in [
            (r#"{"error":"authorization_pending"}"#, Poll::Pending),
            (r#"{"error":"slow_down"}"#, Poll::SlowDown),
            (r#"{"error":"access_denied"}"#, Poll::Denied),
            (r#"{"error":"expired_token"}"#, Poll::Expired),
            (
                r#"{"access_token":"t","token_type":"bearer"}"#,
                Poll::Done("t".into()),
            ),
        ] {
            let c = FakeClient::new();
            c.push_json("https://github.com/login/oauth/access_token", body);
            assert_eq!(
                poll(&c, PlatformKind::GitHub, "github.com", "id", "dc")
                    .await
                    .expect(body),
                want
            );
        }
        let c = FakeClient::new();
        c.push_json(
            "https://github.com/login/oauth/access_token",
            r#"{"error":"incorrect_client_credentials"}"#,
        );
        assert!(matches!(
            poll(&c, PlatformKind::GitHub, "github.com", "id", "dc").await,
            Err(PlatformError::Auth(_))
        ));
        let c = FakeClient::new();
        c.push_status("https://github.com/login/oauth/access_token", 500, "{}");
        assert!(
            poll(&c, PlatformKind::GitHub, "github.com", "id", "dc")
                .await
                .is_err()
        );
    }

    fn code(interval: u64, expires: u64) -> DeviceCode {
        DeviceCode {
            device_code: "dc".into(),
            user_code: "U".into(),
            verification_uri: "v".into(),
            expires_in: expires,
            interval,
        }
    }

    #[tokio::test]
    async fn waiting_polls_until_approved() {
        let c = FakeClient::new();
        let url = "https://github.com/login/oauth/access_token";
        c.push_json(url, r#"{"error":"authorization_pending"}"#);
        c.push_json(url, r#"{"error":"slow_down"}"#);
        c.push_json(url, r#"{"access_token":"tok"}"#);
        let t = wait(
            &c,
            PlatformKind::GitHub,
            "github.com",
            "id",
            &code(1, 900),
            0.0,
        )
        .await
        .expect("approved");
        assert_eq!(t.expose(), "tok");
        assert_eq!(c.requests().len(), 3);
    }

    #[tokio::test]
    async fn waiting_stops_on_no_expiry_or_timeout() {
        let url = "https://github.com/login/oauth/access_token";
        let c = FakeClient::new();
        c.push_json(url, r#"{"error":"access_denied"}"#);
        let e = wait(
            &c,
            PlatformKind::GitHub,
            "github.com",
            "id",
            &code(1, 900),
            0.0,
        )
        .await
        .expect_err("denied");
        assert!(e.to_string().contains("declined"));
        let c = FakeClient::new();
        c.push_json(url, r#"{"error":"expired_token"}"#);
        assert!(
            wait(
                &c,
                PlatformKind::GitHub,
                "github.com",
                "id",
                &code(1, 900),
                0.0
            )
            .await
            .expect_err("expired")
            .to_string()
            .contains("expired")
        );
        let c = FakeClient::new();
        c.push_json(url, r#"{"error":"authorization_pending"}"#);
        let e = wait(
            &c,
            PlatformKind::GitHub,
            "github.com",
            "id",
            &code(1, 3),
            0.0,
        )
        .await
        .expect_err("timeout");
        assert!(e.to_string().contains("expired"));
        assert_eq!(c.requests().len(), 4, "polled at 0, 1, 2 and 3 seconds");
    }

    #[test]
    fn scopes_cover_ssh_key_registration_on_github() {
        assert!(default_scopes(PlatformKind::GitHub).contains("admin:public_key"));
        assert_eq!(default_scopes(PlatformKind::GitLab), "api");
        assert_eq!(default_scopes(PlatformKind::Gitea), "");
    }
}
