//! The server: state, helpers, and the MCP handler.

use std::sync::Arc;

use gitkraft_ci::CiBackend;
use gitkraft_hub::{Hub, args};
use gitkraft_platforms::{Backend, DynClient, ItemKind, RepoId};
use rmcp::RoleServer;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{
    GetPromptRequestParams, GetPromptResponse, Implementation, ListPromptsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, ServerHandler, tool_handler};
use serde_json::Value as Json;

use crate::config::Policy;

pub(crate) type ToolResult = Result<String, McpError>;

const INSTRUCTIONS: &str = "GitKraft: pull requests, issues and CI of GitHub, GitLab, Gitea/Forgejo \
and Codeberg through one set of tools. Repositories are `host/owner/name` (`github.com/o/r`); \
`owner/name` works when there is one account or a github.com one, and `repo` can be left out to \
use the checkout the server was started in. Start with `accounts` to see what is connected, \
`my_pull_requests` / `my_issues` / `inbox` for what needs attention, `ci_runs` and `ci_job_log` \
for failing builds. Item numbers are the PR / MR / issue numbers of the platform. \
Tools that change things exist only when the server runs with --write.";

/// The server. Cheap to clone: everything is shared.
#[derive(Debug, Clone)]
pub struct GitkraftServer {
    pub(crate) hub: Arc<Hub<DynClient>>,
    pub(crate) default_repo: Option<RepoId>,
    pub(crate) policy: Policy,
    tool_router: ToolRouter<Self>,
}

pub(crate) fn invalid(msg: impl Into<String>) -> McpError {
    McpError::invalid_params(msg.into(), None)
}

pub(crate) fn failed(what: &str, e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(format!("{what}: {e}"), None)
}

pub(crate) fn out(v: &Json) -> ToolResult {
    serde_json::to_string(v).map_err(|e| McpError::internal_error(e.to_string(), None))
}

/// The first `max` characters of `s`, and whether anything was cut.
pub(crate) fn truncate(s: &str, max: usize) -> (&str, bool) {
    match s.char_indices().nth(max) {
        Some((i, _)) => (&s[..i], true),
        None => (s, false),
    }
}

/// Drops the noise a model does not need (avatar links) and, with `bodies` false, the long texts.
pub(crate) fn slim(mut v: Json, bodies: bool) -> Json {
    fn walk(v: &mut Json, bodies: bool) {
        match v {
            Json::Object(m) => {
                m.remove("avatar_url");
                if !bodies {
                    m.remove("body");
                }
                m.values_mut().for_each(|x| walk(x, bodies));
            }
            Json::Array(a) => a.iter_mut().for_each(|x| walk(x, bodies)),
            _ => {}
        }
    }
    walk(&mut v, bodies);
    v
}

pub(crate) fn kind_of(s: &str) -> Result<ItemKind, McpError> {
    match s {
        "pr" | "pull_request" | "mr" => Ok(ItemKind::PullRequest),
        "issue" => Ok(ItemKind::Issue),
        other => Err(invalid(format!("kind must be pr or issue, not '{other}'"))),
    }
}

impl GitkraftServer {
    /// Builds the server. The tools that change things are only registered when `policy.write`.
    #[must_use]
    pub fn new(hub: Hub<DynClient>, default_repo: Option<RepoId>, policy: Policy) -> Self {
        let mut tool_router = Self::read_router();
        if policy.write {
            tool_router += Self::write_router();
        }
        Self {
            hub: Arc::new(hub),
            default_repo,
            policy,
            tool_router,
        }
    }

    #[cfg(test)]
    pub(crate) fn tool_names(&self) -> Vec<String> {
        self.tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    /// `host/owner/name`, `owner/name` (the only or the github.com account), or the default repo.
    pub(crate) fn repo(&self, given: Option<&str>) -> Result<RepoId, McpError> {
        args::resolve_repo(
            given,
            self.default_repo.as_ref(),
            &self.hub.hosts(),
            "pass repo as host/owner/name: the server was not started in a git checkout",
        )
        .map_err(invalid)
    }

    pub(crate) fn backend(&self, repo: &RepoId) -> Result<&Backend<DynClient>, McpError> {
        self.hub.backend(&repo.host).ok_or_else(|| {
            invalid(format!(
                "no account for {}: set a token or log in (accounts lists what is connected)",
                repo.host
            ))
        })
    }

    pub(crate) fn ci(&self, repo: &RepoId) -> Result<&CiBackend<DynClient>, McpError> {
        self.hub
            .ci(&repo.host)
            .ok_or_else(|| invalid(format!("no account for {}", repo.host)))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for GitkraftServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new(
            "gitkraft-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(if self.policy.write {
            format!("{INSTRUCTIONS} Writes are ON.")
        } else {
            format!("{INSTRUCTIONS} Read-only.")
        })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult::with_all_items(crate::prompts::catalog(
            self.policy.write,
        )))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, McpError> {
        crate::prompts::render(&request.name, request.arguments.as_ref(), self.policy.write)
            .map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_cuts_on_characters() {
        assert_eq!(truncate("héllo", 2), ("hé", true));
        assert_eq!(truncate("hi", 5), ("hi", false));
    }

    #[test]
    fn slim_drops_avatars_everywhere_and_bodies_on_request() {
        let v = serde_json::json!({"body":"x","author":{"login":"a","avatar_url":"u"},"c":[{"body":"y"}]});
        let s = slim(v.clone(), false);
        assert_eq!(s, serde_json::json!({"author":{"login":"a"},"c":[{}]}));
        assert_eq!(slim(v, true)["body"], "x");
    }

    #[test]
    fn kinds_parse() {
        assert_eq!(kind_of("mr").ok(), Some(ItemKind::PullRequest));
        assert!(kind_of("x").is_err());
    }
}
