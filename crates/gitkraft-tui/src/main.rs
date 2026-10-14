use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // Limit libgit2's memory-mapped window to prevent GB-scale RAM usage
    // on large repositories.
    unsafe {
        git2::opts::set_mwindow_size(64 * 1024 * 1024).ok();
        git2::opts::set_mwindow_mapped_limit(256 * 1024 * 1024).ok();
    }

    let cli = gitkraft_tui::cli::parse_args(std::env::args().skip(1)).unwrap_or_else(|e| {
        eprintln!("{e}\nusage: gitkraft-tui [REPO] [--theme NAME] [--list-themes]");
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
