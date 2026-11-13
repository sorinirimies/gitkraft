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
    let repo = match parse_remote_url(spec) {
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
    let chosen = transport.or_else(|| file.clone_via(&repo.host));
    // A full URL the user typed is used as typed unless they asked for a transport.
    let url = match (is_full_url(spec), chosen) {
        (true, None) => spec.to_string(),
        _ => repo.clone_url(chosen.unwrap_or(Transport::Https)),
    };
    let dest = dest.unwrap_or_else(|| cwd.join(&repo.name));
    Ok(ClonePlan { repo, url, dest })
}

fn is_full_url(spec: &str) -> bool {
    spec.contains("://") || spec.starts_with("git@")
}

#[cfg(test)]
mod tests {
    use super::*;
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
