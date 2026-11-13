use clap::Parser;
use gitkraft_tui::forge_cli::{Cmd, Session};

/// Runs a command that prints and exits. A usage error exits 2, a failure 1.
fn run_command(cmd: Cmd) -> anyhow::Result<()> {
    fn usage(e: String) -> ! {
        eprintln!("{e}");
        std::process::exit(2);
    }
    fn print(r: anyhow::Result<String>) -> anyhow::Result<()> {
        match r {
            Ok(text) => {
                print!("{text}");
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                std::process::exit(1);
            }
        }
    }
    let with = |word: &str, args: Vec<String>| {
        std::iter::once(word.to_string())
            .chain(args)
            .collect::<Vec<_>>()
    };
    match cmd {
        Cmd::Notify { args } => {
            let c = gitkraft_tui::notify_cli::parse(&args).unwrap_or_else(|e| usage(e));
            print(gitkraft_tui::notify_cli::run(c))
        }
        Cmd::Login { args } => {
            let c =
                gitkraft_tui::auth_cli::parse(&with("login", args)).unwrap_or_else(|e| usage(e));
            print(gitkraft_tui::auth_cli::run(c))
        }
        Cmd::Logout { args } => {
            let c =
                gitkraft_tui::auth_cli::parse(&with("logout", args)).unwrap_or_else(|e| usage(e));
            print(gitkraft_tui::auth_cli::run(c))
        }
        Cmd::Clone { args } => {
            let c = gitkraft_tui::clone_cli::parse(&args).unwrap_or_else(|e| usage(e));
            print(gitkraft_tui::clone_cli::run(c))
        }
        Cmd::Ssh { args } => {
            let c = gitkraft_tui::ssh_cli::parse(&args).unwrap_or_else(|e| usage(e));
            print(gitkraft_tui::ssh_cli::run(c))
        }
        Cmd::Completions { shell } => {
            print!("{}", gitkraft_tui::forge_cli::completions(shell));
            Ok(())
        }
        Cmd::Forge(f) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            print(rt.block_on(async {
                let cwd = std::env::current_dir()?;
                Session::connect(cwd, f.repo_flag()).await?.run(&f).await
            }))
        }
    }
}

fn main() -> anyhow::Result<()> {
    gitkraft_core::limit_git_memory();

    let cli = gitkraft_tui::forge_cli::Cli::parse();
    if let Some(cmd) = cli.command {
        return run_command(cmd);
    }

    if cli.list_themes {
        for n in gitkraft_tui::cli::theme_names() {
            println!("{n}");
        }
        return Ok(());
    }
    if let Some(name) = &cli.theme {
        match gitkraft_tui::cli::resolve_theme(name) {
            Some(saved) => {
                let _ = gitkraft_core::features::persistence::save_theme_tui(saved);
            }
            None => {
                eprintln!("unknown theme '{name}' (try --list-themes)");
                std::process::exit(2);
            }
        }
    }

    // Try to pick up a repo path from the first CLI argument, or fall back to
    // the current working directory if it looks like a Git repository.
    let repo_path = cli.repo_path.or_else(|| {
        let cwd = std::env::current_dir().ok()?;
        // Check if cwd (or any ancestor) contains a .git dir
        if cwd.join(".git").exists() {
            Some(cwd)
        } else {
            None
        }
    });

    gitkraft_tui::run(repo_path)
}
