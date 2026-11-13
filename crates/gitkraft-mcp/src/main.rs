use std::process::ExitCode;

use gitkraft_hub::Hub;
use gitkraft_mcp::{Cli, Config, GitkraftServer, HELP};
use gitkraft_platforms::{CachedClient, DynClient, RepoId, ReqwestClient};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> ExitCode {
    match Cli::parse(std::env::args().skip(1), |k| std::env::var(k).ok()) {
        Ok(Cli::Help) => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Cli::Version) => {
            println!("gitkraft-mcp {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Cli::Run(cfg)) => match run(cfg).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("{e}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

async fn run(cfg: Config) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let default_repo = match cfg.repo.as_deref() {
        Some(key) => Some(
            RepoId::from_key(key)
                .ok_or_else(|| anyhow::anyhow!("--repo '{key}' is not host/owner/name"))?,
        ),
        None => gitkraft_hub::accounts::repo_of_dir(&cwd),
    };
    let extra: Vec<String> = default_repo.iter().map(RepoId::key).collect();
    let accounts =
        tokio::task::spawn_blocking(move || gitkraft_hub::accounts::discover_for(&cwd, &extra))
            .await?
            .map_err(|e| anyhow::anyhow!(e))?;
    // stdout carries the protocol, so everything we say goes to stderr
    if accounts.is_empty() {
        eprintln!(
            "gitkraft-mcp: no accounts found; set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or log in with `gitkraft-tui login`"
        );
    }
    eprintln!(
        "gitkraft-mcp {} ready: {} account(s), {}",
        env!("CARGO_PKG_VERSION"),
        accounts.len(),
        match (cfg.policy.write, cfg.policy.allow_force) {
            (false, _) => "read-only",
            (true, false) => "writes on",
            (true, true) => "writes and force merge on",
        }
    );
    let client = DynClient::new(std::sync::Arc::new(CachedClient::new(
        ReqwestClient::new().map_err(|e| anyhow::anyhow!(e.to_string()))?,
    )));
    let hub = Hub::new(accounts, &client);
    let service = GitkraftServer::new(hub, default_repo, cfg.policy)
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP handshake failed: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP session failed: {e}"))?;
    Ok(())
}
