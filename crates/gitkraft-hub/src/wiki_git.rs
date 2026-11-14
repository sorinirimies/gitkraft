//! GitHub wikis, which have no API: each one is a git repository called `<repo>.wiki.git`. They are
//! cloned into the cache folder, read from there and saved by commit and push, with the system `git`
//! and the credentials you already have (an ssh key, or a credential helper such as `gh`'s).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gitkraft_platforms::{RepoId, Transport};

/// Runs `git`. Behind a trait so tests never touch the network or the real git.
pub trait GitRunner: Send + Sync + std::fmt::Debug {
    /// Runs `git args…` in `dir` (the current folder when `None`); returns stdout.
    ///
    /// # Errors
    /// Git could not run, or exited with an error (its message).
    fn run(&self, dir: Option<&Path>, args: &[&str]) -> Result<String, String>;
}

/// The `git` on the path. It never asks for a password on a terminal: a missing credential is an
/// error, so a background task cannot hang.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemGit;

impl GitRunner for SystemGit {
    fn run(&self, dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
        let mut cmd = std::process::Command::new("git");
        cmd.args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null());
        if let Some(d) = dir {
            cmd.current_dir(d);
        }
        let out = cmd.output().map_err(|e| format!("cannot run git: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            Err(err
                .trim()
                .lines()
                .last()
                .unwrap_or("git failed")
                .to_string())
        }
    }
}

/// Where wikis are cloned and how git runs: part of a [`crate::Hub`].
#[derive(Debug, Clone)]
pub struct WikiGit {
    /// The git to run.
    pub git: Arc<dyn GitRunner>,
    /// The folder holding the clones.
    pub cache: PathBuf,
}

/// Git that is not there: what a [`crate::Hub`] has until a front-end gives it [`WikiGit::system`],
/// so tests and tools that never read wikis cannot run a subprocess or reach the network by accident.
#[derive(Debug, Clone, Copy)]
struct NoGit;

impl GitRunner for NoGit {
    fn run(&self, _dir: Option<&Path>, _args: &[&str]) -> Result<String, String> {
        Err("GitHub wikis are git repositories (<repo>.wiki.git): this build was not given git to read them".into())
    }
}

impl Default for WikiGit {
    fn default() -> Self {
        Self {
            git: Arc::new(NoGit),
            cache: std::env::temp_dir().join("gitkraft-wikis-unused"),
        }
    }
}

impl WikiGit {
    /// The system `git`, cloning under the user's cache folder.
    pub fn system() -> Self {
        Self {
            git: Arc::new(SystemGit),
            cache: dirs::cache_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join("gitkraft")
                .join("wikis"),
        }
    }
}

/// The wiki's clone URL: the repository's own with `.wiki` before `.git`.
pub fn wiki_url(repo: &RepoId, transport: Transport) -> String {
    let url = repo.clone_url(transport);
    match url.strip_suffix(".git") {
        Some(base) => format!("{base}.wiki.git"),
        None => format!("{url}.wiki.git"),
    }
}

/// A page in a wiki checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The file name without its extension: what other pages link to.
    pub slug: String,
    /// The name as GitHub shows it (dashes are spaces).
    pub title: String,
}

fn is_page(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "md" | "markdown"))
        && !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('_'))
}

fn slug_file(slug: &str) -> String {
    // a slug is a file name: no folders, no escapes
    let clean: String = slug
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0') {
                '-'
            } else {
                c
            }
        })
        .collect();
    format!("{}.md", clean.trim_matches('.'))
}

/// The pages of a checkout, `Home` first, then by name.
pub fn pages(dir: &Path) -> Vec<Page> {
    let mut v: Vec<Page> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_page(p))
        .filter_map(|p| {
            let slug = p.file_stem()?.to_str()?.to_string();
            Some(Page {
                title: slug.replace('-', " "),
                slug,
            })
        })
        .collect();
    v.sort_by(|a, b| {
        (a.slug != "Home", a.slug.to_lowercase()).cmp(&(b.slug != "Home", b.slug.to_lowercase()))
    });
    v
}

/// The text of page `slug`.
///
/// # Errors
/// There is no such page.
pub fn read(dir: &Path, slug: &str) -> Result<String, String> {
    for ext in ["md", "markdown"] {
        let p = dir.join(format!("{}.{ext}", slug.replace(['/', '\\'], "-")));
        if p.is_file() {
            return std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()));
        }
    }
    Err(format!("no wiki page '{slug}'"))
}

impl WikiGit {
    /// The folder of `repo`'s clone.
    pub fn dir_of(&self, repo: &RepoId) -> PathBuf {
        self.cache
            .join(crate::artifacts::safe_name(&repo.key().replace('/', "_")))
    }

    /// Clones the wiki, or brings an earlier clone up to date. https first, then ssh, so whichever
    /// credentials you have work; both failing says so.
    ///
    /// # Errors
    /// The wiki does not exist (turn it on and create a first page on the web), or no credential works.
    pub fn sync(&self, repo: &RepoId) -> Result<PathBuf, String> {
        let dir = self.dir_of(repo);
        if dir.join(".git").is_dir() {
            self.git
                .run(Some(&dir), &["pull", "--ff-only", "--quiet"])
                .map_err(|e| format!("updating the wiki: {e}"))?;
            return Ok(dir);
        }
        std::fs::create_dir_all(&self.cache)
            .map_err(|e| format!("{}: {e}", self.cache.display()))?;
        let target = dir.to_string_lossy().into_owned();
        let mut last = String::new();
        for t in [Transport::Https, Transport::Ssh] {
            let url = wiki_url(repo, t);
            match self.git.run(None, &["clone", "--quiet", &url, &target]) {
                Ok(_) => return Ok(dir),
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&dir);
                    last = e;
                }
            }
        }
        Err(format!(
            "cannot clone the wiki of {}: {last}. Has the wiki a first page? Private wikis need an ssh key (`gitkraft-tui ssh setup`) or a git credential helper",
            repo.full_name()
        ))
    }

    /// Writes page `slug` and pushes it as one commit.
    ///
    /// # Errors
    /// The wiki cannot be reached or the push is refused.
    pub fn save(
        &self,
        repo: &RepoId,
        slug: &str,
        content: &str,
        message: &str,
    ) -> Result<String, String> {
        let dir = self.sync(repo)?;
        let file = slug_file(slug);
        let path = dir.join(&file);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(content) {
            return Ok(format!("{slug}: no changes to save"));
        }
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        self.git.run(Some(&dir), &["add", "--", &file])?;
        self.git.run(
            Some(&dir),
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        )?;
        self.git
            .run(Some(&dir), &["push", "--quiet"])
            .map_err(|e| format!("pushing the wiki: {e}"))?;
        Ok(format!("saved wiki page {slug}"))
    }
}

/// A git for tests of programs that push: it records every command, answers `rev-parse` with `head`
/// and refuses pushes to the remotes in `refuse` as a moved-on remote would.
#[derive(Debug, Default)]
pub struct ScriptedGit {
    /// Every command run, joined by spaces.
    pub calls: std::sync::Mutex<Vec<String>>,
    /// The branch `rev-parse --abbrev-ref HEAD` gives (`HEAD` when detached).
    pub head: String,
    /// Remotes whose push is rejected.
    pub refuse: Vec<String>,
}

impl GitRunner for ScriptedGit {
    fn run(&self, _dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
        if let Ok(mut c) = self.calls.lock() {
            c.push(args.join(" "));
        }
        match args {
            ["rev-parse", ..] => Ok(format!("{}\n", self.head)),
            ["push", _, remote, _] if self.refuse.iter().any(|r| r == remote) => {
                Err("! [rejected] main -> main (non-fast-forward)".into())
            }
            _ => Ok(String::new()),
        }
    }
}

/// Runs blocking work on its own thread, so an async caller is not held up by a subprocess.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    rx.await.map_err(|_| "the git task stopped".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records every command; `fail` makes the first N clones fail.
    #[derive(Debug, Default)]
    struct FakeGit {
        calls: Mutex<Vec<String>>,
        fail_clones: Mutex<usize>,
    }

    impl GitRunner for FakeGit {
        fn run(&self, _dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
            self.calls.lock().expect("lock").push(args.join(" "));
            if args[0] == "clone" {
                let mut n = self.fail_clones.lock().expect("lock");
                if *n > 0 {
                    *n -= 1;
                    return Err("fatal: could not read Username".into());
                }
                // a clone makes the folder, with a .git inside
                let dest = Path::new(args.last().copied().unwrap_or_default());
                std::fs::create_dir_all(dest.join(".git")).map_err(|e| e.to_string())?;
            }
            Ok(String::new())
        }
    }

    fn wiki(fake: Arc<FakeGit>, tmp: &Path) -> WikiGit {
        WikiGit {
            git: fake,
            cache: tmp.to_path_buf(),
        }
    }

    fn repo() -> RepoId {
        RepoId::new("github.com", "o", "r")
    }

    #[test]
    fn the_wiki_is_the_repository_with_wiki_before_git() {
        assert_eq!(
            wiki_url(&repo(), Transport::Https),
            "https://github.com/o/r.wiki.git"
        );
        assert_eq!(
            wiki_url(&repo(), Transport::Ssh),
            "git@github.com:o/r.wiki.git"
        );
    }

    #[test]
    fn pages_are_the_markdown_files_home_first() {
        let tmp = tempfile::tempdir().expect("tmp");
        for f in [
            "Zeta.md",
            "Home.md",
            "Getting-Started.markdown",
            "_Sidebar.md",
            "logo.png",
        ] {
            std::fs::write(tmp.path().join(f), "x").expect("write");
        }
        std::fs::create_dir(tmp.path().join("images.md")).expect("dir");
        let p = pages(tmp.path());
        let slugs: Vec<_> = p.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(slugs, ["Home", "Getting-Started", "Zeta"]);
        assert_eq!(p[1].title, "Getting Started");
        assert_eq!(read(tmp.path(), "Home").as_deref(), Ok("x"));
        assert!(read(tmp.path(), "Nope").is_err());
    }

    #[test]
    fn a_slug_cannot_leave_the_wiki_folder() {
        let f = slug_file("../../etc/passwd");
        assert!(!f.contains('/') && !f.starts_with('.'), "{f}");
        assert!(!slug_file("a/b").contains('/'));
        assert_eq!(slug_file("Home"), "Home.md");
    }

    #[test]
    fn a_first_sync_clones_over_https_and_falls_back_to_ssh() {
        let tmp = tempfile::tempdir().expect("tmp");
        let fake = Arc::new(FakeGit::default());
        *fake.fail_clones.lock().expect("lock") = 1;
        let dir = wiki(fake.clone(), tmp.path()).sync(&repo()).expect("sync");
        assert!(dir.join(".git").is_dir());
        let calls = fake.calls.lock().expect("lock").clone();
        assert!(
            calls[0].contains("https://github.com/o/r.wiki.git"),
            "{calls:?}"
        );
        assert!(
            calls[1].contains("git@github.com:o/r.wiki.git"),
            "{calls:?}"
        );

        // the second sync pulls
        wiki(fake.clone(), tmp.path()).sync(&repo()).expect("pull");
        assert!(
            fake.calls
                .lock()
                .expect("lock")
                .last()
                .is_some_and(|c| c.starts_with("pull --ff-only"))
        );

        // both transports failing explains what to do
        let tmp = tempfile::tempdir().expect("tmp");
        let fake = Arc::new(FakeGit::default());
        *fake.fail_clones.lock().expect("lock") = 2;
        let e = wiki(fake, tmp.path()).sync(&repo()).expect_err("both fail");
        assert!(e.contains("first page") && e.contains("ssh setup"), "{e}");
    }

    #[test]
    fn a_save_commits_and_pushes_only_when_something_changed() {
        let tmp = tempfile::tempdir().expect("tmp");
        let fake = Arc::new(FakeGit::default());
        let w = wiki(fake.clone(), tmp.path());
        let done = w
            .save(&repo(), "Home", "# Hi\n", "Update Home")
            .expect("save");
        assert_eq!(done, "saved wiki page Home");
        let calls = fake.calls.lock().expect("lock").clone();
        let tail: Vec<_> = calls
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(String::as_str)
            .collect();
        assert!(tail[0].starts_with("add -- Home.md"), "{tail:?}");
        assert!(
            tail[1].contains("commit") && tail[1].contains("Update Home"),
            "{tail:?}"
        );
        assert_eq!(tail[2], "push --quiet");
        assert_eq!(
            std::fs::read_to_string(w.dir_of(&repo()).join("Home.md"))
                .as_deref()
                .ok(),
            Some("# Hi\n")
        );
        let before = fake.calls.lock().expect("lock").len();
        let same = w.save(&repo(), "Home", "# Hi\n", "again").expect("same");
        assert!(same.contains("no changes"), "{same}");
        let after = fake.calls.lock().expect("lock").clone();
        assert!(
            after[before..].iter().all(|c| c.starts_with("pull")),
            "only the sync ran: {after:?}"
        );
    }

    #[test]
    fn blocking_work_runs_off_the_caller() {
        let n = futures::executor::block_on(blocking(|| 40 + 2)).expect("ran");
        assert_eq!(n, 42);
    }
}
