//! A tiny HTTP seam: adapters talk to [`HttpClient`], tests plug in a fake and
//! production uses [`ReqwestClient`].

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use crate::error::{PlatformError, Result};

/// HTTP method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// GET
    Get,
    /// POST
    Post,
    /// PUT
    Put,
    /// PATCH
    Patch,
    /// DELETE
    Delete,
}

/// An outgoing request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Method.
    pub method: Method,
    /// Absolute URL.
    pub url: String,
    /// Header name / value pairs.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// A GET request.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Adds a header.
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }
}

/// A response with lowercase header names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Response {
    /// Status code.
    pub status: u16,
    /// Headers, names lowercased.
    pub headers: Vec<(String, String)>,
    /// Body bytes.
    pub body: Vec<u8>,
}

impl Response {
    /// First header named `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The body as lossy UTF-8.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Sends requests. Implemented by [`ReqwestClient`] and by test fakes.
pub trait HttpClient: Send + Sync {
    /// Sends `req` and returns the response whatever its status; only
    /// transport failures are errors.
    fn send(&self, req: Request) -> impl Future<Output = Result<Response>> + Send;
}

/// The production client: one pooled `reqwest` client with rustls.
#[derive(Debug, Clone)]
pub struct ReqwestClient {
    inner: reqwest::Client,
}

impl ReqwestClient {
    /// Builds a client with a 30 s timeout and a gitkraft user agent.
    pub fn new() -> Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent(concat!("gitkraft/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        Ok(Self { inner })
    }
}

impl HttpClient for ReqwestClient {
    async fn send(&self, req: Request) -> Result<Response> {
        let method = match req.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self.inner.request(method, &req.url);
        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }
        if let Some(body) = req.body {
            builder = builder.body(body);
        }
        let res = builder
            .send()
            .await
            .map_err(|e| PlatformError::Network(e.to_string()))?;
        let status = res.status().as_u16();
        let headers = res
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                Some((
                    k.as_str().to_ascii_lowercase(),
                    v.to_str().ok()?.to_string(),
                ))
            })
            .collect();
        let body = res
            .bytes()
            .await
            .map_err(|e| PlatformError::Network(e.to_string()))?
            .to_vec();
        Ok(Response {
            status,
            headers,
            body,
        })
    }
}

/// Wraps a client with an in-memory ETag cache: unchanged GETs answer `304`,
/// which does not count against rate limits, and the cached body is returned.
#[derive(Debug)]
pub struct CachedClient<C> {
    inner: C,
    cache: Mutex<HashMap<String, Response>>,
}

impl<C: HttpClient> CachedClient<C> {
    /// Wraps `inner`.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn lookup(&self, key: &str) -> Option<Response> {
        self.cache.lock().ok()?.get(key).cloned()
    }
}

impl<C: HttpClient> HttpClient for CachedClient<C> {
    async fn send(&self, mut req: Request) -> Result<Response> {
        if req.method != Method::Get {
            return self.inner.send(req).await;
        }
        // The cache is per URL and per credential, so two accounts never share entries.
        let auth = req
            .headers
            .iter()
            .find(|(k, _)| {
                k.eq_ignore_ascii_case("authorization") || k.eq_ignore_ascii_case("private-token")
            })
            .map(|(_, v)| v.as_str())
            .unwrap_or_default();
        let key = format!("{}\n{}", req.url, auth);
        let cached = self.lookup(&key);
        if let Some(etag) = cached.as_ref().and_then(|c| c.header("etag")) {
            req.headers.push(("If-None-Match".into(), etag.to_string()));
        }
        let res = self.inner.send(req).await?;
        if res.status == 304 {
            if let Some(cached) = cached {
                return Ok(cached);
            }
        } else if res.status == 200
            && res.header("etag").is_some()
            && let Ok(mut cache) = self.cache.lock()
        {
            cache.insert(key, res.clone());
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeClient;

    #[tokio::test]
    async fn etag_cache_replays_body_on_304() {
        let fake = FakeClient::new();
        fake.push(
            "https://x/a",
            Response {
                status: 200,
                headers: vec![("etag".into(), "\"v1\"".into())],
                body: b"hello".to_vec(),
            },
        );
        fake.push(
            "https://x/a",
            Response {
                status: 304,
                ..Response::default()
            },
        );
        let client = CachedClient::new(fake.clone());
        let first = client.send(Request::get("https://x/a")).await;
        assert_eq!(first.map(|r| r.text()), Ok("hello".into()));
        let second = client.send(Request::get("https://x/a")).await;
        assert_eq!(
            second.map(|r| (r.status, r.text())),
            Ok((200, "hello".into()))
        );
        let sent = fake.requests();
        assert!(sent[0].headers.iter().all(|(k, _)| k != "If-None-Match"));
        assert!(
            sent[1]
                .headers
                .contains(&("If-None-Match".into(), "\"v1\"".into()))
        );
    }

    #[tokio::test]
    async fn cache_is_not_shared_between_credentials() {
        let fake = FakeClient::new();
        for _ in 0..2 {
            fake.push(
                "https://x/a",
                Response {
                    status: 200,
                    headers: vec![("etag".into(), "\"v1\"".into())],
                    body: b"hi".to_vec(),
                },
            );
        }
        let client = CachedClient::new(fake.clone());
        let _ = client
            .send(Request::get("https://x/a").header("Authorization", "Bearer A"))
            .await;
        let _ = client
            .send(Request::get("https://x/a").header("Authorization", "Bearer B"))
            .await;
        assert!(
            fake.requests()[1]
                .headers
                .iter()
                .all(|(k, _)| k != "If-None-Match")
        );
    }

    #[test]
    fn response_header_lookup_ignores_case() {
        let res = Response {
            headers: vec![("link".into(), "x".into())],
            ..Response::default()
        };
        assert_eq!(res.header("Link"), Some("x"));
        assert_eq!(res.header("none"), None);
    }
}
