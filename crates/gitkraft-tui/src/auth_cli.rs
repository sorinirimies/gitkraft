//! `gitkraft-tui login|logout`: sign in once, keep the token in the system keyring.
//!
//! ```text
//! login HOST                    OAuth device flow (GitHub, GitLab): approve in the browser
//! login HOST --token            paste a personal access token (any platform)
//! login HOST --client-id ID     use this OAuth application (also accounts.toml / GITKRAFT_OAUTH_CLIENT_ID)
//! logout HOST                   remove the stored token
//! ```

use anyhow::{Result, anyhow, bail};
use gitkraft_hub::accounts::{AccountsFile, oauth_client_id};
use gitkraft_platforms::credentials::{SecretStore, SystemKeyring};
use gitkraft_platforms::oauth::{self, DeviceCode};
use gitkraft_platforms::{
    Account, Backend, CachedClient, HttpClient, PlatformKind, ReqwestClient, Secret,
};
use gitkraft_settings::files;

/// A parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Sign in.
    Login {
        /// Platform host.
        host: String,
        /// Paste a token instead of the device flow.
        token: bool,
        /// OAuth application id.
        client_id: Option<String>,
    },
    /// Forget the token.
    Logout {
        /// Platform host.
        host: String,
    },
}

/// Parses `login …` / `logout …` (the first argument is the command word).
pub fn parse(args: &[String]) -> Result<Command, String> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let host = a
        .get(1)
        .filter(|h| !h.starts_with("--"))
        .map(|h| h.to_lowercase())
        .ok_or_else(|| {
            format!(
                "{} needs a host, e.g. github.com",
                a.first().unwrap_or(&"login")
            )
        })?;
    match a.first().copied() {
        Some("logout") => Ok(Command::Logout { host }),
        Some("login") => {
            let mut token = false;
            let mut client_id = None;
            let mut it = a.iter().skip(2);
            while let Some(arg) = it.next() {
                match *arg {
                    "--token" => token = true,
                    "--client-id" => {
                        client_id = Some(it.next().ok_or("--client-id needs a value")?.to_string())
                    }
                    other => return Err(format!("unknown option '{other}'")),
                }
            }
            Ok(Command::Login {
                host,
                token,
                client_id,
            })
        }
        _ => Err("usage: login HOST [--token] [--client-id ID] | logout HOST".into()),
    }
}

/// What the terminal does for the flow.
pub trait Terminal {
    /// Shows the code the user must type.
    fn announce(&self, code: &DeviceCode);
    /// Reads a pasted token (without echo where possible).
    fn read_token(&self, host: &str) -> Result<String>;
}

/// Everything a run touches, injected so tests need no network, keyring or terminal.
pub struct Ctx<'a, C> {
    /// The platform of a host.
    pub kind_of: &'a dyn Fn(&str) -> Option<PlatformKind>,
    /// HTTP.
    pub client: &'a C,
    /// Where the token is kept.
    pub store: &'a dyn SecretStore,
    /// The terminal.
    pub term: &'a dyn Terminal,
    /// `accounts.toml`.
    pub file: &'a AccountsFile,
    /// Environment lookup.
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// Multiplier for poll waits (tests use 0).
    pub scale: f64,
}

impl<C> std::fmt::Debug for Ctx<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ctx")
            .field("scale", &self.scale)
            .finish_non_exhaustive()
    }
}

/// Runs a command.
pub async fn run_with<C: HttpClient + Clone>(cmd: Command, cx: &Ctx<'_, C>) -> Result<String> {
    let Ctx {
        kind_of,
        client,
        store,
        term,
        file,
        env,
        scale,
    } = *cx;
    match cmd {
        Command::Logout { host } => Ok(if store.delete(&host).map_err(|e| anyhow!(e))? {
            format!("removed the stored token for {host}\n")
        } else {
            format!("no stored token for {host}\n")
        }),
        Command::Login {
            host,
            token,
            client_id,
        } => {
            let kind = kind_of(&host).ok_or_else(|| {
                anyhow!("unknown platform for {host}: add it to accounts.toml as [[account]] kind = \"gitlab\" | \"gitea\" | \"github\"")
            })?;
            let secret = if token {
                let t = term.read_token(&host)?;
                let t = t.trim();
                if t.is_empty() {
                    bail!("no token given");
                }
                Secret::new(t)
            } else {
                let id = oauth_client_id(&host, client_id.as_deref(), env, file).ok_or_else(|| {
                    anyhow!(
                        "no OAuth client id for {host}. Register an OAuth application there and pass --client-id, set GITKRAFT_OAUTH_CLIENT_ID, or add oauth_client_id to accounts.toml; or sign in with a token: `login {host} --token`"
                    )
                })?;
                let code = oauth::start(client, kind, &host, &id, oauth::default_scopes(kind))
                    .await
                    .map_err(|e| anyhow!("{host}: {e}"))?;
                term.announce(&code);
                oauth::wait(client, kind, &host, &id, &code, scale)
                    .await
                    .map_err(|e| anyhow!("{host}: {e}"))?
            };
            // Check it works before keeping it.
            let account = Account::new(kind, host.clone()).with_token(secret.clone());
            let me = Backend::new(&account, client.clone())
                .current_user()
                .await
                .map_err(|e| anyhow!("{host} rejected the token: {e}"))?;
            store.set(&host, &secret).map_err(|e| {
                anyhow!(
                    "signed in as {} but could not store the token: {e}",
                    me.login
                )
            })?;
            Ok(format!(
                "signed in to {host} as {}; the token is in the system keyring\n",
                me.login
            ))
        }
    }
}

struct Stdio;

impl Terminal for Stdio {
    fn announce(&self, code: &DeviceCode) {
        println!(
            "Open {} and enter the code\n\n    {}\n\nWaiting for you to approve…",
            code.verification_uri, code.user_code
        );
    }

    fn read_token(&self, host: &str) -> Result<String> {
        use std::io::{BufRead, Write};
        print!("Token for {host}: ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        Ok(line)
    }
}

/// Runs a command for real; returns the text to print.
pub fn run(cmd: Command) -> Result<String> {
    let file = files::accounts_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
        .and_then(|t| AccountsFile::parse(&t).ok())
        .unwrap_or_default();
    let map = file.host_map();
    let client = CachedClient::new(ReqwestClient::new().map_err(|e| anyhow!(e))?);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = std::sync::Arc::new(client);
    rt.block_on(run_with(
        cmd,
        &Ctx {
            kind_of: &|h: &str| map.detect(h),
            client: &client,
            store: &SystemKeyring,
            term: &Stdio,
            file: &file,
            env: &|n: &str| std::env::var(n).ok(),
            scale: 1.0,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::credentials::MemoryStore;
    use gitkraft_platforms::testing::FakeClient;
    use std::cell::RefCell;

    struct Fake {
        token: &'static str,
        shown: RefCell<Vec<String>>,
    }

    impl Terminal for Fake {
        fn announce(&self, code: &DeviceCode) {
            self.shown.borrow_mut().push(code.user_code.clone());
        }

        fn read_token(&self, _: &str) -> Result<String> {
            Ok(self.token.to_string())
        }
    }

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn kinds(h: &str) -> Option<PlatformKind> {
        match h {
            "github.com" => Some(PlatformKind::GitHub),
            "gitlab.com" => Some(PlatformKind::GitLab),
            "codeberg.org" => Some(PlatformKind::Gitea),
            _ => None,
        }
    }

    #[test]
    fn parsing() {
        assert_eq!(
            parse(&args("login GitHub.com")),
            Ok(Command::Login {
                host: "github.com".into(),
                token: false,
                client_id: None
            })
        );
        assert_eq!(
            parse(&args("login x.org --token --client-id abc")),
            Ok(Command::Login {
                host: "x.org".into(),
                token: true,
                client_id: Some("abc".into())
            })
        );
        assert_eq!(
            parse(&args("logout x.org")),
            Ok(Command::Logout {
                host: "x.org".into()
            })
        );
        assert!(parse(&args("login")).is_err());
        assert!(parse(&args("login x.org --client-id")).is_err());
        assert!(parse(&args("login x.org --what")).is_err());
        assert!(parse(&args("nope x.org")).is_err());
    }

    fn ctx<'a>(
        client: &'a FakeClient,
        store: &'a MemoryStore,
        term: &'a Fake,
        env: &'a dyn Fn(&str) -> Option<String>,
    ) -> Ctx<'a, FakeClient> {
        static FILE: std::sync::OnceLock<AccountsFile> = std::sync::OnceLock::new();
        Ctx {
            kind_of: &kinds,
            client,
            store,
            term,
            file: FILE.get_or_init(AccountsFile::default),
            env,
            scale: 0.0,
        }
    }

    const USER: &str = r#"{"login":"octocat","id":1}"#;

    #[tokio::test]
    async fn the_device_flow_stores_a_verified_token() {
        let c = FakeClient::new();
        c.push_json("https://github.com/login/device/code", r#"{"device_code":"dc","user_code":"ABCD-1234","verification_uri":"https://github.com/login/device","interval":1}"#);
        c.push_json(
            "https://github.com/login/oauth/access_token",
            r#"{"error":"authorization_pending"}"#,
        );
        c.push_json(
            "https://github.com/login/oauth/access_token",
            r#"{"access_token":"gho_x"}"#,
        );
        c.push_json("https://api.github.com/user", USER);
        let store = MemoryStore::default();
        let term = Fake {
            token: "",
            shown: RefCell::default(),
        };
        let env = |n: &str| (n == "GITKRAFT_OAUTH_CLIENT_ID").then(|| "Iv1.abc".to_string());
        let out = run_with(
            Command::Login {
                host: "github.com".into(),
                token: false,
                client_id: None,
            },
            &ctx(&c, &store, &term, &env),
        )
        .await
        .expect("login");
        assert!(out.contains("signed in to github.com as octocat"), "{out}");
        assert_eq!(term.shown.borrow().as_slice(), ["ABCD-1234"]);
        assert_eq!(
            store.get("github.com").map(|s| s.expose().to_string()),
            Some("gho_x".into())
        );
        let user_req = c
            .requests()
            .into_iter()
            .find(|r| r.url.ends_with("/user"))
            .expect("verified");
        assert!(
            user_req
                .headers
                .iter()
                .any(|(k, v)| k == "Authorization" && v == "Bearer gho_x")
        );
    }

    #[tokio::test]
    async fn a_pasted_token_is_verified_before_it_is_kept() {
        let none = |_: &str| None;
        let term = Fake {
            token: " ghp_ok \n",
            shown: RefCell::default(),
        };
        let c = FakeClient::new();
        c.push_json(
            "https://codeberg.org/api/v1/user",
            r#"{"login":"me","id":2}"#,
        );
        let store = MemoryStore::default();
        let cmd = Command::Login {
            host: "codeberg.org".into(),
            token: true,
            client_id: None,
        };
        let out = run_with(cmd.clone(), &ctx(&c, &store, &term, &none))
            .await
            .expect("login");
        assert!(out.contains("as me"));
        assert_eq!(
            store.get("codeberg.org").map(|s| s.expose().to_string()),
            Some("ghp_ok".into())
        );

        let bad = FakeClient::new();
        bad.push_status("https://codeberg.org/api/v1/user", 401, "{}");
        let store2 = MemoryStore::default();
        let err = run_with(cmd, &ctx(&bad, &store2, &term, &none))
            .await
            .expect_err("rejected");
        assert!(err.to_string().contains("rejected the token"));
        assert!(
            store2.get("codeberg.org").is_none(),
            "a bad token is not stored"
        );

        let empty = Fake {
            token: "  ",
            shown: RefCell::default(),
        };
        assert!(
            run_with(
                Command::Login {
                    host: "codeberg.org".into(),
                    token: true,
                    client_id: None
                },
                &ctx(&c, &store2, &empty, &none)
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn missing_client_id_unknown_host_and_gitea_flow_explain_themselves() {
        let none = |_: &str| None;
        let term = Fake {
            token: "",
            shown: RefCell::default(),
        };
        let c = FakeClient::new();
        let store = MemoryStore::default();
        let login = |h: &str| Command::Login {
            host: h.into(),
            token: false,
            client_id: None,
        };
        let e = run_with(login("github.com"), &ctx(&c, &store, &term, &none))
            .await
            .expect_err("no id");
        assert!(
            e.to_string().contains("--client-id") && e.to_string().contains("--token"),
            "{e}"
        );
        let e = run_with(login("nowhere.org"), &ctx(&c, &store, &term, &none))
            .await
            .expect_err("host");
        assert!(e.to_string().contains("accounts.toml"));
        let e = run_with(
            Command::Login {
                host: "codeberg.org".into(),
                token: false,
                client_id: Some("x".into()),
            },
            &ctx(&c, &store, &term, &none),
        )
        .await
        .expect_err("gitea");
        assert!(e.to_string().contains("no device flow"), "{e}");
    }

    #[tokio::test]
    async fn logout_removes_only_what_is_there() {
        let store = MemoryStore::default();
        store.set("github.com", &Secret::new("t")).expect("set");
        let none = |_: &str| None;
        let term = Fake {
            token: "",
            shown: RefCell::default(),
        };
        let c = FakeClient::new();
        let out = run_with(
            Command::Logout {
                host: "github.com".into(),
            },
            &ctx(&c, &store, &term, &none),
        )
        .await
        .expect("logout");
        assert!(out.contains("removed"));
        let out = run_with(
            Command::Logout {
                host: "github.com".into(),
            },
            &ctx(&c, &store, &term, &none),
        )
        .await
        .expect("again");
        assert!(out.contains("no stored token"));
    }
}
