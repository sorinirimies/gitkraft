//! Downloading the files a CI run produced.

use std::path::{Path, PathBuf};

use gitkraft_platforms::{HttpClient, RepoId};

use crate::Hub;

/// A file name that cannot escape its directory: anything but letters, digits, `.`, `-` and `_`
/// becomes `_`, and a leading dot is dropped.
pub fn safe_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() { "artifact".into() } else { s }
}

/// Downloads every artifact of the run into `dir` as `<run>-<name>.zip`; returns the paths
/// and a note for each artifact that was skipped.
pub async fn download_all<C: HttpClient + Clone>(
    hub: &Hub<C>,
    repo: &RepoId,
    run_id: u64,
    dir: &Path,
) -> Result<(Vec<PathBuf>, Vec<String>), String> {
    let ci = hub
        .ci(&repo.host)
        .ok_or_else(|| format!("{}: no account", repo.host))?;
    let list = ci
        .artifacts(repo, run_id)
        .await
        .map_err(|e| format!("artifacts: {e}"))?;
    if list.is_empty() {
        return Err("this run has no artifacts".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut saved = Vec::new();
    let mut skipped = Vec::new();
    for a in list {
        if a.expired {
            skipped.push(format!("{} expired", a.name));
            continue;
        }
        match ci.download_artifact(repo, a.id).await {
            Ok(bytes) => {
                let path = dir.join(format!("{run_id}-{}.zip", safe_name(&a.name)));
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
                saved.push(path);
            }
            Err(e) => skipped.push(format!("{}: {e}", a.name)),
        }
    }
    Ok((saved, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitkraft_platforms::http::Response;
    use gitkraft_platforms::testing::FakeClient;
    use gitkraft_platforms::{Account, PlatformKind, Secret};

    #[test]
    fn names_cannot_escape_the_directory() {
        assert_eq!(safe_name("dist"), "dist");
        assert_eq!(safe_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(safe_name("build output (linux)"), "build_output__linux_");
        assert_eq!(safe_name("..."), "artifact");
        assert_eq!(safe_name(""), "artifact");
        assert_eq!(safe_name(".hidden"), "hidden");
    }

    #[tokio::test]
    async fn artifacts_are_saved_and_the_expired_or_failing_are_reported() {
        let c = FakeClient::new();
        c.push_json(
            "https://api.github.com/repos/o/r/actions/runs/7/artifacts",
            r#"{"artifacts":[{"id":1,"name":"dist","size_in_bytes":4},{"id":2,"name":"old","expired":true},{"id":3,"name":"broken","size_in_bytes":1}]}"#,
        );
        c.push(
            "https://api.github.com/repos/o/r/actions/artifacts/1/zip",
            Response {
                status: 200,
                headers: vec![],
                body: b"PK\x03\x04".to_vec(),
            },
        );
        c.push_status(
            "https://api.github.com/repos/o/r/actions/artifacts/3/zip",
            410,
            "gone",
        );
        let hub = Hub::new(
            vec![Account::new(PlatformKind::GitHub, "github.com").with_token(Secret::new("t"))],
            &c,
        );
        let dir = tempfile::tempdir().expect("tmp");
        let repo = RepoId::new("github.com", "o", "r");
        let (saved, skipped) = download_all(&hub, &repo, 7, &dir.path().join("out"))
            .await
            .expect("download");
        assert_eq!(saved, [dir.path().join("out/7-dist.zip")]);
        assert_eq!(std::fs::read(&saved[0]).expect("file"), b"PK\x03\x04");
        assert_eq!(skipped.len(), 2);
        assert!(
            skipped[0].contains("old expired") && skipped[1].starts_with("broken:"),
            "{skipped:?}"
        );
    }

    #[tokio::test]
    async fn no_artifacts_and_unknown_hosts_are_errors() {
        let c = FakeClient::new();
        c.push_json(
            "https://api.github.com/repos/o/r/actions/runs/7/artifacts",
            r#"{"artifacts":[]}"#,
        );
        let hub = Hub::new(vec![Account::new(PlatformKind::GitHub, "github.com")], &c);
        let dir = tempfile::tempdir().expect("tmp");
        let repo = RepoId::new("github.com", "o", "r");
        assert_eq!(
            download_all(&hub, &repo, 7, dir.path())
                .await
                .expect_err("none"),
            "this run has no artifacts"
        );
        let other = RepoId::new("gitlab.com", "o", "r");
        assert!(
            download_all(&hub, &other, 7, dir.path())
                .await
                .expect_err("host")
                .contains("no account")
        );
    }
}
