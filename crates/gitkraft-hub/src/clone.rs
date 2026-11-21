//! Which repository and URL a `clone` means: `owner/name`, `host/owner/name` or a URL, over the transport
//! you chose, the one `accounts.toml` says for the host, or https.

use std::path::PathBuf;

use gitkraft_platforms::{RepoId, Transport, parse_remote_url};

use crate::accounts::AccountsFile;

/// What to clone and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClonePlan {
    /// The project.
    pub repo: RepoId,
    /// URL handed to git.
    pub url: String,
    /// The folder to create.
    pub dest: PathBuf,
}

/// Works out the clone of `spec`. `transport` overrides the file's `clone_via`; without either it is https.
/// `dest` defaults to the project's name in `cwd`.
pub fn plan(
    spec: &str,
    transport: Option<Transport>,
    dest: Option<PathBuf>,
    file: &AccountsFile,
    cwd: &std::path::Path,
) -> Result<ClonePlan, String> {
    let spec = spec.trim();
    let repo = match parse_remote_url(spec).or_else(|| local_url_repo(spec)) {
        Some(r) => r,
        None => {
            let parts: Vec<&str> = spec.trim_matches('/').split('/').collect();
            match parts.as_slice() {
                [owner, name] if !owner.is_empty() && !name.is_empty() => {
                    RepoId::new("github.com", *owner, name.trim_end_matches(".git"))
                }
                [host, owner, name]
                    if host.contains('.') && !owner.is_empty() && !name.is_empty() =>
                {
                    RepoId::new(*host, *owner, name.trim_end_matches(".git"))
                }
                _ => {
                    return Err(format!(
                        "'{spec}' is not owner/name, host/owner/name or a git URL"
                    ));
                }
            }
        }
    };
    // The checkout folder is named after the project, so the name must be one plain folder name:
    // `.` or `..` would clone into the picked folder itself or above it.
    if matches!(repo.name.as_str(), "" | "." | "..") || repo.name.contains(['/', '\\', '\0']) {
        return Err(format!("'{spec}' does not name a project"));
    }
    let chosen = transport.or_else(|| file.clone_via(&repo.host));
    // A full URL the user typed is used as typed unless they asked for a transport.
    let url = match (is_full_url(spec), chosen) {
        (true, None) => spec.to_string(),
        _ => repo.clone_url(chosen.unwrap_or(Transport::Https)),
    };
    let dest = dest.unwrap_or_else(|| cwd.join(&repo.name));
    Ok(ClonePlan { repo, url, dest })
}

/// A `file://` URL names its project after the last two path segments, with the host
/// `localhost`: `file:///srv/o/r.git` and `file:///C:/x/o/r.git` (what Git for Windows wants) are
/// `localhost/o/r`.
fn local_url_repo(spec: &str) -> Option<RepoId> {
    let path = spec.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    let mut parts = path.trim_end_matches('/').rsplit('/');
    let name = parts.next()?.trim_end_matches(".git");
    let owner = parts
        .next()
        .filter(|o| !o.is_empty() && !o.ends_with(':'))?;
    (!name.is_empty()).then(|| RepoId::new("localhost", owner, name))
}

fn is_full_url(spec: &str) -> bool {
    spec.contains("://") || spec.starts_with("git@")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_named_dot_or_dot_dot_is_refused_so_the_clone_stays_in_its_folder() {
        // Found by the clone_plan fuzz target.
        let none = AccountsFile::default();
        let cwd = std::path::Path::new("/picked");
        for spec in [
            "/\0/.",
            "o/.",
            "o/..",
            "github.com/o/..",
            "https://github.com/o/...git",
            "https://github.com/o/..",
        ] {
            if let Ok(p) = plan(spec, None, None, &none, cwd) {
                assert_eq!(
                    p.dest.parent(),
                    Some(cwd),
                    "{spec:?} -> {}",
                    p.dest.display()
                );
                assert!(!matches!(p.repo.name.as_str(), "." | ".."), "{spec:?}");
            }
        }
        assert!(plan("o/..", None, None, &none, cwd).is_err());
    }
    use std::path::Path;

    fn file(text: &str) -> AccountsFile {
        AccountsFile::parse(text).expect("file")
    }

    #[test]
    fn short_forms_become_urls_over_the_chosen_transport() {
        let none = AccountsFile::default();
        let cwd = Path::new("/w");
        let p = plan("o/r", None, None, &none, cwd).expect("plan");
        assert_eq!(p.url, "https://github.com/o/r.git");
        assert_eq!(p.dest, Path::new("/w/r"));
        let p = plan("o/r", Some(Transport::Ssh), None, &none, cwd).expect("plan");
        assert_eq!(p.url, "git@github.com:o/r.git");
        let p = plan(
            "codeberg.org/o/r.git",
            Some(Transport::Ssh),
            Some("/x".into()),
            &none,
            cwd,
        )
        .expect("plan");
        assert_eq!(
            (p.repo, p.url.as_str(), p.dest),
            (
                RepoId::new("codeberg.org", "o", "r"),
                "git@codeberg.org:o/r.git",
                PathBuf::from("/x")
            )
        );
    }

    #[test]
    fn the_account_file_chooses_the_transport_and_an_explicit_flag_wins() {
        let f = file("[[account]]\nkind = \"gitea\"\nhost = \"git.x.org\"\nclone_via = \"ssh\"\n");
        let cwd = Path::new("/w");
        assert_eq!(
            plan("git.x.org/a/b", None, None, &f, cwd)
                .expect("plan")
                .url,
            "git@git.x.org:a/b.git"
        );
        assert_eq!(
            plan("git.x.org/a/b", Some(Transport::Https), None, &f, cwd)
                .expect("plan")
                .url,
            "https://git.x.org/a/b.git"
        );
    }

    #[test]
    fn a_typed_url_is_kept_unless_a_transport_is_asked_for() {
        let none = AccountsFile::default();
        let cwd = Path::new("/w");
        let url = "https://github.com/o/r";
        assert_eq!(plan(url, None, None, &none, cwd).expect("plan").url, url);
        assert_eq!(
            plan(url, Some(Transport::Ssh), None, &none, cwd)
                .expect("plan")
                .url,
            "git@github.com:o/r.git"
        );
    }

    #[test]
    fn a_local_file_url_clones_as_typed_also_with_a_drive_letter() {
        let none = AccountsFile::default();
        for url in [
            "file://localhost/tmp/x/o/widgets.git",
            "file://localhost/C:/Users/me/AppData/Local/Temp/x/o/widgets.git",
            "file:///C:/Users/me/AppData/Local/Temp/x/o/widgets.git",
            "file:///tmp/x/o/widgets.git",
        ] {
            let p = plan(url, None, None, &none, Path::new("/w")).expect(url);
            assert_eq!(p.url, url);
            assert_eq!(p.dest, Path::new("/w").join("widgets"));
        }
    }

    #[test]
    fn a_file_url_needs_an_owner_folder_and_a_name() {
        let none = AccountsFile::default();
        for bad in ["file://", "file:///", "file:///r.git", "file:///C:/r.git"] {
            assert!(
                plan(bad, None, None, &none, Path::new("/w")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn nonsense_is_explained() {
        let none = AccountsFile::default();
        for bad in ["", "r", "a/b/c/d", "x//y", "/"] {
            assert!(
                plan(bad, None, None, &none, Path::new("/w")).is_err(),
                "{bad}"
            );
        }
    }
}
