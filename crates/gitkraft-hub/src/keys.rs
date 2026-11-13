//! The SSH keys screen's data: keys per account, which of them belong to this machine,
//! and the actions on them. Front-ends render it; nothing here draws or blocks the UI thread.

use std::path::Path;

use gitkraft_platforms::HttpClient;
use gitkraft_platforms::model::SshKey;
use gitkraft_platforms::ssh::render::render_report;
use gitkraft_platforms::ssh::setup::{self, Options};
use gitkraft_platforms::ssh::{Probe, PublicKey, SshTools, local_keys};

use crate::Hub;

/// One key of one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRow {
    /// The account's host.
    pub host: String,
    /// The key.
    pub key: SshKey,
    /// This machine has the matching private key (on disk or in the agent).
    pub mine: bool,
}

/// Everything the screen shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeysData {
    /// Hosts with an account, in order.
    pub hosts: Vec<String>,
    /// Keys of all accounts.
    pub rows: Vec<KeyRow>,
    /// One line per account that could not be read.
    pub errors: Vec<String>,
}

/// Fingerprints of the keys this machine holds: `*.pub` in `ssh_dir` plus the agent.
pub fn local_fingerprints(ssh_dir: &Path, tools: &impl SshTools) -> Vec<String> {
    let mut out: Vec<String> = local_keys(ssh_dir)
        .iter()
        .map(|k| k.public.fingerprint())
        .collect();
    out.extend(
        tools
            .agent_keys()
            .iter()
            .filter_map(|l| PublicKey::parse(l))
            .map(|k| k.fingerprint()),
    );
    out.sort();
    out.dedup();
    out
}

/// Loads the keys of every account.
pub async fn load<C: HttpClient + Clone>(hub: &Hub<C>, mine: &[String]) -> KeysData {
    let hosts: Vec<String> = hub.hosts().iter().map(|h| (*h).to_string()).collect();
    let calls = hosts.iter().map(|h| async move {
        let r = match hub.backend(h) {
            Some(b) => b.ssh_keys().await.map_err(|e| format!("{h}: {e}")),
            None => Err(format!("{h}: no account")),
        };
        (h.clone(), r)
    });
    let results = futures::future::join_all(calls).await;
    let mut data = KeysData {
        hosts,
        ..KeysData::default()
    };
    for (host, result) in results {
        match result {
            Ok(keys) => data.rows.extend(keys.into_iter().map(|key| {
                let mine = key.fingerprint.as_ref().is_some_and(|f| mine.contains(f));
                KeyRow {
                    host: host.clone(),
                    key,
                    mine,
                }
            })),
            Err(e) => data.errors.push(e),
        }
    }
    data
}

/// Removes a key from an account.
pub async fn revoke<C: HttpClient + Clone>(
    hub: &Hub<C>,
    host: &str,
    id: &str,
) -> Result<String, String> {
    let b = hub
        .backend(host)
        .ok_or_else(|| format!("{host}: no account"))?;
    b.delete_ssh_key(id)
        .await
        .map_err(|e| format!("{host}: {e}"))?;
    Ok(format!("removed key {id} from {host}"))
}

/// Creates or reuses this machine's key for `host`, registers it, configures `~/.ssh/config`,
/// loads the agent and tests. Never asks for a passphrase: use `gitkraft-tui ssh setup --passphrase` for that.
pub async fn setup_host<C: HttpClient + Clone>(
    hub: &Hub<C>,
    host: &str,
    ssh_dir: &Path,
    tools: &impl SshTools,
    machine: &str,
) -> Result<String, String> {
    let b = hub
        .backend(host)
        .ok_or_else(|| format!("{host}: no account"))?;
    let opts = Options {
        title: format!("gitkraft {machine}"),
        comment: format!("gitkraft@{machine}"),
        reuse: None,
        interactive: false,
        macos: cfg!(target_os = "macos"),
    };
    match setup::run(b, tools, ssh_dir, host, &opts).await {
        Ok(report) => Ok(render_report(&report, host)),
        Err(e) => Err(match e.hint(b.kind(), host) {
            Some(h) => format!("{e}. {h}"),
            None => e.to_string(),
        }),
    }
}

/// Tests the connection to `host`.
pub fn test_host(tools: &impl SshTools, host: &str) -> (Probe, String) {
    let p = tools.probe(host);
    let line = gitkraft_platforms::ssh::render::describe_probe(&p, host);
    (p, line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::ssh::tools::{FAKE_PUBLIC_KEY, FakeSsh};
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, PlatformKind, Secret};

    fn hub(c: &FakeClient, hosts: &[(PlatformKind, &str)]) -> Hub<FakeClient> {
        Hub::new(
            hosts
                .iter()
                .map(|(k, h)| Account::new(*k, *h).with_token(Secret::new("t")))
                .collect(),
            c,
        )
    }

    const GH: &str = "https://api.github.com/user/keys?per_page=100&limit=50";
    const GL: &str = "https://gitlab.com/api/v4/user/keys?per_page=100&limit=50";

    #[tokio::test]
    async fn keys_of_all_accounts_are_merged_and_marked_when_they_are_ours() {
        let c = FakeClient::new();
        c.push_json(
            GH,
            &format!(r#"[{{"id":1,"title":"laptop","key":"{FAKE_PUBLIC_KEY}"}}]"#),
        );
        c.push_json(GL, r#"[{"id":2,"title":"other","key":"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKkk7oYqHq0VnoB0H8l0pU0b8Wm3cJxk4mT9qk1yJm2W"}]"#);
        let dir = tempfile::tempdir().expect("tmp");
        std::fs::write(dir.path().join("id.pub"), format!("{FAKE_PUBLIC_KEY} me\n")).expect("w");
        let mine = local_fingerprints(dir.path(), &FakeSsh::default());
        let data = load(
            &hub(
                &c,
                &[
                    (PlatformKind::GitHub, "github.com"),
                    (PlatformKind::GitLab, "gitlab.com"),
                ],
            ),
            &mine,
        )
        .await;
        assert_eq!(data.hosts, ["github.com", "gitlab.com"]);
        assert_eq!(data.rows.len(), 2);
        assert!(data.rows[0].mine && !data.rows[1].mine);
        assert!(data.errors.is_empty());
    }

    #[tokio::test]
    async fn an_account_that_fails_is_reported_without_hiding_the_others() {
        let c = FakeClient::new();
        c.push_json(GH, "[]");
        c.push_status(GL, 403, "{}");
        let data = load(
            &hub(
                &c,
                &[
                    (PlatformKind::GitHub, "github.com"),
                    (PlatformKind::GitLab, "gitlab.com"),
                ],
            ),
            &[],
        )
        .await;
        assert_eq!(data.errors.len(), 1);
        assert!(data.errors[0].starts_with("gitlab.com:"));
    }

    #[tokio::test]
    async fn revoke_and_setup_go_through_the_hub() {
        let c = FakeClient::new();
        c.push_status("https://api.github.com/user/keys/9", 204, "");
        let h = hub(&c, &[(PlatformKind::GitHub, "github.com")]);
        assert_eq!(
            revoke(&h, "github.com", "9").await.expect("ok"),
            "removed key 9 from github.com"
        );
        assert!(revoke(&h, "nowhere.org", "9").await.is_err());

        c.push_json(GH, "[]");
        c.push_json(
            "https://api.github.com/user/keys",
            r#"{"id":3,"title":"t","key":"k"}"#,
        );
        let dir = tempfile::tempdir().expect("tmp");
        let text = setup_host(&h, "github.com", dir.path(), &FakeSsh::default(), "laptop")
            .await
            .expect("setup");
        assert!(
            text.contains("registered on github.com") && text.contains("works"),
            "{text}"
        );
        assert!(
            setup_host(&h, "nowhere.org", dir.path(), &FakeSsh::default(), "x")
                .await
                .is_err()
        );
    }

    #[test]
    fn testing_a_host_returns_the_probe_and_a_line() {
        let (p, line) = test_host(
            &FakeSsh {
                probe_result: Some(Probe::Denied),
                ..FakeSsh::default()
            },
            "h",
        );
        assert_eq!(p, Probe::Denied);
        assert!(line.contains("refused"));
    }
}
