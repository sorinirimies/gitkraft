//! Settings from the command line and the environment. Pure: both are passed in.

/// Help text for `--help`.
pub const HELP: &str = "\
gitkraft-mcp - GitKraft as an MCP server: PRs, issues and CI of GitHub, GitLab, Gitea and Codeberg

Usage: gitkraft-mcp [OPTIONS]
  Speaks MCP on stdin/stdout. Tokens come from the same places as the rest of GitKraft:
  GITHUB_TOKEN / GITLAB_TOKEN / CODEBERG_TOKEN, `gitkraft-tui login`, `gh`, `glab`, git's
  credential helper. The git checkout it is started in sets the default repository.

Options (each also reads the environment variable in brackets):
      --write           Also offer the tools that change things: comment, review, merge,
                        labels, close, re-run, dispatch              [GITKRAFT_MCP_WRITE=1]
      --allow-force     With --write: allow a forced merge (admin bypass, queue jump)
                                                                     [GITKRAFT_MCP_FORCE=1]
      --repo <H/O/N>    Default repository, as host/owner/name (default: this checkout's
                        first remote)                                [GITKRAFT_MCP_REPO]
  -h, --help            Print help
  -V, --version         Print version

Without --write the server cannot change anything on any platform.";

/// What the server may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Policy {
    /// Offer the tools that change things.
    pub write: bool,
    /// Allow a forced merge. Only means something with `write`.
    pub allow_force: bool,
}

/// Parsed settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    /// What is allowed.
    pub policy: Policy,
    /// Default repository, `host/owner/name`.
    pub repo: Option<String>,
}

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cli {
    /// Print the help.
    Help,
    /// Print the version.
    Version,
    /// Serve.
    Run(Config),
}

fn truthy(v: Option<String>) -> bool {
    v.is_some_and(|v| {
        matches!(
            v.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

impl Cli {
    /// Parses `args` (without the program name); `env` reads an environment variable.
    ///
    /// # Errors
    /// An unknown option, or an option missing its value.
    pub fn parse(
        args: impl IntoIterator<Item = String>,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, String> {
        let mut cfg = Config {
            policy: Policy {
                write: truthy(env("GITKRAFT_MCP_WRITE")),
                allow_force: truthy(env("GITKRAFT_MCP_FORCE")),
            },
            repo: env("GITKRAFT_MCP_REPO").filter(|r| !r.trim().is_empty()),
        };
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            match a.as_str() {
                "-h" | "--help" => return Ok(Self::Help),
                "-V" | "--version" => return Ok(Self::Version),
                "--write" => cfg.policy.write = true,
                "--allow-force" => cfg.policy.allow_force = true,
                "--repo" => {
                    cfg.repo = Some(
                        args.next()
                            .ok_or("--repo needs a value like github.com/o/r")?,
                    );
                }
                other => return Err(format!("unknown argument '{other}'")),
            }
        }
        if cfg.policy.allow_force && !cfg.policy.write {
            return Err("--allow-force only means something together with --write".into());
        }
        Ok(Self::Run(cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str], env: &[(&str, &str)]) -> Result<Cli, String> {
        Cli::parse(args.iter().map(|s| s.to_string()), |k| {
            env.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn read_only_is_the_default() {
        assert_eq!(parse(&[], &[]), Ok(Cli::Run(Config::default())));
        assert!(!Policy::default().write);
    }

    #[test]
    fn flags_and_environment_turn_writes_on() {
        let Ok(Cli::Run(c)) = parse(
            &["--write", "--allow-force", "--repo", "github.com/o/r"],
            &[],
        ) else {
            panic!("run");
        };
        assert!(c.policy.write && c.policy.allow_force);
        assert_eq!(c.repo.as_deref(), Some("github.com/o/r"));
        let Ok(Cli::Run(c)) = parse(
            &[],
            &[("GITKRAFT_MCP_WRITE", "1"), ("GITKRAFT_MCP_REPO", "h/o/n")],
        ) else {
            panic!("run");
        };
        assert!(c.policy.write && !c.policy.allow_force);
        assert_eq!(c.repo.as_deref(), Some("h/o/n"));
    }

    #[test]
    fn force_without_write_and_bad_arguments_are_refused() {
        assert!(
            parse(&["--allow-force"], &[])
                .unwrap_err()
                .contains("--write")
        );
        assert!(parse(&["--nope"], &[]).unwrap_err().contains("unknown"));
        assert!(parse(&["--repo"], &[]).is_err());
        assert_eq!(parse(&["-h"], &[]), Ok(Cli::Help));
        assert_eq!(parse(&["--version"], &[]), Ok(Cli::Version));
    }
}
