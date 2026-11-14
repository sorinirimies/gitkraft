//! `gitkraft-tui ssh …`: get git over SSH working without leaving gitkraft.
//!
//! ```text
//! ssh status [HOST]             key, ~/.ssh/config block, agent and connection per host
//! ssh setup HOST [--key PATH] [--title T] [--passphrase]
//!                               create (or reuse) a key, register it, configure, load, test
//! ssh list HOST                 keys registered on the account (this machine's marked)
//! ssh revoke HOST ID            remove a key from the account
//! ssh remove HOST               drop gitkraft's block from ~/.ssh/config
//! ssh test HOST                 `ssh -T git@HOST`
//! ssh switch ssh|https [REMOTE] rewrite a remote of this checkout (default origin)
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use gitkraft_hub::accounts::AccountsFile;
use gitkraft_platforms::model::SshKey;
use gitkraft_platforms::ssh::render::{describe_probe, render_report};
use gitkraft_platforms::ssh::setup::{self, Options, key_name};
use gitkraft_platforms::ssh::{PublicKey, SshTools, SystemSsh, local_keys};
use gitkraft_platforms::{
    Account, Backend, CachedClient, HttpClient, ReqwestClient, Transport, parse_remote_url,
};
use gitkraft_settings::files;

/// A parsed `ssh` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show where each host stands.
    Status {
        /// Only this host.
        host: Option<String>,
    },
    /// Create or reuse a key and wire it up.
    Setup {
        /// The platform host.
        host: String,
        /// Use this private key.
        key: Option<PathBuf>,
        /// Title on the platform.
        title: Option<String>,
        /// Let `ssh-keygen` ask for a passphrase.
        passphrase: bool,
    },
    /// List keys on the account.
    List {
        /// The platform host.
        host: String,
    },
    /// Delete a key from the account.
    Revoke {
        /// The platform host.
        host: String,
        /// Key id from `list`.
        id: String,
    },
    /// Remove the `~/.ssh/config` block.
    Remove {
        /// The platform host.
        host: String,
    },
    /// Test the connection.
    Test {
        /// The platform host.
        host: String,
    },
    /// Rewrite a remote's URL.
    Switch {
        /// Target transport.
        to: Transport,
        /// Remote name.
        remote: String,
    },
}

const USAGE: &str =
    "usage: ssh status|setup|list|revoke|remove|test|switch (see `gitkraft-tui ssh` docs)";

/// Parses the arguments after `ssh`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let host = |i: usize| {
        a.get(i)
            .filter(|h| !h.starts_with("--"))
            .map(|h| h.to_lowercase())
            .ok_or_else(|| format!("{} needs a host, e.g. github.com", a[0]))
    };
    match a.first().copied() {
        None => Err(USAGE.into()),
        Some("status") => Ok(Command::Status { host: host(1).ok() }),
        Some("setup") => {
            let mut key = None;
            let mut title = None;
            let mut passphrase = false;
            let mut it = a.iter().skip(2);
            while let Some(arg) = it.next() {
                match *arg {
                    "--key" => key = Some(PathBuf::from(it.next().ok_or("--key needs a path")?)),
                    "--title" => title = Some(it.next().ok_or("--title needs a text")?.to_string()),
                    "--passphrase" => passphrase = true,
                    other => return Err(format!("unknown option '{other}'")),
                }
            }
            Ok(Command::Setup {
                host: host(1)?,
                key,
                title,
                passphrase,
            })
        }
        Some("list") => Ok(Command::List { host: host(1)? }),
        Some("revoke") => Ok(Command::Revoke {
            host: host(1)?,
            id: a
                .get(2)
                .ok_or("revoke needs the key id from `ssh list`")?
                .to_string(),
        }),
        Some("remove") => Ok(Command::Remove { host: host(1)? }),
        Some("test") => Ok(Command::Test { host: host(1)? }),
        Some("switch") => Ok(Command::Switch {
            to: a
                .get(1)
                .and_then(|t| Transport::parse(t))
                .ok_or("switch needs ssh or https")?,
            remote: a
                .get(2)
                .map_or_else(|| "origin".into(), |r| (*r).to_string()),
        }),
        Some(other) => Err(format!("unknown ssh command '{other}'")),
    }
}

/// `url` rewritten to `to`, or `None` when it is already that or is not a platform URL.
pub fn switch_url(url: &str, to: Transport) -> Option<String> {
    if Transport::of_url(url)? == to {
        return None;
    }
    parse_remote_url(url).map(|id| id.clone_url(to))
}

/// Text of `ssh list`; `mine` is the fingerprints of this machine's keys.
pub fn render_keys(keys: &[SshKey], mine: &[String], host: &str) -> String {
    if keys.is_empty() {
        return format!("no SSH keys on {host}\n");
    }
    let mut out = String::new();
    for k in keys {
        let here = k.fingerprint.as_ref().is_some_and(|f| mine.contains(f));
        out.push_str(&format!(
            "{} {:>10}  {}  {}{}\n",
            if here { "●" } else { " " },
            k.id,
            k.fingerprint.as_deref().unwrap_or("-"),
            k.title,
            k.created_at
                .map(|d| format!("  ({})", d.format("%Y-%m-%d")))
                .unwrap_or_default()
        ));
    }
    out.push_str("● = a key of this machine\n");
    out
}

fn machine_name() -> String {
    gitkraft_hub::keys::machine_name()
}

/// Runs a command against `backend_for` (one backend per host) and `tools`, in `ssh_dir`.
pub async fn run_with<C: HttpClient, T: SshTools>(
    cmd: Command,
    backend_for: &impl Fn(&str) -> Result<Backend<C>>,
    tools: &T,
    ssh_dir: &Path,
) -> Result<String> {
    match cmd {
        Command::Setup {
            host,
            key,
            title,
            passphrase,
        } => {
            let backend = backend_for(&host)?;
            let opts = Options {
                title: title.unwrap_or_else(|| format!("gitkraft {}", machine_name())),
                comment: format!("gitkraft@{}", machine_name()),
                reuse: key,
                interactive: passphrase,
                macos: cfg!(target_os = "macos"),
            };
            match setup::run(&backend, tools, ssh_dir, &host, &opts).await {
                Ok(report) => Ok(render_report(&report, &host)),
                Err(e) => {
                    let hint = e
                        .hint(backend.kind(), &host)
                        .map(|h| format!("\nhint: {h}"))
                        .unwrap_or_default();
                    bail!("{e}{hint}")
                }
            }
        }
        Command::List { host } => {
            let keys = backend_for(&host)?
                .ssh_keys()
                .await
                .map_err(|e| anyhow!("{host}: {e}"))?;
            Ok(render_keys(
                &keys,
                &gitkraft_hub::keys::local_fingerprints(ssh_dir, tools),
                &host,
            ))
        }
        Command::Revoke { host, id } => {
            backend_for(&host)?
                .delete_ssh_key(&id)
                .await
                .map_err(|e| anyhow!("{host}: {e}"))?;
            Ok(format!("removed key {id} from {host}\n"))
        }
        Command::Remove { host } => Ok(
            if setup::unconfigure(ssh_dir, &host).map_err(|e| anyhow!(e))? {
                format!(
                    "removed gitkraft's block for {host} from ~/.ssh/config (the key files stay)\n"
                )
            } else {
                format!("no gitkraft block for {host} in ~/.ssh/config\n")
            },
        ),
        Command::Test { host } => Ok(format!(
            "{}\n",
            describe_probe(&tools.probe(&probe_target(&host)), &host)
        )),
        Command::Status { host } => Ok(status_text(host, ssh_dir, tools)),
        Command::Switch { to, remote } => switch_remote(Path::new("."), to, &remote),
    }
}

fn status_text(only: Option<String>, ssh_dir: &Path, tools: &impl SshTools) -> String {
    let cfg = std::fs::read_to_string(ssh_dir.join("config")).unwrap_or_default();
    let hosts: Vec<String> = match only {
        Some(h) => vec![h],
        None => known_hosts(),
    };
    if hosts.is_empty() {
        return "no hosts known: run `gitkraft-tui ssh setup github.com` (or another host)\n"
            .into();
    }
    let agent: Vec<String> = tools
        .agent_keys()
        .iter()
        .filter_map(|l| PublicKey::parse(l))
        .map(|k| k.fingerprint())
        .collect();
    let mut out = String::new();
    for h in hosts {
        let key = ssh_dir.join(key_name(&h));
        let public = local_keys(ssh_dir).into_iter().find(|k| k.path == key);
        let in_agent = public
            .as_ref()
            .is_some_and(|k| agent.contains(&k.public.fingerprint()));
        out.push_str(&format!(
            "{h}\n  key      {}\n  config   {}\n  agent    {}\n  {}\n",
            if public.is_some() {
                format!("{} ✔", key.display())
            } else {
                "none (run ssh setup)".into()
            },
            if gitkraft_platforms::ssh::config::has_block(&cfg, &h) {
                "block present ✔"
            } else {
                "no gitkraft block"
            },
            if in_agent { "loaded ✔" } else { "not loaded" },
            describe_probe(&tools.probe(&probe_target(&h)), &h)
        ));
    }
    out
}

/// What to hand to `ssh -T` for `host`: the login and port this checkout's remote URLs use for it
/// (`gitea@host`, `git@host:30009`), else the bare host.
fn probe_target(host: &str) -> String {
    let urls: Vec<String> = git2::Repository::discover(".")
        .ok()
        .and_then(|r| gitkraft_core::features::remotes::list_remotes(&r).ok())
        .map(|rs| rs.into_iter().filter_map(|r| r.url).collect())
        .unwrap_or_default();
    target_from_urls(host, &urls)
}

/// The ssh target for `host` according to `urls`; the bare host when no ssh URL names it.
fn target_from_urls(host: &str, urls: &[String]) -> String {
    urls.iter()
        .filter(|u| parse_remote_url(u).is_some_and(|id| id.host == host))
        .find_map(|u| gitkraft_platforms::ssh_target_of_url(u))
        .unwrap_or_else(|| host.to_string())
}

/// Hosts of this checkout's remotes plus `accounts.toml`, plus the three well-known platforms.
fn known_hosts() -> Vec<String> {
    let mut hosts: Vec<String> = Vec::new();
    if let Ok(repo) = git2::Repository::discover(".")
        && let Ok(remotes) = gitkraft_core::features::remotes::list_remotes(&repo)
    {
        hosts.extend(
            remotes
                .iter()
                .filter_map(|r| r.url.as_deref().and_then(parse_remote_url))
                .map(|id| id.host),
        );
    }
    if let Some(file) = accounts_file() {
        hosts.extend(file.accounts.iter().map(|a| a.host.to_lowercase()));
    }
    if hosts.is_empty() {
        hosts.extend(["github.com", "gitlab.com", "codeberg.org"].map(String::from));
    }
    hosts.sort();
    hosts.dedup();
    hosts
}

fn accounts_file() -> Option<AccountsFile> {
    files::accounts_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
        .and_then(|t| AccountsFile::parse(&t).ok())
}

fn switch_remote(dir: &Path, to: Transport, name: &str) -> Result<String> {
    let repo = git2::Repository::discover(dir).context("not inside a git checkout")?;
    let remote = repo
        .find_remote(name)
        .with_context(|| format!("no remote '{name}'"))?;
    let url = remote.url().context("the remote has no URL")?.to_string();
    match switch_url(&url, to) {
        Some(new) => {
            repo.remote_set_url(name, &new)?;
            Ok(format!("{name}: {url}\n   → {new}\n"))
        }
        None if Transport::of_url(&url) == Some(to) => {
            Ok(format!("{name} already uses {to:?}: {url}\n"))
        }
        None => bail!("cannot read a platform repository from '{url}'"),
    }
}

/// The real backend for `host`: account kind from `accounts.toml` or the well-known hosts, token from the environment or CLI.
fn real_backend(host: &str) -> Result<Backend<CachedClient<ReqwestClient>>> {
    let map = accounts_file().unwrap_or_default().host_map();
    let kind = map.detect(host).ok_or_else(|| {
        anyhow!("unknown platform for {host}: add it to accounts.toml as [[account]] kind = \"gitlab\" | \"gitea\" | \"github\"")
    })?;
    let mut account = Account::new(kind, host);
    account.token = gitkraft_platforms::auth::resolve_token(kind, host);
    if account.token.is_none() {
        bail!(
            "no token for {host}: set one of {} or log in with the platform's CLI",
            gitkraft_platforms::auth::env_var_names(kind, host).join(", ")
        );
    }
    Ok(Backend::new(
        &account,
        CachedClient::new(ReqwestClient::new().map_err(|e| anyhow!(e))?),
    ))
}

fn ssh_dir() -> Result<PathBuf> {
    Ok(dirs::home_dir().context("no home directory")?.join(".ssh"))
}

/// Runs a command with the real programs and the network; returns the text to print.
pub fn run(cmd: Command) -> Result<String> {
    let dir = ssh_dir()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(run_with(cmd, &|h: &str| real_backend(h), &SystemSsh, &dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::ssh::tools::{FAKE_PUBLIC_KEY, FakeSsh};
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{PlatformKind, Secret};

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parsing_every_command() {
        assert_eq!(parse(&args("status")), Ok(Command::Status { host: None }));
        assert_eq!(
            parse(&args("status GitHub.com")),
            Ok(Command::Status {
                host: Some("github.com".into())
            })
        );
        assert_eq!(
            parse(&args(
                "setup github.com --key /k/id --title laptop --passphrase"
            )),
            Ok(Command::Setup {
                host: "github.com".into(),
                key: Some("/k/id".into()),
                title: Some("laptop".into()),
                passphrase: true
            })
        );
        assert_eq!(
            parse(&args("list gitlab.com")),
            Ok(Command::List {
                host: "gitlab.com".into()
            })
        );
        assert_eq!(
            parse(&args("revoke gitlab.com 5")),
            Ok(Command::Revoke {
                host: "gitlab.com".into(),
                id: "5".into()
            })
        );
        assert_eq!(
            parse(&args("switch ssh")),
            Ok(Command::Switch {
                to: Transport::Ssh,
                remote: "origin".into()
            })
        );
        assert_eq!(
            parse(&args("switch https up")),
            Ok(Command::Switch {
                to: Transport::Https,
                remote: "up".into()
            })
        );
        assert_eq!(
            parse(&args("remove x.org")),
            Ok(Command::Remove {
                host: "x.org".into()
            })
        );
        assert_eq!(
            parse(&args("test x.org")),
            Ok(Command::Test {
                host: "x.org".into()
            })
        );
    }

    #[test]
    fn parsing_errors_say_what_is_missing() {
        assert!(parse(&[]).is_err());
        assert!(
            parse(&args("setup"))
                .expect_err("host")
                .contains("needs a host")
        );
        assert!(parse(&args("setup github.com --key")).is_err());
        assert!(
            parse(&args("setup github.com --bogus"))
                .expect_err("opt")
                .contains("unknown option")
        );
        assert!(
            parse(&args("revoke github.com"))
                .expect_err("id")
                .contains("key id")
        );
        assert!(parse(&args("switch ftp")).is_err());
        assert!(parse(&args("frobnicate")).is_err());
    }

    #[test]
    fn the_probe_uses_the_login_and_port_of_the_remote_urls() {
        let urls: Vec<String> = [
            "git@github.com:o/r.git",
            "gitea@192.168.1.44:sorin/r.git",
            "ssh://git@192.168.1.204:30009/sorin/r.git",
            "https://codeberg.org/o/r.git",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(target_from_urls("github.com", &urls), "git@github.com");
        assert_eq!(
            target_from_urls("192.168.1.44", &urls),
            "gitea@192.168.1.44"
        );
        assert_eq!(
            target_from_urls("192.168.1.204", &urls),
            "git@192.168.1.204:30009"
        );
        assert_eq!(
            target_from_urls("codeberg.org", &urls),
            "codeberg.org",
            "https says nothing"
        );
        assert_eq!(
            target_from_urls("unknown.example", &urls),
            "unknown.example"
        );
    }

    #[test]
    fn switching_rewrites_between_transports_and_leaves_the_rest() {
        assert_eq!(
            switch_url("https://github.com/o/r.git", Transport::Ssh).as_deref(),
            Some("git@github.com:o/r.git")
        );
        assert_eq!(
            switch_url("git@github.com:o/r.git", Transport::Https).as_deref(),
            Some("https://github.com/o/r.git")
        );
        assert_eq!(switch_url("git@github.com:o/r.git", Transport::Ssh), None);
        assert_eq!(switch_url("/local/path", Transport::Ssh), None);
    }

    #[test]
    fn switching_a_remote_of_a_checkout_edits_its_config() {
        let dir = tempfile::tempdir().expect("tmp");
        let repo = git2::Repository::init(dir.path()).expect("init");
        repo.remote("origin", "https://github.com/o/r.git")
            .expect("remote");
        repo.remote("local", "/some/path").expect("remote");
        let out = switch_remote(dir.path(), Transport::Ssh, "origin").expect("switch");
        assert!(out.contains("→ git@github.com:o/r.git"), "{out}");
        assert_eq!(
            repo.find_remote("origin").expect("r").url().ok(),
            Some("git@github.com:o/r.git")
        );
        assert!(
            switch_remote(dir.path(), Transport::Ssh, "origin")
                .expect("same")
                .contains("already uses")
        );
        assert!(switch_remote(dir.path(), Transport::Ssh, "local").is_err());
        assert!(switch_remote(dir.path(), Transport::Ssh, "nope").is_err());
        assert!(switch_remote(&dir.path().join("x/y"), Transport::Ssh, "origin").is_err());
    }

    #[test]
    fn the_key_list_marks_this_machines_keys() {
        let keys = vec![
            SshKey {
                id: "1".into(),
                title: "laptop".into(),
                key: String::new(),
                fingerprint: Some("SHA256:mine".into()),
                created_at: None,
            },
            SshKey {
                id: "2".into(),
                title: "old".into(),
                key: String::new(),
                fingerprint: None,
                created_at: None,
            },
        ];
        let t = render_keys(&keys, &["SHA256:mine".into()], "h");
        assert!(
            t.lines().next().expect("l").starts_with('●')
                && t.lines().nth(1).expect("l").starts_with(' ')
        );
        assert_eq!(render_keys(&[], &[], "h"), "no SSH keys on h\n");
    }

    fn github(c: &FakeClient) -> impl Fn(&str) -> Result<Backend<FakeClient>> + use<> {
        let c = c.clone();
        move |h: &str| {
            Ok(Backend::new(
                &Account::new(PlatformKind::GitHub, h).with_token(Secret::new("t")),
                c.clone(),
            ))
        }
    }

    #[tokio::test]
    async fn setup_list_revoke_remove_and_test_work_end_to_end_with_fakes() {
        let dir = tempfile::tempdir().expect("tmp");
        let c = FakeClient::new();
        c.push_json(
            "https://api.github.com/user/keys?per_page=100&limit=50",
            "[]",
        );
        c.push_json(
            "https://api.github.com/user/keys",
            r#"{"id":8,"title":"t","key":"k"}"#,
        );
        let tools = FakeSsh::default();
        let backend_for = github(&c);

        let out = run_with(
            Command::Setup {
                host: "github.com".into(),
                key: None,
                title: Some("t".into()),
                passphrase: false,
            },
            &backend_for,
            &tools,
            dir.path(),
        )
        .await
        .expect("setup");
        assert!(
            out.contains("created key")
                && out.contains("registered on github.com (key id 8)")
                && out.contains("ssh to github.com works"),
            "{out}"
        );

        let c2 = FakeClient::new();
        c2.push_json(
            "https://api.github.com/user/keys?per_page=100&limit=50",
            &format!(r#"[{{"id":8,"title":"t","key":"{FAKE_PUBLIC_KEY}"}}]"#),
        );
        let listed = run_with(
            Command::List {
                host: "github.com".into(),
            },
            &github(&c2),
            &tools,
            dir.path(),
        )
        .await
        .expect("list");
        assert!(listed.starts_with('●'), "our key is marked: {listed}");

        c2.push_status("https://api.github.com/user/keys/8", 204, "");
        assert!(
            run_with(
                Command::Revoke {
                    host: "github.com".into(),
                    id: "8".into()
                },
                &github(&c2),
                &tools,
                dir.path()
            )
            .await
            .expect("revoke")
            .contains("removed key 8")
        );

        let st = run_with(
            Command::Status {
                host: Some("github.com".into()),
            },
            &backend_for,
            &tools,
            dir.path(),
        )
        .await
        .expect("status");
        assert!(st.contains("block present ✔") && st.contains("✔"), "{st}");
        assert!(
            run_with(
                Command::Remove {
                    host: "github.com".into()
                },
                &backend_for,
                &tools,
                dir.path()
            )
            .await
            .expect("remove")
            .contains("removed gitkraft's block")
        );
        assert!(
            run_with(
                Command::Remove {
                    host: "github.com".into()
                },
                &backend_for,
                &tools,
                dir.path()
            )
            .await
            .expect("again")
            .contains("no gitkraft block")
        );
        assert!(
            run_with(
                Command::Test {
                    host: "github.com".into()
                },
                &backend_for,
                &tools,
                dir.path()
            )
            .await
            .expect("test")
            .contains("works")
        );
    }

    #[tokio::test]
    async fn a_scope_problem_prints_the_hint() {
        let dir = tempfile::tempdir().expect("tmp");
        let c = FakeClient::new();
        c.push_status(
            "https://api.github.com/user/keys?per_page=100&limit=50",
            404,
            "{}",
        );
        let err = run_with(
            Command::Setup {
                host: "github.com".into(),
                key: None,
                title: None,
                passphrase: false,
            },
            &github(&c),
            &FakeSsh::default(),
            dir.path(),
        )
        .await
        .expect_err("scope");
        assert!(err.to_string().contains("admin:public_key"), "{err}");
    }

    #[tokio::test]
    async fn a_host_without_an_account_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().expect("tmp");
        let none = |h: &str| -> Result<Backend<FakeClient>> { Err(anyhow!("no token for {h}")) };
        let err = run_with(
            Command::List {
                host: "x.org".into(),
            },
            &none,
            &FakeSsh::default(),
            dir.path(),
        )
        .await
        .expect_err("none");
        assert!(err.to_string().contains("no token for x.org"));
    }
}
