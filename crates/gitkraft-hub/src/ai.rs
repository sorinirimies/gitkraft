//! AI helpers that only ever *draft*: explain why a CI job failed. Nothing here changes a forge.
//!
//! The model sits behind [`Llm`], so tests run without a network and a front-end can swap the
//! provider. What is sent is redacted first ([`redact`]) and trimmed to the part that matters
//! ([`focus`]). The API key never goes in a config file: environment, then the system keyring.

use gitkraft_platforms::credentials::SecretStore;
use gitkraft_platforms::http::{HttpClient, Method, Request};
use gitkraft_platforms::{DynClient, Secret};
use serde_json::{Value, json};

/// Characters of log a prompt may carry.
pub const MAX_LOG_CHARS: usize = 12_000;

/// Why an AI call did not give an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// No key found; the text says where to put one.
    NoKey(String),
    /// The provider said no (status and message).
    Provider(u16, String),
    /// The network failed or the answer was not what the API documents.
    Transport(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKey(m) | Self::Transport(m) => f.write_str(m),
            Self::Provider(s, m) => write!(f, "the AI provider answered {s}: {m}"),
        }
    }
}

impl std::error::Error for AiError {}

/// A language model that completes one prompt.
pub trait Llm: Send + Sync {
    /// Answers `user` under the instructions in `system`.
    fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> impl std::future::Future<Output = Result<String, AiError>> + Send;
}

/// Which API dialect to speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Anthropic's Messages API.
    Anthropic,
    /// OpenAI's chat completions, which OpenAI-compatible servers (Ollama, LM Studio, vLLM) speak too.
    OpenAi,
}

impl Provider {
    /// `anthropic` or `openai`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "claude" => Some(Self::Anthropic),
            "openai" | "openai-compatible" | "local" | "ollama" => Some(Self::OpenAi),
            _ => None,
        }
    }

    /// The name used in the keyring and in messages.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }

    fn keyring_host(self) -> String {
        format!("ai.{}", self.name())
    }

    fn env_names(self) -> [&'static str; 2] {
        match self {
            Self::Anthropic => ["GITKRAFT_AI_KEY", "ANTHROPIC_API_KEY"],
            Self::OpenAi => ["GITKRAFT_AI_KEY", "OPENAI_API_KEY"],
        }
    }
}

/// Provider, model and endpoint; no secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiConfig {
    /// The API dialect.
    pub provider: Provider,
    /// Model name.
    pub model: String,
    /// Base URL without a trailing slash.
    pub base_url: String,
}

impl AiConfig {
    /// Reads `GITKRAFT_AI_PROVIDER`, `GITKRAFT_AI_MODEL` and `GITKRAFT_AI_BASE_URL`; with none set,
    /// Anthropic is used when `ANTHROPIC_API_KEY` exists, else OpenAI.
    pub fn from_env(env: &impl Fn(&str) -> Option<String>) -> Self {
        let provider = env("GITKRAFT_AI_PROVIDER")
            .and_then(|p| Provider::parse(&p))
            .unwrap_or_else(|| {
                if env("ANTHROPIC_API_KEY").is_some() || env("OPENAI_API_KEY").is_none() {
                    Provider::Anthropic
                } else {
                    Provider::OpenAi
                }
            });
        let model = env("GITKRAFT_AI_MODEL").unwrap_or_else(|| {
            match provider {
                Provider::Anthropic => "claude-sonnet-4-5",
                Provider::OpenAi => "gpt-4o-mini",
            }
            .to_string()
        });
        let base_url = env("GITKRAFT_AI_BASE_URL").unwrap_or_else(|| {
            match provider {
                Provider::Anthropic => "https://api.anthropic.com",
                Provider::OpenAi => "https://api.openai.com/v1",
            }
            .to_string()
        });
        Self {
            provider,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Whether the endpoint is on this machine, where a key is optional.
    #[must_use]
    pub fn is_local(&self) -> bool {
        let rest = self.base_url.split("://").nth(1).unwrap_or(&self.base_url);
        ["localhost", "127.0.0.1", "[::1]"]
            .iter()
            .any(|h| rest.starts_with(h))
    }

    /// The key: environment, then keyring. A local endpoint may go without.
    ///
    /// # Errors
    /// [`AiError::NoKey`] with the ways to provide one.
    pub fn key(
        &self,
        env: &impl Fn(&str) -> Option<String>,
        store: &impl SecretStore,
    ) -> Result<Option<Secret>, AiError> {
        let from_env = self
            .provider
            .env_names()
            .iter()
            .find_map(|n| env(n).filter(|v| !v.trim().is_empty()));
        if let Some(k) = from_env {
            return Ok(Some(Secret::new(k.trim())));
        }
        if let Some(k) = store.get(&self.provider.keyring_host()) {
            return Ok(Some(k));
        }
        if self.is_local() {
            return Ok(None);
        }
        Err(AiError::NoKey(format!(
            "no API key for {}: set {} or run `gitkraft-tui ai-key {}`",
            self.provider.name(),
            self.provider.env_names()[1],
            self.provider.name()
        )))
    }
}

/// The real model over HTTP.
#[derive(Debug)]
pub struct Remote {
    client: DynClient,
    config: AiConfig,
    key: Option<Secret>,
}

impl Remote {
    /// A model reached through `client`.
    #[must_use]
    pub fn new(client: DynClient, config: AiConfig, key: Option<Secret>) -> Self {
        Self {
            client,
            config,
            key,
        }
    }

    fn request(&self, system: &str, user: &str) -> Request {
        let (url, body, mut headers) = match self.config.provider {
            Provider::Anthropic => (
                format!("{}/v1/messages", self.config.base_url),
                json!({
                    "model": self.config.model,
                    "max_tokens": 1500,
                    "system": system,
                    "messages": [{"role": "user", "content": user}],
                }),
                vec![("anthropic-version".to_string(), "2023-06-01".to_string())],
            ),
            Provider::OpenAi => (
                format!("{}/chat/completions", self.config.base_url),
                json!({
                    "model": self.config.model,
                    "messages": [
                        {"role": "system", "content": system},
                        {"role": "user", "content": user},
                    ],
                }),
                Vec::new(),
            ),
        };
        if let Some(k) = &self.key {
            headers.push(match self.config.provider {
                Provider::Anthropic => ("x-api-key".into(), k.expose().into()),
                Provider::OpenAi => ("authorization".into(), format!("Bearer {}", k.expose())),
            });
        }
        headers.push(("content-type".into(), "application/json".into()));
        Request {
            method: Method::Post,
            url,
            headers,
            body: Some(body.to_string().into_bytes()),
        }
    }
}

/// The text of an answer in either dialect.
fn answer_of(provider: Provider, v: &Value) -> Option<String> {
    let text = match provider {
        Provider::Anthropic => v["content"]
            .as_array()?
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join(""),
        Provider::OpenAi => v["choices"][0]["message"]["content"].as_str()?.to_string(),
    };
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

impl Llm for Remote {
    async fn complete(&self, system: &str, user: &str) -> Result<String, AiError> {
        let res = self
            .client
            .send(self.request(system, user))
            .await
            .map_err(|e| AiError::Transport(e.to_string()))?;
        let v: Value = serde_json::from_slice(&res.body).unwrap_or(Value::Null);
        if !(200..300).contains(&res.status) {
            let msg = v["error"]["message"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| res.text().chars().take(200).collect());
            return Err(AiError::Provider(res.status, msg));
        }
        answer_of(self.config.provider, &v)
            .ok_or_else(|| AiError::Transport("the AI answer had no text".into()))
    }
}

const SECRET_PREFIXES: [&str; 12] = [
    "ghp_",
    "gho_",
    "ghs_",
    "ghu_",
    "ghr_",
    "github_pat_",
    "glpat-",
    "sk-",
    "xoxb-",
    "xoxp-",
    "npm_",
    "AKIA",
];
const SECRET_WORDS: [&str; 6] = ["token", "secret", "password", "passwd", "api_key", "apikey"];
const MASK: &str = "[redacted]";

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/')
}

fn looks_secret(word: &str) -> bool {
    if SECRET_PREFIXES.iter().any(|p| word.starts_with(p)) && word.len() >= 12 {
        return true;
    }
    // A JWT: three dot-separated base64 parts starting with `eyJ`.
    word.starts_with("eyJ") && word.matches('.').count() == 2 && word.len() > 40
}

fn redact_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    // `Authorization: Bearer x` and `token=x` / `password: x`.
    if let Some(i) = lower.find("bearer ") {
        return format!("{}{MASK}", &line[..i + 7]);
    }
    for w in SECRET_WORDS {
        let mut from = 0;
        while let Some(p) = lower[from..].find(w) {
            let after = from + p + w.len();
            let rest = &line[after..];
            let trimmed = rest.trim_start_matches(['"', '\'', ' ']);
            if let Some(value) = trimmed.strip_prefix(['=', ':']) {
                let value = value.trim_start_matches(['"', '\'', ' ']);
                if !value.is_empty() && !value.starts_with('$') && !value.starts_with("${{") {
                    let cut = line.len() - value.len();
                    return format!("{}{MASK}", &line[..cut]);
                }
            }
            from = after;
        }
    }
    let mut out = String::with_capacity(line.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if looks_secret(word) {
            out.push_str(MASK);
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for c in line.chars() {
        if is_word_char(c) {
            word.push(c);
        } else {
            flush(&mut word, &mut out);
            out.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Masks what looks like a credential: well-known token prefixes, JWTs, bearer headers and
/// `password=…`-style values (not `$VARIABLE` references).
#[must_use]
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        out.push_str(&redact_line(line));
        out.push('\n');
    }
    out
}

fn is_error_line(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    [
        "error",
        "fail",
        "panicked",
        "fatal",
        "exception",
        "denied",
        "✗",
    ]
    .iter()
    .any(|w| l.contains(w))
}

/// Cuts `log` to at most `max` characters: the tail (where builds fail) plus a few lines of
/// context before the first error line when it lies earlier.
#[must_use]
pub fn focus(log: &str, max: usize) -> String {
    let lines: Vec<&str> = log.lines().collect();
    let total: usize = lines.iter().map(|l| l.chars().count() + 1).sum();
    if total <= max {
        return lines.join("\n");
    }
    let tail_budget = max * 3 / 4;
    let mut start = lines.len();
    let mut used = 0;
    while start > 0 {
        let n = lines[start - 1].chars().count() + 1;
        if used + n > tail_budget {
            break;
        }
        used += n;
        start -= 1;
    }
    let mut out = String::new();
    if let Some(first) = lines
        .iter()
        .position(|l| is_error_line(l))
        .filter(|i| *i < start)
    {
        let from = first.saturating_sub(3);
        let mut head_used = 0;
        for l in &lines[from..(first + 4).min(start)] {
            head_used += l.chars().count() + 1;
            if head_used > max - tail_budget {
                break;
            }
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("[... log cut ...]\n");
    } else if start > 0 {
        out.push_str("[... earlier lines cut ...]\n");
    }
    out.push_str(&lines[start..].join("\n"));
    out
}

/// What the model is told about a failed job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureContext {
    /// `host/owner/name`.
    pub repo: String,
    /// Workflow or pipeline name.
    pub workflow: String,
    /// Job name.
    pub job: String,
    /// Branch it ran on.
    pub branch: String,
    /// The raw job log.
    pub log: String,
}

const EXPLAIN_SYSTEM: &str = "You explain why a CI job failed, for the developer who owns the repository. \
Answer in short plain text with three parts: Cause (one or two sentences, name the failing step and the \
exact error), Fix (concrete changes or commands), and Flaky? (yes only when the log points to \
infrastructure or timing rather than the code). Quote log lines sparingly. Never invent files or output \
that the log does not show; if the log does not say, say so. Secrets in the log are masked: do not ask for them.";

/// The redacted, focused prompt that [`explain_failure`] sends; shown by `--dry-run` so nobody has
/// to take the redaction on trust.
#[must_use]
pub fn failure_prompt(ctx: &FailureContext) -> String {
    format!(
        "Repository: {}\nWorkflow: {}\nJob: {}\nBranch: {}\n\nLog:\n{}",
        ctx.repo,
        ctx.workflow,
        ctx.job,
        ctx.branch,
        focus(&redact(&ctx.log), MAX_LOG_CHARS)
    )
}

/// Asks `llm` why the job failed.
///
/// # Errors
/// Whatever the model call fails with.
pub async fn explain_failure(llm: &impl Llm, ctx: &FailureContext) -> Result<String, AiError> {
    llm.complete(EXPLAIN_SYSTEM, &failure_prompt(ctx)).await
}

/// The model chosen by the environment (`GITKRAFT_AI_*`) and the keyring, reached through `client`.
///
/// Reads the keyring, which may block: call it off the UI thread.
///
/// # Errors
/// No key for a provider that needs one.
pub fn remote_from_env(client: DynClient) -> Result<Remote, AiError> {
    let env = |n: &str| std::env::var(n).ok();
    let config = AiConfig::from_env(&env);
    let key = config.key(&env, &gitkraft_platforms::credentials::SystemKeyring)?;
    Ok(Remote::new(client, config, key))
}

/// [`remote_from_env`] over the real network (no cache: a model answer must never be a stale one).
///
/// Reads the keyring, which may block: call it off the UI thread.
///
/// # Errors
/// No HTTP client, or no key for a provider that needs one.
pub fn remote_default() -> Result<Remote, AiError> {
    let http =
        gitkraft_platforms::ReqwestClient::new().map_err(|e| AiError::Transport(e.to_string()))?;
    remote_from_env(DynClient::new(http))
}

/// Reads the log of `job` and gathers what the model is told about its failure.
///
/// # Errors
/// The log cannot be fetched.
pub async fn failure_context<C: gitkraft_platforms::HttpClient + Clone>(
    ci: &gitkraft_ci::CiBackend<C>,
    run: &gitkraft_ci::Run,
    job: &gitkraft_ci::Job,
) -> Result<FailureContext, String> {
    let log = ci
        .job_log(&run.repo, job.id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(FailureContext {
        repo: run.repo.key(),
        workflow: run.name.clone(),
        job: job.name.clone(),
        branch: run.branch.clone(),
        log: gitkraft_ci::log::strip_ansi(&log),
    })
}

/// The job to explain: `wanted` if it failed, otherwise the first failed one.
#[must_use]
pub fn failed_job(jobs: &[gitkraft_ci::Job], wanted: Option<u64>) -> Option<&gitkraft_ci::Job> {
    wanted
        .and_then(|id| jobs.iter().find(|j| j.id == id))
        .filter(|j| j.status == gitkraft_ci::Status::Failed)
        .or_else(|| {
            jobs.iter()
                .find(|j| j.status == gitkraft_ci::Status::Failed)
        })
}

/// A model that answers from a queue, for tests of the code around it.
#[derive(Debug, Default)]
pub struct FakeLlm {
    answers: std::sync::Mutex<Vec<Result<String, AiError>>>,
    prompts: std::sync::Mutex<Vec<(String, String)>>,
}

impl FakeLlm {
    /// A model that gives `answer` once.
    #[must_use]
    pub fn saying(answer: &str) -> Self {
        Self {
            answers: std::sync::Mutex::new(vec![Ok(answer.to_string())]),
            prompts: std::sync::Mutex::default(),
        }
    }

    /// The `(system, user)` pairs it was asked.
    #[must_use]
    pub fn prompts(&self) -> Vec<(String, String)> {
        self.prompts.lock().map(|p| p.clone()).unwrap_or_default()
    }
}

impl Llm for FakeLlm {
    async fn complete(&self, system: &str, user: &str) -> Result<String, AiError> {
        if let Ok(mut p) = self.prompts.lock() {
            p.push((system.to_string(), user.to_string()));
        }
        self.answers
            .lock()
            .ok()
            .and_then(|mut a| (!a.is_empty()).then(|| a.remove(0)))
            .unwrap_or_else(|| Err(AiError::Transport("no answer queued".into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::credentials::MemoryStore;
    use gitkraft_platforms::testing::FakeClient;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn ctx(log: &str) -> FailureContext {
        FailureContext {
            repo: "github.com/o/r".into(),
            workflow: "CI".into(),
            job: "test".into(),
            branch: "main".into(),
            log: log.into(),
        }
    }

    #[test]
    fn credentials_are_masked_and_variables_are_not() {
        let log = "cloning with ghp_abcdefghijklmnopqrstuvwxyz0123456789\n\
                   Authorization: Bearer abc.def.ghi\n\
                   export API_KEY=hunter2hunter2\n\
                   token: ${{ secrets.X }}\n\
                   password=$DB_PASS\n\
                   AWS AKIAABCDEFGHIJKLMNOP used\n\
                   commit 0123456789abcdef0123456789abcdef01234567 failed";
        let r = redact(log);
        assert!(!r.contains("ghp_abc") && !r.contains("hunter2") && !r.contains("abc.def"));
        assert!(!r.contains("AKIAABCDEFGHIJKLMNOP"));
        assert!(r.contains("secrets.X") && r.contains("$DB_PASS"));
        assert!(r.contains("0123456789abcdef0123456789abcdef01234567"));
    }

    #[test]
    fn a_jwt_is_masked() {
        let jwt = format!(
            "eyJ{}.{}.{}",
            "a".repeat(20),
            "b".repeat(20),
            "c".repeat(20)
        );
        assert!(!redact(&format!("auth {jwt} ok")).contains("eyJ"));
    }

    #[test]
    fn a_short_log_is_kept_and_a_long_one_keeps_the_tail_and_the_first_error() {
        assert_eq!(focus("a\nb", 100), "a\nb");
        let mut log = String::from("start\n");
        log.push_str("error: early problem\n");
        for i in 0..2000 {
            log.push_str(&format!("noise line {i}\n"));
        }
        log.push_str("the very end\n");
        let f = focus(&log, 2000);
        assert!(f.chars().count() <= 2100, "{}", f.chars().count());
        assert!(f.contains("early problem") && f.contains("the very end"));
        assert!(f.contains("log cut"));
        assert!(!f.contains("noise line 5\n"));
    }

    #[test]
    fn config_follows_the_environment() {
        let env = |k: &str| (k == "OPENAI_API_KEY").then(|| "k".to_string());
        assert_eq!(AiConfig::from_env(&env).provider, Provider::OpenAi);
        assert_eq!(AiConfig::from_env(&no_env).provider, Provider::Anthropic);
        let env = |k: &str| match k {
            "GITKRAFT_AI_BASE_URL" => Some("http://localhost:11434/v1/".to_string()),
            "GITKRAFT_AI_PROVIDER" => Some("ollama".to_string()),
            "GITKRAFT_AI_MODEL" => Some("llama3".to_string()),
            _ => None,
        };
        let c = AiConfig::from_env(&env);
        assert_eq!(c.base_url, "http://localhost:11434/v1");
        assert!(c.is_local() && c.model == "llama3");
    }

    #[test]
    fn the_key_comes_from_the_environment_then_the_keyring_and_a_remote_one_is_required() {
        let c = AiConfig::from_env(&no_env);
        let store = MemoryStore::default();
        let err = c.key(&no_env, &store).expect_err("no key");
        assert!(
            err.to_string().contains("ANTHROPIC_API_KEY") && err.to_string().contains("ai-key")
        );
        store
            .set("ai.anthropic", &Secret::new("from-keyring"))
            .expect("set");
        assert_eq!(
            c.key(&no_env, &store).expect("ok").expect("key").expose(),
            "from-keyring"
        );
        let env = |k: &str| (k == "ANTHROPIC_API_KEY").then(|| " from-env ".to_string());
        assert_eq!(
            c.key(&env, &store).expect("ok").expect("key").expose(),
            "from-env"
        );
        let local = AiConfig {
            base_url: "http://127.0.0.1:1234/v1".into(),
            ..AiConfig::from_env(&no_env)
        };
        assert_eq!(local.key(&no_env, &MemoryStore::default()), Ok(None));
    }

    #[tokio::test]
    async fn anthropic_is_called_with_its_headers_and_the_text_is_returned() {
        let fake = FakeClient::new();
        fake.push_json(
            "https://api.anthropic.com/v1/messages",
            r#"{"content":[{"type":"text","text":"Cause: a"},{"type":"text","text":" b"}]}"#,
        );
        let cfg = AiConfig::from_env(&no_env);
        let llm = Remote::new(DynClient::new(fake.clone()), cfg, Some(Secret::new("K")));
        let out = explain_failure(&llm, &ctx("error: boom ghp_abcdefghijklmnopqrstuv"))
            .await
            .expect("answer");
        assert_eq!(out, "Cause: a b");
        let sent = &fake.requests()[0];
        let has = |n: &str, v: &str| sent.headers.iter().any(|(k, x)| k == n && x == v);
        assert!(has("x-api-key", "K") && has("anthropic-version", "2023-06-01"));
        let body = String::from_utf8_lossy(sent.body.as_deref().unwrap_or_default()).into_owned();
        assert!(body.contains("boom") && body.contains("claude-sonnet"));
        assert!(
            !body.contains("ghp_abc"),
            "secret left in the prompt: {body}"
        );
    }

    #[tokio::test]
    async fn an_openai_compatible_server_works_without_a_key() {
        let fake = FakeClient::new();
        fake.push_json(
            "http://localhost:11434/v1/chat/completions",
            r#"{"choices":[{"message":{"content":" fixed "}}]}"#,
        );
        let env = |k: &str| match k {
            "GITKRAFT_AI_BASE_URL" => Some("http://localhost:11434/v1".to_string()),
            "GITKRAFT_AI_PROVIDER" => Some("openai".to_string()),
            _ => None,
        };
        let llm = Remote::new(DynClient::new(fake.clone()), AiConfig::from_env(&env), None);
        assert_eq!(llm.complete("s", "u").await.as_deref(), Ok("fixed"));
        assert!(
            !fake.requests()[0]
                .headers
                .iter()
                .any(|(k, _)| k == "authorization")
        );
    }

    #[tokio::test]
    async fn a_refusal_carries_the_providers_message() {
        let fake = FakeClient::new();
        fake.push_status(
            "https://api.anthropic.com/v1/messages",
            401,
            r#"{"error":{"message":"invalid x-api-key"}}"#,
        );
        let llm = Remote::new(
            DynClient::new(fake),
            AiConfig::from_env(&no_env),
            Some(Secret::new("bad")),
        );
        assert_eq!(
            llm.complete("s", "u").await,
            Err(AiError::Provider(401, "invalid x-api-key".into()))
        );
    }

    #[tokio::test]
    async fn the_failure_prompt_names_the_job_and_the_fake_records_it() {
        let llm = FakeLlm::saying("ok");
        assert_eq!(
            explain_failure(&llm, &ctx("error: x")).await.as_deref(),
            Ok("ok")
        );
        let (system, user) = &llm.prompts()[0];
        assert!(
            system.contains("Cause") && user.contains("Job: test") && user.contains("error: x")
        );
    }

    #[test]
    fn the_job_to_explain_is_the_wanted_one_if_it_failed_else_the_first_failed() {
        use gitkraft_ci::{Job, Status};
        let job = |id, status| Job {
            id,
            status,
            ..Job::default()
        };
        let jobs = [
            job(1, Status::Success),
            job(2, Status::Failed),
            job(3, Status::Failed),
        ];
        assert_eq!(failed_job(&jobs, Some(3)).map(|j| j.id), Some(3));
        assert_eq!(
            failed_job(&jobs, Some(1)).map(|j| j.id),
            Some(2),
            "it passed"
        );
        assert_eq!(failed_job(&jobs, None).map(|j| j.id), Some(2));
        assert_eq!(failed_job(&jobs[..1], None).map(|j| j.id), None);
    }
}
