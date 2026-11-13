//! The platform side of SSH keys: list, add and delete. All four platforms use `/user/keys`.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::api::Api;
use crate::error::Result;
use crate::http::{HttpClient, Method};
use crate::model::SshKey;

use super::key::PublicKey;

#[derive(Debug, Deserialize)]
struct Raw {
    id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

impl From<Raw> for SshKey {
    fn from(r: Raw) -> Self {
        let fingerprint = r
            .fingerprint
            .filter(|f| f.starts_with("SHA256:"))
            .or_else(|| PublicKey::parse(&r.key).map(|k| k.fingerprint()));
        Self {
            id: r.id.to_string(),
            title: r.title,
            key: r.key,
            fingerprint,
            created_at: r.created_at,
        }
    }
}

pub(crate) async fn list<C: HttpClient>(api: &Api<C>) -> Result<Vec<SshKey>> {
    let raw: Vec<Raw> = api.get("/user/keys?per_page=100&limit=50").await?;
    Ok(raw.into_iter().map(SshKey::from).collect())
}

pub(crate) async fn add<C: HttpClient>(api: &Api<C>, title: &str, key: &str) -> Result<SshKey> {
    let res = api
        .post_json(
            "/user/keys",
            &serde_json::json!({ "title": title, "key": key }),
        )
        .await?;
    crate::api::parse::<Raw>(&res).map(SshKey::from)
}

pub(crate) async fn delete<C: HttpClient>(api: &Api<C>, id: &str) -> Result<()> {
    api.send(Method::Delete, &format!("/user/keys/{id}"), None)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use crate::account::{Account, Secret};
    use crate::backend::Backend;
    use crate::http::Method;
    use crate::platform::PlatformKind;
    use crate::testing::FakeClient;

    const ED: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJ0r5GmJcQGHlZ4gkUXZv9m+i1m8c4pQXnTQ4v8nY2ka";

    fn backend(kind: PlatformKind, host: &str, c: &FakeClient) -> Backend<FakeClient> {
        Backend::new(
            &Account::new(kind, host).with_token(Secret::new("t")),
            c.clone(),
        )
    }

    #[tokio::test]
    async fn lists_keys_of_every_platform_with_a_fingerprint() {
        let cases = [
            (
                PlatformKind::GitHub,
                "github.com",
                "https://api.github.com/user/keys?per_page=100&limit=50",
            ),
            (
                PlatformKind::GitLab,
                "gitlab.com",
                "https://gitlab.com/api/v4/user/keys?per_page=100&limit=50",
            ),
            (
                PlatformKind::Gitea,
                "codeberg.org",
                "https://codeberg.org/api/v1/user/keys?per_page=100&limit=50",
            ),
        ];
        for (kind, host, url) in cases {
            let c = FakeClient::new();
            c.push_json(url, &format!(r#"[{{"id":9,"title":"laptop","key":"{ED}","created_at":"2026-01-02T03:04:05Z"}}]"#));
            let keys = backend(kind, host, &c).ssh_keys().await.expect(host);
            assert_eq!(
                (keys[0].id.as_str(), keys[0].title.as_str()),
                ("9", "laptop"),
                "{host}"
            );
            assert!(
                keys[0]
                    .fingerprint
                    .as_deref()
                    .is_some_and(|f| f.starts_with("SHA256:"))
            );
            assert!(keys[0].created_at.is_some());
        }
    }

    #[tokio::test]
    async fn gitea_supplies_its_own_fingerprint_and_a_bad_one_is_recomputed() {
        let c = FakeClient::new();
        c.push_json(
            "https://codeberg.org/api/v1/user/keys?per_page=100&limit=50",
            &format!(r#"[{{"id":1,"title":"a","key":"{ED}","fingerprint":"SHA256:given"}},{{"id":2,"title":"b","key":"{ED}","fingerprint":"aa:bb"}}]"#),
        );
        let keys = backend(PlatformKind::Gitea, "codeberg.org", &c)
            .ssh_keys()
            .await
            .expect("keys");
        assert_eq!(keys[0].fingerprint.as_deref(), Some("SHA256:given"));
        assert_ne!(keys[1].fingerprint.as_deref(), Some("aa:bb"));
    }

    #[tokio::test]
    async fn adding_and_deleting_use_the_documented_calls() {
        let c = FakeClient::new();
        c.push_json(
            "https://gitlab.com/api/v4/user/keys",
            r#"{"id":4,"title":"t","key":"k"}"#,
        );
        c.push_status("https://gitlab.com/api/v4/user/keys/4", 204, "");
        let b = backend(PlatformKind::GitLab, "gitlab.com", &c);
        assert_eq!(b.add_ssh_key("t", ED).await.expect("add").id, "4");
        b.delete_ssh_key("4").await.expect("delete");
        let sent = c.requests();
        assert_eq!(sent[0].method, Method::Post);
        assert!(
            sent[0]
                .headers
                .iter()
                .any(|(k, v)| k == "PRIVATE-TOKEN" && v == "t")
        );
        assert_eq!(sent[1].method, Method::Delete);
        assert!(sent[1].url.ends_with("/user/keys/4"));
    }

    #[tokio::test]
    async fn a_refusal_is_an_error_not_an_empty_list() {
        let c = FakeClient::new();
        c.push_status(
            "https://api.github.com/user/keys?per_page=100&limit=50",
            403,
            "{}",
        );
        assert!(
            backend(PlatformKind::GitHub, "github.com", &c)
                .ssh_keys()
                .await
                .is_err()
        );
    }
}
