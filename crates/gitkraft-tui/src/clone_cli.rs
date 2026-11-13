//! `gitkraft-tui clone SPEC [DIR] [--ssh|--https]`: clone with the system `git`, over the transport
//! you chose or `accounts.toml` says for the host.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use gitkraft_hub::accounts::AccountsFile;
use gitkraft_hub::clone::{ClonePlan, plan};
use gitkraft_platforms::Transport;
use gitkraft_settings::files;

/// A parsed `clone` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// `owner/name`, `host/owner/name` or a URL.
    pub spec: String,
    /// Folder to create.
    pub dest: Option<PathBuf>,
    /// Forced transport.
    pub transport: Option<Transport>,
}

/// Parses the arguments after `clone`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let mut spec = None;
    let mut dest = None;
    let mut transport = None;
    for a in args {
        match a.as_str() {
            "--ssh" => transport = Some(Transport::Ssh),
            "--https" => transport = Some(Transport::Https),
            o if o.starts_with("--") => return Err(format!("unknown option '{o}'")),
            _ if spec.is_none() => spec = Some(a.clone()),
            _ if dest.is_none() => dest = Some(PathBuf::from(a)),
            _ => return Err("clone takes a project and at most one folder".into()),
        }
    }
    Ok(Command {
        spec: spec.ok_or("usage: clone OWNER/NAME|HOST/OWNER/NAME|URL [DIR] [--ssh|--https]")?,
        dest,
        transport,
    })
}

fn accounts_file() -> AccountsFile {
    files::accounts_path()
        .ok()
        .and_then(|p| files::read_optional(&p).ok().flatten())
        .and_then(|t| AccountsFile::parse(&t).ok())
        .unwrap_or_default()
}

/// Clones; returns the line to print.
pub fn run(cmd: Command) -> Result<String> {
    let cwd = std::env::current_dir()?;
    let p =
        plan(&cmd.spec, cmd.transport, cmd.dest, &accounts_file(), &cwd).map_err(|e| anyhow!(e))?;
    clone_plan(&p)
}

fn clone_plan(p: &ClonePlan) -> Result<String> {
    gitkraft_core::features::remotes::transport::clone_repo(&p.url, &p.dest)?;
    Ok(format!(
        "cloned {} into {}\n",
        p.repo,
        Path::new(&p.dest).display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn the_spec_folder_and_transport_are_parsed() {
        assert_eq!(
            parse(&args(&["o/r", "--ssh", "here"])),
            Ok(Command {
                spec: "o/r".into(),
                dest: Some("here".into()),
                transport: Some(Transport::Ssh)
            })
        );
        assert!(parse(&args(&[])).expect_err("none").starts_with("usage"));
        assert!(parse(&args(&["a", "b", "c"])).is_err());
        assert!(
            parse(&args(&["a", "--zip"]))
                .expect_err("flag")
                .contains("--zip")
        );
    }

    #[test]
    fn cloning_reports_where_it_went() {
        let root = tempfile::tempdir().expect("tmp");
        let src = root.path().join("src");
        std::fs::create_dir_all(&src).expect("mk");
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .current_dir(&src)
                    .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                    .args(args)
                    .status()
                    .expect("git")
                    .success()
            );
        };
        git(&["init", "-q"]);
        git(&["commit", "-q", "--allow-empty", "-m", "x"]);
        let p = ClonePlan {
            repo: gitkraft_platforms::RepoId::new("h", "o", "r"),
            url: src.to_string_lossy().into_owned(),
            dest: root.path().join("out"),
        };
        let line = clone_plan(&p).expect("clone");
        assert!(
            line.starts_with("cloned h/o/r into ") && line.contains("out"),
            "{line}"
        );
    }
}
