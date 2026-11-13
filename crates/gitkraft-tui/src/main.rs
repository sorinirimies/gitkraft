use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    gitkraft_core::limit_git_memory();

    // `gitkraft-tui notify …` runs the notification poller and its controls without the UI.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("notify") {
        let cmd = gitkraft_tui::notify_cli::parse(&args[1..]).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
        print!("{}", gitkraft_tui::notify_cli::run(cmd)?);
        return Ok(());
    }

    if args.first().map(String::as_str) == Some("ssh") {
        let cmd = gitkraft_tui::ssh_cli::parse(&args[1..]).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
        match gitkraft_tui::ssh_cli::run(cmd) {
            Ok(text) => print!("{text}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let cli = gitkraft_tui::cli::parse_args(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!(
            "{e}\nusage: gitkraft-tui [REPO] [--theme NAME] [--list-themes] | gitkraft-tui notify … | gitkraft-tui ssh …"
        );
        std::process::exit(2);
    });

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
    let repo_path = cli.repo_path.map(PathBuf::from).or_else(|| {
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
