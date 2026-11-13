//! `ssh setup`: pick or create a key, register it on the platform, point `~/.ssh/config` at it,
//! load it into the agent and test the connection.
//!
//! Every step is idempotent: running it twice changes nothing the second time.

use std::path::{Path, PathBuf};

use crate::backend::Backend;
use crate::error::PlatformError;
use crate::http::HttpClient;
use crate::platform::PlatformKind;

use super::config;
use super::key::PublicKey;
use super::tools::{Probe, SshTools};

/// What the caller decides.
#[derive(Debug, Clone)]
pub struct Options {
    /// Title of the key on the platform, e.g. `gitkraft my-laptop`.
    pub title: String,
    /// Comment inside a newly created key.
    pub comment: String,
    /// Use this existing private key instead of `gitkraft_<host>`.
    pub reuse: Option<PathBuf>,
    /// Let `ssh-keygen` ask for a passphrase.
    pub interactive: bool,
    /// Write the macOS keychain options.
    pub macos: bool,
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A key pair was created.
    KeyCreated(PathBuf),
    /// An existing key pair is used.
    KeyReused(PathBuf),
    /// The public key was added to the account.
    Uploaded {
        /// Platform id of the key.
        id: String,
    },
    /// The account already had this key.
    AlreadyRegistered {
        /// Title it has there.
        title: String,
    },
    /// `~/.ssh/config` now has the block.
    ConfigWritten(PathBuf),
    /// The old config was copied here first.
    ConfigBackedUp(PathBuf),
    /// The key is in the agent.
    AgentLoaded,
    /// The key could not be loaded; the reason.
    AgentSkipped(String),
    /// Result of `ssh -T`.
    Probed(Probe),
}

/// The outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Steps in order.
    pub steps: Vec<Step>,
    /// The private key in use.
    pub key_path: PathBuf,
    /// Its fingerprint.
    pub fingerprint: String,
}

/// Why setup stopped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SetupError {
    /// `ssh-keygen` failed.
    #[error("could not create the key: {0}")]
    Keygen(String),
    /// The `.pub` file is missing or unreadable.
    #[error("no usable public key at {0}")]
    NoPublicKey(PathBuf),
    /// A file could not be written.
    #[error("{0}")]
    Io(String),
    /// The platform refused.
    #[error("{0}")]
    Platform(PlatformError),
}

impl From<PlatformError> for SetupError {
    fn from(e: PlatformError) -> Self {
        Self::Platform(e)
    }
}

impl SetupError {
    /// What to do about it, for the platforms that need a special token scope.
    pub fn hint(&self, kind: PlatformKind, host: &str) -> Option<String> {
        let Self::Platform(e) = self else { return None };
        match (e, kind) {
            (PlatformError::Forbidden(_) | PlatformError::NotFound(_) | PlatformError::Auth(_), PlatformKind::GitHub) => Some(
                "GitHub needs the admin:public_key scope for this: run `gh auth refresh -h github.com -s admin:public_key`, or use a token that has it.".into(),
            ),
            (PlatformError::Forbidden(_) | PlatformError::NotFound(_) | PlatformError::Auth(_), PlatformKind::GitLab) => Some(
                "GitLab needs a token with the api scope (read_user is not enough to add keys).".into(),
            ),
            (PlatformError::Forbidden(_) | PlatformError::NotFound(_) | PlatformError::Auth(_), PlatformKind::Gitea) => Some(format!(
                "{host} needs a token with write:user (and read:user) to manage SSH keys."
            )),
            (PlatformError::Http { status: 422, .. }, _) => Some(
                "The platform says this key is already in use, possibly by another account: SSH keys are unique per platform. Use a different key (`--key PATH`).".into(),
            ),
            _ => None,
        }
    }
}

/// `gitkraft_<host>` with anything odd replaced, so it is a safe file name.
pub fn key_name(host: &str) -> String {
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("gitkraft_{safe}")
}

fn read_public(private: &Path) -> Result<PublicKey, SetupError> {
    let pub_path = private.with_extension("pub");
    let text = std::fs::read_to_string(&pub_path)
        .map_err(|_| SetupError::NoPublicKey(pub_path.clone()))?;
    text.lines()
        .next()
        .and_then(PublicKey::parse)
        .ok_or(SetupError::NoPublicKey(pub_path))
}

fn ensure_dir(dir: &Path) -> Result<(), SetupError> {
    std::fs::create_dir_all(dir).map_err(|e| SetupError::Io(format!("{}: {e}", dir.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn write_config(
    ssh_dir: &Path,
    host: &str,
    identity: &Path,
    macos: bool,
    steps: &mut Vec<Step>,
) -> Result<(), SetupError> {
    let path = ssh_dir.join("config");
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let new = config::upsert(&old, host, &identity.to_string_lossy(), macos);
    if new == old {
        return Ok(());
    }
    let backup = ssh_dir.join("config.gitkraft-backup");
    if path.exists() && !backup.exists() {
        std::fs::copy(&path, &backup)
            .map_err(|e| SetupError::Io(format!("backup {}: {e}", backup.display())))?;
        steps.push(Step::ConfigBackedUp(backup));
    }
    std::fs::write(&path, new).map_err(|e| SetupError::Io(format!("{}: {e}", path.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    steps.push(Step::ConfigWritten(path));
    Ok(())
}

/// Runs the whole setup for `host` with the account behind `backend`.
pub async fn run<C: HttpClient, T: SshTools>(
    backend: &Backend<C>,
    tools: &T,
    ssh_dir: &Path,
    host: &str,
    opts: &Options,
) -> Result<Report, SetupError> {
    ensure_dir(ssh_dir)?;
    let mut steps = Vec::new();

    // 1. the key: an explicit one, our own from an earlier run, or a new one
    let key_path = opts
        .reuse
        .clone()
        .unwrap_or_else(|| ssh_dir.join(key_name(host)));
    if key_path.with_extension("pub").exists() {
        steps.push(Step::KeyReused(key_path.clone()));
    } else if opts.reuse.is_some() {
        return Err(SetupError::NoPublicKey(key_path.with_extension("pub")));
    } else {
        tools
            .generate(&key_path, &opts.comment, opts.interactive)
            .map_err(SetupError::Keygen)?;
        steps.push(Step::KeyCreated(key_path.clone()));
    }
    let public = read_public(&key_path)?;
    let fingerprint = public.fingerprint();

    // 2. register it, unless the account already has it
    let existing = backend.ssh_keys().await?;
    match existing
        .iter()
        .find(|k| k.fingerprint.as_deref() == Some(fingerprint.as_str()))
    {
        Some(k) => steps.push(Step::AlreadyRegistered {
            title: k.title.clone(),
        }),
        None => {
            let added = backend.add_ssh_key(&opts.title, &public.bare()).await?;
            steps.push(Step::Uploaded { id: added.id });
        }
    }

    // 3. ~/.ssh/config
    write_config(ssh_dir, host, &key_path, opts.macos, &mut steps)?;

    // 4. the agent
    let loaded = tools
        .agent_keys()
        .iter()
        .any(|l| PublicKey::parse(l).is_some_and(|k| k.fingerprint() == fingerprint));
    if loaded {
        steps.push(Step::AgentLoaded);
    } else {
        match tools.add_to_agent(&key_path) {
            Ok(()) => steps.push(Step::AgentLoaded),
            Err(e) => steps.push(Step::AgentSkipped(e.trim().to_string())),
        }
    }

    // 5. test
    steps.push(Step::Probed(tools.probe(host)));
    Ok(Report {
        steps,
        key_path,
        fingerprint,
    })
}

/// Removes gitkraft's block for `host` from `~/.ssh/config`. Returns whether there was one.
pub fn unconfigure(ssh_dir: &Path, host: &str) -> Result<bool, SetupError> {
    let path = ssh_dir.join("config");
    let Ok(old) = std::fs::read_to_string(&path) else {
        return Ok(false);
    };
    if !config::has_block(&old, host) {
        return Ok(false);
    }
    std::fs::write(&path, config::remove(&old, host))
        .map_err(|e| SetupError::Io(format!("{}: {e}", path.display())))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{Account, Secret};
    use crate::ssh::tools::{FAKE_PUBLIC_KEY, FakeSsh};
    use crate::testing::FakeClient;

    fn backend(client: &FakeClient) -> Backend<FakeClient> {
        let account = Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"));
        Backend::new(&account, client.clone())
    }

    fn opts() -> Options {
        Options {
            title: "gitkraft laptop".into(),
            comment: "gitkraft@laptop".into(),
            reuse: None,
            interactive: false,
            macos: false,
        }
    }

    const KEYS: &str = "https://api.github.com/user/keys?per_page=100&limit=50";
    const KEYS_POST: &str = "https://api.github.com/user/keys";

    #[tokio::test]
    async fn a_fresh_setup_creates_registers_configures_loads_and_probes() {
        let dir = tempfile::tempdir().expect("tmp");
        std::fs::write(dir.path().join("config"), "Host work\n  User me\n").expect("w");
        let client = FakeClient::new();
        client.push_json(KEYS, "[]");
        client.push_json(
            KEYS_POST,
            r#"{"id":77,"title":"gitkraft laptop","key":"k"}"#,
        );
        let tools = FakeSsh::default();
        let report = run(&backend(&client), &tools, dir.path(), "github.com", &opts())
            .await
            .expect("setup");

        let key = dir.path().join("gitkraft_github.com");
        assert!(matches!(&report.steps[0], Step::KeyCreated(p) if *p == key));
        assert!(report.steps.contains(&Step::Uploaded { id: "77".into() }));
        assert!(
            report
                .steps
                .iter()
                .any(|s| matches!(s, Step::ConfigBackedUp(_)))
        );
        assert!(report.steps.contains(&Step::AgentLoaded));
        assert!(matches!(
            report.steps.last(),
            Some(Step::Probed(Probe::Authenticated { .. }))
        ));
        assert!(report.fingerprint.starts_with("SHA256:"));

        let cfg = std::fs::read_to_string(dir.path().join("config")).expect("cfg");
        assert!(
            cfg.starts_with("Host work")
                && cfg.contains("IdentityFile")
                && cfg.contains("gitkraft_github.com")
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("config.gitkraft-backup")).expect("backup"),
            "Host work\n  User me\n"
        );

        let sent = client.requests();
        let post = sent
            .iter()
            .find(|r| r.method == crate::http::Method::Post)
            .expect("post");
        let body = String::from_utf8(post.body.clone().expect("body")).expect("utf8");
        assert!(
            body.contains(FAKE_PUBLIC_KEY) && !body.contains("gitkraft@laptop"),
            "comment is not uploaded: {body}"
        );
        assert_eq!(tools.calls().len(), 3);
    }

    #[tokio::test]
    async fn running_it_again_changes_nothing() {
        let dir = tempfile::tempdir().expect("tmp");
        let client = FakeClient::new();
        client.push_json(KEYS, "[]");
        client.push_json(KEYS_POST, r#"{"id":1,"title":"t","key":"k"}"#);
        let tools = FakeSsh::default();
        let first = run(&backend(&client), &tools, dir.path(), "github.com", &opts())
            .await
            .expect("1");

        let again = FakeClient::new();
        again.push_json(
            KEYS,
            &format!(r#"[{{"id":1,"title":"gitkraft laptop","key":"{FAKE_PUBLIC_KEY}"}}]"#),
        );
        *tools.agent.lock().expect("lock") = vec![format!("{FAKE_PUBLIC_KEY} x")];
        let cfg_before = std::fs::read_to_string(dir.path().join("config")).expect("cfg");
        let second = run(&backend(&again), &tools, dir.path(), "github.com", &opts())
            .await
            .expect("2");

        assert!(matches!(second.steps[0], Step::KeyReused(_)));
        assert!(second.steps.contains(&Step::AlreadyRegistered {
            title: "gitkraft laptop".into()
        }));
        assert!(!second.steps.iter().any(|s| matches!(
            s,
            Step::Uploaded { .. } | Step::ConfigWritten(_) | Step::KeyCreated(_)
        )));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("config")).expect("cfg"),
            cfg_before
        );
        assert_eq!(first.fingerprint, second.fingerprint);
        assert!(
            again
                .requests()
                .iter()
                .all(|r| r.method == crate::http::Method::Get)
        );
        assert_eq!(
            tools
                .calls()
                .iter()
                .filter(|c| c.starts_with("add"))
                .count(),
            1,
            "the agent already had it the second time"
        );
    }

    #[tokio::test]
    async fn an_existing_key_can_be_reused_and_a_missing_one_is_an_error() {
        let dir = tempfile::tempdir().expect("tmp");
        let mine = dir.path().join("id_mine");
        std::fs::write(&mine, "PRIVATE").expect("w");
        std::fs::write(
            mine.with_extension("pub"),
            format!("{FAKE_PUBLIC_KEY} me\n"),
        )
        .expect("w");
        let client = FakeClient::new();
        client.push_json(KEYS, "[]");
        client.push_json(KEYS_POST, r#"{"id":5,"title":"t","key":"k"}"#);
        let tools = FakeSsh::default();
        let o = Options {
            reuse: Some(mine.clone()),
            ..opts()
        };
        let report = run(&backend(&client), &tools, dir.path(), "github.com", &o)
            .await
            .expect("setup");
        assert_eq!(report.key_path, mine);
        assert!(!tools.calls().iter().any(|c| c.starts_with("generate")));

        let o = Options {
            reuse: Some(dir.path().join("nope")),
            ..opts()
        };
        let err = run(&backend(&client), &tools, dir.path(), "github.com", &o)
            .await
            .expect_err("missing");
        assert!(matches!(err, SetupError::NoPublicKey(_)));
    }

    #[tokio::test]
    async fn a_failing_agent_is_reported_but_does_not_stop_setup() {
        let dir = tempfile::tempdir().expect("tmp");
        let client = FakeClient::new();
        client.push_json(KEYS, "[]");
        client.push_json(KEYS_POST, r#"{"id":1,"title":"t","key":"k"}"#);
        let tools = FakeSsh {
            agent_error: Some("Could not open a connection to your authentication agent.\n".into()),
            ..FakeSsh::default()
        };
        let report = run(&backend(&client), &tools, dir.path(), "github.com", &opts())
            .await
            .expect("setup");
        assert!(report.steps.contains(&Step::AgentSkipped(
            "Could not open a connection to your authentication agent.".into()
        )));
        assert!(matches!(report.steps.last(), Some(Step::Probed(_))));
    }

    #[tokio::test]
    async fn a_missing_scope_explains_itself_per_platform() {
        let dir = tempfile::tempdir().expect("tmp");
        let client = FakeClient::new();
        client.push_status(KEYS, 404, r#"{"message":"Not Found"}"#);
        let err = run(
            &backend(&client),
            &FakeSsh::default(),
            dir.path(),
            "github.com",
            &opts(),
        )
        .await
        .expect_err("scope");
        assert!(
            err.hint(PlatformKind::GitHub, "github.com")
                .is_some_and(|h| h.contains("admin:public_key"))
        );
        assert!(
            err.hint(PlatformKind::GitLab, "gitlab.com")
                .is_some_and(|h| h.contains("api scope"))
        );
        assert!(
            err.hint(PlatformKind::Gitea, "codeberg.org")
                .is_some_and(|h| h.contains("write:user"))
        );
        let dup = SetupError::Platform(PlatformError::Http {
            status: 422,
            message: "in use".into(),
        });
        assert!(
            dup.hint(PlatformKind::GitHub, "github.com")
                .is_some_and(|h| h.contains("--key"))
        );
        assert_eq!(
            SetupError::Io("x".into()).hint(PlatformKind::GitHub, "h"),
            None
        );
    }

    #[test]
    fn key_names_are_safe_file_names() {
        assert_eq!(key_name("github.com"), "gitkraft_github.com");
        assert_eq!(
            key_name("git.example.com:2222/x"),
            "gitkraft_git.example.com_2222_x"
        );
    }

    #[test]
    fn unconfigure_removes_only_our_block() {
        let dir = tempfile::tempdir().expect("tmp");
        assert!(!unconfigure(dir.path(), "github.com").expect("none"));
        let text = config::upsert("Host work\n  User me\n", "github.com", "k", false);
        std::fs::write(dir.path().join("config"), text).expect("w");
        assert!(unconfigure(dir.path(), "github.com").expect("removed"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("config")).expect("cfg"),
            "Host work\n  User me\n"
        );
        assert!(!unconfigure(dir.path(), "github.com").expect("again"));
    }
}
