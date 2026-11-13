//! `gitkraft-web` — GitKraft on localhost.
//!
//! Binds `127.0.0.1:3333`; `--host`, `--port` override; `--no-open` keeps the browser closed.

use gitkraft_hub::Hub;
use gitkraft_hub::accounts::{AccountsFile, discover};
use gitkraft_platforms::{CachedClient, DynClient, ReqwestClient};
use gitkraft_settings::dashboard::Dashboard;
use gitkraft_settings::files;
use gitkraft_web::{Paths, ServerOptions, Session, serve};

const USAGE: &str = "usage: gitkraft-web [--host HOST] [--port PORT] [--no-open]";

fn parse_args(args: &[String]) -> Result<ServerOptions, String> {
    let mut opts = ServerOptions {
        open_browser: true,
        ..ServerOptions::default()
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--host" => opts.host = it.next().ok_or("--host needs a value")?.clone(),
            "--port" => {
                opts.port = it
                    .next()
                    .ok_or("--port needs a value")?
                    .parse()
                    .map_err(|_| "--port must be a number")?
            }
            "--no-open" => opts.open_browser = false,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown option '{other}'\n{USAGE}")),
        }
    }
    Ok(opts)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let read = |p: anyhow::Result<std::path::PathBuf>| {
        p.ok().and_then(|p| files::read_optional(&p).ok().flatten())
    };
    let dashboard = match read(files::dashboard_path()) {
        Some(text) => {
            Dashboard::parse(&text).map_err(|e| anyhow::anyhow!("dashboard.toml: {e}"))?
        }
        None => Dashboard::starter(),
    };
    let file = match read(files::accounts_path()) {
        Some(text) => {
            AccountsFile::parse(&text).map_err(|e| anyhow::anyhow!("accounts.toml: {e}"))?
        }
        None => AccountsFile::default(),
    };
    // Remote URLs of the checkout we were started in, plus the repos the dashboard names.
    let mut urls: Vec<String> = Vec::new();
    if let Ok(repo) = git2::Repository::discover(".")
        && let Ok(remotes) = repo.remotes()
    {
        for name in remotes.iter().flatten().flatten() {
            if let Ok(r) = repo.find_remote(name)
                && let Ok(u) = r.url()
            {
                urls.push(u.to_string());
            }
        }
    }
    for key in dashboard.sections.iter().flat_map(|s| s.repos.iter()) {
        urls.push(format!("https://{key}"));
    }
    let accounts = tokio::task::spawn_blocking(move || {
        discover(&urls, &file, gitkraft_platforms::auth::resolve_token)
    })
    .await?;
    if accounts.is_empty() {
        eprintln!(
            "gitkraft-web: no accounts found; set GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN or log in with `gh`"
        );
    }
    let client = DynClient::new(std::sync::Arc::new(CachedClient::new(
        ReqwestClient::new().map_err(|e| anyhow::anyhow!(e.to_string()))?,
    )));
    let hub = Hub::new(accounts, &client);
    let theme = gitkraft_settings::persistence::load_settings()
        .map(|s| s.theme_index())
        .unwrap_or(0);
    serve(Session::new(hub, dashboard, Paths::user(), theme), opts).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<ServerOptions, String> {
        parse_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn defaults_and_flags() {
        let d = p(&[]).expect("ok");
        assert_eq!(
            (d.host.as_str(), d.port, d.open_browser),
            ("127.0.0.1", 3333, true)
        );
        let o = p(&["--host", "0.0.0.0", "--port", "8080", "--no-open"]).expect("ok");
        assert_eq!(
            (o.host.as_str(), o.port, o.open_browser),
            ("0.0.0.0", 8080, false)
        );
    }

    #[test]
    fn bad_flags_explain_themselves() {
        assert!(
            p(&["--port"])
                .expect_err("missing")
                .contains("needs a value")
        );
        assert!(p(&["--port", "x"]).expect_err("bad").contains("number"));
        assert!(
            p(&["--bogus"])
                .expect_err("unknown")
                .contains("unknown option")
        );
        assert!(p(&["--help"]).expect_err("help").starts_with("usage"));
    }
}
