//! Authenticated JSON calls with status-to-error mapping, shared by every adapter
//! (also used by `gitkraft-ci`).

use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;

use crate::account::Secret;
use crate::error::{PlatformError, Result};
use crate::http::{HttpClient, Method, Request, Response};
use crate::platform::PlatformKind;

/// Base URL, credential and client for one account.
#[derive(Debug)]
pub struct Api<C> {
    /// The HTTP client.
    pub client: C,
    /// REST base URL without trailing slash.
    pub base: String,
    /// Platform kind (decides the auth header).
    pub kind: PlatformKind,
    /// Credential.
    pub token: Option<Secret>,
}

impl<C: HttpClient> Api<C> {
    /// Builds an API handle.
    pub fn new(client: C, kind: PlatformKind, base: String, token: Option<Secret>) -> Self {
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            kind,
            token,
        }
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base, path)
        }
    }

    fn request(&self, method: Method, path: &str, body: Option<&serde_json::Value>) -> Request {
        let mut req = Request {
            method,
            url: self.url(path),
            headers: vec![("Accept".into(), "application/json".into())],
            body: None,
        };
        if let Some(token) = &self.token {
            let (name, value) = match self.kind {
                PlatformKind::GitHub => ("Authorization", format!("Bearer {}", token.expose())),
                PlatformKind::Gitea => ("Authorization", format!("token {}", token.expose())),
                PlatformKind::GitLab => ("PRIVATE-TOKEN", token.expose().to_string()),
            };
            req.headers.push((name.into(), value));
        }
        if let Some(body) = body {
            req.headers
                .push(("Content-Type".into(), "application/json".into()));
            req.body = Some(body.to_string().into_bytes());
        }
        req
    }

    /// Sends a request and maps non-2xx statuses to errors.
    pub async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response> {
        let res = self.client.send(self.request(method, path, body)).await?;
        check(res)
    }

    /// GET and parse JSON.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let res = self.send(Method::Get, path, None).await?;
        parse(&res)
    }

    /// GET with an `Accept` override, returning the body as text (diffs, logs).
    pub async fn get_text(&self, path: &str, accept: &str) -> Result<String> {
        let mut req = self.request(Method::Get, path, None);
        req.headers.retain(|(k, _)| k != "Accept");
        req.headers.push(("Accept".into(), accept.into()));
        let res = check(self.client.send(req).await?)?;
        Ok(res.text())
    }

    /// POST a JSON body.
    pub async fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<Response> {
        self.send(Method::Post, path, Some(body)).await
    }
}

/// Parses a JSON body.
pub fn parse<T: DeserializeOwned>(res: &Response) -> Result<T> {
    serde_json::from_slice(&res.body).map_err(|e| PlatformError::Parse(e.to_string()))
}

/// Maps a status to a result.
pub fn check(res: Response) -> Result<Response> {
    match res.status {
        200..=299 => Ok(res),
        401 => Err(PlatformError::Auth(excerpt(&res))),
        404 => Err(PlatformError::NotFound(excerpt(&res))),
        429 => Err(PlatformError::RateLimited {
            retry_after_secs: retry_after(&res),
        }),
        403 if res.header("x-ratelimit-remaining") == Some("0")
            || res.header("retry-after").is_some() =>
        {
            Err(PlatformError::RateLimited {
                retry_after_secs: retry_after(&res),
            })
        }
        403 => Err(PlatformError::Forbidden(excerpt(&res))),
        status => Err(PlatformError::Http {
            status,
            message: excerpt(&res),
        }),
    }
}

fn excerpt(res: &Response) -> String {
    let text = res.text();
    let text = text.trim();
    let end = text.char_indices().nth(200).map_or(text.len(), |(i, _)| i);
    text[..end].to_string()
}

fn retry_after(res: &Response) -> Option<u64> {
    if let Some(secs) = res
        .header("retry-after")
        .and_then(|v| v.trim().parse().ok())
    {
        return Some(secs);
    }
    let reset: u64 = res.header("x-ratelimit-reset")?.trim().parse().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(reset.saturating_sub(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(status: u16, headers: &[(&str, &str)]) -> Response {
        Response {
            status,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: b"boom".to_vec(),
        }
    }

    #[test]
    fn maps_statuses_to_errors() {
        assert!(check(res(200, &[])).is_ok());
        assert!(matches!(check(res(401, &[])), Err(PlatformError::Auth(_))));
        assert!(matches!(
            check(res(404, &[])),
            Err(PlatformError::NotFound(_))
        ));
        assert!(matches!(
            check(res(403, &[])),
            Err(PlatformError::Forbidden(_))
        ));
        assert!(matches!(
            check(res(500, &[])),
            Err(PlatformError::Http { status: 500, .. })
        ));
    }

    #[test]
    fn rate_limits_carry_the_wait() {
        assert_eq!(
            check(res(429, &[("retry-after", "42")])).err(),
            Some(PlatformError::RateLimited {
                retry_after_secs: Some(42)
            })
        );
        assert!(matches!(
            check(res(403, &[("x-ratelimit-remaining", "0")])),
            Err(PlatformError::RateLimited { .. })
        ));
    }

    #[test]
    fn auth_header_depends_on_platform() {
        for (kind, name, value) in [
            (PlatformKind::GitHub, "Authorization", "Bearer t"),
            (PlatformKind::Gitea, "Authorization", "token t"),
            (PlatformKind::GitLab, "PRIVATE-TOKEN", "t"),
        ] {
            let api = Api::new(
                crate::testing::FakeClient::new(),
                kind,
                "https://h/api".into(),
                Some(Secret::new("t")),
            );
            let req = api.request(Method::Get, "/x", None);
            assert!(
                req.headers.contains(&(name.to_string(), value.to_string())),
                "{kind:?}"
            );
            assert_eq!(req.url, "https://h/api/x");
        }
    }
}
