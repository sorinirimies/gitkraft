//! GitKraft web — your pull requests, issues, CI runs, pulse and notifications on localhost.
//!
//! Server rendered with [Topcoat](https://github.com/tokio-rs/topcoat); every page is a
//! plain GET, every action a JSON POST, so it works without a build step.
//!
//! | module | what |
//! |---|---|
//! | [`session`] | the hub, the dashboard and a short cache |
//! | [`pages`] | the pages |
//! | [`api`] | JSON and static routes |
//! | [`actions`] | what the buttons do |
//! | [`render`] | the calendar, charts and logs as HTML |
//! | [`style`] | the stylesheet and script |

pub mod actions;
pub mod api;
pub mod pages;
pub mod render;
pub mod session;
pub mod style;

pub use session::{Paths, Session};

use topcoat::router::{Router, RouterBuilderDiscoverExt};

/// Where and how to serve.
#[derive(Debug, Clone)]
pub struct ServerOptions {
    /// Interface to bind.
    pub host: String,
    /// Port.
    pub port: u16,
    /// Open the page in the browser once listening.
    pub open_browser: bool,
    /// Extra local checkouts, besides the working directory, for putting pull requests in worktrees.
    pub checkouts: Vec<std::path::PathBuf>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3333,
            open_browser: false,
            checkouts: Vec::new(),
        }
    }
}

/// The router serving `session`.
pub fn router(session: Session) -> Router {
    Router::builder().discover().app_context(session).build()
}

/// URL to open for a bound address (`0.0.0.0` is not browsable).
pub fn browser_url(addr: std::net::SocketAddr) -> String {
    if addr.ip().is_unspecified() {
        format!("http://localhost:{}", addr.port())
    } else if addr.is_ipv6() {
        format!("http://[{}]:{}", addr.ip(), addr.port())
    } else {
        format!("http://{addr}")
    }
}

/// Binds and serves until the process is killed.
pub async fn serve(session: Session, opts: ServerOptions) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind((opts.host.as_str(), opts.port)).await?;
    let addr = listener.local_addr()?;
    println!("gitkraft-web listening on {}", browser_url(addr));
    if !addr.ip().is_loopback() {
        eprintln!(
            "gitkraft-web: warning — bound to {addr}; there is no login, anyone who can reach it acts with your tokens."
        );
    }
    if opts.open_browser {
        let url = browser_url(addr);
        if let Err(e) = open_in_browser(&url) {
            eprintln!("gitkraft-web: could not open a browser ({e}); visit {url}");
        }
    }
    topcoat::serve(listener, router(session)).await
}

fn open_in_browser(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    #[cfg(target_os = "macos")]
    let mut cmd = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut cmd = Command::new("xdg-open");
    cmd.arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}
