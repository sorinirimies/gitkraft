//! A fake [`HttpClient`] that replays queued responses.
//! Public so `gitkraft-ci` and the front-ends can test without a network.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::{PlatformError, Result};
use crate::http::{HttpClient, Request, Response};

type Answer = dyn Fn(&Request) -> Response + Send + Sync;

/// A function that answers every request to one URL.
#[derive(Clone)]
struct Responder(Arc<Answer>);

impl std::fmt::Debug for Responder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Responder")
    }
}

#[derive(Debug, Default)]
struct Inner {
    responders: HashMap<String, Responder>,
    queues: HashMap<String, VecDeque<Response>>,
    sent: Vec<Request>,
}

/// Replays responses queued per URL (matched on the URL with its query string
/// stripped last, so `push("…/pulls", …)` also answers `…/pulls?state=open`).
#[derive(Debug, Clone, Default)]
pub struct FakeClient {
    inner: Arc<Mutex<Inner>>,
}

impl FakeClient {
    /// An empty fake: every call fails with a network error until you push.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `response` for `url`; the last queued response repeats.
    pub fn push(&self, url: &str, response: Response) {
        if let Ok(mut inner) = self.inner.lock() {
            inner
                .queues
                .entry(url.to_string())
                .or_default()
                .push_back(response);
        }
    }

    /// Answers every request to `url` by calling `answer`, for URLs that carry several kinds of
    /// call (GraphQL) and so cannot be told apart by address. Wins over queued responses.
    pub fn respond_with(
        &self,
        url: &str,
        answer: impl Fn(&Request) -> Response + Send + Sync + 'static,
    ) {
        if let Ok(mut inner) = self.inner.lock() {
            inner
                .responders
                .insert(url.to_string(), Responder(Arc::new(answer)));
        }
    }

    /// Queues a `200` JSON response.
    pub fn push_json(&self, url: &str, body: &str) {
        self.push(
            url,
            Response {
                status: 200,
                headers: vec![("content-type".into(), "application/json".into())],
                body: body.as_bytes().to_vec(),
            },
        );
    }

    /// Queues a response with `status` and a text body.
    pub fn push_status(&self, url: &str, status: u16, body: &str) {
        self.push(
            url,
            Response {
                status,
                headers: Vec::new(),
                body: body.as_bytes().to_vec(),
            },
        );
    }

    /// Every request sent so far.
    pub fn requests(&self) -> Vec<Request> {
        self.inner
            .lock()
            .map(|i| i.sent.clone())
            .unwrap_or_default()
    }
}

impl HttpClient for FakeClient {
    async fn send(&self, req: Request) -> Result<Response> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| PlatformError::Network("poisoned".into()))?;
        inner.sent.push(req.clone());
        let stripped = req.url.split('?').next().unwrap_or_default().to_string();
        let dynamic = [req.url.as_str(), stripped.as_str()]
            .into_iter()
            .find_map(|k| inner.responders.get(k).cloned());
        if let Some(r) = dynamic {
            drop(inner);
            return Ok((r.0)(&req));
        }
        let key = [req.url.as_str(), stripped.as_str()]
            .into_iter()
            .find(|k| inner.queues.contains_key(*k))
            .map(str::to_string);
        let Some(key) = key else {
            return Err(PlatformError::Network(format!(
                "no fake response for {}",
                req.url
            )));
        };
        let queue = inner.queues.entry(key).or_default();
        let res = if queue.len() > 1 {
            queue.pop_front()
        } else {
            queue.front().cloned()
        };
        res.ok_or_else(|| PlatformError::Network("empty fake queue".into()))
    }
}
